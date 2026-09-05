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
  <circle cx="230" cy="230" r="218" fill="rgba(2,6,23,0.55)" />"##,
    );

    if n > 0 {
        let slice = 360.0 / n as f32;
        for (i, name) in names.iter().take(n).enumerate() {
            let d0 = -90.0 + i as f32 * slice;
            let d1 = d0 + slice;
            let mid = (d0 + d1) / 2.0;
            let fill = if hovered == Some(i) {
                "rgba(14,165,233,0.85)"
            } else {
                "rgba(51,65,85,0.75)"
            };
            let (lx, ly) = polar((OUTER + INNER) / 2.0, mid);
            svg.push_str(&format!(
                r##"  <path d="{}" fill="{fill}" stroke="rgba(226,232,240,0.25)" stroke-width="1"/>
  <text x="{lx:.1}" y="{ly:.1}" text-anchor="middle" font-size="24">♪</text>
  <text x="{lx:.1}" y="{ly:.1}" dy="19" text-anchor="middle" font-size="14" fill="#f8fafc">{}</text>
"##,
                wedge(d0, d1),
                escape_xml(&short_label(name))
            ));
        }
    }

    svg.push_str(&format!(
        r##"  <circle cx="230" cy="230" r="69" fill="rgba(15,23,42,0.92)" stroke="rgba(71,85,105,0.8)" stroke-width="1"/>
  <text x="230" y="227" text-anchor="middle" font-size="20" font-weight="bold" fill="#e2e8f0">Klipp</text>
  <text x="230" y="247" text-anchor="middle" font-size="13" fill="#94a3b8">{n} sons</text>
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
