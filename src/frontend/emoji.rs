use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct EmojiOpt {
    #[serde(rename = "e")]
    pub emoji: String,
    #[serde(rename = "n")]
    pub name: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct EmojiSub {
    pub label: String,
    pub emojis: Vec<EmojiOpt>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub(crate) struct EmojiGroup {
    pub label: String,
    pub subgroups: Vec<EmojiSub>,
}

pub(crate) struct EmojiData {
    pub groups: Vec<EmojiGroup>,
    /// Índice CLDR pt-BR: emoji -> "nome + palavras-chave" (já minúsculo).
    pub search_pt: HashMap<String, String>,
}

pub(crate) fn data() -> &'static EmojiData {
    static DATA: OnceLock<EmojiData> = OnceLock::new();
    DATA.get_or_init(|| {
        let groups: Vec<EmojiGroup> =
            serde_json::from_str(include_str!("../../assets/emoji-groups.json"))
                .unwrap_or_default();
        let search_pt: HashMap<String, String> =
            serde_json::from_str(include_str!("../../assets/emoji-search-pt.json"))
                .unwrap_or_default();
        EmojiData { groups, search_pt }
    })
}

/// Busca textual: pt-BR usa o índice CLDR, o resto usa o nome em inglês.
/// Retorna (emoji, nome em inglês).
pub(crate) fn search_emoji(lang: &str, query: &str) -> Vec<(String, String)> {    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return vec![];
    }
    let data = data();
    let pt = crate::core::i18n::resolve(lang) == "pt-BR";
    let mut out = Vec::new();
    for group in &data.groups {
        for sub in &group.subgroups {
            for opt in &sub.emojis {
                let hay = if pt {
                    data.search_pt
                        .get(&opt.emoji)
                        .map(|h| h.as_str())
                        .unwrap_or(&opt.name)
                } else {
                    &opt.name
                };
                if hay.to_lowercase().contains(&q) {
                    out.push((opt.emoji.clone(), opt.name.clone()));
                }
            }
        }
    }
    out
}

/// Fonte para textos de emoji: família colorida primeiro, com fallback para
/// a pilha de UI (cobre "♪" e emojis novos ausentes na empacotada).
#[allow(dead_code)]
pub(crate) fn font() -> open_gpui::Font {
    let mut f = open_gpui::font("Noto Color Emoji");
    f.fallbacks = Some(open_gpui::FontFallbacks::from_fonts(vec![
        "IBM Plex Sans".to_string(),
        "Ubuntu".to_string(),
        "Cantarell".to_string(),
        "Noto Sans".to_string(),
        "DejaVu Sans".to_string(),
        "Arial".to_string(),
    ]));
    f
}

/// Tamanho do raster (px). 56 cobre telas 2x para botões de ~28px;
/// o strike nativo da fonte é 109px (mais rápido), mas 56px usa ~4x
/// menos RAM — e a aba maior (People, ~2400 emojis) precisa caber no cache.
const RASTER_PX: f32 = 56.0;
/// Teto do cache (~12KB por emoji em 56px RGBA). Precisa cobrir a maior
/// aba inteira: se o cache despeja enquanto o worker ainda preenche,
/// os mesmos emojis entram em loop de re-raster (cada conclusão dá bump)
/// e a grade pisca sem parar. Ver teste `cache_cobre_maior_aba` abaixo.
const CACHE_CAP: usize = 2600;
/// Workers de raster em background.
const WORKERS: usize = 3;

struct RasterState {
    font_system: cosmic_text::FontSystem,
    bytes: Arc<Vec<u8>>,
    scale: swash::scale::ScaleContext,
}

struct JobQueue {
    queue: std::collections::VecDeque<String>,
    /// Na fila, em raster ou já no cache: evita duplicar trabalho.
    seen: std::collections::HashSet<String>,
    /// Sem glifo rasterizável (ex.: "♪"): nunca reenfileira.
    failed: std::collections::HashSet<String>,
    /// Ordem de inserção para despejo além do teto.
    order: std::collections::VecDeque<String>,
    cache: HashMap<String, Arc<open_gpui::RenderImage>>,
}

static JOBS: OnceLock<Mutex<JobQueue>> = OnceLock::new();
static WORKERS_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn jobs() -> &'static Mutex<JobQueue> {
    JOBS.get_or_init(|| {
        Mutex::new(JobQueue {
            queue: Default::default(),
            seen: Default::default(),
            failed: Default::default(),
            order: Default::default(),
            cache: Default::default(),
        })
    })
}

