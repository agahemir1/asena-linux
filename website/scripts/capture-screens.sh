#!/bin/bash
# Development VM only. Runs the Asena session headless with a separate demo
# home and takes the website screenshots with grim. The user's own home,
# settings and normal session are not touched.
#
#   capture-screens.sh start tr|en    start the session at 3840x2160, scale 2
#   capture-screens.sh run CMD...      run CMD inside the session's environment
#   capture-screens.sh shot NAME       save target/site-capture/shots/NAME.png
#   capture-screens.sh stop
#
# Demo content is real data: the wallpapers in Pictures, project documents,
# and a few notes and calendar entries written here.
set -euo pipefail
# Under tr_TR the bracket range [a-z] leaves out "i"; clients get their locale from the session.
export LC_ALL=C.UTF-8
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
work=$repo/target/site-capture
env_file=$work/session.env
real_home=$HOME

# Notifications and file dialogs print the home path; a short path under /home
# reads like a normal account. Without passwordless sudo the work path is used.
home_path() {
    local lang=$1 short
    if [[ $lang == tr ]]; then short=/home/kullanici; else short=/home/user; fi
    if [[ -L $short || ! -e $short ]] && sudo -n ln -sfn "$work/home-$lang" "$short" 2>/dev/null; then
        printf '%s\n' "$short"
    else
        printf '%s\n' "$work/home-$lang"
    fi
}

