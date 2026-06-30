/**
 * English message catalog — the primary language and fallback for the app.
 *
 * Keys are organised by feature area. Every other locale catalog must match
 * the shape of this object (see `pt-BR.ts`).
 */
export const en = {
  "app.fatalAudio": "The audio API is unavailable (preload didn't load). Restart the app.",
  "app.addSoundsAria": "Add sounds",
  "app.addSoundsTitle": "Add sounds",

  "toolbar.library": "Library",
  "toolbar.searchPlaceholder": "Search sounds…",
  "toolbar.searchAria": "Search sounds",
  "toolbar.clearSearch": "Clear search",
  "toolbar.densityAria": "Grid density",
  "toolbar.densityComfort": "Comfort view",
  "toolbar.densityCompact": "Compact view",
  "toolbar.sortLabel": "Sort",
  "toolbar.sortNameAsc": "Name (A-Z)",
  "toolbar.sortNameDesc": "Name (Z-A)",
  "toolbar.sortRecent": "Recent",
  "toolbar.overlayOnlyAria": "Picker only",
  "toolbar.overlayOnlyTitle": "Show only quick-picker sounds",
  "toolbar.browseOnlineAria": "Browse sounds online",
  "toolbar.browseOnlineTitle": "Browse and download sounds from myinstants.com",
  "toolbar.hideHints": "Hide hints",
  "toolbar.showHints": "Show hints",
  "toolbar.settingsAria": "Settings",

  "shortcut.hintPrefix": "Click a pad to play it. In any app, ",
  "shortcut.hintMiddle": " opens the quick picker · ",
  "shortcut.hintSuffix": " closes.",

  "transport.stopTitle": "Stop",
  "transport.idleTitle": "Not playing",
  "transport.stopAria": "Stop playback",
  "transport.idleAria": "Not playing",
  "transport.playing": "Playing",
  "transport.stopped": "Stopped",
  "transport.readyToPlay": "Ready to play",
  "transport.unmute": "Unmute",
  "transport.mute": "Mute",
  "transport.volumeAria": "Volume",

  "empty.searchTitle": "Nothing found",
  "empty.libraryTitle": "Your library is empty",
  "empty.searchBody": "No sounds match “{query}”.",
  "empty.libraryBody":
    "Add audio files with “Add”. They are stored in your user data folder and persist across updates.",
  "empty.addSounds": "Add sounds",

  "editor.title": "Edit sound",
  "editor.closeAria": "Close",
  "editor.overlayGroup": "In quick picker (overlay)",
  "editor.overlayLabel": "Available in the quick picker (global shortcut)",
  "editor.name": "Name",
  "editor.namePlaceholder": "Sound name",
  "editor.emoji": "Emoji",
  "editor.searchPlaceholder": "Search emojis…",
  "editor.searchAria": "Search emojis",
  "editor.noResults": "No emojis found",
  "editor.cancel": "Cancel",
  "editor.save": "Save",

  "pad.removeFromPicker": "Remove from quick picker",
  "pad.addToPicker": "Add to quick picker",
  "pad.edit": "Edit sound",
  "pad.playing": "Playing",
  "pad.play": "Play",

  "error.prefix": "Error:",
  "error.retry": "Try again",

  "overlay.aria": "Quick sound picker",
  "overlay.soundsAria": "Available sounds",
  "overlay.playAria": "Play {label}",
  "overlay.playing": "Playing",
  "overlay.soundboard": "Soundboard",
  "overlay.noSounds": "No sounds",
  "overlay.pick": "Pick a sound",
  "overlay.prevPage": "Previous page",
  "overlay.nextPage": "Next page",

  "settings.title": "Settings",
  "settings.closeAria": "Close settings",
  "settings.micPassthroughGroup": "Real microphone (pass-through)",
  "settings.micPassthroughLabel": "Pass my microphone through along with the sounds",
  "settings.micLabel": "Microphone",
  "settings.micNone": "No microphone found",
  "settings.monitoringGroup": "Monitoring",
  "settings.backgroundGroup": "Background",
  "settings.backgroundLabel": "Keep running in the background when the window is closed",
  "settings.backgroundHint":
    "When off, closing the main window quits Klipp instead of hiding it to the tray.",
  "settings.hearClipsLabel":
    "Hear the sounds in my headphones/speakers (just the sounds, not my voice)",
  "settings.themeGroup": "Theme",
  "settings.themeLight": "Light",
  "settings.themeDark": "Dark",
  "settings.themeSystem": "System",
  "settings.discordGroup": "Discord device",
  "settings.discordIntro": "Set the",
  "settings.discordInputDevice": "input device",
  "settings.discordTo": "to",
  "settings.globalShortcutHint":
    "Global shortcut {shortcut} opens the quick picker · {esc} closes.",
  "settings.shortcutGroup": "Global shortcut",
  "settings.shortcutRecordAria": "Record a new shortcut",
  "settings.shortcutRecording": "Press a key combination…",
  "settings.shortcutReset": "Reset to default",
  "settings.shortcutTaken": "That shortcut couldn't be registered. Try another combination.",
  "settings.shortcutInvalid": "That key combination isn't supported.",

  "settings.languageGroup": "Language",
  "settings.languageSystem": "System",
  "settings.languageEn": "English",
  "settings.languagePtBR": "Português (Brasil)",

  "online.title": "Browse sounds online",
  "online.closeAria": "Close online sounds",
  "online.searchPlaceholder": "Search myinstants.com…",
  "online.searchAria": "Search myinstants.com",
  "online.attribution": "Search results come from myinstants.com.",
  "online.hint":
    "Type a name and press Enter to search myinstants.com, then preview and download sounds to your library.",
  "online.preview": "Preview",
  "online.pause": "Stop preview",
  "online.download": "Download to library",
  "online.downloaded": "Added to your library",
  "online.loadMore": "Load more",
  "online.searching": "Searching…",
  "online.endOfResults": "End of results",
} as const;

export type TranslationKey = keyof typeof en;

/** Shape every locale catalog must satisfy. Values are plain strings so that
 * translated catalogs aren't pinned to the English literal types. */
export type Messages = Record<TranslationKey, string>;
