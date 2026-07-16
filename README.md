# Klipp

Klipp is a desktop soundboard for Linux. Import your own audio, organize it in
one place, and play sounds through a quick overlay without interrupting what
you are doing.

## Features

- Import and manage your own sound clips
- Search for and import sounds online
- Open a quick sound selector with a global shortcut
- Play sounds through your normal audio output
- Route sounds through an optional virtual microphone
- System tray and desktop notification support
- Automatic updates through the official Flatpak repository

## Install

Klipp is distributed as a Flatpak for `x86_64` Linux systems.

First, make sure [Flatpak](https://flatpak.org/setup/) is installed. Then install
Klipp from its official repository:

```sh
flatpak install --user \
  https://ernestomuniz.github.io/klipp/klipp.flatpakref
```

The `.flatpakref` file adds the signed Klipp repository and installs the app.
Flatpak may also download the required FreeDesktop and Electron runtimes from
Flathub.

Launch Klipp from your desktop's application menu or run:

```sh
flatpak run io.github.ErnestoMuniz.Klipp
```

## Update

Updates are delivered through the Klipp Flatpak repository. Install all
available Flatpak updates with:

```sh
flatpak update
```

You can update only Klipp with:

```sh
flatpak update io.github.ErnestoMuniz.Klipp
```

## Virtual microphone support

Virtual microphone routing requires a host `pactl` command compatible with
PulseAudio or PipeWire-Pulse. Klipp remains usable as a normal soundboard when
this is unavailable.

Global shortcuts and direct keyboard hooks depend on desktop support. Klipp is
tested on X11 and uses Electron's shortcut fallback where direct hooks are not
available, including some Wayland environments.

## Uninstall

```sh
flatpak uninstall io.github.ErnestoMuniz.Klipp
```

Application data is kept under `~/.var/app/io.github.ErnestoMuniz.Klipp/` unless
you explicitly choose to remove it.

## License

Copyright (C) 2026 Ernesto Muniz.

Klipp, including versions published before the addition of this notice, is
licensed under the [GNU General Public License version 3](LICENSE). See the
license for the terms under which you may use, modify, and redistribute the
software. Klipp is provided without any warranty.
