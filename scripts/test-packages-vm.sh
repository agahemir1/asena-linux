#!/bin/bash
# Development VM only (K21, Phase 6F-A). Installs target/debug/asena-packages
# as a D-Bus activated system service for the length of the test, then talks
# to it on the system bus: unauthorized calls, a temporary polkit rule for the
# calling user, installing and removing `hello`, refusals for protected
# packages, bad names and foreign plans, the upgrade list and scheduling, and
# repairing a package whose maintainer script failed.
# Everything it installs is removed on exit; it never restarts the machine.
# With --hold it only installs the service and a polkit rule for the calling
# user and waits until it is stopped, so Software can be tried in a session.
# Results: target/evidence/packages-test.
# Tools: cargo build -p asena-packages; sudo apt-get install python3-gi
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
[[ ${ASENA_PACKAGES_TEST_VM:-} == 1 ]] || {
    printf '%s\n' 'Set ASENA_PACKAGES_TEST_VM=1: this script installs a root service and packages.' >&2
    exit 1
}
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
binary=$repo/target/debug/asena-packages
[[ -x $binary ]] || { printf '%s\n' 'Build it first: cargo build -p asena-packages' >&2; exit 1; }
evidence=$repo/target/evidence/packages-test
rm -rf -- "$evidence"
mkdir -p "$evidence"

libexec=/usr/local/libexec/asena-packages
activation=/usr/local/share/dbus-1/system-services/org.asenalinux.Packages.service
busconf=/etc/dbus-1/system.d/org.asenalinux.Packages.conf
actions=/usr/share/polkit-1/actions/org.asenalinux.packages.policy
rule=/etc/polkit-1/rules.d/49-asena-packages-test.rules
unit=/run/systemd/system/asena-packages.service
# The postinst of asena-repair-test fails while this file exists.
repair_flag=/run/asena-repair-test-fail
local_package=''
for path in "$activation" "$busconf" "$actions" "$rule" "$unit"; do
    [[ ! -e $path ]] || { printf 'Refusing to replace %s.\n' "$path" >&2; exit 1; }
done

# shellcheck disable=SC2317 # Called through the EXIT trap.
cleanup() {
    trap - EXIT
    sudo systemctl stop asena-packages.service 2>/dev/null || true
    sudo rm -f "$repair_flag"
    if dpkg-query -W asena-repair-test >/dev/null 2>&1; then
        sudo dpkg --purge asena-repair-test >/dev/null 2>&1 || true
    fi
    sudo dpkg --purge asena-local-file-test >/dev/null 2>&1 || true
    if [[ -n $local_package ]]; then rm -rf -- "$local_package" "$local_package.deb"; fi
    sudo rm -f "$libexec" "$activation" "$busconf" "$actions" "$rule" "$unit"
    sudo systemctl daemon-reload
    sudo busctl call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus ReloadConfig >/dev/null
    if [[ -L /system-update ]] && [[ $(readlink /system-update) == /var/cache/apt/archives ]]; then
        sudo rm -f /system-update
    fi
}
trap cleanup EXIT

sudo install -D -m 0755 "$binary" "$libexec"
sudo install -D -m 0644 "$repo/data/dbus-1/system-services/org.asenalinux.Packages.service" "$activation"
sudo install -D -m 0644 "$repo/data/dbus-1/system.d/org.asenalinux.Packages.conf" "$busconf"
sudo install -D -m 0644 "$repo/data/polkit/org.asenalinux.packages.policy" "$actions"
sed "s|/usr/libexec/asena-packages|$libexec|" "$repo/data/systemd/system/asena-packages.service" |
    sudo install -D -m 0644 /dev/stdin "$unit"
sudo systemctl daemon-reload
sudo busctl call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus ReloadConfig >/dev/null
allow_caller() {
    printf 'polkit.addRule(function(action, subject) {\n    if (action.id.indexOf("org.asenalinux.packages.") == 0 && subject.user == "%s") {\n        return polkit.Result.YES;\n    }\n});\n' \
        "$(id -un)" | sudo install -D -m 0644 /dev/stdin "$rule"
}
if [[ ${1:-} == --hold ]]; then
    allow_caller
    printf '%s\n' 'The package service is installed; stop this script to remove it.'
    trap 'exit 0' INT TERM
    while true; do
        sleep 3600 &
        wait $!
    done
fi
if dpkg-query -W -f='${Status}' hello 2>/dev/null | grep -q 'ok installed'; then
    printf '%s\n' 'hello is already installed; remove it before the test.' >&2
    exit 1
fi

