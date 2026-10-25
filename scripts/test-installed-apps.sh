#!/bin/bash
# Drive installed applications from the clean test root on the existing dev display.
set -euo pipefail
[[ $(uname -s) == Linux && $EUID == 0 ]] || { printf '%s\n' 'Run as root in the Debian development VM.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if [[ ${ASENA_INSTALLED_NAMESPACE:-0} != 1 ]]; then
    exec unshare --mount --propagation private env ASENA_INSTALLED_NAMESPACE=1 bash "$0"
fi
root=$repo/target/debian-test-root
work=$repo/target/evidence/debian/installed-apps
[[ -x $root/usr/bin/asena-calculator ]] || { printf '%s\n' 'Install the test packages first.' >&2; exit 1; }
shell_pid=$(pgrep -u 1000 -n -f '/target/debug/asena-shell$')
while IFS= read -r -d '' variable; do
    case $variable in XDG_RUNTIME_DIR=*|WAYLAND_DISPLAY=*) export "${variable?}" ;; esac
done <"/proc/$shell_pid/environ"
: "${XDG_RUNTIME_DIR:?No development runtime}" "${WAYLAND_DISPLAY:?No development display}"
mkdir -p "$work" "$root$XDG_RUNTIME_DIR" "$root"/{proc,dev,sys}
mount -t proc proc "$root/proc"
mount --rbind /dev "$root/dev"
mount --make-rslave "$root/dev"
mount --rbind /sys "$root/sys"
mount --make-rslave "$root/sys"
mount --bind "$XDG_RUNTIME_DIR" "$root$XDG_RUNTIME_DIR"
export XDG_DATA_DIRS=/usr/local/share:/usr/share
unset GSETTINGS_SCHEMA_DIR ASENA_LOCALE_DIR LD_LIBRARY_PATH LOCPATH DBUS_SESSION_BUS_ADDRESS
active=''
cleanup() { if [[ -n $active ]]; then kill -- "-$active" 2>/dev/null || true; fi; }
trap cleanup EXIT
for pair in calculator:Calculator files:Files notes:Notes store:Store; do
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
    grim -o HEADLESS-1 "$work/$binary.png"
    kill -- "-$active"
    wait "$active" 2>/dev/null || true
    active=''
    printf 'PASS: installed %s window and screenshot, with system data paths\n' "$binary"
done
