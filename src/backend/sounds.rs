use std::path::PathBuf;

use super::decode::decode;
use crate::core::state::{Shared, Sound};

/// Cache de durações por (nome, mtime): um rescan não perde o que já foi
/// sondado (evita o "…" piscando na biblioteca) e falhas de decode não são
/// re-sondadas a cada varredura. `None` = tentado e falhou.
static DUR_CACHE: std::sync::OnceLock<std::sync::Mutex<DurCache>> = std::sync::OnceLock::new();
/// Sonda em voo único: pedidos sobrepostos voltam, o loop drena o resto.
static PROBE_RUNNING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

type DurCache = std::collections::HashMap<(String, u64), Option<f32>>;

fn dur_cache() -> &'static std::sync::Mutex<DurCache> {
    DUR_CACHE.get_or_init(|| std::sync::Mutex::new(DurCache::new()))
}

const AUDIO_EXTS: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a", "opus", "aac"];

pub fn is_audio_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn sounds_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("klipp")
        .join("sounds")
}

/// Emoji padrão de um som sem emoji escolhido (igual ao app Electron).
pub const DEFAULT_EMOJI: &str = "♪";

/// Metadata editável por som (dialog "Edit sound"), chaveada pelo nome do
/// arquivo — estável mesmo se o display mudar (igual ao app Electron).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct StoredMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    emoji: Option<String>,
}

fn metadata_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("klipp")
        .join("sound-metadata.json")
}

