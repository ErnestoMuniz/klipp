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

/// Relógio do player (`0:00`, `1:05`): minutos sem zero à esquerda.
pub(crate) fn format_clock(secs: f32) -> String {
    let total = secs.max(0.0).floor() as u32;
    format!("{}:{:02}", total / 60, total % 60)
}

/// Fração 0–1 do progresso, segura contra duração zerada/negativa.
pub(crate) fn progress_fraction(elapsed_secs: f32, duration_secs: f32) -> f32 {
    if duration_secs <= 0.0 {
        return 0.0;
    }
    (elapsed_secs / duration_secs).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{format_clock, progress_fraction};

    #[test]
    fn clock_formata_minutos_e_segundos() {
        assert_eq!(format_clock(0.0), "0:00");
        assert_eq!(format_clock(3.9), "0:03");
        assert_eq!(format_clock(65.0), "1:05");
        assert_eq!(format_clock(-5.0), "0:00");
    }

    #[test]
    fn fracao_prende_nas_pontas_e_rejeita_duracao_invalida() {
        assert_eq!(progress_fraction(5.0, 10.0), 0.5);
        assert_eq!(progress_fraction(15.0, 10.0), 1.0);
        assert_eq!(progress_fraction(-2.0, 10.0), 0.0);
        assert_eq!(progress_fraction(5.0, 0.0), 0.0);
        assert_eq!(progress_fraction(5.0, -1.0), 0.0);
    }
}