prepare_home() {
    local lang=$1 home=$work/home-$1
    local desktop documents downloads music pictures videos
    if [[ $lang == tr ]]; then
        desktop=Masaüstü documents=Belgeler downloads=İndirilenler music=Müzik pictures=Resimler videos=Videolar
    else
        desktop=Desktop documents=Documents downloads=Downloads music=Music pictures=Pictures videos=Videos
    fi
    rm -rf -- "$home"
    mkdir -p "$home/.config" "$home/.local/share/asena/notes" "$home/.local/share/asena/calendar" \
        "$home/.local/share/asena/clock"
    for dir in "$desktop" "$documents" "$downloads" "$music" "$pictures" "$videos"; do mkdir -p "$home/$dir"; done
    cat >"$home/.config/user-dirs.dirs" <<EOF
XDG_DESKTOP_DIR="\$HOME/$desktop"
XDG_DOCUMENTS_DIR="\$HOME/$documents"
XDG_DOWNLOAD_DIR="\$HOME/$downloads"
XDG_MUSIC_DIR="\$HOME/$music"
XDG_PICTURES_DIR="\$HOME/$pictures"
XDG_VIDEOS_DIR="\$HOME/$videos"
EOF
    cp "$repo"/data/wallpapers/*.jpg "$home/$pictures/"
    # Applications the VM has for development but the image does not ship (iso/config/package-lists).
    mkdir -p "$home/.local/share/applications"
    for entry in org.gnome.Nautilus org.gnome.Ptyxis vim; do
        printf '%s\n' '[Desktop Entry]' 'Type=Application' "Name=$entry" 'Hidden=true' \
            >"$home/.local/share/applications/$entry.desktop"
    done
    # Flathub is a user remote of the VM account; the store shows it as configured there.
    if [[ -d $real_home/.local/share/flatpak ]]; then ln -s "$real_home/.local/share/flatpak" "$home/.local/share/flatpak"; fi
    local month
    month=$(date +%Y%m)
    if [[ $lang == tr ]]; then
        cp "$repo/docs/VISION.md" "$home/$documents/Vizyon.md"
        cp "$repo/data/branding/os-release" "$home/$documents/os-release"
        printf '%s\n' 'Kitap listesi' '' '- Dede Korkut Kitabı' '- Divanü Lügati'"'"'t-Türk' '- Orhun Yazıtları üzerine bir inceleme' \
            >"$home/.local/share/asena/notes/kitap-listesi.md"
        printf '%s\n' 'Pazar alışverişi' '' '- Domates, biber' '- Beyaz peynir' '- Simit' \
            >"$home/.local/share/asena/notes/pazar.md"
        printf '%s\n' 'Europe/Istanbul' 'Europe/Berlin' 'Asia/Tokyo' 'America/New_York' \
            >"$home/.local/share/asena/clock/world-clocks.txt"
        write_calendar "$home" "$month" 'Cumhuriyet Bayramı' 'Ekip toplantısı' 'Kadıköy'
    else
        cp "$repo/docs/VISION.md" "$home/$documents/Vision.md"
        cp "$repo/data/branding/os-release" "$home/$documents/os-release"
        printf '%s\n' 'Reading list' '' '- The Book of Dede Korkut' '- Orkhon inscriptions, a study' \
            >"$home/.local/share/asena/notes/reading-list.md"
        printf '%s\n' 'Groceries' '' '- Tomatoes, peppers' '- White cheese' '- Simit' \
            >"$home/.local/share/asena/notes/groceries.md"
        printf '%s\n' 'Europe/London' 'Europe/Istanbul' 'Asia/Tokyo' 'America/New_York' \
            >"$home/.local/share/asena/clock/world-clocks.txt"
        write_calendar "$home" "$month" 'Republic Day (Türkiye)' 'Team meeting' 'Kadıköy'
    fi
    (cd "$home/$pictures" && bsdtar -a -cf "$home/$documents/$( [[ $lang == tr ]] && echo duvar-kagitlari || echo wallpapers).zip" -- *.jpg)
}

write_calendar() {
    local home=$1 month=$2 holiday=$3 meeting=$4 place=$5 year=${2:0:4}
    printf '%s\r\n' 'BEGIN:VCALENDAR' 'VERSION:2.0' 'PRODID:-//Asena//Calendar//TR' \
        'BEGIN:VEVENT' 'UID:site-1' "SUMMARY:$holiday" "DTSTART;VALUE=DATE:${year}1029" 'END:VEVENT' \
        'BEGIN:VEVENT' 'UID:site-2' "SUMMARY:$meeting" "LOCATION:$place" "DTSTART:${month}24T100000" \
        "DTEND:${month}24T110000" 'END:VEVENT' 'END:VCALENDAR' >"$home/.local/share/asena/calendar/events.ics"
}

case ${1:-} in
    start)
        lang=${2:-tr}
        [[ $lang == tr || $lang == en ]] || { printf '%s\n' 'Language must be tr or en.' >&2; exit 2; }
        [[ ! -f $env_file ]] || { printf '%s\n' 'A capture session is already running; stop it first.' >&2; exit 1; }
        mkdir -p "$work/shots"
        prepare_home "$lang"
        runtime=$(mktemp -d /tmp/asena-site.XXXXXX)
        chmod 700 "$runtime"
        original_runtime=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
        if [[ -S $original_runtime/pipewire-0 ]]; then ln -s "$original_runtime/pipewire-0" "$runtime/pipewire-0"; fi
        home=$(home_path "$lang")
        if [[ $lang == tr ]]; then locale_vars=(LOCPATH="$repo/target/locale-data" LC_ALL=tr_TR.UTF-8 LANGUAGE=tr); else locale_vars=(LC_ALL=C.UTF-8 LANGUAGE=en); fi
        # Same launch as scripts/dev-session.sh, without its cargo build: the VM
        # may be building other work, and the shots use the binaries already built.
        [[ -x $repo/target/debug/asena-session ]] || { printf '%s\n' 'Build the workspace first.' >&2; exit 1; }
        config_dir=$(mktemp -d /tmp/asena-site-config.XXXXXX)
        cp "$repo/data/wayfire/wayfire.ini" "$config_dir/wayfire.ini"
        render_device=''
        for candidate in /dev/dri/by-path/pci-*-card; do
            if [[ -e $candidate ]]; then render_device=$(readlink -f -- "$candidate"); break; fi
        done
        # shellcheck source=scripts/native-env.sh
        . "$repo/scripts/native-env.sh"
        setsid env "${locale_vars[@]}" TZ=Europe/Istanbul HOME="$home" \
            XDG_RUNTIME_DIR="$runtime" XDG_CONFIG_HOME="$home/.config" XDG_DATA_HOME="$home/.local/share" \
            XDG_STATE_HOME="$home/.local/state" XDG_CACHE_HOME="$home/.cache" \
            XDG_DATA_DIRS="$home/.local/share/flatpak/exports/share:/var/lib/flatpak/exports/share:/usr/local/share:/usr/share" \
            PULSE_SERVER="unix:$original_runtime/pulse/native" GSK_RENDERER=cairo \
            WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=1 WLR_RENDERER_ALLOW_SOFTWARE=1 \
            ${render_device:+WLR_RENDER_DRM_DEVICE="$render_device"} \
            ASENA_SOURCE_DIR="$repo" ASENA_DEV=1 ASENA_DEV_CONFIG="$config_dir/wayfire.ini" \
            dbus-run-session -- "$repo/target/debug/asena-session" --dev >"$work/session.log" 2>&1 </dev/null &
        session_pid=$!
        shell_pid=''
        for _ in {1..600}; do
            for candidate in $(pgrep -f '/target/debug/asena-shell$' || true); do
                if tr '\0' '\n' <"/proc/$candidate/environ" 2>/dev/null | grep -Fqx "XDG_RUNTIME_DIR=$runtime"; then
                    shell_pid=$candidate
                fi
            done
            [[ -n $shell_pid ]] && break
            kill -0 "$session_pid" || { cat "$work/session.log"; exit 1; }
            sleep 0.2
        done
        [[ -n $shell_pid ]] || { tail -n 30 "$work/session.log"; exit 1; }
        {
            printf 'session_pid=%q\n' "$session_pid"
            printf 'runtime=%q\n' "$runtime"
            printf 'config_dir=%q\n' "$config_dir"
            while IFS= read -r -d '' variable; do
                case $variable in
                    WAYLAND_DISPLAY=*|DBUS_SESSION_BUS_ADDRESS=*|XDG_*=*|HOME=*|LC_ALL=*|LANGUAGE=*|TZ=*|PATH=*|ASENA_*=*|GDK_BACKEND=*|GSETTINGS_SCHEMA_DIR=*|LOCPATH=*|PULSE_SERVER=*|GSK_RENDERER=*)
                        printf 'export %q\n' "$variable" ;;
                esac
            done <"/proc/$shell_pid/environ"
        } >"$env_file"
        # shellcheck source=/dev/null
        . "$env_file"
        wlr-randr --output HEADLESS-1 --custom-mode 3840x2160 --scale 2
        gsettings set org.asenalinux.power idle-enabled false
        gsettings set org.asenalinux.desktop background-uri "file://$repo/data/wallpapers/asena-gece.jpg"
        gsettings set org.asenalinux.shell pinned-apps \
            "['org.asenalinux.Files.desktop', 'firefox-esr.desktop', 'org.asenalinux.Store.desktop', 'org.asenalinux.Settings.desktop']"
        # The headless output draws the pointer into the frame; park it off the top-right corner.
        wlrctl pointer move 4000 -3000 || true
        gsettings set org.gnome.desktop.wm.preferences button-layout 'appmenu:minimize,maximize,close'
        printf '%s\n' "Capture session ($lang) is running."
        ;;
    run)
        shift
        # shellcheck source=/dev/null
        . "$env_file"
        exec "$@"
        ;;
    shot)
        name=${2:?shot needs a name}
        [[ $name =~ ^[a-z0-9-]+$ ]] || { printf '%s\n' 'Use lowercase letters, digits and dashes.' >&2; exit 2; }
        # shellcheck source=/dev/null
        . "$env_file"
        # Wayfire reloads its configuration whenever the daemon rewrites it and
        # the headless output then falls back to scale 1.
        if ! wlr-randr | grep -Fq 'Scale: 2.000000'; then
            wlr-randr --output HEADLESS-1 --custom-mode 3840x2160 --scale 2
            sleep 3
        fi
        grim -l 1 "$work/shots/$name.png"
        printf '%s\n' "$work/shots/$name.png"
        ;;
    stop)
        [[ -f $env_file ]] || exit 0
        # shellcheck source=/dev/null
        . "$env_file"
        kill -TERM -- "-$session_pid" 2>/dev/null || true
        for _ in {1..50}; do kill -0 "$session_pid" 2>/dev/null || break; sleep 0.1; done
        rm -rf -- "$runtime" "$config_dir" "$env_file"
        ;;
    *)
        printf '%s\n' 'Usage: capture-screens.sh start tr|en | run CMD... | shot NAME | stop' >&2
        exit 2
        ;;
esac
