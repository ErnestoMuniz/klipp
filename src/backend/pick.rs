use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::backend::sounds;
use crate::core::i18n::{t, t_fmt};
use crate::core::state::Shared;

/// Abre o seletor de arquivos do portal, copia os escolhidos para a
/// biblioteca e atualiza o estado. Roda em background (portal bloqueia).
pub async fn pick_and_import(shared: Arc<Mutex<Shared>>) -> anyhow::Result<()> {
    use ashpd::desktop::file_chooser::{FileFilter, OpenFileRequest};

    let filter = FileFilter::new("Audio")
        .glob("*.mp3")
        .glob("*.wav")
        .glob("*.ogg")
        .glob("*.flac")
        .glob("*.m4a")
        .glob("*.opus")
        .glob("*.aac");
    let lang = shared.lock().unwrap().lang.clone();
    let title = t(&lang, "pick.title");
    let req = match OpenFileRequest::default()
        .title(title.as_str())
        .multiple(true)
        .filter(filter)
        .send()
        .await
    {
        Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => {
            return Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            let mut s = shared.lock().unwrap();
            let lang = s.lang.clone();
            s.last_error = Some(if msg.contains("An app id is required") {
                t(&lang, "err.pick_portal")
            } else {
                t_fmt(&lang, "err.pick", &[("msg", &msg)])
            });
            s.bump();
            return Ok(());
        }
        Ok(req) => req,
    };
    let files = req.response()?;
    let paths: Vec<PathBuf> = files.uris().iter().filter_map(uri_to_path).collect();
    if paths.is_empty() {
        return Ok(());
    }
    let n = sounds::import_paths(&paths);
    log::info!("{n} sons importados via seletor");
    refresh_library(&shared);
    Ok(())
}

/// Re-lê a biblioteca, sonda durações e marca re-render.
pub fn refresh_library(shared: &Arc<Mutex<Shared>>) {
    let mut s = shared.lock().unwrap();
    s.sounds = sounds::list_sounds();
    s.bump();
    drop(s);
    sounds::probe_missing_durations(shared.clone());
}

fn uri_to_path(uri: &ashpd::Uri) -> Option<PathBuf> {
    let s = uri.as_str();
    let stripped = s.strip_prefix("file://")?;
    // Remove host vazio ("file:///..." -> "/...").
    let path = stripped.strip_prefix("localhost").unwrap_or(stripped);
    Some(PathBuf::from(percent_decode(path)))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push(hi << 4 | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
