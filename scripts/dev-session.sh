#!/bin/bash
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
# Nested inside the current desktop by default. Harnesses set
# WLR_BACKENDS=headless and need no parent display.
export WLR_BACKENDS=${WLR_BACKENDS:-wayland}
if [[ $WLR_BACKENDS == wayland ]]; then
    [[ -n ${WAYLAND_DISPLAY:-} && -n ${XDG_RUNTIME_DIR:-} ]] || {
        printf '%s\n' 'Run inside a Wayland desktop or WSLg. Without a GPU: WLR_RENDERER_ALLOW_SOFTWARE=1 scripts/dev-session.sh' >&2
        exit 1
    }
    export WLR_WL_OUTPUTS=${WLR_WL_OUTPUTS:-1}
else
    [[ -n ${XDG_RUNTIME_DIR:-} ]] || { printf '%s\n' 'XDG_RUNTIME_DIR is required.' >&2; exit 1; }
    # Wayfire renders with OpenGL ES only. In a VM without a GPU, llvmpipe
    # needs a KMS device with dumb buffers; vgem has none, so take the first
    # PCI display device.
    export WLR_RENDERER_ALLOW_SOFTWARE=${WLR_RENDERER_ALLOW_SOFTWARE:-1}
    if [[ -z ${WLR_RENDER_DRM_DEVICE:-} ]]; then
        for candidate in /dev/dri/by-path/pci-*-card; do
            if [[ -e $candidate ]]; then export WLR_RENDER_DRM_DEVICE; WLR_RENDER_DRM_DEVICE=$(readlink -f -- "$candidate"); break; fi
        done
    fi
fi
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/native-env.sh
. "$repo/scripts/native-env.sh"
cd "$repo"
bash scripts/compile-data.sh
# Session launch paths deliberately use this workspace's target/debug directory.
unset CARGO_TARGET_DIR
cargo build --workspace --locked
export ASENA_SOURCE_DIR=$repo
export ASENA_DEV=1
# The private bus reads D-Bus service files from XDG_DATA_DIRS only when it
# starts. Flatpak apps are often D-Bus activated, so their export directories
# must be known before dbus-run-session. asena-session keeps this prefix.
export XDG_DATA_DIRS="${XDG_DATA_HOME:-$HOME/.local/share}/flatpak/exports/share:/var/lib/flatpak/exports/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
session_pid=
dev_config_dir=
cleanup() {
    local status=$?
    trap - EXIT INT TERM
    if [[ -n $session_pid ]]; then
        # A dedicated process group contains compositor, private bus and clients.
        kill -TERM -- "-$session_pid" 2>/dev/null || true
        for _ in {1..20}; do
            kill -0 -- "-$session_pid" 2>/dev/null || break
            sleep 0.1
        done
        kill -KILL -- "-$session_pid" 2>/dev/null || true
        wait "$session_pid" 2>/dev/null || true
    fi
    if [[ -n $dev_config_dir ]]; then
        # The daemon replaces its files through temporary files in the same directory.
        find "$dev_config_dir" -maxdepth 1 -type f \( -name wayfire.ini -o -name '.wayfire.ini.*.tmp' \
            -o -name displays.json -o -name '.displays.json.*.tmp' \) -delete
        rmdir -- "$dev_config_dir"
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
dev_config_dir=$(mktemp -d /tmp/asena-dev.XXXXXX)
cp "$repo/data/wayfire/wayfire.ini" "$dev_config_dir/wayfire.ini"
export ASENA_DEV_CONFIG=$dev_config_dir/wayfire.ini
setsid --wait dbus-run-session -- "$repo/target/debug/asena-session" --dev &
session_pid=$!
wait "$session_pid"
