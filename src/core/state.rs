use std::collections::HashSet;
use std::path::PathBuf;

use crate::core::settings::Settings;

/// Um clipe de áudio descoberto em disco.
/// `duration_secs` é preenchida em background pelo prober.
/// `display`/`emoji` vêm de `sound-metadata.json` (dialog "Edit sound").
#[derive(Clone, Debug)]
pub struct Sound {
    pub name: String,
    pub path: PathBuf,
    pub duration_secs: Option<f32>,
    /// mtime para ordenar por "Recent" (0 = desconhecido).
    pub modified_secs: u64,
    /// Nome de exibição (None = deriva do arquivo).
    pub display: Option<String>,
    /// Emoji do pad ("♪" por padrão).
    pub emoji: String,
}

/// Resultado de busca online (MyInstants): nada em disco até baixar.
#[derive(Clone, Debug)]
pub struct OnlineSound {
    pub id: String,
    pub title: String,
    pub mp3: String,
}

/// Barras da forma de onda do player: envelope estático (pico 0–1 por
/// trecho), calculado uma vez por play. Fixo para a trilha clicável
/// de 640px caber exata (128 × 5px).
pub const WAVEFORM_BARS: usize = 128;

/// Estado compartilhado entre backend (threads/services) e frontend (GPUI).
/// `version` funciona como geração: incremente sempre que a UI precisar re-renderizar.
pub struct Shared {
    pub sounds: Vec<Sound>,
    pub playing: Option<String>,
    pub shortcut: String,
    pub shortcut_rebinding: bool,
    pub mic: String,
    pub mic_source: String,
    /// Fontes reais para o seletor do drawer: (nome, descrição).
    pub mic_sources: Vec<(String, String)>,
    pub mic_open: bool,
    pub last_error: Option<String>,
    pub overlay_active: bool,
    /// Âncora do pie em coordenadas globais do desktop (soma de todos os
    /// displays). Cada janela de overlay converte para seu espaço local.
    pub overlay_anchor: Option<(f32, f32)>,
    /// A âncora veio de fonte aproximada. O primeiro mouse_move real
    /// confirma a posição exata e limpa o flag.
    pub anchor_needs_confirm: bool,
    /// Geração de ativação do overlay (para reiniciar animações de show).
    pub overlay_seq: u64,
    /// Fechando com fade-out: continua renderizando por ~110ms.
    pub overlay_fading: bool,
    pub overlay_fade_start: u128,
    pub pie_hovered: Option<usize>,
    /// Cursor sobre o botão central (stop): destaque + ação.
    pub center_hovered: bool,
    pub play_request: Option<String>,
    /// Pedido de parar o áudio (botão central do pie).
    pub stop_request: bool,
    pub confirm_request: bool,
    pub show_hint: bool,
    pub search: String,
    pub search_focused: bool,
    /// Ctrl+A na busca da biblioteca (ver `browse_selected`).
    pub search_selected: bool,
    pub volume: f32,
    /// Mudo (toggle no ícone de som): zera o ganho sem perder `volume`.
    pub muted: bool,
    /// Volume a restaurar ao desmutar.
    pub pre_mute_volume: f32,
    pub density: String,
    pub sort: String,
    pub sort_open: bool,
    /// Tema: "system" | "light" | "dark".
    pub theme: String,
    pub lang: String,
    pub lang_open: bool,
    pub about_open: bool,
    /// Fechando: overlay continua montado tocando o fade reverso.
    pub about_closing: bool,
    pub about_anim_start: u128,
    pub mic_pass: bool,
    pub hear_clips: bool,
    pub settings_open: bool,
    /// Fechando: overlay continua montado tocando a animação reversa.
    pub settings_closing: bool,
    pub settings_anim_start: u128,
    /// Nomes fora dos favoritos. Vazio = tudo favorito (padrão).
    pub unfavorited: HashSet<String>,
    /// Toolbar: mostrar só favoritos.
    pub only_favorites: bool,
    /// Card em edição via dialog "Edit sound": rascunhos + estado do picker.
    pub editor_open: bool,
    /// Fechando: overlay continua montado tocando o fade reverso.
    pub editor_closing: bool,
    pub editor_anim_start: u128,
    /// Nome do arquivo em edição (chave da metadata).
    pub editor_name: String,
    pub editor_display: String,
    pub editor_emoji: String,
    /// Aba ativa do picker (label do grupo) + busca de emoji.
    pub editor_group: String,
    pub editor_emoji_query: String,
    pub editor_name_focused: bool,
    pub editor_search_focused: bool,
    pub editor_name_selected: bool,
    pub editor_search_selected: bool,
    /// Arquivos externos sendo arrastados sobre a janela.
    pub drop_active: bool,
    /// Drawer "Browse sounds online" (MyInstants).
    pub browse_open: bool,
    /// Fechando: overlay continua montado tocando a animação reversa.
    pub browse_closing: bool,
    pub browse_anim_start: u128,
    /// Busca online: texto, resultados, busca em curso e geração
    /// (respostas antigas de Enters rápidos são descartadas).
    pub browse_query: String,
    pub browse_focused: bool,
    /// Última query enviada ao Enter: só ela autoriza o estado
    /// "sem resultados" (digitar sem Enter mostra o convite, não erro).
    pub browse_searched: String,
    /// Ctrl+A com o input focado: texto todo selecionado (destaque +
    /// Backspace apaga tudo, digitar substitui).
    pub browse_selected: bool,
    pub browse_results: Vec<OnlineSound>,
    pub browse_searching: bool,
    pub browse_search_id: u64,
    pub browse_error: Option<String>,
    /// Id em download e id em preview (para o estado por linha).
    pub browse_downloading: Option<String>,
    pub browse_previewing: Option<String>,
    /// Preview ainda baixando o mp3 (cobre a janela entre o clique e o play).
    pub browse_preview_loading: bool,
    /// Quando o play do preview começou: o tick só limpa `browse_previewing`
    /// 1s depois (cobre o vão entre `loading=false` e `playing=Some`).
    pub browse_preview_started_ms: u128,
    /// Progresso do playback (barra do player): duração total, offset de
    /// seek e instante de início. `elapsed = offset + (now - start)/1000`.
    /// `seek_request` é consumido pela thread de playback (pulo imediato).
    /// `play_peaks` é a forma de onda (ver `WAVEFORM_BARS`).
    pub play_duration_secs: Option<f32>,
    pub play_offset_secs: f32,
    pub play_start_ms: u128,
    pub seek_request: Option<f32>,
    pub play_peaks: Vec<f32>,
    pub version: u64,
}

