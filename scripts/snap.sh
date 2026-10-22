#!/bin/bash
pid=$(pgrep -n asena-shell || true)
if [[ -z "$pid" ]]; then
    echo "asena-shell not found" >&2
    exit 1
fi
XDG_RUNTIME_DIR=$(tr '\0' '\n' < /proc/"$pid"/environ | grep '^XDG_RUNTIME_DIR=' | cut -d= -f2-)
export XDG_RUNTIME_DIR
DBUS_SESSION_BUS_ADDRESS=$(tr '\0' '\n' < /proc/"$pid"/environ | grep '^DBUS_SESSION_BUS_ADDRESS=' | cut -d= -f2-)
export DBUS_SESSION_BUS_ADDRESS
WAYLAND_DISPLAY=$(tr '\0' '\n' < /proc/"$pid"/environ | grep '^WAYLAND_DISPLAY=' | cut -d= -f2-)
export WAYLAND_DISPLAY

call_dbus() {
    gdbus call --session --dest org.asenalinux.Shell --object-path /org/asenalinux/Shell --method "org.asenalinux.Shell.$1" >/dev/null 2>&1 || true
}

pkill -f asena-settings >/dev/null 2>&1 || true
sleep 0.2

# 1. Capture Menu (Default Grid)
call_dbus ToggleMenu
sleep 0.4
grim /tmp/menu.png

# 2. Capture Menu (Search Results)
wtype "ayar"
sleep 0.4
grim /tmp/menu_search.png

# Clear search and close menu
call_dbus ToggleMenu
sleep 0.3

# 3. Capture Quick Settings
call_dbus ToggleQuickSettings
sleep 0.4
grim /tmp/qs.png
call_dbus ToggleQuickSettings
sleep 0.3

# 4. Send test notification and capture Notification Center
gdbus call --session \
    --dest org.freedesktop.Notifications \
    --object-path /org/freedesktop/Notifications \
    --method org.freedesktop.Notifications.Notify \
    "Asena Sistem" 0 "dialog-information" "Sistem Hazır" "Asena Linux masaüstü ortamı başarıyla yapılandırıldı." "[]" "{}" 5000 >/dev/null 2>&1 || true

sleep 0.3
call_dbus ToggleNotificationCenter
sleep 0.4
grim /tmp/nc.png
call_dbus ToggleNotificationCenter
sleep 0.2

ls -lh /tmp/menu.png /tmp/menu_search.png /tmp/qs.png /tmp/nc.png
