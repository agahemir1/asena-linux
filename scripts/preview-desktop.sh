#!/bin/bash
# Local VM preview only. This never installs or replaces the normal session.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/native-env.sh
. "$repo/scripts/native-env.sh"
cd "$repo"
# shellcheck source=scripts/private-runtime.sh
. "$repo/scripts/private-runtime.sh"
preview=$repo/target/preview
mkdir -p "$preview"
is_running() {
    [[ -f $preview/service.pid ]] || return 1
    local service_pid
    service_pid=$(cat "$preview/service.pid")
    [[ $service_pid =~ ^[0-9]+$ && -r /proc/$service_pid/cmdline ]] || return 1
    tr '\0' ' ' <"/proc/$service_pid/cmdline" | grep -Fq 'preview-desktop.sh serve'
}
case ${1:-start} in
    stop)
        if is_running; then kill -TERM "$(cat "$preview/service.pid")"; fi
        for _ in {1..100}; do if ! is_running; then exit 0; fi; sleep 0.1; done
        printf '%s\n' 'The preview did not stop.' >&2
        exit 1
        ;;
    status)
        if is_running && [[ -f $preview/ready ]]; then printf '%s\n' 'Desktop preview is running.'; exit 0; fi
        printf '%s\n' 'Desktop preview is stopped.'; exit 1 ;;
    start)
        if is_running; then printf '%s\n' 'Desktop preview is already running.'; exit 0; fi
        for program in wayvnc websockify wayfire; do command -v "$program" >/dev/null; done
        bash scripts/compile-data.sh
        cargo build --workspace --locked
        rm -f "$preview/ready"
        setsid env LOCPATH="$repo/target/locale-data" LC_ALL=tr_TR.UTF-8 LANGUAGE=tr TZ=Europe/Istanbul \
            bash "$repo/scripts/preview-desktop.sh" serve >"$preview/service.log" 2>&1 </dev/null &
        printf '%s\n' "$!" >"$preview/service.pid"
        for _ in {1..150}; do
            if [[ -f $preview/ready ]]; then printf '%s\n' 'Desktop preview is ready.'; exit 0; fi
            if ! is_running; then cat "$preview/service.log"; exit 1; fi
            sleep 0.1
        done
        tail -n 20 "$preview/service.log"
        exit 1 ;;
    serve) ;;
    *) printf '%s\n' 'Usage: preview-desktop.sh [start|stop|status]' >&2; exit 2 ;;
