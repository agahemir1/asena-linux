#!/bin/bash
# Called by the isolated two-output smoke session, with SSH recovery available.
set -euo pipefail
[[ ${ASENA_LOCK_TEST_VM:-0} == 1 ]] || exit 2
evidence=${1:?Evidence directory required}
password_file=$HOME/.local/state/asena-lock-test-password
[[ -f $password_file && ! -L $password_file && -O $password_file ]] || exit 2
locker_pid=''
locker_child=0
lease_pid=''
cleanup() {
    if [[ -n $lease_pid ]]; then kill "$lease_pid" 2>/dev/null || true; wait "$lease_pid" 2>/dev/null || true; fi
    if [[ -n $locker_pid ]]; then kill "$locker_pid" 2>/dev/null || true; wait "$locker_pid" 2>/dev/null || true; fi
}
trap cleanup EXIT
lock_scope=$(python3 - <<'PYCODE'
import os
runtime = os.environb[b'XDG_RUNTIME_DIR']
display = os.environb[b'WAYLAND_DISPLAY']
socket = display if os.path.isabs(display) else os.path.join(runtime, display)
print('v1:' + socket.hex() + ':' + os.environb.get(b'XDG_SESSION_ID', b'').hex())
PYCODE
)
if [[ ${ASENA_POWER_TEST_VM:-0} == 1 ]]; then bash scripts/probe-idle.sh >"$evidence/idle-probe.log" 2>&1; fi
ready() {
    timeout 10 gdbus wait --address "$DBUS_SESSION_BUS_ADDRESS" org.asenalinux.Lock
    timeout 8 gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Lock \
        --object-path /org/asenalinux/Lock --method org.asenalinux.Lock.WaitReady "$lock_scope"
}
timeout 12 target/debug/asenactl lock
for _ in {1..50}; do
    candidate=$(pgrep -n -f '/target/debug/asena-lock$' || true)
    if [[ -n $candidate ]] && tr '\0' '\n' <"/proc/$candidate/environ" | grep -Fx "XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR" >/dev/null; then locker_pid=$candidate; break; fi
    sleep 0.1
done
[[ -n $locker_pid ]]
log_file=$evidence/shell.log
ready
if gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Lock --object-path /org/asenalinux/Lock --method org.asenalinux.Lock.WaitReady wrong-session >"$evidence/lock-wrong-scope.log" 2>&1; then exit 1; fi
if gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Daemon --object-path /org/asenalinux/Daemon --method org.asenalinux.Daemon.Lock wrong-session >"$evidence/daemon-wrong-scope.log" 2>&1; then exit 1; fi
target/debug/asenactl lock
sleep 0.3
[[ $(grep -c 'Lock surface assigned' "$log_file") == "${ASENA_LOCK_TEST_OUTPUTS:-2}" ]]
timeout 8 grim "$evidence/lock.png"
ASENA_LOG_STDERR=1 timeout 8 target/debug/asena-lock >"$evidence/lock-duplicate.log" 2>&1
wtype -- 'incorrect-asena-test-password'
wtype -k Return
for _ in {1..100}; do grep -Fq 'accepted=false' "$log_file" && break; sleep 0.1; done
grep -Fq 'accepted=false' "$log_file"
kill -0 "$locker_pid"
ready
timeout 8 grim "$evidence/lock-wrong-password.png"
# DPMS is a separate destructive-output fixture. On the nested Wayland backend
# it can invalidate input surfaces even after a restore request is acknowledged.
printf '%s\n' 'DPMS is not part of the authentication fixture.' >"$evidence/power-probe.log"
rm -f -- "$evidence/lease-marker"
ASENA_LEASE_MARKER="$evidence/lease-marker" target/debug/examples/lock_lease_probe >"$evidence/lease-probe.log" 2>&1 &
lease_pid=$!
for _ in {1..100}; do [[ $(cat "$evidence/lease-marker" 2>/dev/null || true) == held ]] && break; sleep 0.05; done
[[ $(cat "$evidence/lease-marker") == held ]]
sleep 0.1
gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Lock --object-path /org/asenalinux/Lock --method org.freedesktop.DBus.Properties.Get org.asenalinux.Lock SleepPending | grep -F true
python3 - "$password_file" <<'PY'
import subprocess, sys
from pathlib import Path
subprocess.run(['wtype', '-'], input=Path(sys.argv[1]).read_bytes(), check=True)
PY
wtype -k Return
kill -0 "$locker_pid"
ready
wait "$lease_pid"
lease_pid=''
gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Lock --object-path /org/asenalinux/Lock --method org.freedesktop.DBus.Properties.Get org.asenalinux.Lock SleepPending | grep -F false
rm -f -- "$evidence/lease-marker"
ASENA_LEASE_MARKER="$evidence/lease-marker" target/debug/examples/lock_lease_probe --abandon >>"$evidence/lease-probe.log" 2>&1
for _ in {1..100}; do
    if gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Lock --object-path /org/asenalinux/Lock --method org.freedesktop.DBus.Properties.Get org.asenalinux.Lock SleepPending | grep -Fq false; then break; fi
    sleep 0.05
