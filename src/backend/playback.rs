use std::ffi::{c_char, c_void, CString};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use libpulse_simple_sys as pss;
use libpulse_sys as pulse;

use super::audio_graph;
use super::decode::decode;
use crate::core::state::{Shared, WAVEFORM_BARS};

pub struct Engine {
    inner: Arc<Mutex<EngineInner>>,
}

struct EngineInner {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Engine {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::new(Mutex::new(EngineInner {
                stop: Arc::new(AtomicBool::new(false)),
                thread: None,
            })),
        })
    }

    /// Para o playback atual sem bloquear: o join da thread antiga
    /// acontece numa thread reaper. Seguro para chamar da UI thread
    /// (cliques rápidos não congelam mais o app).
    pub fn stop(&self) {
        self.stop_previous();
    }

    /// Join bloqueante, só para o desligamento do app.
    pub fn shutdown(&self) {
        let old = {
            let mut inner = self.inner.lock().unwrap();
            inner.stop.store(true, Ordering::SeqCst);
            inner.thread.take()
        };
        if let Some(thread) = old {
            let _ = thread.join();
        }
    }

    fn stop_previous(&self) {
        let old = {
            let mut inner = self.inner.lock().unwrap();
            inner.stop.store(true, Ordering::SeqCst);
            inner.thread.take()
        };
        if let Some(handle) = old {
            std::thread::spawn(move || {
                let _ = handle.join();
            });
        }
    }

    pub fn play(&self, path: PathBuf, name: String, shared: Arc<Mutex<Shared>>) {
        self.stop_previous();

        let stop = Arc::new(AtomicBool::new(false));
        let stop_task = stop.clone();
        let mut inner = self.inner.lock().unwrap();
        inner.stop = stop;
        let handle = std::thread::Builder::new()
            .name("klipp-playback".into())
            .spawn(move || {
                let decoded = match decode(&path) {
                    Ok(d) => d,
                    Err(err) => {
                        log::error!("decode falhou ({name}): {err}");
                        set_shared(&shared, |s| {
                            s.last_error = Some(format!("{name}: {err}"));
                            s.bump();
                        });
                        return;
                    }
                };
                log::info!(
                    "decode ok ({name}): {} amostras @ {}Hz",
                    decoded.samples.len(),
                    decoded.rate
                );

                set_shared(&shared, |s| {
                    s.playing = Some(name.clone());
                    s.play_duration_secs = Some(decoded.duration_secs());
                    s.play_offset_secs = 0.0;
                    s.play_start_ms = now_ms();
                    s.seek_request = None;
                    s.play_peaks = decoded.peaks(WAVEFORM_BARS);
                    s.bump();
                });

                let frames = to_stereo(&decoded.samples);
                let duration = decoded.duration_secs().max(0.001);
                let rate = decoded.rate.max(1) as f32;
                log::info!("conectando ao sink {}", audio_graph::CLIPS_SINK);
                match PulseWriter::new(
                    "klipp",
                    "Klipp",
                    Some(audio_graph::CLIPS_SINK),
                    decoded.rate,
                ) {
                    Ok(writer) => {
                        log::info!("tocando...");
                        // Frames f32 interleaved -> bytes LE (FLOAT32LE é little-endian).
                        // Ganho lido por chunk para o slider de volume valer no meio do play.
                        // Seek: `seek_request` reposiciona o índice (amostras já
                        // decodificadas em memória, pulo imediato sem re-decode).
                        let mut idx = 0usize;
                        while idx < frames.len() {
                            if stop_task.load(Ordering::SeqCst) {
                                break;
                            }
                            let seek_to: Option<f32> =
                                shared.lock().map(|mut s| s.seek_request.take()).unwrap_or(None);
                            if let Some(target) = seek_to {
                                let clamped = target.clamp(0.0, duration);
                                idx = ((clamped * rate) as usize).min(frames.len());
                                set_shared(&shared, |s| {
                                    s.play_offset_secs = clamped;
                                    s.play_start_ms = now_ms();
                                    s.bump();
                                });
                                continue;
                            }
                            let end = (idx + 512).min(frames.len());
                            let chunk = &frames[idx..end];
                            let vol = shared
                                .lock()
                                .map(|s| {
                                    if s.muted {
                                        0.0
                                    } else {
                                        s.volume.clamp(0.0, 1.0)
                                    }
                                })
                                .unwrap_or(1.0);
                            let mut bytes = Vec::with_capacity(chunk.len() * 8);
                            for frame in chunk {
                                bytes.extend_from_slice(&(frame[0] * vol).to_le_bytes());
                                bytes.extend_from_slice(&(frame[1] * vol).to_le_bytes());
                            }
                            if let Err(err) = writer.write(&bytes) {
                                log::warn!("escrita no sink falhou: {err}");
                                break;
                            }
                            idx = end;
                        }
                    }
                    Err(err) => {
                        log::error!("não foi possível conectar ao sink: {err}");
                        set_shared(&shared, |s| {
                            s.last_error = Some(format!("falha ao conectar no sink: {err}"));
                            s.bump();
                        });
                    }
                }

                set_shared(&shared, |s| {
                    s.playing = None;
                    s.play_duration_secs = None;
                    s.play_offset_secs = 0.0;
                    s.seek_request = None;
                    s.play_peaks.clear();
                    s.bump();
                });
            })
            .expect("spawn playback thread");
        inner.thread = Some(handle);
    }
}

