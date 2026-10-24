#!/bin/bash
# Temporary system bus activation and authorization checks. VM only.
set -euo pipefail
[[ ${ASENA_RECOVERY_TEST_VM:-} == 1 ]] || { printf '%s\n' 'Run only in the disposable Debian VM with ASENA_RECOVERY_TEST_VM=1.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
paths=(/usr/bin/asena-recovery /usr/lib/asena/recovery.py
    /etc/dbus-1/system.d/org.asenalinux.Recovery.conf
    /usr/local/share/dbus-1/system-services/org.asenalinux.Recovery.service
    /usr/share/polkit-1/actions/org.asenalinux.recovery.policy
    /run/systemd/system/asena-recovery.service)
for path in "${paths[@]}"; do [[ ! -e $path ]] || { printf 'Refusing to replace %s\n' "$path" >&2; exit 1; }; done
cleanup() {
    trap - EXIT
    sudo systemctl stop asena-recovery.service || true
    sudo rm -f -- "${paths[@]}"
    sudo systemctl daemon-reload
    sudo busctl call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus ReloadConfig >/dev/null
}
trap cleanup EXIT
sudo install -D -m 0755 "$repo/target/debug/asena-recovery" "${paths[0]}"
sudo install -D -m 0644 "$repo/data/recovery/recovery.py" "${paths[1]}"
sudo install -D -m 0644 "$repo/data/dbus-1/system.d/org.asenalinux.Recovery.conf" "${paths[2]}"
sudo install -D -m 0644 "$repo/data/dbus-1/system-services/org.asenalinux.Recovery.service" "${paths[3]}"
sudo install -D -m 0644 "$repo/data/polkit/org.asenalinux.recovery.policy" "${paths[4]}"
sudo install -D -m 0644 "$repo/data/systemd/system/asena-recovery.service" "${paths[5]}"
sudo systemctl daemon-reload
sudo busctl call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus ReloadConfig >/dev/null
python3 -I - <<'PY'
import json
import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib
bus = Gio.bus_get_sync(Gio.BusType.SYSTEM)
def call(method, parameters=None):
    return bus.call_sync('org.asenalinux.Recovery', '/org/asenalinux/Recovery',
                         'org.asenalinux.Recovery', method, parameters, None,
                         Gio.DBusCallFlags.NONE, 30000, None)
status = json.loads(call('Status').unpack()[0])
assert not status['available'] and not status['offline'], status
for method, parameters in [('Schedule', GLib.Variant('(s)', ('keep',))),
                           ('Reset', GLib.Variant('(sss)', ('remove', 'user', 'test-password')))]:
    try:
        call(method, parameters)
        raise AssertionError('Unprivileged reset was accepted')
    except GLib.Error as error:
        assert Gio.DBusError.get_remote_error(error) == 'org.freedesktop.DBus.Error.AccessDenied', error
print('PASS: system bus activation/status; unauthorized Schedule and Reset rejected')
PY
