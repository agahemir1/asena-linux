#!/bin/bash
# Private headless compositor and unprivileged protocol fixture, never greetd/PAM.
set -euo pipefail
[[ $(uname -s) == Linux && ${ASENA_GREETER_TEST_VM:-0} == 1 ]] || exit 2
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
# shellcheck source=scripts/native-env.sh
. scripts/native-env.sh
# shellcheck source=scripts/private-runtime.sh
. scripts/private-runtime.sh
bash scripts/compile-data.sh
cargo build --workspace --locked
temporary=$(mktemp -d /tmp/asena-smoke.greeter.XXXXXX)
case ${ASENA_GREETER_TEST_SMALL:-0} in 0|1) ;; *) exit 2;; esac
evidence=$repo/target/evidence/greeter-${ASENA_GREETER_TEST_SMALL:-0}
mkdir -p "$evidence" "$temporary/runtime" "$temporary/config"
rm -f -- "$evidence/start.png" "$evidence/rejected.png" "$evidence/information.png"
chmod 700 "$temporary/runtime"
compositor='' fixture='' greeter=''
session_file=/usr/share/wayland-sessions/asena.desktop
session_created=0
cleanup() {
    trap - EXIT
    for child in "$greeter" "$fixture"; do if [[ -n $child ]]; then kill "$child" 2>/dev/null || true; wait "$child" 2>/dev/null || true; fi; done
    if [[ -n $compositor ]]; then kill -TERM -- "-$compositor" 2>/dev/null || true; wait "$compositor" 2>/dev/null || true; fi
    if [[ $session_created == 1 ]] && cmp -s "$temporary/session.desktop" "$session_file"; then sudo rm -- "$session_file"; fi
    remove_private_runtime "$temporary"
}
trap cleanup EXIT
[[ ! -e $session_file && ! -L $session_file ]] || exit 2
cat >"$temporary/session.desktop" <<'DESKTOP'
[Desktop Entry]
Name=Asena test
Type=Application
Exec=/usr/bin/true '--name=space value' '$(not-a-command)'
DesktopNames=Asena;
DESKTOP
sudo install -m 0644 "$temporary/session.desktop" "$session_file"
session_created=1
export XDG_RUNTIME_DIR=$temporary/runtime XDG_CONFIG_HOME=$temporary/config
export XDG_STATE_HOME=$temporary/state XDG_DATA_HOME=$temporary/data
export GSETTINGS_SCHEMA_DIR=$repo/target/schemas ASENA_LOCALE_DIR=$repo/target/locale
export GSK_RENDERER=cairo ASENA_LOG_STDERR=1
# A bare host compositor; production runs the greeter in cage.
printf '[core]\nplugins = foreign-toplevel\nxwayland = false\n' >"$XDG_CONFIG_HOME/wayfire.ini"
device=''
for candidate in /dev/dri/by-path/pci-*-card; do
    if [[ -e $candidate ]]; then device=$(readlink -f -- "$candidate"); break; fi
done
env -u WAYLAND_DISPLAY WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=1 WLR_RENDERER_ALLOW_SOFTWARE=1 \
    WLR_RENDER_DRM_DEVICE="${WLR_RENDER_DRM_DEVICE:-$device}" \
    setsid dbus-run-session -- wayfire -c "$XDG_CONFIG_HOME/wayfire.ini" >"$evidence/compositor.log" 2>&1 &
compositor=$!
socket=''
for _ in {1..100}; do
    for candidate in "$XDG_RUNTIME_DIR"/wayland-*; do
        if [[ -S $candidate ]]; then socket=${candidate##*/}; break; fi
    done
    [[ -n $socket ]] && break
    sleep 0.1
done
[[ -n $socket ]]
export WAYLAND_DISPLAY=$socket GREETD_SOCK=$temporary/greetd.sock
if [[ ${ASENA_GREETER_TEST_SMALL:-0} == 1 ]]; then
    wlr-randr --output HEADLESS-1 --custom-mode 640x480
    mkdir -p "$XDG_CONFIG_HOME/gtk-4.0"
    # GDK_DPI_SCALE does not reliably change Wayland fonts. Use the ordinary
    # per-user GTK stylesheet in this private fixture and inspect the image.
    printf '* { font-size: 22px; }\n' >"$XDG_CONFIG_HOME/gtk-4.0/gtk.css"
fi
python3 scripts/greeter-fixture.py "$temporary" >"$evidence/protocol.log" 2>&1 &
fixture=$!
stage() {
    local expected=$1
    for _ in {1..150}; do [[ $(cat "$temporary/stage" 2>/dev/null || true) == "$expected" ]] && return 0; sleep 0.05; done
    printf 'Timed out at %s\n' "$expected" >&2; return 1
}
stage listening
dbus-run-session -- target/debug/asena-greeter >"$evidence/greeter.log" 2>&1 &
greeter=$!
timeout 10 wlrctl toplevel waitfor app_id:org.asenalinux.Greeter
for _ in {1..100}; do grep -Fq 'Greeter sessions loaded' "$evidence/greeter.log" && break; sleep 0.05; done
grep -Fq 'Greeter sessions loaded' "$evidence/greeter.log"
sleep 0.3
grim "$evidence/start.png"
wtype -M ctrl -k a -m ctrl -- asena
wtype -k Return
stage prompt-1
sleep 0.2
wtype -- asena-ui-fixture
wtype -k Return
stage rejected
sleep 0.3
grim "$evidence/rejected.png"
wtype -k Return
stage prompt-2
sleep 0.2
wtype -k Escape
stage cancelled
sleep 0.3
wtype -k Return
stage prompt-3
sleep 0.2
wtype -- asena-ui-fixture
wtype -k Return
stage information
sleep 0.2
grim "$evidence/information.png"
wtype -k Return
stage started
wait "$fixture"
fixture=''
for _ in {1..100}; do kill -0 "$greeter" 2>/dev/null || break; sleep 0.05; done
if kill -0 "$greeter" 2>/dev/null; then exit 1; fi
wait "$greeter"
greeter=''
[[ $(cat "$XDG_STATE_HOME/asena-greeter/last-user") == asena ]]
[[ $(stat -c %a "$XDG_STATE_HOME/asena-greeter/last-user") == 600 ]]
if grep -Fq asena-ui-fixture "$evidence/greeter.log" "$evidence/protocol.log"; then exit 1; fi
printf '%s\n' 'Greeter GUI protocol, cancel/retry, exit and username-only persistence passed. Real greetd/TTY login remains open.'
