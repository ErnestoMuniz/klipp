/// Geometria + geração do SVG do pie selector (overlay).
/// Arquivo puro (sem GPUI): fácil de testar e de iterar no frontend.

pub const PIE: f32 = 520.0;
pub const CENTER: f32 = PIE / 2.0;
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

/// Dentro do botão central (raio interno): parar o áudio.
pub fn in_center(pos_x: f32, pos_y: f32, anchor: (f32, f32)) -> bool {
    let dx = pos_x - anchor.0;
    let dy = pos_y - anchor.1;
    (dx * dx + dy * dy).sqrt() <= INNER
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

/// Cores do pie por tema (neutras do projeto, ver `theme.rs`: dark = grafite,
/// light = papel/creme; hover = accent do app).
pub struct PieColors {
    pub backdrop: &'static str,
    pub backdrop_op: &'static str,
    pub backdrop_stroke: &'static str,
    pub wedge: &'static str,
    pub wedge_hover: &'static str,
    pub wedge_stroke: &'static str,
    pub center: &'static str,
    pub center_stroke: &'static str,
}

pub fn pie_colors(light: bool) -> PieColors {
    if light {
        PieColors {
            backdrop: "#ffffff",
            backdrop_op: "0.95",
            backdrop_stroke: "#dbd8d1",
            wedge: "#f6f4ee",
            // = theme::accent() light
            wedge_hover: "#2a80e2",
            wedge_stroke: "#c5c2bb",
            center: "#f6f4ee",
            center_stroke: "#c5c2bb",
        }
    } else {
        PieColors {
            // = theme::titlebar/panel/card/border dark
            backdrop: "#101010",
            backdrop_op: "0.92",
            backdrop_stroke: "#2e2e2e",
            wedge: "#252525",
            // = theme::accent() dark
            wedge_hover: "#42a1ff",
            wedge_stroke: "#3f3f3f",
            center: "#1e1e1e",
            center_stroke: "#3f3f3f",
        }
    }
}

/// Centro geométrico da fatia `i` (coords da caixa 460x460): posiciona os
/// rótulos GPUI (emoji + nome) sobre a fatia.
pub fn slice_mid(i: usize, n: usize) -> (f32, f32) {
    let n = n.min(PAGE).max(1) as f32;
    let slice = 360.0 / n;
    let mid = -90.0 + i as f32 * slice + slice / 2.0;
    polar((OUTER + INNER) / 2.0, mid)
}

/// Pie completo (fundo + fatias + centro, só formas — textos são divs GPUI).
pub fn gen_pie_svg(count: usize, light: bool) -> String {
    gen_pie_inner(count, None, light, false)
}

/// Só a fatia `idx` em destaque, resto transparente (camada de hover).
pub fn gen_pie_highlight_svg(count: usize, idx: usize, light: bool) -> String {
    gen_pie_inner(count, Some(idx), light, true)
}

fn gen_pie_inner(count: usize, highlight: Option<usize>, light: bool, solo: bool) -> String {
    let c = pie_colors(light);
    let n = count.min(PAGE);
    // Canvas 520 (pie 460 + 30px de margem): a sombra borrada espalha ~36px.
    let mut svg = String::from(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="520" height="520" viewBox="0 0 520 520">
  <defs>
    <filter id="pie-shadow">
      <feDropShadow dx="6" dy="10" stdDeviation="12" flood-color="#000000" flood-opacity="0.45" />
    </filter>
  </defs>"##,
    );
    if !solo {
        svg.push_str(&format!(
            r##"
  <circle cx="260" cy="260" r="218" fill="{}" fill-opacity="{}" stroke="{}" stroke-opacity="0.6" stroke-width="1.5" filter="url(#pie-shadow)" />"##,
            c.backdrop, c.backdrop_op, c.backdrop_stroke
        ));
    }

    if n > 0 {
        let slice = 360.0 / n as f32;
        for i in 0..n {
            let is_hovered = highlight == Some(i);
            if solo && !is_hovered {
                continue;
            }
            let fill = if is_hovered { c.wedge_hover } else { c.wedge };
            let d0 = -90.0 + i as f32 * slice;
            let d1 = d0 + slice;
            svg.push_str(&format!(
                r##"
  <path d="{}" fill="{fill}" fill-opacity="0.97" stroke="{}" stroke-opacity="0.7" stroke-width="1.5"/>"##,
                wedge(d0, d1),
                c.wedge_stroke,
            ));
        }
    }

    if !solo {
        svg.push_str(&format!(
            r##"
  <circle cx="260" cy="260" r="69" fill="{}" stroke="{}" stroke-opacity="0.9" stroke-width="1.5"/>
</svg>"##,
            c.center, c.center_stroke
        ));
    } else {
        svg.push_str("</svg>");
    }
    svg
}

/// Rasteriza o pie em RGBA (460x460).
///
/// O elemento `svg()` do GPUI renderiza como máscara de alpha tingida de
/// uma cor só — cores e textos do SVG são descartados (pie todo branco).
/// Rasterizando com resvg e exibindo via `img()`, as cores saem fiéis.
/// O SVG tem só formas (textos são divs GPUI por cima).
pub fn render_pie_image(count: usize, light: bool) -> Option<image::RgbaImage> {
    render_svg_image(&gen_pie_svg(count, light))
}

/// Rasteriza só a fatia em destaque (fundo transparente).
pub fn render_pie_highlight_image(
    count: usize,
    idx: usize,
    light: bool,
) -> Option<image::RgbaImage> {
    render_svg_image(&gen_pie_highlight_svg(count, idx, light))
}

fn render_svg_image(svg_text: &str) -> Option<image::RgbaImage> {
    use std::sync::OnceLock;
    // Carregado uma vez: varrer as fontes do sistema a cada hover dava o
    // delay (~200ms) na troca de destaque.
    static FONTDB: OnceLock<std::sync::Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    let db = FONTDB.get_or_init(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_system_fonts();
        std::sync::Arc::new(db)
    });
    let mut opts = resvg::usvg::Options::default();
    // Sem isso o fontdb fica vazio e nenhum glifo renderiza (texto some).
    opts.fontdb = db.clone();
    let tree = resvg::usvg::Tree::from_str(&svg_text, &opts).ok()?;
    let w = PIE as u32;
    let h = PIE as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let mut img = image::RgbaImage::from_raw(w, h, pixmap.take())?;
    // RenderImage é BGRA (ver o raster do emoji): sem a troca, azul vira
    // laranja (R↔B). Cinzas não mostravam o bug.
    for pixel in img.pixels_mut() {
        let r = pixel[0];
        pixel[0] = pixel[2];
        pixel[2] = r;
    }
    Some(img)
}

