#!/bin/bash
# Drive installed applications on a private headless Wayfire display.
set -euo pipefail
[[ $(uname -s) == Linux && $EUID == 0 ]] || { printf '%s\n' 'Run as root in the Debian development VM.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if [[ ${ASENA_INSTALLED_NAMESPACE:-0} != 1 ]]; then
    exec unshare --mount --propagation private env ASENA_INSTALLED_NAMESPACE=1 bash "$0" "$@"
fi
root=$repo/target/debian-test-root
work=$repo/target/evidence/debian/installed-apps
[[ -x $root/usr/bin/asena-calculator ]] || { printf '%s\n' 'Install the test packages first.' >&2; exit 1; }
temporary=$(mktemp -d /tmp/asena-smoke.XXXXXX)
export XDG_RUNTIME_DIR=$temporary/runtime
mkdir -p "$work" "$XDG_RUNTIME_DIR" "$root$XDG_RUNTIME_DIR" "$root"/{proc,dev,sys}
cp "$repo/data/wayfire/wayfire.ini" "$temporary/wayfire.ini"
chmod 0700 "$XDG_RUNTIME_DIR"
chown -R 1000:1000 "$temporary"
active='' shell_active='' display_active=''
# shellcheck source=scripts/private-runtime.sh
. "$repo/scripts/private-runtime.sh"
cleanup() {
    for child in "$active" "$shell_active" "$display_active"; do
        [[ -z $child ]] || kill -- "-$child" 2>/dev/null || true
    done
    for child in "$active" "$shell_active" "$display_active"; do
        [[ -z $child ]] || wait "$child" 2>/dev/null || true
    done
    if mountpoint -q "$root$XDG_RUNTIME_DIR"; then umount -R "$root$XDG_RUNTIME_DIR" || true; fi
    chown --no-dereference 0:0 "$temporary"
    remove_private_runtime "$temporary"
}
trap cleanup EXIT
drm_device=''
for candidate in /dev/dri/by-path/pci-*-card; do
    if [[ -e $candidate ]]; then drm_device=$(readlink -f "$candidate"); break; fi
done
[[ -n $drm_device ]]
setsid runuser -u "$(id -nu 1000)" -- env -i HOME=/home/asena PATH=/usr/bin:/bin \
    XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=1 \
    WLR_RENDERER_ALLOW_SOFTWARE=1 WLR_RENDER_DRM_DEVICE="$drm_device" \
    ASENA_AUTOSTART=/dev/null wayfire -c "$temporary/wayfire.ini" > "$work/wayfire.log" 2>&1 &
display_active=$!
export WAYLAND_DISPLAY=''
for _ in {1..200}; do
    for socket in "$XDG_RUNTIME_DIR"/wayland-*; do
        if [[ -S $socket ]]; then WAYLAND_DISPLAY=${socket##*/}; break; fi
    done
    [[ -z $WAYLAND_DISPLAY ]] || break
    kill -0 "$display_active"
    sleep 0.1
done
[[ -n $WAYLAND_DISPLAY ]]
mount -t proc proc "$root/proc"
mount --rbind /dev "$root/dev"
mount --make-rslave "$root/dev"
mount --rbind /sys "$root/sys"
mount --make-rslave "$root/sys"
mount --bind "$XDG_RUNTIME_DIR" "$root$XDG_RUNTIME_DIR"
export XDG_DATA_DIRS=/usr/local/share:/usr/share
unset GSETTINGS_SCHEMA_DIR ASENA_LOCALE_DIR LD_LIBRARY_PATH LOCPATH DBUS_SESSION_BUS_ADDRESS
if [[ -x $root/usr/bin/asena-shell ]]; then
    setsid chroot "$root" runuser -u asena-package-test -- env -i \
        PATH=/usr/bin:/bin HOME=/home/asena-package-test USER=asena-package-test \
        XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" WAYLAND_DISPLAY="$WAYLAND_DISPLAY" \
        XDG_DATA_DIRS=/usr/local/share:/usr/share XDG_CURRENT_DESKTOP=Asena \
        GDK_BACKEND=wayland GSK_RENDERER=cairo LANG=C.UTF-8 LANGUAGE=tr \
        dbus-run-session -- /usr/bin/asena-shell > "$work/shell.log" 2>&1 &
    shell_active=$!
    sleep 1
    kill -0 "$shell_active"
fi
pairs=("$@")
if [[ ${#pairs[@]} == 0 ]]; then pairs=(calculator:Calculator files:Files notes:Notes store:Store); fi
for pair in "${pairs[@]}"; do
    binary=${pair%%:*} id=${pair#*:}
    if wlrctl toplevel list | grep -F "org.asenalinux.$id:" >/dev/null; then
        printf 'Close the existing %s window before this test.\n' "$id" >&2; exit 1
    fi
    setsid chroot "$root" runuser -u asena-package-test -- env -i \
        PATH=/usr/bin:/bin HOME=/home/asena-package-test USER=asena-package-test \
        XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" WAYLAND_DISPLAY="$WAYLAND_DISPLAY" \
        XDG_DATA_DIRS=/usr/local/share:/usr/share GDK_BACKEND=wayland GSK_RENDERER=cairo \
        LANG=C.UTF-8 LANGUAGE=tr dbus-run-session -- \
        "/usr/bin/asena-$binary" > "$work/$binary.log" 2>&1 &
    active=$!
    found=0
    for _ in {1..100}; do
        if wlrctl toplevel list | grep -F "org.asenalinux.$id:" >/dev/null; then found=1; break; fi
        kill -0 "$active"
        sleep 0.1
    done
    [[ $found == 1 ]] || { cat "$work/$binary.log"; exit 1; }
    sleep 1
    wlrctl toplevel focus "app_id:org.asenalinux.$id"
    if [[ $binary == calculator ]]; then wtype '2+2'; wtype -k Return; fi
    if [[ $binary == store ]]; then sleep 8; else sleep 0.5; fi
    grim -o HEADLESS-1 "$work/$binary.png"
    kill -- "-$active"
    wait "$active" 2>/dev/null || true
    active=''
    printf 'PASS: installed %s window and screenshot, with system data paths\n' "$binary"
done
