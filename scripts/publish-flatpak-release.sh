#!/usr/bin/env bash
set -euo pipefail

command -v gh >/dev/null || {
  echo "GitHub CLI (gh) is required: https://cli.github.com/" >&2
  exit 1
}

version="$(node -p "require('./package.json').version")"
bundle="release/Klipp-${version}-x86_64.flatpak"
tag="v${version}"

test -f "$bundle" || {
  echo "Missing $bundle; build it first with: vp run build:flatpak" >&2
  exit 1
}

gh release create "$tag" "$bundle" \
  --title "Klipp ${version}" \
  --generate-notes

echo "Published $tag. The Publish Flatpak repository workflow will update GitHub Pages."
