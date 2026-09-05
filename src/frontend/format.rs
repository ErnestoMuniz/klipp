use open_gpui::Rgba;

/// Nome de exibição: sem extensão, `_`/`-` viram espaço.
pub(crate) fn display_name(name: &str) -> String {
    let base = name.rsplit_once('.').map(|(b, _)| b).unwrap_or(name);
    base.replace(['_', '-'], " ")
}

/// Rótulo do pad: display da metadata ou derivado do arquivo.
pub(crate) fn sound_label(sound: &crate::core::state::Sound) -> String {
    sound
        .display
        .clone()
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| display_name(&sound.name))
}

/// Fase do caret: ~530ms aceso / ~530ms apagado.
pub(crate) fn caret_on() -> bool {
    now_ms() % 1060 < 530
}

pub(crate) fn now_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Texto escuro sobre o azul de destaque.
pub(crate) fn rgb_dark() -> Rgba {
    open_gpui::rgb(0x0a0a0a)
}

pub(crate) fn format_duration(secs: Option<f32>) -> String {
    match secs {
        None => "…".to_string(),
        Some(s) => {
            let total = s.round() as u32;
            if total < 60 {
                format!("{total}s")
            } else {
                format!("{}:{:02}", total / 60, total % 60)
            }
        }
    }
}