done
gdbus call --address "$DBUS_SESSION_BUS_ADDRESS" --dest org.asenalinux.Lock --object-path /org/asenalinux/Lock --method org.freedesktop.DBus.Properties.Get org.asenalinux.Lock SleepPending | grep -F false
# Through asena-daemon: wlr-randr --off/--on aborts wlroots 0.18 on these
# mode-less outputs while the daemon follows the heads (probe-displays.py).
if ! timeout 20 python3 -I scripts/probe-displays.py HEADLESS-1 --disable-and-revert >"$evidence/locked-output-config.log" 2>&1; then
    printf '%s\n' 'Output configuration while locked failed; this does not verify hotplug.' >>"$evidence/locked-output-config.log"
fi
ready
unlock_with_test_password() {
# Read the disposable secret through stdin. Never put it in argv or output.
python3 - "$password_file" <<'PY'
import subprocess, sys
from pathlib import Path
subprocess.run(['wtype', '-'], input=Path(sys.argv[1]).read_bytes(), check=True)
PY
wtype -k Return
for _ in {1..150}; do kill -0 "$locker_pid" 2>/dev/null || break; sleep 0.1; done
if kill -0 "$locker_pid" 2>/dev/null; then printf '%s\n' 'Correct-password unlock timed out' >&2; exit 1; fi
if [[ $locker_child == 1 ]]; then
    wait "$locker_pid"
else
    # The daemon owns and reaps this child. Observe its authentication result.
    grep -Fq 'accepted=true' "$log_file"
fi
locker_pid=''
}
unlock_with_test_password
cp "$log_file" "$evidence/lock.log"
sleep 0.2
timeout 8 grim "$evidence/unlocked.png"
ASENA_LOG_STDERR=1 target/debug/asena-lock >"$evidence/lock-crash.log" 2>&1 &
locker_pid=$!
ready
kill -KILL "$locker_pid"
wait "$locker_pid" 2>/dev/null || true
locker_pid=''
sleep 0.2
timeout 8 grim "$evidence/lock-after-crash.png"
ASENA_LOG_STDERR=1 target/debug/asena-lock >"$evidence/lock-recovery.log" 2>&1 &
locker_pid=$!
locker_child=1
if ready; then
    # The compositor permits a replacement locker after the old owner has died. The new
    # locker must still authenticate; acquiring an abandoned lock is no unlock.
    sleep 0.2
    kill -0 "$locker_pid"
    if grep -Fq 'accepted=true' "$evidence/lock-recovery.log"; then
        printf '%s\n' 'Recovery locker authenticated without input.' >&2
        exit 1
    fi
    timeout 8 grim "$evidence/lock-recovery.png"
    unlock_with_test_password
    printf '%s\n' 'Abandoned lock accepted a new locker; PAM was required to unlock.'
else
    kill "$locker_pid" 2>/dev/null || true
    wait "$locker_pid" 2>/dev/null || true
    locker_pid=''
    grep -F 'Compositor rejected session lock' "$evidence/lock-recovery.log"
fi
printf '%s\n' 'Lock surfaces, duplicate, wrong/correct PAM password and crash/recovery probe passed. Hotplug remains a separate gate.'
