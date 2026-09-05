/// Geometria + geração do SVG do pie selector (overlay).
/// Arquivo puro (sem GPUI): fácil de testar e de iterar no frontend.

pub const PIE: f32 = 460.0;
const CENTER: f32 = PIE / 2.0;
pub const OUTER: f32 = 218.0;
pub const INNER: f32 = 69.0;
pub const PAGE: usize = 8;

fn polar(r: f32, deg: f32) -> (f32, f32) {
    let rad = deg.to_radians();
    (CENTER + rad.cos() * r, CENTER + rad.sin() * r)
}

fn wedge(d0: f32, d1: f32) -> String {
    let (ox0, oy0) = polar(OUTER, d0);
    let (ox1, oy1) = polar(OUTER, d1);
    let (ix0, iy0) = polar(INNER, d0);
    let (ix1, iy1) = polar(INNER, d1);
    let large = if d1 - d0 > 180.0 { 1 } else { 0 };
    format!(
        "M {ix0:.1} {iy0:.1} L {ox0:.1} {oy0:.1} A {OUTER:.0} {OUTER:.0} 0 {large} 1 {ox1:.1} {oy1:.1} L {ix1:.1} {iy1:.1} A {INNER:.0} {INNER:.0} 0 {large} 0 {ix0:.1} {iy0:.1} Z"
    )
}

/// Rótulo curto de fatia: os nomes já chegam sem extensão (display),
/// então aqui só trunca pelo número de caracteres.
pub fn short_label(label: &str) -> String {
    if label.chars().count() > 13 {
        let truncated: String = label.chars().take(12).collect();
        format!("{truncated}…")
    } else {
        label.to_string()
    }
}

/// Converte posição do mouse em índice da fatia (ângulo a partir do topo, horário).
pub fn hit_test(
    pos_x: f32,
    pos_y: f32,
    anchor: (f32, f32),
    count: usize,
) -> (Option<usize>, bool) {
    use std::f32::consts::PI;
    if count == 0 {
        return (None, false);
    }
    let dx = pos_x - anchor.0;
    let dy = pos_y - anchor.1;
    let r = (dx * dx + dy * dy).sqrt();
    if !(INNER..=OUTER).contains(&r) {
        return (None, false);
    }
    let n = count.min(PAGE);
    let slice = 2.0 * PI / n as f32;
    let mut a = dy.atan2(dx) + PI / 2.0;
    if a < 0.0 {
        a += 2.0 * PI;
    }
    let idx = ((a % (2.0 * PI)) / slice) as usize % n;
    (Some(idx), true)
}