fn load_metadata() -> std::collections::HashMap<String, StoredMeta> {
    std::fs::read_to_string(metadata_path())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Persiste display + emoji de um som (nome de arquivo como chave).
/// Strings vazias limpam o campo (volta ao padrão).
pub fn save_metadata(name: &str, display_name: &str, emoji: &str) {
    let mut all = load_metadata();
    let display_name = display_name.trim();
    let emoji = emoji.trim();
    let entry = all.entry(name.to_string()).or_default();
    entry.display_name = (!display_name.is_empty()).then(|| display_name.to_string());
    entry.emoji = (!emoji.is_empty()).then(|| emoji.to_string());
    if entry.display_name.is_none() && entry.emoji.is_none() {
        all.remove(name);
    }
    let path = metadata_path();
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
    if let Ok(raw) = serde_json::to_string_pretty(&all) {
        let _ = std::fs::write(path, raw);
    }
}

/// Resolve display + emoji guardados para um nome de arquivo.
fn resolve_meta(meta: &std::collections::HashMap<String, StoredMeta>, name: &str) -> (Option<String>, String) {
    match meta.get(name) {
        Some(m) => (
            m.display_name
                .as_ref()
                .map(|d| d.trim().to_string())
                .filter(|d| !d.is_empty()),
            m.emoji
                .as_ref()
                .map(|e| e.trim().to_string())
                .filter(|e| !e.is_empty())
                .unwrap_or_else(|| DEFAULT_EMOJI.to_string()),
        ),
        None => (None, DEFAULT_EMOJI.to_string()),
    }
}

pub fn list_sounds() -> Vec<Sound> {
    let dir = sounds_dir();
    let _ = std::fs::create_dir_all(&dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    let mut sounds: Vec<Sound> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|entry| {
            is_audio_file(&entry.path())
        })
        .map(|entry| {
            let path = entry.path();
            let modified_secs = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Sound {
                name: entry.file_name().to_string_lossy().to_string(),
                path,
                duration_secs: None,
                modified_secs,
                display: None,
                emoji: DEFAULT_EMOJI.to_string(),
            }
        })
        .collect();
    // Reaproveita durações já sondadas (o rescan não faz o "…" piscar).
    if !sounds.is_empty() {
        let cache = dur_cache().lock().unwrap();
        for sound in sounds.iter_mut() {
            if let Some(known) = cache
                .get(&(sound.name.clone(), sound.modified_secs))
                .copied()
                .flatten()
            {
                sound.duration_secs = Some(known);
            }
        }
    }
    // Metadata editável (display + emoji do dialog "Edit sound").
    if !sounds.is_empty() {
        let meta = load_metadata();
        for sound in sounds.iter_mut() {
            let (display, emoji) = resolve_meta(&meta, &sound.name);
            sound.display = display;
            sound.emoji = emoji;
        }
    }
    sounds.sort_by(|a, b| a.name.cmp(&b.name));
    sounds
}

/// Preenche `duration_secs` dos sons que ainda não têm, numa thread
/// de background. A UI repinta via `version` (poller do frontend).
/// Voo único com drenagem em loop; só dá `bump` em sucesso novo
/// (falha não repinta nem é re-sondada à toa, via `DUR_CACHE`).
pub fn probe_missing_durations(shared: std::sync::Arc<std::sync::Mutex<Shared>>) {
    use std::sync::atomic::Ordering;
    if PROBE_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("klipp-probe".into())
        .spawn(move || {
            loop {
                let pending: Vec<(String, PathBuf, (String, u64))> = shared
                    .lock()
                    .map(|s| {
                        let cache = dur_cache().lock().unwrap();
                        s.sounds
                            .iter()
                            .filter(|snd| {
                                snd.duration_secs.is_none()
                                    && !cache
                                        .contains_key(&(snd.name.clone(), snd.modified_secs))
                            })
                            .map(|snd| {
                                (
                                    snd.name.clone(),
                                    snd.path.clone(),
                                    (snd.name.clone(), snd.modified_secs),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                if pending.is_empty() {
                    break;
                }
                for (name, path, key) in pending {
                    let duration = decode(&path).ok().map(|d| d.duration_secs());
                    let mut s = match shared.lock() {
                        Ok(s) => s,
                        Err(_) => break,
                    };
                    // Ordem de locks: shared -> cache (igual ao snapshot).
                    dur_cache().lock().unwrap().insert(key, duration);
                    if let Some(sound) = s.sounds.iter_mut().find(|snd| snd.name == name)
                        && sound.duration_secs.is_none()
                        && duration.is_some()
                    {
                        sound.duration_secs = duration;
                        s.bump();
                    }
                }
            }
            PROBE_RUNNING.store(false, Ordering::SeqCst);
        });
    if spawned.is_err() {
        PROBE_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Copia arquivos de áudio para a pasta da biblioteca, sem sobrescrever
/// (acrescenta " (n)" se o nome colidir). Retorna quantos foram importados.
pub fn import_paths(paths: &[std::path::PathBuf]) -> usize {
    let dir = sounds_dir();
    let _ = std::fs::create_dir_all(&dir);
    let mut count = 0;
    for src in paths {
        if !src.is_file() || !is_audio_file(src) {
            continue;
        }
        let file_name = match src.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let dest = unique_dest(&dir, &file_name);
        if std::fs::copy(src, &dest).is_ok() {
            count += 1;
        }
    }
    count
}

fn unique_dest(dir: &std::path::Path, file_name: &str) -> PathBuf {
    let (base, ext) = file_name
        .rsplit_once('.')
        .map(|(b, e)| (b.to_string(), format!(".{e}")))
        .unwrap_or_else(|| (file_name.to_string(), String::new()));
    let mut candidate = dir.join(file_name);
    let mut n = 2u32;
    while candidate.exists() {
        candidate = dir.join(format!("{base} ({n}){ext}"));
        n += 1;
    }
    candidate
}

/// Apaga um som do disco. Retorna `true` se removeu.
pub fn delete_sound(name: &str) -> bool {
    let path = sounds_dir().join(name);
    std::fs::remove_file(path).is_ok()
}

/// Observa a pasta de sons (inotify) e atualiza o [`Shared`] a cada mudança
/// externa (criar/apagar/renomear pelo gerenciador de arquivos).
/// Debounce de 400ms coalesce rajadas (salvar = vários eventos).
/// Roda para sempre na thread chamadora; só retorna em erro fatal.
pub fn watch_loop(
    shared: std::sync::Arc<std::sync::Mutex<Shared>>,
    engine: std::sync::Arc<super::playback::Engine>,
) -> anyhow::Result<()> {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

    let dir = sounds_dir();
    std::fs::create_dir_all(&dir)?;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher =
        RecommendedWatcher::new(move |res| {
            let _ = tx.send(res);
        }, Config::default())?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;

    loop {
        // Espera bloqueado pelo primeiro evento; erros de `recv` = fim.
        let Ok(first) = rx.recv() else {
            return Ok(());
        };
        if let Err(err) = first {
            log::warn!("fs watch: {err:?}");
        }
        // Debounce: junta a rajada e re-escaneia uma vez.
        std::thread::sleep(std::time::Duration::from_millis(400));
        while rx.try_recv().is_ok() {}

        let mut s = shared.lock().unwrap();
        // Sumiu da biblioteca o que tocava? Para o playback.
        // Só vale para som da biblioteca: preview online tem título que nunca
        // está em `sounds` e não pode ser morto por um rescan qualquer.
        let playing_was_local = match s.playing.clone() {
            Some(name) => s.sounds.iter().any(|snd| snd.name == name),
            None => false,
        };
        s.sounds = list_sounds();
        if let Some(playing) = s.playing.clone()
            && playing_was_local
            && !s.sounds.iter().any(|snd| snd.name == playing)
        {
            engine.stop();
            s.playing = None;
        }
        s.bump();
        drop(s);
        probe_missing_durations(shared.clone());
    }
}
