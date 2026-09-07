#!/usr/bin/env bash
# Build + instala o Flatpak do Klipp localmente.
# Uso:
#   ./packaging/build.sh            # build + install --user
#   ./packaging/build.sh --bundle   # além disso, gera .flatpak em packaging/
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$(dirname "$HERE")"
MANIFEST="$HERE/io.github.ErnestoMuniz.Klipp.yml"
BUILD_DIR="$APP_DIR/.flatpak-build"
REPO_DIR="$HOME/.local/share/flatpak-test-repo"

need() { command -v "$1" >/dev/null 2>&1 || { echo "faltando: $1" >&2; return 1; }; }

# 1. dependências de build -------------------------------------------------
if ! need flatpak-builder; then
  echo "flatpak-builder não encontrado. Instale com:" >&2
  echo "  sudo dnf install -y flatpak-builder" >&2
  exit 1
fi

echo "==> garantindo SDK 25.08 + extensão rust-stable (user)..."
flatpak install -y --user flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08 || true
flatpak install -y --user flathub org.freedesktop.Sdk.Extension.rust-stable//25.08 || true

# 2. regenera cargo-sources.json a partir do Cargo.lock --------------------
if [ "$APP_DIR/Cargo.lock" -nt "$HERE/cargo-sources.json" ]; then
  echo "==> Cargo.lock mudou, regenerando cargo-sources.json..."
  GEN="/tmp/flatpak-cargo-generator.py"
  if [ ! -f "$GEN" ]; then
    curl -sSL -o "$GEN" https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
  fi
  # uv já existe nesta máquina; fallback: python3 com tomlkit+aiohttp
  if command -v uv >/dev/null 2>&1; then
    uv run --with tomlkit --with aiohttp python "$GEN" "$APP_DIR/Cargo.lock" -o "$HERE/cargo-sources.json"
  else
    python3 "$GEN" "$APP_DIR/Cargo.lock" -o "$HERE/cargo-sources.json"
  fi
else
  echo "==> cargo-sources.json atualizado, pulando regeneração."
fi

# 3. build -----------------------------------------------------------------
echo "==> flatpak-builder..."
flatpak-builder --user --install --force-clean "$BUILD_DIR" "$MANIFEST"

echo "==> ok. Rode com: flatpak run io.github.ErnestoMuniz.Klipp"

if [ "${1:-}" = "--bundle" ]; then
  OUT="$HERE/Klipp-0.1.0-x86_64.flatpak"
  echo "==> gerando bundle $OUT ..."
  mkdir -p "$REPO_DIR"
  flatpak-builder --repo="$REPO_DIR" --force-clean "$BUILD_DIR" "$MANIFEST"
  flatpak build-bundle "$REPO_DIR" "$OUT" io.github.ErnestoMuniz.Klipp
  echo "==> bundle pronto: $OUT"
fi