/// Imagem pronta de um emoji, rasterizada da fonte empacotada via swash e
/// composta via cosmic (lida com ZWJ/flags). Retorna o cache na hora ou
/// `None` (placeholder) enfileirando o raster em background — o worker dá
/// `bump` e o poller repinta em ~66ms. Nunca congela a UI.
/// `None` definitivo quando não há glifo (ex.: "♪" — o chamador cai para
/// texto normal).
pub(crate) fn image(
    emoji: &str,
    shared: &std::sync::Arc<std::sync::Mutex<crate::core::state::Shared>>,
) -> Option<Arc<open_gpui::RenderImage>> {
    if let Some(hit) = jobs().lock().unwrap().cache.get(emoji) {
        return Some(hit.clone());
    }
    {
        let mut jobs = jobs().lock().unwrap();
        if jobs.failed.contains(emoji) || !jobs.seen.insert(emoji.to_string()) {
            return None;
        }
        jobs.queue.push_back(emoji.to_string());
    }
    ensure_workers(shared);
    None
}

fn ensure_workers(shared: &std::sync::Arc<std::sync::Mutex<crate::core::state::Shared>>) {
    use std::sync::atomic::Ordering;
    if WORKERS_ON.swap(true, Ordering::SeqCst) {
        return;
    }
    for _ in 0..WORKERS {
        let shared = shared.clone();
        std::thread::Builder::new()
            .name("klipp-emoji".into())
            .spawn(move || worker_loop(shared))
            .ok();
    }
}

fn worker_loop(shared: std::sync::Arc<std::sync::Mutex<crate::core::state::Shared>>) {
    loop {
        let job = jobs().lock().unwrap().queue.pop_front();
        let Some(emoji) = job else {
            std::thread::sleep(std::time::Duration::from_millis(20));
            continue;
        };
        match render_emoji(&emoji) {
            Some(image) => {
                let mut jobs = jobs().lock().unwrap();
                jobs.cache.insert(emoji.clone(), image);
                jobs.order.push_back(emoji);
                while jobs.cache.len() > CACHE_CAP {
                    if let Some(old) = jobs.order.pop_front() {
                        jobs.cache.remove(&old);
                        jobs.seen.remove(&old);
                    } else {
                        break;
                    }
                }
                drop(jobs);
                shared.lock().map(|mut s| s.bump()).ok();
            }
            None => {
                jobs().lock().unwrap().failed.insert(emoji);
            }
        }
    }
}

fn raster_state() -> Option<std::sync::MutexGuard<'static, RasterState>> {
    static STATE: OnceLock<Mutex<RasterState>> = OnceLock::new();
    let state = STATE.get_or_init(|| {
        let bytes = Arc::new(load_font_bytes().unwrap_or_default());
        let mut font_system = cosmic_text::FontSystem::new_with_locale_and_db(
            "en-US".to_string(),
            cosmic_text::fontdb::Database::new(),
        );
        font_system.db_mut().load_font_data(bytes.to_vec());
        Mutex::new(RasterState {
            font_system,
            bytes,
            scale: swash::scale::ScaleContext::new(),
        })
    });
    let guard = state.lock().unwrap();
    if guard.bytes.is_empty() {
        return None;
    }
    Some(guard)
}

fn render_emoji(emoji: &str) -> Option<Arc<open_gpui::RenderImage>> {
    struct Placed {
        gid: u16,
        x: f32,
    }
    // 1) Forma o cluster e extrai (gid, x). O borrow do shaper termina aqui.
    let (placed, w, h, baseline) = {
        let mut state = raster_state()?;
        let fs = &mut state.font_system;
        let mut buf = cosmic_text::Buffer::new(fs, cosmic_text::Metrics::new(RASTER_PX, RASTER_PX));
        buf.set_size(Some(600.0), Some(200.0));
        buf.set_text(
            emoji,
            &cosmic_text::Attrs::new().family(cosmic_text::Family::Name("Noto Color Emoji")),
            cosmic_text::Shaping::Advanced,
            None,
        );
        buf.shape_until_scroll(fs, false);
        let run = buf.layout_runs().next()?;
        let placed: Vec<Placed> = run
            .glyphs
            .iter()
            .map(|g| Placed {
                gid: g.glyph_id,
                x: g.x,
            })
            .collect();
        (
            placed,
            run.line_w.ceil().max(1.0) as u32,
            run.line_height.ceil().max(1.0) as u32,
            run.line_y - run.line_top,
        )
    };
    if placed.is_empty() {
        return None;
    }
    // 2) Rasteriza cada glifo e compõe (src-over) no canvas da linha.
    let mut state = raster_state()?;
    let bytes = state.bytes.clone();
    let fr = swash::FontRef::from_index(&bytes, 0)?;
    let mut canvas = vec![0u8; (w * h * 4) as usize];
    let mut any = false;
    for glyph in &placed {
        let mut scaler = state.scale.builder(fr).size(RASTER_PX).hint(true).build();
        let sources = [
            swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit),
            swash::scale::Source::ColorOutline(0),
        ];
        let mut renderer = swash::scale::Render::new(&sources);
        renderer.format(swash::zeno::Format::Alpha);
        let Some(img) = renderer.render(&mut scaler, glyph.gid) else {
            continue;
        };
        if img.placement.width == 0 || img.placement.height == 0 {
            continue;
        }
        any = true;
        let dx = (glyph.x + img.placement.left as f32).round() as i32;
        let dy = (baseline - img.placement.top as f32).round() as i32;
        blit(&mut canvas, w, h, &img.data, img.placement.width, dx, dy);
    }
    if !any {
        return None;
    }
    let (side, square) = square_pad(w, h, &canvas)?;
    let mut rgba = image::RgbaImage::from_raw(side, side, square)?;
    for pixel in rgba.pixels_mut() {
        let r = pixel[0];
        pixel[0] = pixel[2];
        pixel[2] = r;
    }
    let frames = smallvec::SmallVec::from_elem(image::Frame::new(rgba), 1);
    Some(Arc::new(open_gpui::RenderImage::new(frames)))
}

