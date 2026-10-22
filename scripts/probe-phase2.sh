#!/bin/bash
# Called inside the disposable smoke session, never against the user's desktop.
set -euo pipefail
evidence=${1:?Evidence directory required}
for program in wtype wl-paste notify-send grim; do command -v "$program" >/dev/null; done
timeout 10 gdbus wait --address "$DBUS_SESSION_BUS_ADDRESS" org.freedesktop.Notifications
target/debug/asenactl menu toggle
sleep 0.2
timeout 8 grim -o HEADLESS-1 "$evidence/menu.png"
wtype -- '2+2*3'
sleep 0.2
timeout 8 grim -o HEADLESS-1 "$evidence/calculator.png"
wtype -k Return
sleep 0.2
[[ $(timeout 3 wl-paste --no-newline) == 8 ]]
for query in isik ayar AYAR Ayar; do
    target/debug/asenactl menu toggle
    wtype -M ctrl -k a -m ctrl -k BackSpace
    wtype -- "$query"
    sleep 0.2
    wtype -k Return
    sleep 0.2
done
python3 - "$XDG_STATE_HOME/asena/usage.json" <<'PY'
import json, sys
from pathlib import Path
usage = json.loads(Path(sys.argv[1]).read_text())
assert usage['org.asenalinux.Light.desktop'] == 1, usage
assert usage['org.asenalinux.TestSettings.desktop'] == 3, usage
print('Turkish search, application activation and usage persistence passed.')
PY
target/debug/asenactl menu toggle
wtype -k Escape
target/debug/asenactl menu toggle
wtype -k Escape
id=$(notify-send -p -a Test -t 0 'Başlık' '<b>kalın</b> &amp; <script>metin</script>')
notify-send -a Test -r "$id" -t 0 'Güncel başlık' '<i>değişti</i>'
sleep 0.3
timeout 8 grim -o HEADLESS-1 "$evidence/notification.png"
gsettings set org.asenalinux.notifications do-not-disturb true
notify-send -a Test 'Sessiz bildirim' 'Geçmişte saklanır'
notify-send -a Test -u critical -t 0 'Kritik bildirim' 'Açık kalır'
sleep 0.3
timeout 8 grim -o HEADLESS-1 "$evidence/dnd-critical.png"
target/debug/asenactl notifications toggle
sleep 0.3
timeout 8 grim -o HEADLESS-1 "$evidence/notification-center.png"
target/debug/asenactl notifications toggle
gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Shell --object-path /org/asenalinux/Shell --method org.asenalinux.Shell.ShowOsd volume 0.4
sleep 0.2
timeout 8 grim -o HEADLESS-1 "$evidence/osd.png"
sleep 1.6
timeout 8 grim -o HEADLESS-1 "$evidence/osd-hidden.png"
printf '%s\n' 'Phase 2 menu calculator/clipboard, Turkish launch, usage and surface smoke completed.'
