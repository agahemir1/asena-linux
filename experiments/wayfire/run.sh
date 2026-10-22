#!/bin/bash
# User-authorized isolated VM experiment. No default compositor switch.
# Runs the same Asena shell binary on Wayfire twice, with and without the
# Dual Kawase blur plugin, and records captures, protocols and CPU/RSS.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo"
# shellcheck source=scripts/native-env.sh
. "$repo/scripts/native-env.sh"
# shellcheck source=scripts/private-runtime.sh
. "$repo/scripts/private-runtime.sh"
evidence=$repo/target/evidence/wayfire
probe=(python3 -I "$repo/experiments/wayfire/probe.py")
mkdir -p "$evidence"
if [[ ${ASENA_WAYFIRE_BUS:-0} != 1 ]]; then
    for program in wayfire foot grim wayland-info wlr-randr gdbus python3; do command -v "$program" >/dev/null; done
    # llvmpipe needs a KMS device with dumb buffers. vgem has none, so the
    # first PCI display device is used unless one is given.
    device=${WLR_RENDER_DRM_DEVICE:-}
    if [[ -z $device ]]; then
        for candidate in /dev/dri/by-path/pci-*-card; do
            if [[ -e $candidate ]]; then device=$(readlink -f -- "$candidate"); break; fi
        done
    fi
    [[ -n $device && -r $device && -w $device ]] || { printf '%s\n' 'A readable and writable KMS device is required for the Wayfire experiment.' >&2; exit 1; }
    export WLR_RENDER_DRM_DEVICE=$device
    bash scripts/compile-data.sh
    cargo build --workspace --all-targets --locked
    temporary=$(mktemp -d /tmp/asena-wayfire.XXXXXX)
    mkdir -p "$temporary/runtime" "$temporary/config"
    chmod 700 "$temporary/runtime"
    original_runtime=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
    export PULSE_SERVER=${PULSE_SERVER:-unix:$original_runtime/pulse/native}
    export XDG_RUNTIME_DIR=$temporary/runtime XDG_CONFIG_HOME=$temporary/config
    export ASENA_WAYFIRE_TEMP=$temporary ASENA_WAYFIRE_BUS=1
    exec dbus-run-session -- bash "$repo/experiments/wayfire/run.sh"
fi
temporary=${ASENA_WAYFIRE_TEMP:?}
[[ $temporary == /tmp/asena-wayfire.* && -O $temporary ]] || exit 1
# Never leave files from an earlier run looking like this run's evidence.
find "$evidence" -maxdepth 1 -type f -delete
wayfire_pid='' shell_pid='' foot_pid=''
stop_children() {
    for child in "$foot_pid" "$shell_pid" "$wayfire_pid"; do [[ -z $child ]] || kill "$child" 2>/dev/null || true; done
    for child in "$foot_pid" "$shell_pid" "$wayfire_pid"; do [[ -z $child ]] || wait "$child" 2>/dev/null || true; done
    foot_pid='' shell_pid='' wayfire_pid=''
}
cleanup() {
    trap - EXIT
    stop_children
    remove_private_runtime "$temporary"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
export WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=2 WLR_RENDERER=gles2 WLR_RENDERER_ALLOW_SOFTWARE=1
export GSK_RENDERER=cairo GDK_BACKEND=wayland
export ASENA_DEV=1 ASENA_SOURCE_DIR=$repo ASENA_DEV_STYLE=$repo/experiments/wayfire/glass.css
export ASENA_LOCALE_DIR=$repo/target/locale GSETTINGS_SCHEMA_DIR=$repo/target/schemas
export XDG_DATA_DIRS=$repo/target/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}
export _WAYFIRE_SOCKET=$XDG_RUNTIME_DIR/wayfire.socket WAYFIRE_SOCKET=$XDG_RUNTIME_DIR/wayfire.socket