impl Shared {
    pub fn new(settings: &Settings) -> Self {
        Self {
            sounds: vec![],
            playing: None,
            shortcut: settings.shortcut.clone(),
            shortcut_rebinding: false,
            mic: "inicializando…".into(),
            mic_source: settings.mic_source.clone(),
            mic_sources: vec![],
            mic_open: false,
            last_error: None,
            overlay_active: false,
            overlay_anchor: None,
            anchor_needs_confirm: false,
            overlay_seq: 0,
            overlay_fading: false,
            overlay_fade_start: 0,
            pie_hovered: None,
            center_hovered: false,
            play_request: None,
            stop_request: false,
            confirm_request: false,
            show_hint: true,
            search: String::new(),
            search_focused: false,
            search_selected: false,
            volume: settings.volume.clamp(0.0, 1.0),
            muted: false,
            pre_mute_volume: settings.volume.clamp(0.0, 1.0),
            density: settings.density.clone(),
            sort: settings.sort.clone(),
            sort_open: false,
            theme: settings.theme.clone(),
            lang: settings.language.clone(),
            lang_open: false,
            about_open: false,
            about_closing: false,
            about_anim_start: 0,
            mic_pass: settings.mic_passthrough,
            hear_clips: settings.hear_clips,
            settings_open: false,
            settings_closing: false,
            settings_anim_start: 0,
            unfavorited: HashSet::new(),
            only_favorites: false,
            editor_open: false,
            editor_closing: false,
            editor_anim_start: 0,
            editor_name: String::new(),
            editor_display: String::new(),
            editor_emoji: String::new(),
            editor_group: String::new(),
            editor_emoji_query: String::new(),
            editor_name_focused: false,
            editor_search_focused: false,
            editor_name_selected: false,
            editor_search_selected: false,
            drop_active: false,
            browse_open: false,
            browse_closing: false,
            browse_anim_start: 0,
            browse_query: String::new(),
            browse_focused: false,
            browse_searched: String::new(),
            browse_selected: false,
            browse_results: vec![],
            browse_searching: false,
            browse_search_id: 0,
            browse_error: None,
            browse_downloading: None,
            browse_previewing: None,
            browse_preview_loading: false,
            browse_preview_started_ms: 0,
            play_duration_secs: None,
            play_offset_secs: 0.0,
            play_start_ms: 0,
            seek_request: None,
            play_peaks: vec![],
            version: 1,
        }
    }

