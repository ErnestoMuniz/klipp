use std::path::PathBuf;
use std::time::Duration;

use crate::core::state::{OnlineSound, Sound};

const SEARCH_URL: &str = "https://myinstants-api.vercel.app/search";
const UA: &str = "klipp/0.1 (desktop soundboard)";

/// Busca sons no MyInstants via API pública não-oficial.
/// Retorna `(id, título, url do mp3)` — sem tocar em disco.
pub fn search(query: &str) -> anyhow::Result<Vec<OnlineSound>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(vec![]);
    }
    let resp = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build()
        .get(SEARCH_URL)
        .query("q", query)
        .set("User-Agent", UA)
        .call()
        .map_err(|err| anyhow::anyhow!("busca online: {err}"))?;
    let json: serde_json::Value = resp
        .into_json()
        .map_err(|err| anyhow::anyhow!("resposta inválida: {err}"))?;
    let items = json
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(items
        .iter()
        .filter_map(|item| {
            Some(OnlineSound {
                id: item.get("id")?.as_str()?.to_string(),
                title: item.get("title")?.as_str()?.to_string(),
                mp3: item.get("mp3")?.as_str()?.to_string(),
            })
        })
        .filter(|s| !s.title.is_empty() && s.mp3.starts_with("http"))
        .collect())
}

/// Baixa os bytes de um mp3 remoto (preview ou importação).
///
/// O CDN do MyInstants (Cloudflare) às vezes responde a clientes não-browser
/// com página de bloqueio (HTML) ou corpo truncado — que antes virava
/// "preview que toca uma fração de segundo e para" ou "nem chega a tocar".
/// Por isso valida Content-Type, Content-Length e assinatura dos bytes,
/// com uma segunda tentativa em conexão nova antes de desistir.
pub fn fetch_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let mut last_err = anyhow::anyhow!("download falhou");
    for _ in 0..2 {
        match fetch_once(url) {
            Ok(bytes) => return Ok(bytes),
            Err(err) => last_err = err,
        }
    }
    Err(last_err)
}

fn fetch_once(url: &str) -> anyhow::Result<Vec<u8>> {
    let resp = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(60))
        .build()
        .get(url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
        )
        .call()
        .map_err(|err| anyhow::anyhow!("download: {err}"))?;
    let content_type = resp.header("Content-Type").unwrap_or("").to_string();
    if !(content_type.starts_with("audio/") || content_type == "application/octet-stream") {
        anyhow::bail!("myinstants recusou o download (resposta não-áudio)");
    }
    let declared: Option<usize> = resp.header("Content-Length").and_then(|v| v.parse().ok());
    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|err| anyhow::anyhow!("leitura: {err}"))?;
    if let Some(total) = declared
        && bytes.len() < total
    {
        anyhow::bail!("download truncado ({} de {total} bytes)", bytes.len());
    }
    if !is_audio_bytes(&bytes) {
        anyhow::bail!("myinstants recusou o download (conteúdo inválido)");
    }
    Ok(bytes)
}

/// Assinatura mínima de contêiner de áudio (cobre mp3/wav/ogg/flac/m4a).
fn is_audio_bytes(bytes: &[u8]) -> bool {
    if bytes.len() < 16 {
        return false;
    }
    if bytes.starts_with(b"ID3") || bytes.starts_with(b"OggS") || bytes.starts_with(b"fLaC") {
        return true;
    }
    if bytes.starts_with(b"RIFF") && bytes[8..].starts_with(b"WAVE") {
        return true;
    }
    if bytes[4..].starts_with(b"ftyp") {
        return true;
    }
    // Sync de frame MPEG (mp3 sem tag ID3): 0xFF + 3 bits altos.
    if bytes[0] == 0xFF && bytes[1] & 0xE0 == 0xE0 {
        return true;
    }
    // ADTS/AAC: 0xFFFx.
    if bytes[0] == 0xFF && bytes[1] & 0xF6 == 0xF0 {
        return true;
    }
    false
}

/// Baixa um som online direto para a biblioteca local.
/// Retorna o nome do arquivo salvo.
pub fn download_to_library(title: &str, mp3_url: &str) -> anyhow::Result<String> {
    let bytes = fetch_bytes(mp3_url)?;
    let dir = super::sounds::sounds_dir();
    let _ = std::fs::create_dir_all(&dir);
    let ext = audio_ext(mp3_url);
    let file_name = format!("{}.{ext}", sanitize_filename(title));
    let dest = unique_dest(&dir, &file_name);
    std::fs::write(&dest, bytes)?;
    Ok(dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or(file_name))
}

/// Caminho temporário para o preview (toca sem importar).
pub fn preview_path(id: &str, mp3_url: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("klipp-preview");
    let _ = std::fs::create_dir_all(&dir);
    // Limpa previews antigos (melhor esforço, ignora o atual em uso).
    // Compara com o nome sanitizado: é esse o formato gravado em disco.
    let keep = sanitize_filename(id);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(&keep) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    dir.join(format!("{}.{}", sanitize_filename(id), audio_ext(mp3_url)))
}

