# Klipp: Plano de migração Electron → Rust + GPUI

> Decisões registradas em 2026-09-02. O código antigo fica em `../klipp-old` como
> referência; este repositório (`klipp`) é o novo app em Rust.

## 1. Contexto

`klipp-old` é uma soundboard de desktop para Linux escrita em Electron
(React 19 + Tailwind 4 no renderer, TypeScript no main process). O app:

- Importa e gerencia clips de áudio próprios
- Busca e importa sons do myinstants.com
- Abre um seletor rápido (overlay radial) com atalho global
- Roteia sons para um microfone virtual (`soundboard-mic` via PipeWire/pactl)
- Tem tray, i18n (pt-BR/en), tema claro/escuro/sistema, Flatpak

O objetivo é reescrever com paridade total de recursos em Rust + GPUI.

### Decisões fechadas

| Tema | Decisão |
|---|---|
| Escopo | **Paridade total** de recursos |
| Dados do usuário | **Começar do zero** — novo diretório, usuário reimporta sons |
| Distribuição | **Flatpak** (manter pipeline de release existente) |
| GPUI | **Oficial no crates.io** (`gpui` + `gpui_platform`), pinado; pre-1.0 com breaking changes |
| Áudio | **pactl (graph) + rodio/symphonia (playback)** |
| Plataforma | **Wayland-first**; X11 fica para um milestone futuro |
| Overlay | Janela on-demand criada/destruída por ativação (sem workaround do Electron) |

## 2. Mapeamento Electron → Rust

| Responsabilidade | Hoje (Electron) | No Rust | Crate |
|---|---|---|---|
| Janela principal (frameless, custom chrome) | `BrowserWindow` | `open_window` + titlebar none | `gpui` |
| Overlay (transparente, sempre-no-topo) | `BrowserWindow` por display, persistente, click-through | `open_window` on-demand: LayerShell `Layer::Overlay` no Wayland; fallback `PopUp` fullscreen | `gpui` |
| Atalho global (segurar → abre, soltar → confirma) | `globalShortcut` (portal) + uiohook (X11) | Wayland: portal GlobalShortcuts via ashpd (`Activated`/`Deactivated`); X11 depois: `global-hotkey` (XGrabKey) + `rdev` (KeyRelease) | `ashpd` (+ `global-hotkey`, `rdev` no futuro) |
| Tray | `Tray` + `Menu` | StatusNotifierItem via DBus | `ksni` |
| File dialogs, abrir pasta, notificações, abrir URL | `dialog`, `shell` | Nativos na `Platform` trait (`prompt_for_paths`, `reveal_path`, `open_with_system`, `show_system_notification`) | `gpui` |
| Gráfico de áudio (mic virtual) | `execFile("pactl", ...)` | Idêntico — comandos pactl | `std::process` |
| Playback (volume/mute/stop) | `HTMLAudioElement` + env `PULSE_SINK` | `rodio` (cpal + symphonia); rotas para sink `klipp-clips` (PULSE_SINK ou fallback `pactl move-sink-input`) | `rodio` |
| Watch + imports de arquivos | `fs.watch` + copy | `notify` + `std::fs` | `notify` |
| Scraping myinstants | `fetch` + regex | `reqwest` (async via executor do GPUI) + `regex` | `reqwest`, `regex` |
| Settings/shortcut/window-state/prefs | JSON no userData + localStorage | serde + JSON em XDG config | `serde`, `dirs` |
| Sons + metadados | `userData/sounds` | `~/.local/share/klipp/sounds/` + `sound-metadata.json` | — |
| i18n | TS pt-BR/en | Mapas const idênticos | — |
| Icons (lucide) | npm | SVGs embutidos no binário | `include_dir` |

### Caminhos de dados (app novo, começar do zero)

- Config: `~/.config/klipp/` (settings.json, shortcut.json, window-state.json, prefs.json)
- Dados: `~/.local/share/klipp/sounds/` + `sound-metadata.json`
- No Flatpak, o sandbox redireciona para `~/.var/app/io.github.ErnestoMuniz.Klipp/`
  (`config/klipp/` e `data/klipp/`) — manter o mesmo `appId` do old.
- Extensões suportadas na importação: mp3, wav, ogg, flac, m4a, opus, aac.
  **webm é aceito mas pode falhar no playback** — symphonia não decodifica
  matroska. Se incomodar, evoluir para gstreamer.

## 3. Arquitetura do overlay (Wayland-first, sem workaround do Electron)