gsettings set org.asenalinux.shell pinned-apps "['foot.desktop']"
image_uri=$(python3 -I -c 'import pathlib, sys; print(pathlib.Path(sys.argv[1]).as_uri())' "$repo/experiments/wayfire/pattern.svg")
gsettings set org.asenalinux.desktop background-uri "$image_uri"
# High contrast blocks for the window that passes behind the shell surfaces.
# Redrawn on resize so the pattern always fills the window.
cat >"$temporary/pattern.sh" <<'EOF'
export LC_ALL=C.UTF-8
draw() {
    local rows columns row='' line
    read -r rows columns < <(stty size)
    while (( ${#row} < columns )); do row+='██████    '; done
    row=${row:0:columns}
    printf '[H[2J'
    for (( line = 0; line < rows; line++ )); do
        printf '%s' "$row"
        (( line + 1 == rows )) || printf '
'
    done
}
trap draw WINCH
printf '[?25l'
draw
while :; do sleep 600 & wait $!; done
EOF

start_compositor() {
    local config=$1 label=$2 socket
    rm -f -- "$WAYFIRE_SOCKET"
    env -u WAYLAND_DISPLAY wayfire -c "$config" >"$evidence/compositor-$label.log" 2>&1 &
    wayfire_pid=$!
    export WAYLAND_DISPLAY=''
    for _ in {1..150}; do
        for socket in "$XDG_RUNTIME_DIR"/wayland-*; do
            if [[ -S $socket && $socket != *.lock ]]; then WAYLAND_DISPLAY=${socket##*/}; break; fi
        done
        [[ -n $WAYLAND_DISPLAY && -S $WAYFIRE_SOCKET ]] && break
        kill -0 "$wayfire_pid"
        sleep 0.1
    done
    [[ -n $WAYLAND_DISPLAY && -S $WAYFIRE_SOCKET ]]
    dbus-update-activation-environment WAYLAND_DISPLAY GDK_BACKEND
    # Only the shell gets the Turkish locale; Wayfire parses decimal options.
    env LOCPATH="$repo/target/locale-data" LC_ALL=tr_TR.UTF-8 LANGUAGE=tr \
        target/debug/asena-shell >"$evidence/shell-$label.log" 2>&1 &
    shell_pid=$!
    timeout 15 gdbus wait --session org.asenalinux.Shell
    target/debug/asenactl reload-style
    sleep 2
}

capture() {
    timeout 10 grim -t ppm -o "$primary" "$evidence/$1-$label.ppm"
}

toggle_surface() {
    target/debug/asenactl "$1" toggle
    sleep "$2"
    capture "$3"
    target/debug/asenactl "$1" toggle
    sleep 0.6
}

run_scene() {
    local config=$1 view output other
    label=$2
    start_compositor "$config" "$label"
    # Shell surfaces open on one output; compare everything there.
    target/debug/asenactl menu toggle
    primary=$("${probe[@]}" view-output asena-menu)
    target/debug/asenactl menu toggle
    sleep 0.6
    output=$("${probe[@]}" output-id "$primary")
    other=HEADLESS-1
    [[ $primary != HEADLESS-1 ]] || other=HEADLESS-2
    printf '%s\n' "$primary" >"$evidence/primary-$label.txt"
    capture panel
    timeout 10 grim -t ppm -o "$other" "$evidence/scale2-$label.ppm"
    foot --app-id=asena-pattern bash "$temporary/pattern.sh" >"$evidence/foot-$label.log" 2>&1 &
    foot_pid=$!
    view=$("${probe[@]}" wait-view asena-pattern)
    # The lower part of the window lies behind the panel (y 672 to 720).
    "${probe[@]}" place "$view" "$output" 200 470 880 300
    sleep 1
    capture behind-1
    "${probe[@]}" place "$view" "$output" 216 470 880 300
    sleep 1
    capture behind-2
    "${probe[@]}" measure "$wayfire_pid" 5 "$view" 200 470 880 300 4 60 0.05 >"$evidence/usage-$label.json"
    "${probe[@]}" place "$view" "$output" 0 40 1280 620
    "${probe[@]}" ipc window-rules/focus-view "{\"id\": $view}" >/dev/null
    sleep 1
    toggle_surface menu 1 menu
    toggle_surface quick-settings 1 quick-settings
    toggle_surface notifications 1.5 sidebar
    "${probe[@]}" ipc window-rules/list-views >"$evidence/views-$label.json"
    if [[ $label == on ]]; then
        env -u WAYLAND_DEBUG wayland-info >"$evidence/protocols.txt"
        wlr-randr >"$evidence/outputs.txt"
        kill "$foot_pid"
        wait "$foot_pid" 2>/dev/null || true
        foot --app-id=asena-probe sh -c 'sleep 60' >"$evidence/probe-foot.log" 2>&1 &
        foot_pid=$!
        "${probe[@]}" wait-view asena-probe >/dev/null
        target/debug/examples/toplevel_probe >"$evidence/toplevel-probe.log" 2>&1
        wait "$foot_pid" 2>/dev/null || true
        foot_pid=''
    fi
    stop_children
}

run_scene "$repo/experiments/wayfire/wayfire.ini" on
# Restart without the blur plugin. Keep CSS, pattern, outputs and binary equal.
sed 's/plugins = blur /plugins = /' "$repo/experiments/wayfire/wayfire.ini" >"$temporary/no-blur.ini"
run_scene "$temporary/no-blur.ini" off

for image in "$evidence"/*.ppm; do
    "${probe[@]}" png "$image" "${image%.ppm}.png"
done
"${probe[@]}" report "$evidence" >"$evidence/metrics.json"
printf '%s\n' "Wayfire blur experiment finished. Evidence: $evidence"