/// Compõe src RGBA sobre dst (src-over), com recorte.
fn blit(dst: &mut [u8], dw: u32, dh: u32, src: &[u8], sw: u32, dx: i32, dy: i32) {
    for (i, px) in src.chunks_exact(4).enumerate() {
        let gx = (i as u32 % sw) as i32 + dx;
        let gy = (i as u32 / sw) as i32 + dy;
        if gx < 0 || gy < 0 || gx >= dw as i32 || gy >= dh as i32 {
            continue;
        }
        let o = ((gy as u32 * dw + gx as u32) * 4) as usize;
        let sa = px[3] as f32 / 255.0;
        if sa <= 0.0 {
            continue;
        }
        let da = dst[o + 3] as f32 / 255.0;
        let oa = sa + da * (1.0 - sa);
        dst[o] = ((px[0] as f32 * sa + dst[o] as f32 * da * (1.0 - sa)) / oa) as u8;
        dst[o + 1] = ((px[1] as f32 * sa + dst[o + 1] as f32 * da * (1.0 - sa)) / oa) as u8;
        dst[o + 2] = ((px[2] as f32 * sa + dst[o + 2] as f32 * da * (1.0 - sa)) / oa) as u8;
        dst[o + 3] = (oa * 255.0) as u8;
    }
}

/// Recorta a bbox não-transparente e centraliza num quadrado.
fn square_pad(w: u32, h: u32, px: &[u8]) -> Option<(u32, Vec<u8>)> {
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            if px[((y * w + x) * 4 + 3) as usize] > 0 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x1 < x0 || y1 < y0 {
        return None;
    }
    let (tw, th) = (x1 - x0 + 1, y1 - y0 + 1);
    let side = tw.max(th);
    let (ox, oy) = ((side - tw) / 2, (side - th) / 2);
    let mut sq = vec![0u8; (side * side * 4) as usize];
    for y in 0..th {
        let src_o = (((y0 + y) * w + x0) * 4) as usize;
        let dst_o = (((oy + y) * side + ox) * 4) as usize;
        sq[dst_o..dst_o + (tw * 4) as usize]
            .copy_from_slice(&px[src_o..src_o + (tw * 4) as usize]);
    }
    Some((side, sq))
}

/// Localiza `assets/fonts/NotoColorEmoji.ttf` (dev e instalado).
fn load_font_bytes() -> Option<Vec<u8>> {
    const FILE: &str = "NotoColorEmoji.ttf";
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.to_path_buf()));
    let mut candidates = vec![
        std::path::PathBuf::from("assets").join("fonts").join(FILE),
        std::path::PathBuf::from("klipp").join("assets").join("fonts").join(FILE),
    ];
    if let Some(dir) = exe_dir {
        candidates.push(dir.join("assets").join("fonts").join(FILE));
        candidates.push(dir.join("..").join("share").join("klipp").join("fonts").join(FILE));
        candidates.push(dir.join("..").join("share").join("fonts").join(FILE));
    }
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .and_then(|p| std::fs::read(p).ok())
}

#[cfg(test)]
mod tests {
    use super::{CACHE_CAP, data};

    /// A maior aba precisa caber inteira no cache: se o worker ainda
    /// preenche enquanto o render despeja, os mesmos emojis entram em loop
    /// de re-raster (cada conclusão dá bump) e a grade pisca sem parar.
    #[test]
    fn cache_cobre_maior_aba() {
        let biggest = data()
            .groups
            .iter()
            .map(|g| g.subgroups.iter().map(|s| s.emojis.len()).sum::<usize>())
            .max()
            .unwrap_or(0);
        assert!(
            biggest <= CACHE_CAP,
            "aba com {biggest} emojis não cabe no cache ({CACHE_CAP})"
        );
    }
}
