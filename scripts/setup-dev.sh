#!/bin/bash
set -euo pipefail

if [[ $(uname -s) != Linux ]]; then
    printf '%s\n' 'Run this script on Debian 13 Linux.' >&2
    exit 1
fi
# shellcheck source=/dev/null
. /etc/os-release
if [[ ${ID:-} != debian || ${VERSION_ID:-} != 13 ]]; then
    printf '%s\n' 'Debian 13 is required; no packages were changed.' >&2
    exit 1
fi
root=()
if (( EUID != 0 )); then
    root=(sudo)
fi
"${root[@]}" apt-get update
"${root[@]}" apt-get install -y --no-install-recommends \
    build-essential pkg-config rustc cargo rustfmt rust-clippy meson ninja-build curl libpam0g-dev \
    libpulse-dev libgtk-4-dev libadwaita-1-dev libgtk4-layer-shell-dev libglib2.0-dev-bin \
    libpolkit-agent-1-dev libpolkit-gobject-1-dev \
    libwayland-dev wayland-protocols wayland-utils blueprint-compiler \
    gettext libgettextpo-dev wayfire xwayland dbus-daemon dbus-user-session \
    xdg-desktop-portal xdg-desktop-portal-wlr xdg-desktop-portal-gtk \
    fonts-inter fonts-jetbrains-mono adwaita-icon-theme librsvg2-common \
    wlsunset swayidle grim slurp foot wlr-randr \
    libgtk-4-media-gstreamer gstreamer1.0-plugins-good libarchive-tools \
    firefox-esr-l10n-tr locales flatpak wlrctl wtype \
    shellcheck desktop-file-utils libxml2-utils python3 ca-certificates git libc-l10n
printf '%s\n' 'Debian Rust is installed in /usr/bin. Existing rustup settings were left intact.'
/usr/bin/rustc --version
/usr/bin/cargo --version
pkg-config --modversion gtk4 libadwaita-1 gtk4-layer-shell-0
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
chmod +x "$repo"/scripts/*.sh "$repo/scripts/asena-session"
bash "$repo/scripts/compile-data.sh"

bash "$repo/scripts/backport-layer-shell.sh"
printf '%s\n' 'Before direct Cargo commands: source scripts/native-env.sh'
