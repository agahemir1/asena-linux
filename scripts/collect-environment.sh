#!/bin/bash
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
mkdir -p "$repo/target/evidence"
{
    date -u +'%Y-%m-%dT%H:%M:%SZ'
    cat /etc/os-release
    uname -srmo
    command -v rustc cargo
    rustc --version
    cargo --version
    /usr/bin/rustc --version
    pkg-config --modversion gtk4 libadwaita-1 gtk4-layer-shell-0
    wayfire --version
    apt-cache policy wayfire xwayland flatpak libgtk-4-dev libadwaita-1-dev libgtk4-layer-shell-dev \
        rustc cargo greetd cage xdg-desktop-portal-wlr xdg-desktop-portal-gtk \
        blueprint-compiler fonts-inter fonts-jetbrains-mono wlsunset swayidle grim slurp \
        nautilus ptyxis gnome-console gnome-text-editor loupe papers evince celluloid \
        gnome-calculator gnome-software gnome-software-plugin-flatpak \
        gnome-system-monitor file-roller firefox-esr
} > "$repo/target/evidence/environment.txt" 2>&1
printf '%s\n' "$repo/target/evidence/environment.txt"
printf '%s\n' 'Capture nested protocol globals with ASENA_CAPTURE_PROTOCOLS=1 scripts/dev-session.sh'