pub fn gen_pie_svg(names: &[String], hovered: Option<usize>) -> String {
    let n = names.len().min(PAGE);
    let mut svg = String::from(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="460" height="460" viewBox="0 0 460 460">
  <circle cx="230" cy="230" r="218" fill="#020617" fill-opacity="0.88" stroke="#e2e8f0" stroke-opacity="0.35" stroke-width="1.5" />"##,
    );

    if n > 0 {
        let slice = 360.0 / n as f32;
        for (i, name) in names.iter().take(n).enumerate() {
            let d0 = -90.0 + i as f32 * slice;
            let d1 = d0 + slice;
            let mid = (d0 + d1) / 2.0;
            let (fill, fill_op) = if hovered == Some(i) {
                ("#0ea5e9", "0.97")
            } else {
                ("#1e293b", "0.97")
            };
            let (lx, ly) = polar((OUTER + INNER) / 2.0, mid);
            svg.push_str(&format!(
                r##"  <path d="{}" fill="{fill}" fill-opacity="{fill_op}" stroke="#f1f5f9" stroke-opacity="0.65" stroke-width="1.5"/>
  <text x="{lx:.1}" y="{ly:.1}" text-anchor="middle" font-size="24" font-family="'DejaVu Sans','Noto Sans',sans-serif" fill="#ffffff">{note}</text>
  <text x="{lx:.1}" y="{ly:.1}" dy="19" text-anchor="middle" font-size="14" font-weight="bold" font-family="'DejaVu Sans','Noto Sans',sans-serif" fill="#ffffff" stroke="#000000" stroke-opacity="0.85" stroke-width="3" paint-order="stroke">{label}</text>
"##,
                wedge(d0, d1),
                note = "♪",
                label = escape_xml(&short_label(name))
            ));
        }
    }

    svg.push_str(&format!(
        r##"  <circle cx="230" cy="230" r="69" fill="#0f172a" stroke="#94a3b8" stroke-opacity="0.9" stroke-width="1.5"/>
  <text x="230" y="227" text-anchor="middle" font-size="20" font-weight="bold" font-family="'DejaVu Sans','Noto Sans',sans-serif" fill="#f8fafc">Klipp</text>
  <text x="230" y="247" text-anchor="middle" font-size="13" font-weight="bold" font-family="'DejaVu Sans','Noto Sans',sans-serif" fill="#ffffff">{n} sons</text>
</svg>"##
    ));
    svg
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Rasteriza o pie em RGBA (460x460).
///
/// O elemento `svg()` do GPUI renderiza como máscara de alpha tingida de
/// uma cor só — cores e textos do SVG são descartados (pie todo branco).
/// Rasterizando com resvg e exibindo via `img()`, as cores saem fiéis.
pub fn render_pie_image(names: &[String], hovered: Option<usize>) -> Option<image::RgbaImage> {
    let svg_text = gen_pie_svg(names, hovered);
    let mut opts = resvg::usvg::Options::default();
    // Sem isso o fontdb fica vazio e nenhum glifo renderiza (texto some).
    let mut db = resvg::usvg::fontdb::Database::new();
    db.load_system_fonts();
    opts.fontdb = std::sync::Arc::new(db);
    let tree = resvg::usvg::Tree::from_str(&svg_text, &opts).ok()?;
    let w = PIE as u32;
    let h = PIE as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    image::RgbaImage::from_raw(w, h, pixmap.take())
}

#[cfg(test)]
mod tests {
    use super::{gen_pie_svg, render_pie_image};

    #[test]
    fn pie_tem_contraste_fatias_escuras_e_texto_branco() {
        let names = vec!["boom".to_string(), "airhorn".to_string()];
        let svg = gen_pie_svg(&names, Some(0));
        // Fatias opacas escuras (legível sobre qualquer wallpaper).
        assert!(svg.contains("#1e293b"), "fatia normal escura");
        assert!(svg.contains("#0ea5e9"), "fatia hover");
        // Textos brancos com contorno para leitura.
        assert!(svg.contains("paint-order=\"stroke\""), "label com contorno");
        assert!(svg.contains("fill=\"#ffffff\""), "texto branco");
        // Sem dim translúcido fraco que lavava o pie.
        assert!(!svg.contains("0.55"), "sem alpha lavado");
        assert!(!svg.contains("0.75"), "sem alpha lavado");
    }

    #[test]
    fn raster_tem_tamanho_e_conteudo() {
        let names = vec!["boom".to_string(), "airhorn".to_string()];
        let img = render_pie_image(&names, Some(0)).expect("rasteriza");
        assert_eq!(img.dimensions(), (460, 460));
        // Centro do pie (círculo #0f172a opaco): pixel escuro e opaco.
        let center = img.get_pixel(230, 230);
        assert_eq!(center[3], 255, "centro opaco");
        assert!(center[0] < 40 && center[1] < 40 && center[2] < 60, "centro escuro: {center:?}");
        // Canto fora do círculo: transparente (fundo da surface aparece).
        assert_eq!(img.get_pixel(5, 5)[3], 0, "canto transparente");
        // Texto renderiza (fontes do sistema carregadas): há pixels quase
        // brancos dos rótulos. Sem fonte, o pie sai mudo.
        let bright = img
            .pixels()
            .filter(|p| p[3] > 128 && p[0] > 200 && p[1] > 200 && p[2] > 200)
            .count();
        assert!(bright > 100, "texto visível no raster (pixels claros: {bright})");
    }
}