#[cfg(test)]
mod tests {
    use super::{
        gen_pie_highlight_svg, gen_pie_svg, in_center, pie_colors, render_pie_highlight_image,
        render_pie_image, slice_mid, INNER,
    };

    #[test]
    fn pie_dark_tem_contraste() {
        let svg = gen_pie_svg(2, false);
        assert!(svg.contains("#252525"), "fatia neutra escura");
        assert!(!svg.contains("#42a1ff"), "base não destaca nada");
        assert!(!svg.contains("<text"), "sem texto no SVG");
        let hl = gen_pie_highlight_svg(2, 0, false);
        assert!(hl.contains("#42a1ff"), "destaque accent");
        assert!(!hl.contains("r=\"218\""), "destaque sem fundo");
        let _ = pie_colors(true);
        let (mx, my) = slice_mid(0, 2);
        assert!((mx - 230.0).abs() < 230.0 && (my - 230.0).abs() < 230.0);
    }

    #[test]
    fn pie_light_tem_contraste() {
        let svg = gen_pie_svg(1, true);
        assert!(svg.contains("#f6f4ee"), "fatia clara");
        let hl = gen_pie_highlight_svg(1, 0, true);
        assert!(hl.contains("#2a80e2"), "destaque accent");
    }

    #[test]
    fn geometria_meio_e_centro() {
        // Fatia 0 de 2: metade direita, meio em (403.5, 260).
        let (mx, my) = slice_mid(0, 2);
        assert!((mx - 403.5).abs() < 1.0 && (my - 260.0).abs() < 1.0);
        assert!(in_center(260.0, 260.0, (260.0, 260.0)));
        assert!(in_center(260.0 + INNER, 260.0, (260.0, 260.0)));
        assert!(!in_center(260.0 + INNER + 1.0, 260.0, (260.0, 260.0)));
        assert!(!in_center(403.0, 260.0, (260.0, 260.0)));
    }

    #[test]
    fn raster_tem_tamanho_e_conteudo() {
        let img = render_pie_image(2, false).expect("rasteriza");
        assert_eq!(img.dimensions(), (520, 520));
        // Centro do pie (círculo #1e1e1e opaco): pixel escuro e opaco.
        let center = img.get_pixel(260, 260);
        assert_eq!(center[3], 255, "centro opaco");
        assert!(center[0] < 60 && center[1] < 60 && center[2] < 60, "centro escuro: {center:?}");
        // Canto fora de tudo: transparente (fundo da surface aparece).
        assert_eq!(img.get_pixel(5, 5)[3], 0, "canto transparente");
        // Sombra borrada além da borda do pie (borda em x=478): tem alpha.
        assert!(img.get_pixel(495, 260)[3] > 0, "sombra?"
        );
        // Destaque: só a fatia, fundo transparente.
        let hl = render_pie_highlight_image(2, 0, false).expect("rasteriza hl");
        assert_eq!(hl.dimensions(), (520, 520));
        assert_eq!(hl.get_pixel(5, 5)[3], 0, "destaque sem fundo");
        assert!(hl.pixels().any(|p| p[3] > 128), "destaque tem conteúdo");
        // Cor do destaque fiel: fatia 0 de 2 cobre a metade direita —
        // (403, 260) é o centro dela. Hover #42a1ff = azul, não laranja:
        // RenderImage é BGRA (índice 0 = canal azul!). Sem a troca R↔B
        // na rasterização o azul virava laranja.
        let mid = hl.get_pixel(403, 260);
        assert!(mid[3] > 128, "meio da fatia opaco: {mid:?}");
        assert!(
            (mid[0] as i16 - mid[2] as i16) > 100,
            "destaque azul (B≫R): {mid:?}"
        );
        // Tema claro rasteriza também.
        let light = render_pie_image(2, true).expect("rasteriza claro");
        assert_eq!(light.dimensions(), (520, 520));
        assert_eq!(light.get_pixel(5, 5)[3], 0, "claro sem fundo nas bordas");
    }
}
