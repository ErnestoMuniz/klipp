#!/usr/bin/env bash
# Verifica que o AppImage publicado permite update delta via AppImageUpdate.
# Checa, para cada `Klipp-*-x86_64.AppImage` em DIR (default: packaging):
#   1. existe o `.zsync` lado a lado (sem ele não há update delta);
#   2. o AppImage tem update-info embutida
#      `gh-releases-zsync|owner|repo|canal|padrão` (via `strings`);
#   3. owner/repo batem com o remote `origin` (pega rename/fork errado);
#   4. o padrão (com `*`) casa com os arquivos gerados;
#   5. o cabeçalho do `.zsync` (Filename/Length/SHA-1/URL) bate com o `.AppImage`;
#   6. se `GITHUB_REF_NAME=vX.Y.Z` (CI de tag), a versão do Cargo.toml é X.Y.Z
#      (evita publicar release nova com o mesmo nome de arquivo da anterior).
# Uso:
#   ./packaging/appimage/check-update.sh [DIR]
set -euo pipefail

DIR="${1:-packaging}"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

fail() { echo "ERRO: $*" >&2; exit 1; }
ok() { echo "ok: $*"; }

[ -d "$DIR" ] || fail "diretório não existe: $DIR"
shopt -s nullglob
IMAGES=( "$DIR"/Klipp-*-x86_64.AppImage )
[ "${#IMAGES[@]}" -gt 0 ] || fail "nenhum Klipp-*-x86_64.AppImage em $DIR"

CARGO_VERSION="$(sed -n 's/^version *= *"\([^"]*\)"/\1/p' "$REPO_ROOT/Cargo.toml" | head -1)"
[ -n "$CARGO_VERSION" ] || fail "não achei version no Cargo.toml"

# Esperado: GITHUB_REF_NAME=v0.4.0 -> 0.4.0 deve ser igual ao Cargo.toml.
if [ -n "${GITHUB_REF_NAME:-}" ] && [[ "$GITHUB_REF_NAME" == v* ]]; then
  TAG_VERSION="${GITHUB_REF_NAME#v}"
  [ "$TAG_VERSION" = "$CARGO_VERSION" ] \
    || fail "tag $GITHUB_REF_NAME diverge do Cargo.toml ($CARGO_VERSION) — o arquivo teria o mesmo nome da release anterior e o update quebraria"
  ok "tag $GITHUB_REF_NAME == Cargo.toml ($CARGO_VERSION)"
fi

# owner/repo esperados a partir do remote origin (se der para descobrir).
EXPECTED_OWNER=""; EXPECTED_REPO=""
if REMOTE_URL="$(git -C "$REPO_ROOT" remote get-url origin 2>/dev/null)"; then
  if [[ "$REMOTE_URL" =~ github\.com[/:]([^/]+)/([^/]+)(\.git)?$ ]]; then
    EXPECTED_OWNER="${BASH_REMATCH[1]}"
    EXPECTED_REPO="${BASH_REMATCH[2]%.git}"
  fi
fi

for IMG in "${IMAGES[@]}"; do
  BASE="$(basename "$IMG")"
  ZSYNC="$IMG.zsync"
  echo "==> $BASE"
  [ -f "$ZSYNC" ] || fail "falta o $ZSYNC lado a lado (sem ele não há update delta)"

  UPDATE_INFO="$(strings "$IMG" | grep -o 'gh-releases-zsync|[^"'"'"' ]*' | head -1 || true)"
  [ -n "$UPDATE_INFO" ] || fail "$BASE não tem update-info embutida (appimagetool -u não aplicado?)"

  IFS='|' read -r _ OWNER REPO CHANNEL PATTERN <<< "$UPDATE_INFO"
  [ -n "${OWNER:-}" ] && [ -n "${REPO:-}" ] && [ -n "${CHANNEL:-}" ] && [ -n "${PATTERN:-}" ] \
    || fail "update-info malformada: $UPDATE_INFO"
  ok "update-info: $UPDATE_INFO"

  if [ -n "$EXPECTED_OWNER" ]; then
    [ "$OWNER" = "$EXPECTED_OWNER" ] && [ "$REPO" = "$EXPECTED_REPO" ] \
      || fail "update-info aponta para $OWNER/$REPO mas o origin é $EXPECTED_OWNER/$EXPECTED_REPO"
    ok "owner/repo batem com origin ($OWNER/$REPO)"
  else
    echo "aviso: não deu para descobrir o remote origin; pulando checagem de owner/repo" >&2
  fi

  # O padrão com `*` precisa casar com o .zsync gerado (é assim que o
  # AppImageUpdate localiza o asset na release `latest`).
  # shellcheck disable=SC2053
  [[ "$(basename "$ZSYNC")" == $PATTERN ]] \
    || fail "padrão '$PATTERN' não casa com $(basename "$ZSYNC")"
  ok "padrão '$PATTERN' casa com $(basename "$ZSYNC")"

  # Cabeçalho do .zsync x arquivo real.
  Z_FILENAME="$(sed -n 's/^Filename: //p' "$ZSYNC" | head -1 | tr -d '\r')"
  Z_LENGTH="$(sed -n 's/^Length: //p' "$ZSYNC" | head -1 | tr -d '\r')"
  Z_SHA1="$(sed -n 's/^SHA-1: //p' "$ZSYNC" | head -1 | tr -d '\r')"
  Z_URL="$(sed -n 's/^URL: //p' "$ZSYNC" | head -1 | tr -d '\r')"
  ACTUAL_SIZE="$(stat -c%s "$IMG")"
  ACTUAL_SHA1="$(sha1sum "$IMG" | cut -d' ' -f1)"
  [ "$Z_FILENAME" = "$BASE" ] || fail ".zsync Filename '$Z_FILENAME' != '$BASE'"
  [ "$Z_LENGTH" = "$ACTUAL_SIZE" ] || fail ".zsync Length $Z_LENGTH != tamanho real $ACTUAL_SIZE"
  [ "$Z_SHA1" = "$ACTUAL_SHA1" ] || fail ".zsync SHA-1 não bate com o .AppImage (zsync velho?)"
  [ -n "$Z_URL" ] || fail ".zsync sem campo URL"
  ok ".zsync consistente (Filename/Length/SHA-1/URL)"
done

echo "==> update delta OK (${#IMAGES[@]} AppImage(s) em $DIR)"
