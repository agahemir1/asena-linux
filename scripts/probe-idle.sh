#!/bin/bash
# Only an isolated VM session with temporary credentials and a private dconf bus.
set -euo pipefail
[[ ${ASENA_POWER_TEST_VM:-0} == 1 && ${ASENA_LOCK_TEST_VM:-0} == 1 ]] || exit 2
helpers() {
    local candidate
    for candidate in $(pgrep -x swayidle || true); do
        if tr '\0' '\n' <"/proc/$candidate/environ" 2>/dev/null | grep -Fxq "XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR"; then printf '%s\n' "$candidate"; fi
    done
}
cleanup() { gsettings set org.asenalinux.power idle-enabled false; }
trap cleanup EXIT
gsettings set org.asenalinux.power idle-enabled false
gsettings set org.asenalinux.power off-seconds 0
gsettings set org.asenalinux.power suspend-seconds 0
gsettings set org.asenalinux.power lock-seconds 0
gsettings set org.asenalinux.power dim-seconds 60
gsettings set org.asenalinux.power idle-enabled true
for _ in {1..100}; do [[ -n $(helpers) ]] && break; sleep 0.05; done
previous=$(helpers)
[[ -n $previous && $(wc -w <<<"$previous") == 1 ]]
target/debug/asenactl idle dim
target/debug/asenactl idle restore
gsettings set org.asenalinux.power dim-seconds 61
for _ in {1..100}; do current=$(helpers); [[ -n $current && $current != "$previous" ]] && break; sleep 0.05; done
[[ -n $current && $current != "$previous" && $(wc -w <<<"$current") == 1 ]]
if kill -0 "$previous" 2>/dev/null; then exit 1; fi
# Invalid ordering must stop the helper instead of silently changing the values.
gsettings set org.asenalinux.power lock-seconds 59
for _ in {1..100}; do [[ -z $(helpers) ]] && break; sleep 0.05; done
[[ -z $(helpers) ]]
if target/debug/asenactl idle dim; then exit 1; fi
gsettings set org.asenalinux.power idle-enabled false
gsettings set org.asenalinux.power dim-seconds 1
gsettings set org.asenalinux.power lock-seconds 3
gsettings set org.asenalinux.power idle-enabled true
timeout 12 gdbus wait --address "$DBUS_SESSION_BUS_ADDRESS" org.asenalinux.Lock
# The compositor and the shared lock bus, not process existence, acknowledge it.
scope=$(python3 - <<'PY'
import os
r=os.environb[b'XDG_RUNTIME_DIR']; d=os.environb[b'WAYLAND_DISPLAY']
print('v1:'+(d if os.path.isabs(d) else os.path.join(r,d)).hex()+':'+os.environb.get(b'XDG_SESSION_ID',b'').hex())
PY
)
timeout 8 gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Lock --object-path /org/asenalinux/Lock --method org.asenalinux.Lock.WaitReady "$scope"
cleanup
for _ in {1..100}; do [[ -z $(helpers) ]] && break; sleep 0.05; done
[[ -z $(helpers) ]]
printf '%s\n' 'Single helper, replacement/reaping, invalid settings and automatic lock passed. No backlight or suspend hardware acceptance.'
