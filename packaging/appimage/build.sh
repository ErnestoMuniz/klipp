#!/usr/bin/env bash
# Monta o AppImage do Klipp (sem sandbox: cursor/D-Bus do host funcionam).
# Uso:
#   ./packaging/appimage/build.sh            # gera packaging/Klipp-<ver>-x86_64.AppImage
# Pré-requisito: toolchain Rust + appimagetool (baixado sozinho para /tmp se faltar).
# Updates incrementais: o .zsync embutido permite `AppImageUpdate` delta a
# partir do GitHub Releases (basta subir .AppImage + .zsync juntos na tag).
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$(dirname "$(dirname "$HERE")")"
PKG_DIR="$(dirname "$HERE")"
OUT_DIR="$PKG_DIR"
WORK="$APP_DIR/.appimage-build"
APPDIR="$WORK/AppDir"

VERSION="$(sed -n 's/^version *= *"\([^"]*\)"/\1/p' "$APP_DIR/Cargo.toml" | head -1)"
ARCH="$(uname -m)"
APP_ID="io.github.ErnestoMuniz.Klipp"
BIN="klipp"

if [ -z "$VERSION" ]; then
  echo "não achei version no Cargo.toml" >&2
  exit 1
fi

APPIMAGE_TOOL="${APPIMAGETOOL:-/tmp/appimagetool-x86_64.AppImage}"
if [ ! -x "$APPIMAGE_TOOL" ]; then
  echo "==> baixando appimagetool..."
  curl -sSL -o "$APPIMAGE_TOOL" \
    https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$APPIMAGE_TOOL"
fi

echo "==> cargo build --release..."
cargo build --release --manifest-path "$APP_DIR/Cargo.toml"

echo "==> montando AppDir..."
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/metainfo" "$APPDIR/usr/share/klipp/fonts"

install -m0755 "$APP_DIR/target/release/$BIN" "$APPDIR/usr/bin/$BIN"
install -m0644 "$PKG_DIR/$APP_ID.desktop" "$APPDIR/$APP_ID.desktop"
install -m0644 "$PKG_DIR/$APP_ID.desktop" "$APPDIR/usr/share/applications/$APP_ID.desktop"
install -m0644 "$PKG_DIR/$APP_ID.metainfo.xml" "$APPDIR/usr/share/metainfo/$APP_ID.metainfo.xml"
install -m0644 "$APP_DIR/assets/fonts/NotoColorEmoji.ttf" "$APPDIR/usr/share/klipp/fonts/NotoColorEmoji.ttf"
for s in 16 32 48 64 128 256 512; do
  dest="$APPDIR/usr/share/icons/hicolor/${s}x${s}/apps"
  mkdir -p "$dest"
  install -m0644 "$PKG_DIR/icons/hicolor/${s}x${s}/apps/$APP_ID.png" "$dest/$APP_ID.png"
done
# Ícone de topo exigido pelo appimagetool (+ .DirIcon).
install -m0644 "$PKG_DIR/icons/hicolor/256x256/apps/$APP_ID.png" "$APPDIR/$APP_ID.png"
cp "$APPDIR/$APP_ID.png" "$APPDIR/.DirIcon"
install -m0755 "$HERE/AppRun" "$APPDIR/AppRun"

OUT="$OUT_DIR/Klipp-${VERSION}-${ARCH}.AppImage"
rm -f "$OUT" "${OUT}.zsync"
echo "==> appimagetool -> $OUT"
# UPDATE_INFORMATION permite update delta via AppImageUpdate a partir da release.
# (-u embute no binário; com zsyncmake instalado também gera o .zsync lado a lado.)
UPDATE_INFORMATION="gh-releases-zsync|ErnestoMuniz|klipp|latest|Klipp-*-${ARCH}.AppImage.zsync"
"$APPIMAGE_TOOL" -u "$UPDATE_INFORMATION" "$APPDIR" "$OUT"
chmod +x "$OUT"
# O appimagetool cospe o .zsync no CWD: move para junto do .AppImage.
for z in ./*.zsync "$APP_DIR"/*.zsync; do
  [ -f "$z" ] || continue
  case "$z" in "$OUT_DIR"/*) ;; *) mv -f "$z" "$OUT_DIR/" ;; esac
done
ls -la "$OUT" "$OUT_DIR"/Klipp-*.zsync 2>/dev/null || true
echo "==> ok: $OUT"
echo "    rode: ./$(basename "$OUT")"
echo "    atalho global no KDE: Settings do app -> Atalho global (nativo, sem portal)"
