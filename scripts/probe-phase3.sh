#!/bin/bash
# Invoked only inside the disposable smoke session with VM audio fixtures.
set -euo pipefail
evidence=${1:?Evidence directory required}
[[ $(pactl get-default-sink) == asena_test ]] || exit 2
timeout 10 gdbus wait --address "$DBUS_SESSION_BUS_ADDRESS" org.asenalinux.Daemon
gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Daemon \
    --object-path /org/asenalinux/Daemon --method org.freedesktop.DBus.Properties.Get \
    org.asenalinux.Daemon NightLightAvailable | tee "$evidence/night-light.txt"
grep -F false "$evidence/night-light.txt"
audio_ready=0
for _ in {1..50}; do
    if target/debug/asenactl volume up >>"$evidence/audio-startup.log" 2>&1; then audio_ready=1; break; fi
    sleep 0.1
done
[[ $audio_ready == 1 ]] || { cat "$evidence/audio-startup.log"; exit 1; }
target/debug/asenactl volume mute
LC_ALL=C pactl get-sink-mute asena_test | grep -F yes
target/debug/asenactl volume mute
LC_ALL=C pactl get-sink-mute asena_test | grep -F no
target/debug/asenactl mic mute
LC_ALL=C pactl get-source-mute asena_test_mic | grep -F yes
target/debug/asenactl mic mute
LC_ALL=C pactl get-source-mute asena_test_mic | grep -F no
if target/debug/asenactl brightness up >"$evidence/no-backlight.txt" 2>&1; then exit 1; fi
sleep 1.6
target/debug/asenactl quick-settings toggle
sleep 0.3
timeout 8 grim -o HEADLESS-1 "$evidence/quick-settings.png"
gsettings set org.asenalinux.desktop color-scheme prefer-dark
sleep 0.3
timeout 8 grim -o HEADLESS-1 "$evidence/quick-settings-dark.png"
gsettings set org.asenalinux.desktop color-scheme prefer-light
sleep 0.3
timeout 8 grim -o HEADLESS-1 "$evidence/quick-settings-light.png"
target/debug/asenactl quick-settings toggle
target/debug/asenactl menu toggle
notify-send -u critical -t 0 'Test' 'Suppressed while menu is open'
sleep 0.2
timeout 8 grim -o HEADLESS-1 "$evidence/menu-with-pending-notification.png"
wtype -k Escape
printf '%s\n' 'Native audio actions, observed mute state, absent backlight, daemon and appearance smoke passed.'
