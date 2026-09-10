/// Catálogo i18n (EN + pt-BR), subconjunto do `klipp-old/src/i18n/locales`.
/// `Settings.language`: "system" | "en" | "pt-BR".
pub fn resolve(pref: &str) -> &'static str {
    match pref {
        "en" => "en",
        "pt-BR" => "pt-BR",
        _ => {
            if std::env::var("LANG")
                .map(|v| v.to_lowercase().starts_with("pt"))
                .unwrap_or(false)
            {
                "pt-BR"
            } else {
                "en"
            }
        }
    }
}

pub fn t(lang: &str, key: &str) -> String {
    let lang = resolve(lang);
    (if lang == "pt-BR" { pt(key) } else { en(key) }).to_string()
}

/// Substitui `{nome}` pelos pares dados.
pub fn t_fmt(lang: &str, key: &str, args: &[(&str, &str)]) -> String {
    let mut s = t(lang, key);
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

fn en(key: &str) -> &'static str {
    match key {
        "drop.label" => "Drop sounds to add them to your library",
        "toolbar.library" => "Library",
        "toolbar.search_ph" => "Search sounds…",
        "toolbar.clear" => "Clear search",
        "toolbar.sort_asc" => "Name (A-Z)",
        "toolbar.sort_desc" => "Name (Z-A)",
        "toolbar.sort_recent" => "Recent",
        "hint.prefix" => "Click a pad to play it. In any app, ",
        "hint.middle" => " opens the quick picker · ",
        "hint.suffix" => " closes.",
        "player.playing" => "Playing",
        "player.stopped" => "Stopped",
        "player.ready" => "Ready to play",
        "player.idle" => "No audio playing",
        "pad.play" => "Play",
        "pad.playing" => "Playing",
        "empty.title" => "Your library is empty",
        "empty.body" => "Add audio files with “Add”. They are stored in your user data folder and persist across updates.",
        "empty.add" => "Add sounds",
        "empty.none_title" => "Nothing found",
        "empty.none_body" => "No sounds match “{query}”.",
        "editor.name_ph" => "Sound name",
        "settings.title" => "Settings",
        "settings.mic_group" => "Real microphone (pass-through)",
        "settings.mic_label" => "Pass my microphone through along with the sounds",
        "settings.mic_none" => "No microphone found",
        "settings.mon_group" => "Monitoring",
        "settings.hear_label" => {
            "Hear the sounds in my headphones/speakers (just the sounds, not my voice)"
        }
        "settings.theme_group" => "Theme",
        "settings.theme_light" => "Light",
        "settings.theme_dark" => "Dark",
        "settings.theme_system" => "System",
        "settings.lang_group" => "Language",
        "settings.lang_system" => "System",
        "settings.lang_en" => "English",
        "settings.lang_pt" => "Português (Brasil)",
        "settings.shortcut_group" => "Global shortcut",
        "settings.shortcut_rec" => "Press a key combination…",
        "cursor.title" => "Exact cursor on GNOME",
        "cursor.body" => {
            "Klipp needs a tiny companion extension so the quick picker opens exactly at your cursor. One click to install — then log out of your session and back in once."
        }
        "cursor.install" => "Install extension",
        "cursor.installing" => "Installing…",
        "cursor.checking" => "Checking…",
        "cursor.login" => "Installed! Log out of your session and log back in to activate it.",
        "cursor.banner_login" => "Exact cursor pending: log out of your session and log back in once.",
        "cursor.banner_disabled" => "The cursor extension is installed but disabled.",
        "cursor.enable" => "Enable",
        "cursor.ok" => "OK",
        "settings.discord_group" => "Discord device",
        "settings.discord_intro" => "Set the",
        "settings.discord_input" => "input device",
        "settings.discord_to" => "to",
        "settings.tray_group" => "Background",
        "settings.tray_label" => "Keep running in the tray when the window is closed",
        "tray.show" => "Show Klipp",
        "tray.quit" => "Quit",
        "settings.about" => "About Klipp",
        "about.title" => "About Klipp",
        "about.version" => "Version {version}",
        "about.description" => {
            "A desktop soundboard that keeps your clips close and lets you play them anywhere through a quick overlay."
        }
        "about.creator" => "Created by",
        "about.platform" => "Platform",
        "about.tech" => "Built with",
        "about.github" => "View project on GitHub",
        "update.check" => "Check for updates",
        "update.checking" => "Checking…",
        "update.up_to_date" => "You're up to date.",
        "update.available" => "Version {version} available",
        "update.update_now" => "Update now",
        "update.downloading" => "Downloading {pct}…",
        "update.applying" => "Applying update…",
        "update.error" => "Update failed: {msg}",
        "update.get" => "Get version {version}",
        "editor.title" => "Edit sound",
        "editor.name" => "Name",
        "editor.emoji" => "Emoji",
        "editor.search_ph" => "Search emojis…",
        "editor.no_results" => "No emojis found",
        "editor.cancel" => "Cancel",
        "editor.save" => "Save",
        "browse.title" => "Browse sounds online",
        "browse.search_ph" => "Search myinstants…",
        "browse.source" => "Search results come from myinstants.com.",
        "browse.hint" => "Type to search sounds online.",
        "browse.enter_hint" => "Press Enter to search.",
        "browse.searching" => "Searching…",
        "browse.no_results" => "No sounds found for “{query}”.",
        "browse.downloading" => "Downloading…",
        "pick.title" => "Add sounds",
        "shortcut.portal_desc" => "Open the sound picker",
        "err.pick" => "File picker: {msg}",
        "err.pick_portal" => {
            "File picker unavailable: portal requires app-id — use scripts/dev-run.sh for dev"
        }
        "err.sink" => "Failed to connect to sink: {msg}",
        "err.shortcut" => "Shortcut: {msg}",
        "err.shortcut_taken" => "shortcut already in use — pick another combination",
        "err.shortcut_invalid" => "unsupported combination",
        "err.shortcut_portal" => {
            "Global shortcut unavailable: portal requires app-id — bind the AppImage in your desktop's custom shortcuts"
        }
        "err.shortcut_gnome" => {
            "GNOME shortcut: {msg} — add it manually in Settings → Keyboard → Custom Shortcuts with command {cmd}"
        }
        "err.shortcut_hyprland" => {
            "Hyprland sets the key in the compositor config. Add these lines to your Hyprland config (e.g. ~/.config/hypr/bindings.lua on Omarchy) and restart the session: {line}"
        }
        "err.shortcut_hyprland_restart" => {
            "Hyprland shortcut saved. Restart your session (log out and back in) to apply it."
        }
        "err.import_none" => "No valid audio files to import",
        "err.mic" => "Microphone: {msg}",
        _ => "",
    }
}