esac
temporary=$(mktemp -d /tmp/asena-preview.XXXXXX)
session_pid='' vnc_pid='' web_pid='' terminal_pid='' settings_pid=''
cleanup() {
    trap - EXIT
    for child in "$web_pid" "$vnc_pid" "$terminal_pid" "$settings_pid"; do [[ -z $child ]] || kill "$child" 2>/dev/null || true; done
    [[ -z $session_pid ]] || kill -TERM -- "-$session_pid" 2>/dev/null || true
    for child in "$web_pid" "$vnc_pid" "$session_pid"; do [[ -z $child ]] || wait "$child" 2>/dev/null || true; done
    rm -f "$preview/ready" "$preview/service.pid"
    remove_private_runtime "$temporary"
}
trap cleanup EXIT
trap 'exit 0' TERM INT
mkdir -p "$temporary/runtime" "$preview/config" "$preview/state"
chmod 700 "$temporary/runtime"
original_runtime=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
export PULSE_SERVER=${PULSE_SERVER:-unix:$original_runtime/pulse/native}
if [[ -S $original_runtime/bus ]]; then ln -s "$original_runtime/bus" "$temporary/runtime/bus"; fi
# Screen capture portals and audio clients use the user's PipeWire.
if [[ -S $original_runtime/pipewire-0 ]]; then ln -s "$original_runtime/pipewire-0" "$temporary/runtime/pipewire-0"; fi
export XDG_RUNTIME_DIR=$temporary/runtime XDG_CONFIG_HOME=$preview/config XDG_STATE_HOME=$preview/state
# The Asena session's own Wayfire runs headless; wayvnc shows its first output.
# ASENA_PREVIEW_OUTPUTS=2 adds a second output for display layout checks.
outputs=${ASENA_PREVIEW_OUTPUTS:-1}
[[ $outputs =~ ^[1-4]$ ]] || { printf '%s
' 'ASENA_PREVIEW_OUTPUTS must be 1 to 4.' >&2; exit 2; }
export GSK_RENDERER=cairo WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=$outputs
env -u WAYLAND_DISPLAY setsid bash scripts/dev-session.sh >"$preview/session.log" 2>&1 &
session_pid=$!
socket=''
for _ in {1..300}; do
    for candidate in "$XDG_RUNTIME_DIR"/wayland-*; do
        if [[ -S $candidate ]]; then socket=${candidate##*/}; break; fi
    done
    [[ -n $socket ]] && break
    kill -0 "$session_pid"
    sleep 0.1
done
[[ -n $socket ]] || exit 1
export WAYLAND_DISPLAY=$socket
shell_pid=''
for _ in {1..150}; do
    shell_pid=$(pgrep -n -f '/target/debug/asena-shell$' || true)
    if [[ -n $shell_pid ]] && tr '\0' '\n' <"/proc/$shell_pid/environ" | grep -Fx "XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR" >/dev/null; then break; fi
    shell_pid=''
    sleep 0.1
done
[[ -n $shell_pid ]] || exit 1
export DBUS_SESSION_BUS_ADDRESS GSETTINGS_SCHEMA_DIR=$repo/target/schemas
DBUS_SESSION_BUS_ADDRESS=$(tr '\0' '\n' <"/proc/$shell_pid/environ" | sed -n 's/^DBUS_SESSION_BUS_ADDRESS=//p')
# Preview clients use the session's data, locale and desktop identity.
while IFS= read -r -d '' variable; do
    case $variable in
        XDG_DATA_DIRS=*|ASENA_LOCALE_DIR=*|XDG_CURRENT_DESKTOP=*|XDG_SESSION_TYPE=*|GDK_BACKEND=*|DISPLAY=*|PATH=*)
            export "${variable?}" ;;
    esac
done <"/proc/$shell_pid/environ"
gsettings set org.asenalinux.power idle-enabled false 2>/dev/null || true
# Same window buttons for third-party GTK applications as the packaged override.
gsettings set org.gnome.desktop.wm.preferences button-layout 'appmenu:minimize,maximize,close' 2>/dev/null || true
gsettings set org.asenalinux.desktop background-uri 'file:///usr/share/backgrounds/asena/asena-gece.jpg' 2>/dev/null || true
gsettings set org.asenalinux.desktop background-mode 'zoom' 2>/dev/null || true
if [[ $(gsettings get org.asenalinux.shell pinned-apps) == '@as []' ]]; then
    gsettings set org.asenalinux.shell pinned-apps "['org.asenalinux.Files.desktop', 'firefox-esr.desktop', 'org.asenalinux.Settings.desktop', 'foot.desktop']"
fi
foot >"$preview/terminal.log" 2>&1 &
terminal_pid=$!
target/debug/asena-settings >"$preview/settings.log" 2>&1 &
settings_pid=$!
wayvnc --render-cursor -o HEADLESS-1 127.0.0.1 5900 >"$preview/wayvnc.log" 2>&1 &
vnc_pid=$!
websockify --web=/usr/share/novnc 0.0.0.0:6080 127.0.0.1:5900 >"$preview/web.log" 2>&1 &
web_pid=$!
sleep 1
kill -0 "$vnc_pid" "$web_pid" "$session_pid"
touch "$preview/ready"
while kill -0 "$session_pid" 2>/dev/null; do
    sleep 2
done
