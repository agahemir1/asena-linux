#!/bin/sh
set -eu

if [ "${ASENA_DEV:-0}" = 1 ]; then
    # Keep shell, daemon and locker diagnostics in the development log.
    if [ -n "${ASENA_DEV_LOG:-}" ] && (: >>"$ASENA_DEV_LOG") 2>/dev/null; then
        exec >>"$ASENA_DEV_LOG" 2>&1
    fi
    # Do not import the nested display or bus into the real user manager.
    if [ "${ASENA_CAPTURE_PROTOCOLS:-0}" = 1 ]; then
        wayland-info > "$ASENA_SOURCE_DIR/target/wayland-info.txt" 2>&1
    fi
    # The nested session owns a private bus. Without this, D-Bus activated
    # services such as xdg-desktop-portal inherit the host compositor and open
    # file, print and app chooser dialogs where the nested session cannot show them.
    activation=''
    for name in WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP \
        XDG_SESSION_TYPE XDG_DATA_DIRS GSETTINGS_SCHEMA_DIR ASENA_LOCALE_DIR \
        GDK_BACKEND QT_QPA_PLATFORM PATH; do
        if printenv "$name" >/dev/null; then activation="$activation $name"; fi
    done
    # shellcheck disable=SC2086 # Variable names are fixed tokens.
    dbus-update-activation-environment $activation
    daemon_pid=''
    cleanup() {
        if [ -n "$daemon_pid" ]; then kill "$daemon_pid" 2>/dev/null || true; wait "$daemon_pid" 2>/dev/null || true; fi
    }
    trap cleanup EXIT
    if [ -n "${ASENA_DAEMON:-}" ]; then "$ASENA_DAEMON" & daemon_pid=$!; fi
    if "$ASENA_SHELL"; then status=0; else status=$?; fi
    cleanup
    trap - EXIT
    exit "$status"
fi

activation=''
for name in WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP XDG_SESSION_TYPE \
    XDG_SESSION_ID XDG_DATA_DIRS GDK_BACKEND QT_QPA_PLATFORM _JAVA_AWT_WM_NONREPARENTING; do
    if printenv "$name" >/dev/null; then activation="$activation $name"; fi
done
# shellcheck disable=SC2086 # Variable names are fixed tokens.
dbus-update-activation-environment --systemd $activation
systemctl --user start asena-session.target
