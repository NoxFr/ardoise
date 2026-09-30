#!/usr/bin/env bash
# Installe Ardoise (binaire, icône, lanceur) selon la spécification XDG, sans droits root.
set -euo pipefail

dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bin_dir="${HOME}/.local/bin"
icon_dir="${HOME}/.local/share/icons/hicolor/256x256/apps"
apps_dir="${HOME}/.local/share/applications"

install -Dm755 "${dir}/ardoise" "${bin_dir}/ardoise"
install -Dm644 "${dir}/icon.png" "${icon_dir}/ardoise.png"
install -Dm644 "${dir}/ardoise.desktop" "${apps_dir}/ardoise.desktop"

command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -qf "${HOME}/.local/share/icons/hicolor" || true
command -v update-desktop-database >/dev/null && update-desktop-database -q "${apps_dir}" || true

echo "Ardoise installé. Ajoute ${bin_dir} à ton PATH si besoin, puis lance-le depuis le menu ou avec \`ardoise\`."
