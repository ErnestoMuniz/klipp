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
        "settings.shortcut_hint" => "Global shortcut {shortcut} opens the quick picker · Esc closes.",
        "settings.discord_group" => "Discord device",
        "settings.discord_intro" => "Set the",
        "settings.discord_input" => "input device",
        "settings.discord_to" => "to",
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
        "settings.shortcut_hint" => "Atalho global {shortcut} abre o seletor rápido · Esc fecha.",
        "settings.discord_group" => "Dispositivo no Discord",
        "settings.discord_intro" => "Defina o",
        "settings.discord_input" => "dispositivo de entrada",
        "settings.discord_to" => "como",
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
        _ => "",
    }
}