### Por que o Electron usava janelas persistentes

Criar `BrowserWindow` + bootar renderer cromado leva centenas de ms, então o
app criava **uma janela invisível por display** e alternava click-through em
runtime. No GPUI a janela é nativa — abrir no keydown é barato (fração de ms),
o primeiro frame via wgpu é rápido.

### Abordagem nova — janela on-demand, criada e destruída por ativação

1. `Activated` do portal → `cx.open_window()`: janela fullscreen transparente
   no output onde o cursor está.
   - Wayland: `WindowKind::LayerShell` com `Layer::Overlay`,
     `KeyboardInteractivity::OnDemand`, `window_background: Transparent`.
   - Compositor sem LayerShell (ex.: GNOME/Mutter): o GPUI devolve
     `LayerShellNotSupportedError` → fallback: janela xdg fullscreen
     (`PopUp`, transparente, `focus: false`). Mesma UI, só muda a casca.
2. O cursor entra na janela → o compositor entrega pointer-enter com a posição
   atual → o pie menu é ancorado ali (Wl não permite consultar posição global
   do cursor; técnica já usada no old via `capturePointer`).
3. `Deactivated` do portal → confirma o slice sob o cursor (padrão
   segurar/soltar) ou esconde se não houver hover.
4. Click no slice → toca (fallback existente). Click fora / Escape /
   `Deactivated` sem hover → esconde.
5. Esconder = **destruir a janela**. Sem janelas persistentes, sem toggle de
   click-through, sem sincronização multi-display (o overlay vive só no
   output onde o cursor está — mesma UX do old).

### Semântica do atalho

- **Primário**: segurar (Activated) abre, soltar (Deactivated) confirma —
  melhor que o Electron, que no Flatpak só tinha fallback de click.
- **Contingência** (se Deactivated for instável em algum compositor): abre no
  press, seleção via click do mouse. Já é o comportamento provado do old no
  Flatpak.

### X11 (milestone futuro, pós-1.0)

- Overlay: `PopUp` fullscreen transparente + `_NET_WM_STATE_ABOVE` via raw
  window handle (`HasWindowHandle`); posição do cursor via x11rb.
- Atalho: `global-hotkey` (XGrabKey) + `rdev` (KeyRelease) para o padrão
  segurar/soltar.

## 4. Stack de áudio

### Gráfico (inalterado, via pactl CLI)

- `module-null-sink` `klipp-clips` (clips) e `klipp-mix` (mix)
- `module-remap-source` `soundboard-mic` (mic virtual para Discord)
- `module-loopback`: clips→mix (sempre), mic→mix (passthrough, opcional),
  clips→default sink (hear-clips, opcional)
- Trackear módulos carregados (`loaded`/`existed`) e desfazer no cleanup —
  port direto do `AudioManager` do old
- Dentro do Flatpak o runtime freedesktop já traz pactl (comprovado no old)

### Playback (rodio)

- `rodio` = cpal (backend PulseAudio/PipeWire) + symphonia (decoder 100% Rust)
- Roteamento para `klipp-clips`: idealmente env `PULSE_SINK=klipp-clips`
  (mesma técnica do Chromium); **validar no spike** se o backend pulse do cpal
  honra. Fallback determinístico: criar stream no sink padrão e imediatamente
  `pactl move-sink-input <id> klipp-clips`.
- Volume/mute são gain no app (igual ao `element.volume` atual).

## 5. Fases

### Fase 0 — Spike técnico

Valida as 5 decisões antes de escrever UI:

1. **Overlay on-demand**: janela LayerShell criada no keydown — tempo de
   abertura, render do primeiro frame, destruição, fallback
   `LayerShellNotSupportedError` → PopUp.
2. **Atalho**: ashpd `GlobalShortcuts` (Activated/Deactivated) integrado ao
   executor do GPUI; recorder de atalho (captura a combinação e persiste).
3. **Playback → klipp-clips**: validar `PULSE_SINK` com cpal; validar fallback
   `move-sink-input`.
4. **Emoji** no pie menu via font-kit.
5. **Tray** ksni + dialogs nativos (`prompt_for_paths`) rodando no loop do GPUI.

Entregável: decisões de arquitetura documentadas (overlay, roteamento).

### Fase 1 — Esqueleto

- Crate `klipp`, janela principal, entity `App`
- Persistência: settings/shortcut/window-state/prefs em XDG config
- Single-instance, comportamento close → hide/quit (`runInBackground`),
  ciclo de vida (SIGINT/SIGTERM, cleanup)

