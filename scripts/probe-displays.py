"""Display change acceptance probe for a running development session.

Talks to asena-daemon like Settings does: apply, confirm, revert, the 15
second deadline and a client that exits before confirming. Restores scale 1.
Usage: probe-displays.py OUTPUT [--disable-and-revert]
"""
import json
import os
import subprocess
import sys
import time

import gi

gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib  # noqa: E402

NAME = 'org.asenalinux.Daemon'
PATH = '/org/asenalinux/Daemon'


def scope():
    display = os.environ['WAYLAND_DISPLAY']
    socket = display if os.path.isabs(display) else os.path.join(os.environ['XDG_RUNTIME_DIR'], display)
    session = os.environ.get('XDG_SESSION_ID', '')
    return 'v1:' + socket.encode().hex() + ':' + session.encode().hex()


def call(bus, method, args=None, signature=None):
    parameters = GLib.Variant(signature, args) if signature else None
    reply = bus.call_sync(NAME, PATH, NAME, method, parameters, None, Gio.DBusCallFlags.NONE, 30000, None)
    return reply.unpack()


def layout(bus):
    return json.loads(call(bus, 'DisplayLayout')[0])


def output(bus, name):
    return next(item for item in layout(bus)['outputs'] if item['state']['name'] == name)


def configs(bus, name, **changes):
    states = [item['state'] for item in layout(bus)['outputs']]
    for state in states:
        if state['name'] == name:
            state.update(changes)
    return json.dumps(states)


def apply(bus, name, **changes):
    return call(bus, 'ApplyDisplays', (scope(), configs(bus, name, **changes)), '(ss)')[0]


def refused(bus, name, **changes):
    try:
        apply(bus, name, **changes)
    except GLib.Error as error:
        return error.message
    raise SystemExit(f'Expected a refusal for {changes}')


def refused_all_off(bus):
    states = [item['state'] for item in layout(bus)['outputs']]
    for state in states:
        state['enabled'] = False
    try:
        call(bus, 'ApplyDisplays', (scope(), json.dumps(states)), '(ss)')
    except GLib.Error as error:
        return error.message
    raise SystemExit('Expected a refusal with every display off')


def wait_for(bus, name, scale, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        state = layout(bus)
        if abs(output(bus, name)['state']['scale'] - scale) < 1e-6 and not state['pending']:
            return time.monotonic()
        time.sleep(0.1)
    raise SystemExit(f'{name} did not return to scale {scale} within {seconds} s')


def wait_enabled(bus, name, enabled, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if output(bus, name)['state']['enabled'] == enabled:
            return
        time.sleep(0.1)
    raise SystemExit(f'{name} enabled did not become {enabled} within {seconds} s')


def disable_and_revert(bus, name):
    """Removes and restores an output through the daemon.

    wlroots 0.18 aborts when a client that bound while a mode-less output
    (headless, nested) was off turns it on while another client follows the
    heads, as asena-daemon does. The daemon itself is safe to use here.
    """
    assert apply(bus, name, enabled=False) == 15
    wait_enabled(bus, name, False, 5)
    call(bus, 'RevertDisplays', (scope(),), '(s)')
    wait_enabled(bus, name, True, 5)
    assert not layout(bus)['pending']
    print(f'{name} turned off and restored through asena-daemon.')


def main():
    name = sys.argv[1]
    if sys.argv[2:] == ['--apply-and-exit']:
        # A Settings window that disappears before the user answers.
        bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        print(apply(bus, name, scale=2.0), flush=True)
        os._exit(0)
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    if sys.argv[2:] == ['--disable-and-revert']:
        disable_and_revert(bus, name)
        return
    if not layout(bus)['available']:
        raise SystemExit('Output management is unavailable')
    if output(bus, name)['state']['scale'] != 1.0:
        raise SystemExit(f'{name} must start at scale 1')

    seconds = apply(bus, name, scale=2.0)
    assert seconds == 15, seconds
    assert layout(bus)['pending'] and output(bus, name)['state']['scale'] == 2.0
    print('Applied scale 2; waiting for the deadline.')
    message = refused(bus, name, scale=1.5)
    print('Second change while pending refused:', message)
    started = time.monotonic()
    reverted = wait_for(bus, name, 1.0, 20)
    print(f'Reverted after {reverted - started:.1f} s without confirmation.')

    apply(bus, name, scale=2.0)
    call(bus, 'RevertDisplays', (scope(),), '(s)')
    wait_for(bus, name, 1.0, 3)
    print('Explicit revert restored scale 1.')

    child = subprocess.run([sys.executable, '-I', __file__, name, '--apply-and-exit'],
                           capture_output=True, text=True, check=True, timeout=30)
    assert child.stdout.strip() == '15', child.stdout
    started = time.monotonic()
    reverted = wait_for(bus, name, 1.0, 5)
    print(f'Client exit reverted after {reverted - started:.1f} s.')

    apply(bus, name, scale=2.0)
    call(bus, 'ConfirmDisplays', (scope(),), '(s)')
    time.sleep(16)
    assert output(bus, name)['state']['scale'] == 2.0 and not layout(bus)['pending']
    print('Confirmed scale 2 stayed after 16 s.')
    try:
        call(bus, 'ConfirmDisplays', (scope(),), '(s)')
        raise SystemExit('Confirming twice must fail')
    except GLib.Error:
        pass

    print('Invalid scale refused:', refused(bus, name, scale=8.0))
    print('All displays off refused:', refused_all_off(bus))
    try:
        call(bus, 'ApplyDisplays', ('wrong-session', configs(bus, name, scale=1.0)), '(ss)')
        raise SystemExit('Another session must be refused')
    except GLib.Error as error:
        print('Other session refused:', error.message)

    assert apply(bus, name, scale=1.0) == 15
    call(bus, 'ConfirmDisplays', (scope(),), '(s)')
    assert output(bus, name)['state']['scale'] == 1.0
    assert apply(bus, name, scale=1.0) == 0
    print('Restored and confirmed scale 1; an unchanged layout needs no confirmation.')


if __name__ == '__main__':
    main()