fn pt(key: &str) -> &'static str {
    match key {
        "drop.label" => "Solte os áudios para adicioná-los à biblioteca",
        "toolbar.library" => "Biblioteca",
        "toolbar.search_ph" => "Pesquisar áudios…",
        "toolbar.clear" => "Limpar pesquisa",
        "toolbar.sort_asc" => "Nome (A-Z)",
        "toolbar.sort_desc" => "Nome (Z-A)",
        "toolbar.sort_recent" => "Recentes",
        "hint.prefix" => "Clique num pad para o tocar. Em qualquer app, ",
        "hint.middle" => " abre o seletor rápido · ",
        "hint.suffix" => " fecha.",
        "player.playing" => "A tocar",
        "player.stopped" => "Parado",
        "player.ready" => "Pronto a reproduzir",
        "player.idle" => "Nenhum áudio tocando",
        "pad.play" => "Tocar",
        "pad.playing" => "A tocar",
        "empty.title" => "A sua biblioteca está vazia",
        "empty.body" => "Adicione ficheiros de áudio com “Adicionar”. Eles são guardados na pasta de dados do utilizador e persistem entre atualizações.",
        "empty.add" => "Adicionar áudios",
        "empty.none_title" => "Nada encontrado",
        "empty.none_body" => "Nenhum áudio corresponde a “{query}”.",
        "editor.name_ph" => "Nome do áudio",
        "settings.title" => "Definições",
        "settings.mic_group" => "Microfone real (pass-through)",
        "settings.mic_label" => "Passar o meu microfone junto com os áudios",
        "settings.mic_none" => "Nenhum microfone encontrado",
        "settings.mon_group" => "Monitorização",
        "settings.hear_label" => {
            "Ouvir os áudios nos meus fones/caixas (só os áudios, sem a minha voz)"
        }
        "settings.theme_group" => "Tema",
        "settings.theme_light" => "Claro",
        "settings.theme_dark" => "Escuro",
        "settings.theme_system" => "Sistema",
        "settings.lang_group" => "Idioma",
        "settings.lang_system" => "Sistema",
        "settings.lang_en" => "English",
        "settings.lang_pt" => "Português (Brasil)",
        "settings.shortcut_group" => "Atalho global",
        "settings.shortcut_rec" => "Pressione uma combinação…",
        "cursor.title" => "Cursor exato no GNOME",
        "cursor.body" => {
            "O Klipp precisa de uma extensão companion minúscula para o seletor rápido abrir exato no cursor. Um clique para instalar — depois saia da sua sessão e entre de novo."
        }
        "cursor.install" => "Instalar extensão",
        "cursor.installing" => "Instalando…",
        "cursor.checking" => "Verificando…",
        "cursor.login" => "Instalado! Saia da sua sessão e entre novamente para ativar.",
        "cursor.banner_login" => "Cursor exato pendente: saia da sua sessão e entre novamente.",
        "cursor.banner_disabled" => "A extensão do cursor está instalada, mas desativada.",
        "cursor.enable" => "Habilitar",
        "cursor.ok" => "OK",
        "settings.tray_group" => "Segundo plano",
        "settings.discord_group" => "Dispositivo no Discord",
        "settings.discord_intro" => "Defina o",
        "settings.discord_input" => "dispositivo de entrada",
        "settings.discord_to" => "como",
        "settings.tray_label" => "Continuar no tray ao fechar a janela",
        "tray.show" => "Mostrar Klipp",
        "tray.quit" => "Sair",
        "settings.about" => "Sobre o Klipp",
        "about.title" => "Sobre o Klipp",
        "about.version" => "Versão {version}",
        "about.description" => {
            "Um soundboard para desktop que mantém os seus áudios por perto e permite reproduzi-los em qualquer lugar através de um seletor rápido."
        }
        "about.creator" => "Criado por",
        "about.platform" => "Plataforma",
        "about.tech" => "Criado com",
        "about.github" => "Ver projeto no GitHub",
        "update.check" => "Verificar atualizações",
        "update.checking" => "Verificando…",
        "update.up_to_date" => "Você está atualizado.",
        "update.available" => "Versão {version} disponível",
        "update.update_now" => "Atualizar agora",
        "update.downloading" => "Baixando {pct}…",
        "update.applying" => "Aplicando atualização…",
        "update.error" => "Falha na atualização: {msg}",
        "update.get" => "Baixar versão {version}",
        "editor.title" => "Editar áudio",
        "editor.name" => "Nome",
        "editor.emoji" => "Emoji",
        "editor.search_ph" => "Pesquisar emojis…",
        "editor.no_results" => "Nenhum emoji encontrado",
        "editor.cancel" => "Cancelar",
        "editor.save" => "Salvar",
        "browse.title" => "Buscar áudios online",
        "browse.search_ph" => "Pesquisar no myinstants…",
        "browse.source" => "Os resultados vêm de myinstants.com.",
        "browse.hint" => "Digite para buscar áudios online.",
        "browse.enter_hint" => "Pressione Enter para buscar.",
        "browse.searching" => "Buscando…",
        "browse.no_results" => "Nenhum áudio para “{query}”.",
        "browse.downloading" => "Baixando…",
        "pick.title" => "Adicionar áudios",
        "shortcut.portal_desc" => "Abrir o seletor de sons",
        "err.pick" => "Seletor de arquivos: {msg}",
        "err.pick_portal" => {
            "Seletor indisponível: o portal exige app-id — use scripts/dev-run.sh no dev"
        }
        "err.sink" => "Falha ao conectar no sink: {msg}",
        "err.shortcut" => "Atalho: {msg}",
        "err.shortcut_taken" => "atalho já em uso — escolha outra combinação",
        "err.shortcut_invalid" => "combinação não suportada",
        "err.shortcut_portal" => {
            "Atalho global indisponível: o portal exige app-id — vincule o AppImage nos atalhos custom do sistema"
        }
        "err.shortcut_gnome" => {
            "Atalho do GNOME: {msg} — adicione à mão em Configurações → Teclado → Atalhos personalizados com o comando {cmd}"
        }
        "err.shortcut_hyprland" => {
            "No Hyprland a tecla é definida no config do compositor. Adicione estas linhas ao seu config (ex. ~/.config/hypr/bindings.lua no Omarchy) e reinicie a sessão: {line}"
        }
        "err.shortcut_hyprland_restart" => {
            "Atalho do Hyprland salvo. Reinicie a sessão (sair e entrar de novo) para aplicar."
        }
        "err.import_none" => "Nenhum arquivo de áudio válido para importar",
        "err.mic" => "Microfone: {msg}",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    /// Catálogo espelhado: toda chave existe nos dois idiomas.
    /// (`resolve` cai para EN quando a chave falta — sem esse teste a
    /// falta passa silenciosa.)
    #[test]
    fn catalogo_en_e_pt_tem_as_mesmas_chaves() {
        // Chaves conhecidas dos dois lados (amostra das novas chaves de erro).
        for key in [
            "pick.title",
            "shortcut.portal_desc",
            "err.pick",
            "err.pick_portal",
            "err.sink",
            "err.shortcut",
            "err.shortcut_taken",
            "err.shortcut_invalid",
            "err.shortcut_portal",
            "err.shortcut_gnome",
            "err.shortcut_hyprland",
            "err.shortcut_hyprland_restart",
            "cursor.title",
            "cursor.body",
            "cursor.install",
            "cursor.installing",
            "cursor.checking",
            "cursor.login",
            "cursor.banner_login",
            "cursor.banner_disabled",
            "cursor.enable",
            "cursor.ok",
            "err.import_none",
            "err.mic",
            "browse.downloading",
            "player.ready",
            "player.idle",
            "update.check",
            "update.checking",
            "update.up_to_date",
            "update.available",
            "update.update_now",
            "update.downloading",
            "update.applying",
            "update.error",
            "update.get",
        ] {
            assert_ne!(super::t("en", key), "", "falta em en: {key}");
            assert_ne!(super::t("pt-BR", key), "", "falta em pt-BR: {key}");
        }
        // Placeholders preservados na tradução.
        let msg = "x";
        for key in [
            "err.pick",
            "err.sink",
            "err.shortcut",
            "err.mic",
            "err.shortcut_gnome",
            "update.error",
        ] {
            for lang in ["en", "pt-BR"] {
                assert!(
                    super::t_fmt(lang, key, &[("msg", msg)]).contains(msg),
                    "{key} perdeu {{msg}} em {lang}"
                );
            }
        }
        for key in ["about.version", "update.available", "update.get"] {
            for lang in ["en", "pt-BR"] {
                assert!(
                    super::t_fmt(lang, key, &[("version", msg)]).contains(msg),
                    "{key} perdeu {{version}} em {lang}"
                );
            }
        }
        for lang in ["en", "pt-BR"] {
            assert!(
                super::t_fmt(lang, "err.shortcut_hyprland", &[("line", msg)]).contains(msg),
                "err.shortcut_hyprland perdeu {{line}} em {lang}"
            );
        }
    }
}
