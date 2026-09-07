use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub shortcut: String,
    /// Densidade da biblioteca: "comfort" | "compact".
    #[serde(default = "default_density")]
    pub density: String,
    /// Microfone real junto no mix.
    #[serde(default = "default_true")]
    pub mic_passthrough: bool,
    /// Ouvir os clips no sink padrão.
    #[serde(default = "default_true")]
    pub hear_clips: bool,
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Ordenação: "name-asc" | "name-desc" | "recent".
    #[serde(default = "default_sort")]
    pub sort: String,
    /// Tema: "system" | "light" | "dark" (system = segue o escuro por enquanto).
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Idioma: "system" | "en" | "pt-BR".
    #[serde(default = "default_lang")]
    pub language: String,
    /// Fonte do mic real escolhida no drawer (vazio = auto-detect).
    #[serde(default)]
    pub mic_source: String,
    /// Manter rodando no tray ao fechar a janela (padrão ligado).
    #[serde(default = "default_true")]
    pub run_in_background: bool,
    /// Banner de dica do atalho (lâmpada da toolbar, padrão ligado).
    #[serde(default = "default_true")]
    pub show_hints: bool,
}

fn default_density() -> String {
    "comfort".into()
}

fn default_volume() -> f32 {
    1.0
}

fn default_sort() -> String {
    "name-asc".into()
}

fn default_theme() -> String {
    "system".into()
}

fn default_lang() -> String {
    "system".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "Alt+Shift+S".into(),
            density: default_density(),
            sort: default_sort(),
            theme: default_theme(),
            language: default_lang(),
            mic_passthrough: true,
            hear_clips: true,
            volume: 1.0,
            mic_source: String::new(),
            run_in_background: true,
            show_hints: true,
        }
    }
}

pub fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("klipp")
        .join("settings.json")
}

pub fn load() -> Settings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) {
    if let Some(dir) = settings_path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(raw) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(settings_path(), raw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_in_background_sobrevive_ao_roundtrip() {
        // Desligado persiste desligado (o toggle vale entre sessões).
        let mut s = Settings::default();
        assert!(s.run_in_background);
        s.run_in_background = false;
        let raw = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&raw).unwrap();
        assert!(!back.run_in_background);
        // settings.json antigo (sem as chaves) assume ligado.
        let old: Settings = serde_json::from_str(r#"{"shortcut":"x"}"#).unwrap();
        assert!(old.run_in_background);
        assert!(old.show_hints);
    }
}
