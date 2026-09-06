use std::ffi::{c_char, c_void, CString};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use libpulse_simple_sys as pss;
use libpulse_sys as pulse;

use super::audio_graph;
use super::decode::decode;
use crate::core::i18n::t_fmt;
use crate::core::state::{Shared, WAVEFORM_BARS};

pub struct Engine {
    inner: Arc<Mutex<EngineInner>>,
}

struct EngineInner {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Época do último play: threads obsoletas (decode lento ou write
    /// bloqueado) nunca tocam a UI de um play mais novo.
    seq: u64,
}

impl Engine {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::new(Mutex::new(EngineInner {
                stop: Arc::new(AtomicBool::new(false)),
                thread: None,
                seq: 0,
            })),
        })
    }

    /// Para o playback atual sem bloquear: o join da thread antiga
    /// acontece numa thread reaper. Seguro para chamar da UI thread
    /// (cliques rápidos não congelam mais o app).
    pub fn stop(&self) {
        self.stop_previous();
    }

    /// Pausa mantendo o índice: a thread dorme sem escrever no sink.
    /// Congela o elapsed no offset para a UI não andar pausado.
    pub fn pause(&self, shared: &Arc<Mutex<Shared>>) {
        let Ok(mut s) = shared.lock() else { return };
        if s.playing.is_some() && !s.play_paused {
            let wall = now_ms().saturating_sub(s.play_start_ms) as f32 / 1000.0;
            if let Some(d) = s.play_duration_secs.filter(|d| *d > 0.0) {
                s.play_offset_secs = (s.play_offset_secs + wall).clamp(0.0, d);
            }
            s.play_paused = true;
            s.seek_request = None;
            s.bump();
        }
    }

    /// Retoma de onde pausou (o offset congelado vira a nova origem).
    pub fn resume(&self, shared: &Arc<Mutex<Shared>>) {
        let Ok(mut s) = shared.lock() else { return };
        if s.playing.is_some() && s.play_paused {
            s.play_start_ms = now_ms();
            s.play_paused = false;
            s.bump();
        }
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
        // Nova época publicada de forma síncrona: qualquer thread antiga
        // que acordar depois (decode lento, write bloqueado) se reconhece
        // obsoleta e não toca a UI nem o áudio do play atual.
        let seq = {
            let mut inner = self.inner.lock().unwrap();
            inner.stop = stop;
            inner.seq += 1;
            inner.seq
        };
        set_shared(&shared, |s| {
            s.play_seq = seq;
        });
        let mut inner = self.inner.lock().unwrap();
        let handle = std::thread::Builder::new()
            .name("klipp-playback".into())
            .spawn(move || {
                let decoded = match decode(&path) {
                    Ok(d) => d,
                    Err(err) => {
                        log::error!("decode falhou ({name}): {err}");
                        set_shared(&shared, |s| {
                            // Erro de play superado: o dono atual mostra o dele.
                            if s.play_seq == seq {
                                s.last_error = Some(format!("{name}: {err}"));
                                s.bump();
                            }
                        });
                        return;
                    }
                };
                log::info!(
                    "decode ok ({name}): {} amostras @ {}Hz",
                    decoded.samples.len(),
                    decoded.rate
                );

                let peaks = decoded.peaks(WAVEFORM_BARS);
                let duration_secs = decoded.duration_secs();
                let claimed = shared
                    .lock()
                    .map(|mut s| claim_ui(&mut s, seq, &name, duration_secs, peaks, now_ms()))
                    .unwrap_or(false);
                if !claimed {
                    log::info!("play superado antes do início ({name}): descartado");
                    return;
                }

                let frames = to_stereo(&decoded.samples);
                let duration = duration_secs.max(0.001);
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
                        // Pausado: dorme sem escrever (seek continua valendo).
                        let mut idx = 0usize;
                        while idx < frames.len() {
                            if stop_task.load(Ordering::SeqCst) {
                                break;
                            }
                            let (seek_to, paused, vol) = shared
                                .lock()
                                .map(|mut s| {
                                    let seek = s.seek_request.take();
                                    let paused = s.play_paused;
                                    let vol = if s.muted {
                                        0.0
                                    } else {
                                        s.volume.clamp(0.0, 1.0)
                                    };
                                    (seek, paused, vol)
                                })
                                .unwrap_or((None, false, 1.0));
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
                            if paused {
                                std::thread::sleep(std::time::Duration::from_millis(20));
                                continue;
                            }
                            let end = (idx + 512).min(frames.len());
                            let chunk = &frames[idx..end];
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
                            if s.play_seq != seq {
                                return;
                            }
                            let lang = s.lang.clone();
                            let msg = err.to_string();
                            s.last_error =
                                Some(t_fmt(&lang, "err.sink", &[("msg", &msg)]));
                            s.bump();
                        });
                    }
                }

                set_shared(&shared, |s| {
                    clear_ui_if_current(s, seq);
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

/// Assume a UI do playback (`playing`, duração, picos…), mas só se `seq`
/// ainda for a época atual. Trocar de som no meio do decode do anterior:
/// o thread obsoleto descarta tudo (sem áudio fantasma nem UI trocada).
/// Retorna se assumiu.
fn claim_ui(
    s: &mut Shared,
    seq: u64,
    name: &str,
    duration_secs: f32,
    peaks: Vec<f32>,
    now_ms: u128,
) -> bool {
    if s.play_seq != seq {
        return false;
    }
    s.playing = Some(name.to_string());
    s.play_duration_secs = Some(duration_secs);
    s.play_offset_secs = 0.0;
    s.play_start_ms = now_ms;
    s.seek_request = None;
    s.play_paused = false;
    s.play_peaks = peaks;
    s.bump();
    true
}

/// Limpa a UI no fim do playback, mas só se `seq` ainda for o dono.
/// Sem isso, o thread antigo acordando tarde apagava o `playing` do som
/// novo (áudio tocando sem efeitos na UI).
/// Retorna se limpou.
fn clear_ui_if_current(s: &mut Shared, seq: u64) -> bool {
    if s.play_seq != seq {
        return false;
    }
    s.playing = None;
    s.play_duration_secs = None;
    s.play_offset_secs = 0.0;
    s.seek_request = None;
    s.play_paused = false;
    s.play_peaks.clear();
    s.bump();
    true
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

#[cfg(test)]
mod tests {
    use super::{clear_ui_if_current, claim_ui, Engine};
    use crate::core::settings::Settings;
    use crate::core::state::Shared;

    fn shared() -> std::sync::Arc<std::sync::Mutex<Shared>> {
        std::sync::Arc::new(std::sync::Mutex::new(Shared::new(&Settings::default())))
    }

    #[test]
    fn claim_so_da_epoca_atual() {
        let shared = shared();
        let mut s = shared.lock().unwrap();
        s.play_seq = 2;
        // Obsoleto não assume nem mexe.
        assert!(!claim_ui(&mut s, 1, "a", 5.0, vec![1.0], 1000));
        assert_eq!(s.playing, None);
        // Atual assume.
        assert!(claim_ui(&mut s, 2, "b", 5.0, vec![1.0], 1000));
        assert_eq!(s.playing.as_deref(), Some("b"));
        assert_eq!(s.play_peaks, vec![1.0]);
    }

    #[test]
    fn clear_obsoleto_preserva_o_novo() {
        let shared = shared();
        let mut s = shared.lock().unwrap();
        // Dono atual: som novo.
        s.play_seq = 2;
        s.playing = Some("novo".into());
        s.play_peaks = vec![0.5];
        // Thread antiga acordando tarde: não apaga nada.
        assert!(!clear_ui_if_current(&mut s, 1));
        assert_eq!(s.playing.as_deref(), Some("novo"));
        assert_eq!(s.play_peaks, vec![0.5]);
        // Dono encerrando: limpa.
        assert!(clear_ui_if_current(&mut s, 2));
        assert_eq!(s.playing, None);
        assert!(s.play_peaks.is_empty());
    }

    #[test]
    fn play_publica_epoca_e_erro_tardio_nao_vence() {
        let engine = Engine::new();
        let shared = shared();
        // Dois plays de arquivos inexistentes: decode falha rápido.
        engine.play("a.mp3".into(), "a".into(), shared.clone());
        engine.play("b.mp3".into(), "b".into(), shared.clone());
        // A época síncrona já é a do último play.
        assert_eq!(shared.lock().unwrap().play_seq, 2);
        // O erro final é sempre o do dono: ignora o "a:" transitório caso
        // o primeiro thread reporte antes do segundo existir.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if let Some(err) = shared.lock().unwrap().last_error.clone()
                && err.starts_with("b:")
            {
                break;
            }
            assert!(std::time::Instant::now() < deadline, "dono não reportou");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
