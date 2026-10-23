#!/bin/bash
# Development VM only. Takes the website's screenshot set for one language with
# capture-screens.sh. Run from the repository root:
#   bash website/scripts/take-shots.sh tr
# Shots land in target/site-capture/shots; convert-screens.mjs (on any machine
# with the website's node_modules) turns them into website/src/assets/screens.
set -euo pipefail
export LC_ALL=C.UTF-8
lang=${1:?language: tr or en}
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
capture=$repo/website/scripts/capture-screens.sh
run() { bash "$capture" run "$@" 2>/dev/null; }
shot() { bash "$capture" shot "$lang-$1" >/dev/null; }
pids_of() {
    local runtime pid
    runtime=$(bash "$capture" run printenv XDG_RUNTIME_DIR)
    for pid in $(pgrep -f "$1" || true); do
        if tr '\0' '\n' <"/proc/$pid/environ" 2>/dev/null | grep -Fqx "XDG_RUNTIME_DIR=$runtime"; then printf '%s\n' "$pid"; fi
    done
}
close_apps() {
    local pid
    for pid in $(pids_of '(^|/)asena-(files|settings|text|notes|calendar|clock|calculator|photos|player|archive|screenshot|store)( |$)'); do
        kill "$pid" 2>/dev/null || true
    done
    sleep 1.5
}
surface() { # surface NAME TOGGLE
    run asenactl "$2" toggle
    sleep 2
    shot "$1"
    run asenactl "$2" toggle
    sleep 1
}
app() { # app NAME SECONDS COMMAND...
    local name=$1 wait=$2
    shift 2
    run setsid "$@" >/dev/null 2>&1 &
    sleep "$wait"
    shot "app-$name"
    close_apps
}
home=$(run printenv HOME)
if [[ $lang == tr ]]; then documents=$home/Belgeler; archive=duvar-kagitlari.zip; text=Vizyon.md
else documents=$home/Documents; archive=wallpapers.zip; text=Vision.md; fi

# The daemon may open Software to report the VM's last offline update.
close_apps
run gsettings set org.asenalinux.desktop color-scheme prefer-dark
run gsettings set org.asenalinux.desktop accent-color teal
run gsettings set org.asenalinux.desktop transparency true
run gsettings set org.asenalinux.shell panel-edge bottom
run gsettings set org.asenalinux.shell menu-layout compact
sleep 3

# Desktop surfaces over the empty desktop, so they cross-fade on the site.
shot desktop-dark
surface menu-dark menu
surface quick-dark quick-settings
run asenactl screenshot screen
sleep 3
surface notifications-dark notifications
run gsettings set org.asenalinux.shell menu-layout classic
sleep 2
surface menu-classic-dark menu
run gsettings set org.asenalinux.shell menu-layout grid
sleep 2
surface menu-grid-dark menu
run gsettings set org.asenalinux.shell menu-layout compact
run gsettings set org.asenalinux.desktop transparency false
sleep 3
surface menu-opaque-dark menu
run gsettings set org.asenalinux.desktop transparency true
sleep 3

# One application at a time, centred by Wayfire's place plugin.
app files 4 asena-files
app settings 5 asena-settings appearance
app text 4 asena-text "$documents/$text"
app notes 4 asena-notes
app calendar 4 asena-calendar
app clock 4 asena-clock
app calculator 4 asena-calculator
app photos 8 asena-photos
app player 4 asena-player
app archive 5 asena-archive "$documents/$archive"
app capture 4 asena-screenshot
app store 25 asena-store

# Accent colours: Settings > Appearance with the desktop folder behind it.
run setsid asena-settings appearance >/dev/null 2>&1 &
sleep 5
for accent in blue teal green yellow orange red pink purple slate; do
    run gsettings set org.asenalinux.desktop accent-color "$accent"
    sleep 3
    shot "accent-$accent"
done
run gsettings set org.asenalinux.desktop accent-color teal
close_apps

# Light style.
run gsettings set org.asenalinux.desktop color-scheme prefer-light
sleep 3
shot desktop-light
surface menu-light menu
surface quick-light quick-settings
app settings-light 5 asena-settings appearance
run gsettings set org.asenalinux.desktop color-scheme prefer-dark
sleep 3

# Panel edges. The menu is left closed: after a runtime edge change it stays at
# the bottom left (seen 23 October 2026, reported in docs/reports/website.md).
for edge in top left right; do
    run gsettings set org.asenalinux.shell panel-edge "$edge"
    sleep 4
    shot "panel-$edge"
done
run gsettings set org.asenalinux.shell panel-edge bottom
sleep 3
shopt -s nullglob
taken=("$repo/target/site-capture/shots/$lang-"*.png)
printf '%s shots\n' "${#taken[@]}"