### Fase 2 — Áudio

- Port do `AudioManager` (ensureGraph, módulos, mic sources, passthrough,
  hear-clips, cleanup)
- Playback rodio com volume/mute/stop
- Watcher da pasta de sons, metadata/import/delete

### Fase 3 — Soundboard UI (maior bloco)

Tema claro/escuro/sistema, topbar com botões de janela, toolbar
(busca/ordenação/densidade/overlay-only), grid de pads, drag & drop, editor,
settings drawer (mic, atalho, run-in-background), transport bar,
empty/error/about.

### Fase 4 — Overlay radial

- Janela on-demand (arquitetura da seção 3)
- Pie menu SVG (port do `QuickOverlay`: paging, hover, confirmação por
  Deactivated ou click)
- Fallback PopUp para compositores sem LayerShell, detectado em runtime

### Fase 5 — Atalho global + tray

- Integração da Fase 0.2: recorder na UI, persistência, re-bind em runtime
- Tray ksni: mostrar/sair

### Fase 6 — myinstants + i18n

- Port do scraper (regex), UI de busca/preview/import (`OnlineSounds`)
- i18n pt-BR + en aplicado em toda a UI

### Fase 7 — Flatpak

- Manifest com freedesktop SDK 24.08 + extensão rust-stable
- Testes no sandbox (atalho via portal, pavucontrol, áudio)
- Manter pipeline de release do GitHub Pages do old

### Milestone futuro

- X11: overlay com `_NET_WM_STATE_ABOVE`, XGrabKey + rdev, posição do cursor

## 6. Riscos

| Risco | Mitigação |
|---|---|
| GPUI pre-1.0 (breaking changes entre versões) | Pinar versão; upgrades em commits isolados |
| LayerShell não existe em todo compositor (GNOME não suporta) | `LayerShellNotSupportedError` → fallback PopUp; spike valida |
| `Deactivated` do portal pode ter latência/instabilidade | Fallback click-to-select já provado no old |
| `PULSE_SINK` pode não ser honrado pelo cpal | Fallback `move-sink-input` é garantido |
| webm não decodifica no symphonia | Aceitar na importação, erro no playback; evoluir p/ gstreamer se incomodar |
| Renderização Linux do GPUI (wgpu desde fev/2026) | Spike valida em GPU do dev (NVIDIA/AMD) |

## 7. Ponto de avaliação

Após a Fase 2 (núcleo: janela + áudio + atalho): se estiver bom, o resto é
volume de port de UI. Decidir aí se vale seguir.

## 8. Estrutura de diretórios proposta (repositório `klipp`)

```
klipp/
├── Cargo.toml
├── assets/            # logo, tray, SVGs lucide embutidos
├── packaging/         # manifest + scripts Flatpak
├── src/
│   ├── main.rs        # entrypoint, ciclo de vida
│   ├── app.rs         # estado global, singleton, janela principal
│   ├── settings.rs    # settings/shortcut/window-state/prefs (serde + XDG)
│   ├── shortcut.rs    # portal GlobalShortcuts, recorder
│   ├── hotkeys.rs     # (futuro) X11 XGrabKey + rdev
│   ├── tray.rs        # ksni
│   ├── myinstants.rs  # scraper
│   ├── i18n/          # pt-BR, en
│   ├── audio/
│   │   ├── graph.rs   # pactl: sinks, remap, loopbacks, cleanup
│   │   ├── playback.rs# rodio/symphonia, rota p/ klipp-clips
│   │   └── library.rs # watcher, metadata, import/delete
│   └── ui/
│       ├── soundboard/  # port dos componentes do old
│       └── overlay/     # pie menu, janela on-demand
```

## 9. Guia de referência (código-fonte no old)

| Funcionalidade | Arquivo no `klipp-old` |
|---|---|
| Ciclo de vida, janela, tray, overlays, IPC | `electron/main.ts` |
| AudioManager, protocolo de som, IPC de áudio | `electron/audio.ts` |
| Atalho (parse/persist) | `electron/shortcut.ts` |
| Settings do app | `electron/appSettings.ts` |
| Estado da janela | `electron/windowState.ts` |
| Scraper myinstants | `electron/myinstants.ts` |
| API do renderer | `electron/preload.ts` |
| Soundboard | `src/components/Soundboard/*` |
| Overlay radial | `src/components/QuickOverlay/QuickOverlay.tsx` + `src/Overlay.css` |
| i18n | `src/i18n/*` |