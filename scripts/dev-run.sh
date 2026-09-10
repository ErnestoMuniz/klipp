#!/usr/bin/env bash
# Dev run: faz o xdg-desktop-portal aceitar o klipp como app com app-id
# (sem sandbox o portal recusa "An app id is required").
#
# O portal identifica o chamador pelo app-id. Fora de sandbox ele não
# consegue inferir, então o frontend recusa as APIs que exigem identidade
# (seletor de arquivos e atalho global). Aqui usamos o gancho de teste do
# próprio xdg-desktop-portal (`host` + `XDG_DESKTOP_PORTAL_TEST_HOST_APPID`)
# — mais simples e estável que forjar metadados de flatpak (o formato antigo
# derruba o xdg-desktop-portal >= 1.20 em `g_desktop_app_info_new`).
#
# Em produção (AppImage) o atalho global usa atalho custom do DE.
set -euo pipefail

APP_ID="io.github.ErnestoMuniz.Klipp"

# Binário: $KLIPP_BIN tem prioridade; senão debug (dev padrão), senão release.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${KLIPP_BIN:-}"
if [[ -z "$BIN" ]]; then
  for candidate in "$ROOT/target/debug/klipp" "$ROOT/target/release/klipp"; do
    if [[ -x "$candidate" ]]; then
      BIN="$candidate"
      break
    fi
  done
fi
if [[ -z "$BIN" || ! -x "$BIN" ]]; then
  echo "klipp: binário não encontrado — rode 'cargo build' ou defina KLIPP_BIN" >&2
  exit 1
fi

systemctl --user set-environment XDG_DESKTOP_PORTAL_TEST_APP_INFO_KIND=host
systemctl --user set-environment XDG_DESKTOP_PORTAL_TEST_HOST_APPID="$APP_ID"
systemctl --user reset-failed xdg-desktop-portal 2>/dev/null || true
systemctl --user stop xdg-desktop-portal xdg-desktop-portal-kde 2>/dev/null || true

cleanup() {
  systemctl --user unset-environment XDG_DESKTOP_PORTAL_TEST_APP_INFO_KIND \
    XDG_DESKTOP_PORTAL_TEST_HOST_APPID || true
  systemctl --user reset-failed xdg-desktop-portal 2>/dev/null || true
  # Restart (não start): o portal foi reativado durante o run já com o
  # app-id falso; sem reiniciar, o app-id vaza para as próximas sessões.
  systemctl --user restart xdg-desktop-portal 2>/dev/null || true
}
trap cleanup EXIT
trap 'exit 130' INT TERM

sleep 1
echo "klipp: dev-run com app-id '$APP_ID' ($BIN)" >&2
RUST_LOG=info,zbus=warn,zbus::proxy=error,wgpu=warn,usvg=error,tracing::span=off,open_gpui_sum_tree=off,gpui_sum_tree=off,sum_tree=off \
  "$BIN" "$@"
