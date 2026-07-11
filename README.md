# Klipp

Klipp is a desktop soundboard with a quick-overlay selector, sound imports,
and optional PipeWire/PulseAudio virtual microphone routing.

## Build a local Flatpak installer

The Flatpak package is intentionally distributed outside Flathub. It is an
`x86_64` installer with the stable application ID
`io.github.ErnestoMuniz.Klipp`.

### Prerequisites

- Linux on `x86_64`
- Flatpak and `flatpak-builder`
- Node.js and pnpm (the repository pins pnpm through Vite+)
- The Flathub remote, used only to download the Electron/FreeDesktop runtimes

On Fedora, install the build prerequisites with:

```sh
sudo dnf install flatpak flatpak-builder
```

On Debian or Ubuntu, install them with:

```sh
sudo apt install flatpak flatpak-builder
```

If the Flathub remote is not configured, add it before building or installing:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
```

Install JavaScript dependencies and create the installer:

```sh
vp install
vp run build:flatpak
```

If you use pnpm directly, `pnpm build:flatpak` runs the same package script.

The resulting bundle is written to `release/Klipp-<version>-x86_64.flatpak`.
Temporary Flatpak Builder files are kept under `release/` and are removed by
the packaging tool after a successful build.

## Install and run

```sh
flatpak install --user ./release/Klipp-<version>-x86_64.flatpak
flatpak run io.github.ErnestoMuniz.Klipp
```

To uninstall the app while keeping its data, run:

```sh
flatpak uninstall io.github.ErnestoMuniz.Klipp
```

Klipp stores its Flatpak data under
`~/.var/app/io.github.ErnestoMuniz.Klipp/` and keeps it across uninstall and
reinstall unless it is explicitly removed.

## Permissions and desktop support

This first package prioritizes the existing Klipp feature set over a minimal
sandbox. It requests network access for online sound search/imports, display
and GPU access for Electron, PulseAudio/PipeWire access for playback, and tray
and notification D-Bus access.

Klipp also has broad device access for its native keyboard listener and can
invoke the host `pactl` through Flatpak's host bridge. Those permissions are
needed to preserve the global quick-overlay shortcut and virtual microphone
audio graph. The package is explicitly tested for X11. On Wayland, Electron's
Global Shortcuts portal is used where the desktop supports it; direct keyboard
hooks may be unavailable, in which case Klipp continues with Electron's
shortcut fallback.

The host needs a `pactl` command compatible with PulseAudio or PipeWire-Pulse
for virtual microphone routing. If it is unavailable, Klipp remains usable as
a normal soundboard and reports the audio-routing failure in the app.