    /// Marca o estado como sujo para forçar `cx.notify()` no próximo tick.
    pub fn bump(&mut self) {
        self.version += 1;
    }

    /// Sons favoritos (o overlay mostra só estes): tudo menos `unfavorited`.
    pub fn favorite_sounds(&self) -> Vec<Sound> {
        self.sounds
            .iter()
            .filter(|s| !self.unfavorited.contains(&s.name))
            .cloned()
            .collect()
    }

    /// Posição atual do playback (elapsed, duration) em segundos.
    /// `None` quando parado ou sem duração conhecida.
    pub fn playback_pos(&self, now_ms: u128) -> Option<(f32, f32)> {
        let duration = self.play_duration_secs.filter(|d| *d > 0.0)?;
        if self.playing.is_none() {
            return None;
        }
        let elapsed = if self.seek_request.is_some() {
            // Seek pendente: mostra o alvo de imediato (sem esperar a thread).
            self.seek_request.unwrap_or(self.play_offset_secs)
        } else {
            let wall = now_ms.saturating_sub(self.play_start_ms) as f32 / 1000.0;
            self.play_offset_secs + wall
        };
        Some((elapsed.clamp(0.0, duration), duration))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::Settings;

    fn sound(name: &str) -> Sound {
        Sound {
            name: name.into(),
            path: std::path::PathBuf::from(name),
            duration_secs: None,
            modified_secs: 0,
            display: None,
            emoji: "♪".into(),
        }
    }

    #[test]
    fn overlay_mostra_so_favoritos() {
        let mut s = Shared::new(&Settings::default());
        s.sounds = vec![sound("a"), sound("b"), sound("c")];
        assert_eq!(s.favorite_sounds().len(), 3);
        s.unfavorited.insert("b".into());
        let fav: Vec<_> = s
            .favorite_sounds()
            .iter()
            .map(|x| x.name.clone())
            .collect();
        assert_eq!(fav, vec!["a".to_string(), "c".to_string()]);
    }

    #[test]
    fn playback_pos_sem_play_ou_duracao_e_none() {
        let mut s = Shared::new(&Settings::default());
        assert_eq!(s.playback_pos(1000), None);
        s.playing = Some("a".into());
        assert_eq!(s.playback_pos(1000), None);
        s.play_duration_secs = Some(0.0);
        assert_eq!(s.playback_pos(1000), None);
    }

    #[test]
    fn playback_pos_avanca_com_wall_clock_e_prende_no_total() {
        let mut s = Shared::new(&Settings::default());
        s.playing = Some("a".into());
        s.play_duration_secs = Some(10.0);
        s.play_offset_secs = 2.0;
        s.play_start_ms = 1000;
        assert_eq!(s.playback_pos(3000), Some((4.0, 10.0)));
        // Passou do fim: prende em 10.
        assert_eq!(s.playback_pos(20000), Some((10.0, 10.0)));
    }

    #[test]
    fn playback_pos_mostra_alvo_do_seek_pendente() {
        let mut s = Shared::new(&Settings::default());
        s.playing = Some("a".into());
        s.play_duration_secs = Some(10.0);
        s.play_offset_secs = 1.0;
        s.play_start_ms = 1000;
        s.seek_request = Some(8.0);
        assert_eq!(s.playback_pos(1500), Some((8.0, 10.0)));
    }
}
