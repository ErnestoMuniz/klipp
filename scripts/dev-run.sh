#!/usr/bin/env bash
# Dev run: faz o xdg-desktop-portal aceitar o klipp como app com app-id
# (fora do Flatpak o portal recusa "An app id is required").
# Em produção (Flatpak) isso não é necessário.
set -euo pipefail

FAKE_INFO="$(mktemp)"
cat > "$FAKE_INFO" <<'EOF'
[Application]
name=io.github.ErnestoMuniz.Klipp

[Instance]
instance-id=fake
app-path=/
runtime-path=/

[Context]
shared=network;ipc;
EOF

systemctl --user set-environment XDG_DESKTOP_PORTAL_TEST_APP_INFO_KIND=flatpak
systemctl --user set-environment XDG_DESKTOP_PORTAL_TEST_FLATPAK_METADATA="$FAKE_INFO"
systemctl --user stop xdg-desktop-portal xdg-desktop-portal-kde 2>/dev/null || true

cleanup() {
  systemctl --user unset-environment XDG_DESKTOP_PORTAL_TEST_APP_INFO_KIND || true
  systemctl --user unset-environment XDG_DESKTOP_PORTAL_TEST_FLATPAK_METADATA || true
  systemctl --user start xdg-desktop-portal 2>/dev/null || true
  rm -f "$FAKE_INFO"
}
trap cleanup EXIT

sleep 1
exec env RUST_LOG=info,zbus=warn,zbus::proxy=error,wgpu=warn,usvg=error,tracing::span=off,open_gpui_sum_tree=off,gpui_sum_tree=off,sum_tree=off "$(dirname "$0")/../target/debug/klipp" "$@"