client() {
    python3 -I - "$@" <<'PY'
import json, sys
import gi
gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib

bus = Gio.bus_get_sync(Gio.BusType.SYSTEM)

def call(method, args=None, signature=None):
    try:
        reply = bus.call_sync("org.asenalinux.Packages", "/org/asenalinux/Packages",
                              "org.asenalinux.Packages", method,
                              GLib.Variant(signature, args) if signature else None,
                              None, Gio.DBusCallFlags.NONE, 600000, None)
        return "ok", reply.unpack()
    except GLib.Error as error:
        return "error", Gio.DBusError.get_remote_error(error) or error.message

command = sys.argv[1]
if command == "plan-file":
    descriptors = Gio.UnixFDList.new()
    with open(sys.argv[2], 'rb') as package:
        handle = descriptors.append(package.fileno())
    reply, _ = bus.call_with_unix_fd_list_sync(
        "org.asenalinux.Packages", "/org/asenalinux/Packages", "org.asenalinux.Packages",
        "PlanFile", GLib.Variant('(h)', (handle,)), None, Gio.DBusCallFlags.NONE,
        600000, descriptors, None)
    plan = json.loads(reply.unpack()[0])
    assert any(change['name'] == 'asena-local-file-test' for change in plan['changes']), plan
    status, value = call('Commit', (plan['id'],), '(s)')
    assert status == 'ok', value
    print('local file install passed', plan['changes'])
elif command == "plan-commit":
    install, remove = sys.argv[2].split(",") if sys.argv[2] else [], sys.argv[3].split(",") if sys.argv[3] else []
    status, value = call("Plan", (install, remove), "(asas)")
    print("plan", status, value if status == "error" else json.loads(value[0])["changes"])
    if status == "ok":
        print("commit", *call("Commit", (json.loads(value[0])["id"],), "(s)"))
elif command == "plan":
    install, remove = sys.argv[2].split(",") if sys.argv[2] else [], sys.argv[3].split(",") if sys.argv[3] else []
    status, value = call("Plan", (install, remove), "(asas)")
    print("plan", status, value if status == "error" else value[0])
elif command == "commit":
    print("commit", *call("Commit", (sys.argv[2],), "(s)"))
else:
    print(command, *call(command))
PY
}

{
    printf '== unauthorized refresh (no rule, no agent)\n'
    client Refresh
    printf '== bad names\n'
    client plan -o ''
    client plan 'hello world' ''
    printf '== protected removal\n'
    client plan '' sudo
    printf '== unknown package\n'
    client plan asena-no-such-package ''
} >"$evidence/refusals.txt" 2>&1

# A plan from one connection cannot be committed from another.
plan_id=$(client plan hello '' | sed -n 's/^plan ok //p' | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
client commit "$plan_id" >"$evidence/foreign-plan.txt" 2>&1

allow_caller
local_package=$(mktemp -d)
mkdir -p "$local_package/DEBIAN" "$local_package/usr/share/asena-local-file-test"
printf 'Package: asena-local-file-test\nVersion: 1.0\nArchitecture: all\nMaintainer: Asena <asena@localhost>\nDescription: Local file installation test\n' >"$local_package/DEBIAN/control"
printf '%s\n' 'local package installed' >"$local_package/usr/share/asena-local-file-test/proof"
dpkg-deb --root-owner-group --build "$local_package" "$local_package.deb" >/dev/null
client plan-file "$local_package.deb" >"$evidence/local-file.txt" 2>&1
[[ $(cat /usr/share/asena-local-file-test/proof) == 'local package installed' ]]
client plan-commit '' asena-local-file-test >>"$evidence/local-file.txt" 2>&1
[[ ! -e /usr/share/asena-local-file-test/proof ]]
{
    printf '== refresh\n'
    client Refresh
    printf '== install hello\n'
    client plan-commit hello ''
    dpkg-query -W -f='${Package} ${Version} ${Status}\n' hello
    printf '== remove hello\n'
    client plan-commit '' hello
    dpkg-query -W -f='${Package} ${Status}\n' hello 2>&1 || true
    printf '== upgrades\n'
    client Upgrades
    printf '== schedule and cancel\n'
    client ScheduleUpgrade
    ls -l /system-update 2>&1 || true
    client CancelUpgrade
    ls -l /system-update 2>&1 || true
    printf '== properties\n'
    busctl get-property org.asenalinux.Packages /org/asenalinux/Packages org.asenalinux.Packages Busy NeedsRepair UpgradeScheduled
} >"$evidence/authorized.txt" 2>&1

# A failing postinst leaves its package half configured; Repair finishes it
# once the cause is gone.
package=$(mktemp -d)
mkdir -p "$package/DEBIAN"
printf 'Package: asena-repair-test\nVersion: 1.0\nArchitecture: all\nMaintainer: Asena <asena@localhost>\nDescription: Repair test package\n' \
    >"$package/DEBIAN/control"
printf '#!/bin/sh\n[ ! -e %s ]\n' "$repair_flag" >"$package/DEBIAN/postinst"
chmod 0755 "$package/DEBIAN/postinst"
dpkg-deb --root-owner-group --build "$package" "$package.deb" >/dev/null
{
    printf '== failing postinst\n'
    sudo touch "$repair_flag"
    sudo dpkg -i "$package.deb" 2>&1 | tail -n 2 || true
    dpkg-query -W -f='${Package} ${db:Status-Status}\n' asena-repair-test
    busctl get-property org.asenalinux.Packages /org/asenalinux/Packages org.asenalinux.Packages NeedsRepair
    printf '== repair\n'
    sudo rm -f "$repair_flag"
    client Repair
    dpkg-query -W -f='${Package} ${db:Status-Status}\n' asena-repair-test
    busctl get-property org.asenalinux.Packages /org/asenalinux/Packages org.asenalinux.Packages NeedsRepair
} >"$evidence/repair.txt" 2>&1
rm -rf -- "$package" "$package.deb"
sudo dpkg --purge asena-repair-test >/dev/null
journalctl -u asena-packages.service --since '-15 min' --no-pager >"$evidence/journal.txt" 2>&1 || true
cat "$evidence/refusals.txt" "$evidence/foreign-plan.txt" "$evidence/local-file.txt" "$evidence/authorized.txt" "$evidence/repair.txt"