fn set_shared(shared: &Arc<Mutex<Shared>>, f: impl FnOnce(&mut Shared)) {
    if let Ok(mut s) = shared.lock() {
        f(&mut s);
    }
}

fn now_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn to_stereo(samples: &[f32]) -> Vec<[f32; 2]> {
    let mut out = Vec::with_capacity(samples.len() / 2 + 1);
    let mut i = 0;
    while i + 1 < samples.len() {
        out.push([samples[i], samples[i + 1]]);
        i += 2;
    }
    if i < samples.len() {
        out.push([samples[i], samples[i]]);
    }
    out
}

/// Wrapper mínimo sobre pa_simple com CString corretos (o crate pulse-simple
/// passa `&str` sem NUL terminator, corrompendo o nome do sink).
struct PulseWriter {
    ptr: *mut pss::pa_simple,
}

// SAFETY: usado apenas na thread de playback, de forma síncrona.
unsafe impl Send for PulseWriter {}

impl PulseWriter {
    fn new(name: &str, desc: &str, device: Option<&str>, rate: u32) -> anyhow::Result<Self> {
        let name_c = CString::new(name)?;
        let desc_c = CString::new(desc)?;
        let dev_c = device.map(CString::new).transpose()?;
        let spec = pulse::sample::pa_sample_spec {
            format: pulse::sample::PA_SAMPLE_FLOAT32LE,
            rate,
            channels: 2,
        };
        let ptr = unsafe {
            pss::pa_simple_new(
                std::ptr::null(),
                name_c.as_ptr() as *const c_char,
                pulse::stream::PA_STREAM_PLAYBACK,
                dev_c.as_ref().map_or(std::ptr::null(), |d| d.as_ptr()),
                desc_c.as_ptr() as *const c_char,
                &spec,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        if ptr.is_null() {
            return Err(anyhow::anyhow!("servidor recusou a conexão"));
        }
        Ok(Self { ptr })
    }

    fn write(&self, bytes: &[u8]) -> anyhow::Result<()> {
        let rc = unsafe {
            pss::pa_simple_write(
                self.ptr,
                bytes.as_ptr() as *const c_void,
                bytes.len(),
                std::ptr::null_mut(),
            )
        };
        if rc < 0 {
            return Err(anyhow::anyhow!("pa_simple_write falhou ({rc})"));
        }
        Ok(())
    }
}

impl Drop for PulseWriter {
    fn drop(&mut self) {
        unsafe { pss::pa_simple_free(self.ptr) };
    }
}
