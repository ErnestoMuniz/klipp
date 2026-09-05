use std::sync::atomic::{AtomicBool, Ordering};

use open_gpui::{rgb, Rgba};

/// Paleta completa. Valores do Electron (`index.css`: `:root` = light,
/// `:root.dark` = dark).
struct Palette {
    bg: u32,
    titlebar: u32,
    panel: u32,
    card: u32,
    card_inner: u32,
    border: u32,
    dashed: u32,
    text: u32,
    muted: u32,
    accent: u32,
    accent_hover: u32,
    danger: u32,
    danger_hover: u32,
    hint_bg: u32,
    hint_border: u32,
    card_hover: u32,
    sel_bg: u32,
}

const DARK: Palette = Palette {
    bg: 0x171717,
    titlebar: 0x101010,
    panel: 0x1e1e1e,
    card: 0x252525,
    card_inner: 0x1a1a1a,
    border: 0x2e2e2e,
    dashed: 0x3f3f3f,
    text: 0xf0f0f0,
    muted: 0x9c9c9c,
    accent: 0x42a1ff,
    accent_hover: 0x5cadff,
    danger: 0x7f1d1d,
    danger_hover: 0x962222,
    hint_bg: 0x201b08,
    hint_border: 0x8a7325,
    card_hover: 0x2c2c2c,
    sel_bg: 0x12314f,
};

const LIGHT: Palette = Palette {
    bg: 0xf0eee9,
    titlebar: 0xf6f4ee,
    panel: 0xf6f4ee,
    card: 0xffffff,
    card_inner: 0xebe9e3,
    border: 0xdbd8d1,
    dashed: 0xc5c2bb,
    text: 0x18130d,
    muted: 0x8a8377,
    accent: 0x2a80e2,
    accent_hover: 0x42a1ff,
    danger: 0x7f1d1d,
    danger_hover: 0x962222,
    hint_bg: 0xfaf0d2,
    hint_border: 0xcfb95c,
    card_hover: 0xe9e6de,
    sel_bg: 0xe3f1fd,
};

static LIGHT_MODE: AtomicBool = AtomicBool::new(false);

/// Troca o tema global. Chamado no início do `render` a partir de `Shared.theme`.
pub fn set_light(light: bool) {
    LIGHT_MODE.store(light, Ordering::Relaxed);
}

fn pal() -> &'static Palette {
    if LIGHT_MODE.load(Ordering::Relaxed) {
        &LIGHT
    } else {
        &DARK
    }
}

pub fn bg() -> Rgba {
    rgb(pal().bg)
}
pub fn titlebar() -> Rgba {
    rgb(pal().titlebar)
}
pub fn panel() -> Rgba {
    rgb(pal().panel)
}
pub fn card() -> Rgba {
    rgb(pal().card)
}
pub fn card_inner() -> Rgba {
    rgb(pal().card_inner)
}
pub fn border() -> Rgba {
    rgb(pal().border)
}
pub fn dashed() -> Rgba {
    rgb(pal().dashed)
}
pub fn text() -> Rgba {
    rgb(pal().text)
}
pub fn muted() -> Rgba {
    rgb(pal().muted)
}
pub fn accent() -> Rgba {
    rgb(pal().accent)
}
pub fn accent_hover() -> Rgba {
    rgb(pal().accent_hover)
}
pub fn danger() -> Rgba {
    rgb(pal().danger)
}
pub fn danger_hover() -> Rgba {
    rgb(pal().danger_hover)
}
pub fn hint_bg() -> Rgba {
    rgb(pal().hint_bg)
}
pub fn hint_border() -> Rgba {
    rgb(pal().hint_border)
}
pub fn card_hover() -> Rgba {
    rgb(pal().card_hover)
}
/// Fundo da opção selecionada em dropdowns.
pub fn sel_bg() -> Rgba {
    rgb(pal().sel_bg)
}