fn audio_ext(url: &str) -> &'static str {
    let base = url.split('?').next().unwrap_or(url).to_lowercase();
    for ext in ["mp3", "wav", "ogg", "flac", "m4a", "opus", "aac"] {
        if base.ends_with(&format!(".{ext}")) {
            return match ext {
                "wav" => "wav",
                "ogg" => "ogg",
                "flac" => "flac",
                "m4a" => "m4a",
                "opus" => "opus",
                "aac" => "aac",
                _ => "mp3",
            };
        }
    }
    "mp3"
}

/// Encontra na biblioteca o arquivo correspondente a um título online
/// (o download salva como `sanitize(título).ext`, com " (n)" se colidir).
/// Retorna o nome do arquivo local, se existir.
pub fn library_match_name(sounds: &[Sound], title: &str) -> Option<String> {
    let want = sanitize_filename(title).to_lowercase();
    sounds.iter().find_map(|s| {
        let stem = s
            .name
            .rsplit_once('.')
            .map(|(base, _)| base)
            .unwrap_or(&s.name)
            .to_lowercase();
        (stem == want || strip_copy_suffix(&stem) == want).then(|| s.name.clone())
    })
}

/// Remove sufixo de colisão " (n)" do fim do nome (sem extensão).
fn strip_copy_suffix(stem: &str) -> &str {
    if let Some(base) = stem.strip_suffix(')')
        && let Some((head, num)) = base.rsplit_once(" (")
        && !num.is_empty()
        && num.chars().all(|c| c.is_ascii_digit())
    {
        return head;
    }
    stem
}
/// Nome de arquivo seguro a partir do título/id.
pub(crate) fn sanitize_filename(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim().trim_matches('.').trim();
    let cut: String = trimmed.chars().take(80).collect();
    if cut.is_empty() { "sound".to_string() } else { cut }
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

#[cfg(test)]
mod tests {
    use super::{is_audio_bytes, library_match_name};
    use crate::core::state::Sound;

    fn sound(name: &str) -> Sound {
        Sound {
            name: name.to_string(),
            path: std::path::PathBuf::from(name),
            duration_secs: None,
            modified_secs: 0,
            display: None,
            emoji: "♪".to_string(),
        }
    }

    #[test]
    fn matches_downloaded_titles() {
        let lib = vec![
            sound("teste dinheiro.mp3"),
            sound("Quer tomar bomba pode aplicar (2).mp3"),
            sound("outro.wav"),
        ];
        // Exato (como o download salva).
        assert_eq!(
            library_match_name(&lib, "teste dinheiro"),
            Some("teste dinheiro.mp3".to_string())
        );
        // Case-insensitive + sufixo de colisão " (n)".
        assert_eq!(
            library_match_name(&lib, "quer TOMAR bomba PODE aplicar"),
            Some("Quer tomar bomba pode aplicar (2).mp3".to_string())
        );
        // Extensão ignorada.
        assert_eq!(
            library_match_name(&lib, "outro"),
            Some("outro.wav".to_string())
        );
        // Ausente.
        assert_eq!(library_match_name(&lib, "nunca baixado"), None);
        // Título com caracteres invalidados no arquivo.
        let lib2 = vec![sound("a_b_c.mp3")];
        assert_eq!(
            library_match_name(&lib2, "a/b:c"),
            Some("a_b_c.mp3".to_string())
        );
    }

    #[test]
    fn rejects_cloudflare_block_page() {
        // Página "Attention Required!" que o Cloudflare serve no lugar do mp3.
        let html = b"<!DOCTYPE html>\n<html class=\"no-js\" lang=\"en-US\">";
        assert!(!is_audio_bytes(html));
        assert!(!is_audio_bytes(b""));
        assert!(!is_audio_bytes(b"ID3"));
    }

    #[test]
    fn accepts_common_containers() {        assert!(is_audio_bytes(b"ID3\x04\x00\x00\x00\x00\x00\x01\x02\x03\x04\x05\x06\x07"));
        // MP3 sem ID3 (sync de frame).
        assert!(is_audio_bytes(&[0xFF, 0xFB, 0x90, 0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2]));
        // WAV, OGG, FLAC, M4A.
        assert!(is_audio_bytes(b"RIFF\x24\x00\x00\x00WAVEfmt \x10\x00"));
        assert!(is_audio_bytes(b"OggS\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"));
        assert!(is_audio_bytes(b"fLaC\x00\x00\x00\x22\x10\x00\x10\x00\x00\x00\x00\x00\x00\x00"));
        assert!(is_audio_bytes(b"\x00\x00\x00\x20ftypM4A \x00\x00\x00\x00"));
    }
}
