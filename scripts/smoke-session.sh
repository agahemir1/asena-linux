#!/bin/bash
# Disposable headless development session. No real config writes.
set -euo pipefail
[[ $(uname -s) == Linux ]] || exit 1
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/native-env.sh
. "$repo/scripts/native-env.sh"
cd "$repo"
# shellcheck source=scripts/private-runtime.sh
. "$repo/scripts/private-runtime.sh"
for program in wayfire foot grim wlr-randr gdbus; do command -v "$program" >/dev/null; done
bash scripts/compile-data.sh
cargo build --workspace --all-targets --locked
label=${GSK_RENDERER:-default}
case $label in
    ''|*[!abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-]*) exit 2 ;;
esac
temporary=$(mktemp -d /tmp/asena-smoke.XXXXXX)
evidence=$repo/target/evidence/session-smoke-$label
mkdir -p "$evidence" "$temporary/runtime" "$temporary/config"
chmod 700 "$temporary/runtime"
original_runtime=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
export PULSE_SERVER=${PULSE_SERVER:-unix:$original_runtime/pulse/native}
if [[ -S $original_runtime/bus ]]; then ln -s "$original_runtime/bus" "$temporary/runtime/bus"; fi
manager_environment() {
    XDG_RUNTIME_DIR="$original_runtime" systemctl --user show-environment |
        sed -n -e '/^WAYLAND_DISPLAY=/p' -e '/^XDG_CURRENT_DESKTOP=/p' -e '/^XDG_SESSION_TYPE=/p'
}
manager_environment >"$evidence/manager-before.txt"
inner_pid=''
app_pids=()
cleanup() {
    trap - EXIT
    for child in "${app_pids[@]}"; do kill "$child" 2>/dev/null || true; done
    if [[ -n $inner_pid ]]; then kill -TERM -- "-$inner_pid" 2>/dev/null || true; wait "$inner_pid" 2>/dev/null || true; fi
    remove_private_runtime "$temporary"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
export XDG_RUNTIME_DIR=$temporary/runtime XDG_CONFIG_HOME=$temporary/config
export XDG_STATE_HOME=$temporary/state XDG_DATA_HOME=$temporary/data
mkdir -p "$XDG_DATA_HOME/applications"
cat >"$XDG_DATA_HOME/applications/org.asenalinux.Light.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Işık
Exec=/bin/true
DESKTOP
cat >"$XDG_DATA_HOME/applications/org.asenalinux.TestSettings.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Ayarlar
Exec=/bin/true
DESKTOP
# The development session's own Wayfire runs headless with two outputs.
export WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=2
if [[ ${ASENA_SMOKE_LOCK:-0} == 1 ]]; then
    [[ ${ASENA_LOCK_TEST_OUTPUTS:-2} =~ ^[12]$ ]] || exit 2
    export WLR_HEADLESS_OUTPUTS=${ASENA_LOCK_TEST_OUTPUTS:-2}
fi
if [[ ${ASENA_SMOKE_PHASE2:-0} == 1 || ${ASENA_SMOKE_PHASE3:-0} == 1 ]]; then export WLR_HEADLESS_OUTPUTS=1; fi
env -u WAYLAND_DISPLAY setsid bash scripts/dev-session.sh >"$evidence/shell.log" 2>&1 &
inner_pid=$!
export GSETTINGS_SCHEMA_DIR=$repo/target/schemas ASENA_LOCALE_DIR=$repo/target/locale
shell_pid=
for _ in {1..300}; do
    shell_pid=$(pgrep -n -f '/target/debug/asena-shell$' || true)
    if [[ -n $shell_pid ]] &&
        tr '\0' '\n' <"/proc/$shell_pid/environ" | grep -Fx "XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR" >/dev/null; then break; fi
    shell_pid=''
    sleep 0.1
done
[[ -n $shell_pid ]] || { cat "$evidence/shell.log"; exit 1; }
export WAYLAND_DISPLAY
WAYLAND_DISPLAY=$(tr '\0' '\n' <"/proc/$shell_pid/environ" | sed -n 's/^WAYLAND_DISPLAY=//p')
export DBUS_SESSION_BUS_ADDRESS
DBUS_SESSION_BUS_ADDRESS=$(tr '\0' '\n' <"/proc/$shell_pid/environ" | sed -n 's/^DBUS_SESSION_BUS_ADDRESS=//p')
timeout 10 gdbus wait --address "$DBUS_SESSION_BUS_ADDRESS" org.asenalinux.Shell
target/debug/asenactl menu toggle
target/debug/asenactl quick-settings toggle
target/debug/asenactl notifications toggle
target/debug/asenactl notifications toggle
target/debug/asenactl reload-style
if gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Shell --object-path /org/asenalinux/Shell --method org.asenalinux.Shell.ShowOsd invalid 0.5 >"$evidence/invalid-osd.log" 2>&1; then exit 1; fi
env -u WAYLAND_DEBUG wayland-info >"$evidence/protocols.txt"
if [[ ${ASENA_SMOKE_PHASE2:-0} == 1 || ${ASENA_SMOKE_PHASE3:-0} == 1 || ${ASENA_SMOKE_LOCK:-0} == 1 ]]; then
    if [[ ${ASENA_SMOKE_LOCK:-0} == 1 ]]; then
        bash scripts/probe-lock.sh "$evidence" >"$evidence/lock-probe.log" 2>&1
    elif [[ ${ASENA_SMOKE_PHASE3:-0} == 1 ]]; then
        bash scripts/probe-phase3.sh "$evidence" >"$evidence/phase3.log" 2>&1
    else
        bash scripts/probe-phase2.sh "$evidence" >"$evidence/phase2.log" 2>&1
    fi
    kill -TERM -- "-$inner_pid"
    wait "$inner_pid" || true
    inner_pid=''
    if kill -0 "$shell_pid" 2>/dev/null; then printf '%s\n' 'Shell survived session cleanup' >&2; exit 1; fi
    unset DBUS_SESSION_BUS_ADDRESS WAYLAND_DISPLAY
    manager_environment >"$evidence/manager-after.txt"
    cmp "$evidence/manager-before.txt" "$evidence/manager-after.txt"
    printf '%s\n' 'Surface probe and session cleanup passed.'
    exit 0
fi
config=$(tr '\0' '\n' <"/proc/$shell_pid/environ" | sed -n 's/^ASENA_DEV_CONFIG=//p')
# Fixed output positions for the pointer test. The daemon writes its copy at
# startup; append after that write so it is not replaced. Wayfire reloads it.
for _ in {1..100}; do head -n 1 "$config" | grep -Fq 'Generated by asena-daemon' && break; sleep 0.1; done
head -n 1 "$config" | grep -Fq 'Generated by asena-daemon'
printf '\n[output:HEADLESS-1]\nposition = 0,0\n\n[output:HEADLESS-2]\nposition = 1280,0\n' >>"$config"
sleep 0.5
wlr-randr >"$evidence/outputs.txt"
gsettings set org.asenalinux.shell pinned-apps "['foot.desktop']"
foot --app-id=foot sh -c 'printf "Asena taskbar test\n"; sleep 60' >"$evidence/foot.log" 2>&1 &
foot_pid=$!
app_pids+=("$foot_pid")
sleep 1
timeout 8 grim -o HEADLESS-2 "$evidence/taskbar.png"
if command -v wlrctl >/dev/null; then
    # The pinned foot button on the HEADLESS-2 panel.
    wlrctl pointer move -10000 -10000
    wlrctl pointer move 1360 696
    wlrctl pointer click left
    sleep 0.3
    wlrctl toplevel find app_id:foot state:minimized
    wlrctl pointer click left
    sleep 0.3
    wlrctl toplevel find app_id:foot state:active
    printf '%s\n' 'Taskbar click minimize/focus passed.' >"$evidence/click.log"
fi
if [[ ${ASENA_SMOKE_APPS:-0} == 1 ]]; then
    nautilus --new-window "$temporary" >"$evidence/nautilus.log" 2>&1 &
    app_pids+=("$!")
    ptyxis --standalone >"$evidence/ptyxis.log" 2>&1 &
    app_pids+=("$!")
    mkdir -p "$temporary/firefox"
    firefox-esr --no-remote --profile "$temporary/firefox" about:blank >"$evidence/firefox.log" 2>&1 &
    app_pids+=("$!")
    timeout 20 wlrctl toplevel waitfor app_id:org.gnome.Nautilus
    timeout 20 wlrctl toplevel waitfor app_id:org.gnome.Ptyxis
    timeout 30 wlrctl toplevel waitfor app_id:firefox app_id:firefox-esr
    sleep 1
    wlrctl toplevel focus app_id:org.gnome.Ptyxis
    sleep 0.3
    timeout 8 grim -o HEADLESS-2 "$evidence/ptyxis.png"
    wlrctl toplevel focus app_id:firefox app_id:firefox-esr
    sleep 0.3
    timeout 8 grim -o HEADLESS-2 "$evidence/default-apps.png"
fi
foot --app-id=asena-probe sh -c 'sleep 60' >"$evidence/probe-window.log" 2>&1 &
probe_pid=$!
app_pids+=("$probe_pid")
target/debug/examples/toplevel_probe >"$evidence/toplevel-probe.log" 2>&1
wait "$probe_pid" || true
gsettings set org.asenalinux.desktop background-uri 'file:///nonexistent-asena-wallpaper.png'
sleep 0.5
kill -0 "$shell_pid"
wlr-randr --output HEADLESS-2 --scale 2
sleep 0.5
timeout 8 grim -o HEADLESS-2 "$evidence/scale-2.png"
wlr-randr --output HEADLESS-2 --scale 1
# Output removal and return go through asena-daemon; see probe-displays.py
# for the wlroots 0.18 abort that wlr-randr --off/--on triggers here.
python3 -I scripts/probe-displays.py HEADLESS-1 --disable-and-revert >"$evidence/output-disable.log" 2>&1
# Asena's wlroots build (scripts/build-wlroots.sh) carries the upstream fix.
if dpkg-query -W -f='${Version}' libwlroots-0.18 2>/dev/null | grep -Fq +asena; then
    {
        wlr-randr --output HEADLESS-1 --off
        wlr-randr --output HEADLESS-1 --on
        printf '%s\n' 'wlr-randr off/on passed with the patched wlroots.'
    } >>"$evidence/output-disable.log" 2>&1
fi
kill "$foot_pid" 2>/dev/null || true
python3 - "$shell_pid" >"$evidence/idle.txt" <<'PY'
import os, sys, time
from pathlib import Path
pid = sys.argv[1]
def ticks():
    fields = Path('/proc/' + pid + '/stat').read_text().rsplit(')', 1)[1].split()
    return int(fields[11]) + int(fields[12])
start_ticks, start = ticks(), time.monotonic()
time.sleep(30)
elapsed = time.monotonic() - start
print('Observation seconds:', round(elapsed, 3))
print('CPU percent:', round((ticks() - start_ticks) / os.sysconf('SC_CLK_TCK') / elapsed * 100, 3))
print(next(line for line in Path('/proc/' + pid + '/status').read_text().splitlines() if line.startswith('VmRSS:')))
PY
kill -TERM -- "-$inner_pid"
wait "$inner_pid" || true
inner_pid=
if kill -0 "$shell_pid" 2>/dev/null; then printf '%s\n' 'Shell survived session cleanup' >&2; exit 1; fi
unset DBUS_SESSION_BUS_ADDRESS WAYLAND_DISPLAY
manager_environment >"$evidence/manager-after.txt"
cmp "$evidence/manager-before.txt" "$evidence/manager-after.txt"
if dbus-run-session -- target/debug/asenactl menu toggle >"$evidence/no-shell.log" 2>&1; then exit 1; fi
if target/debug/asenactl lock >"$evidence/unsupported-lock.log" 2>&1; then exit 1; fi
printf '%s\n' 'Panel, D-Bus, window requests, scale and cleanup checks passed.'
