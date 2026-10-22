"""Unprivileged greetd protocol fixture. Never authenticates or starts a session."""
import json
import shlex
import socket
import struct
import sys
from pathlib import Path

root = Path(sys.argv[1])
marker = root / 'stage'


def receive(client):
    def exact(size):
        data = b''
        while len(data) < size:
            part = client.recv(size - len(data))
            if not part:
                raise RuntimeError('Unexpected EOF')
            data += part
        return data
    size, = struct.unpack('=I', exact(4))
    assert 0 < size <= 65536
    return json.loads(exact(size))


def send(client, value):
    data = json.dumps(value).encode()
    client.sendall(struct.pack('=I', len(data)) + data)


with socket.socket(socket.AF_UNIX) as listener:
    listener.bind(str(root / 'greetd.sock'))
    listener.settimeout(20)
    listener.listen(1)
    marker.write_text('listening')
    for attempt in (1, 2, 3):
        client, _ = listener.accept()
        with client:
            client.settimeout(15)
            assert receive(client) == {'type': 'create_session', 'username': 'asena'}
            send(client, {'type': 'auth_message', 'auth_message_type': 'secret',
                          'auth_message': 'Password'})
            marker.write_text(f'prompt-{attempt}')
            request = receive(client)
            if attempt == 2:
                assert request == {'type': 'cancel_session'}
                send(client, {'type': 'success'})
                marker.write_text('cancelled')
                continue
            assert request == {'type': 'post_auth_message_response',
                               'response': 'asena-ui-fixture'}
            if attempt == 1:
                send(client, {'type': 'error', 'error_type': 'auth_error',
                              'description': 'Fixture rejected'})
                marker.write_text('rejected')
                continue
            send(client, {'type': 'auth_message', 'auth_message_type': 'info',
                          'auth_message': 'Fixture information'})
            marker.write_text('information')
            assert receive(client) == {'type': 'post_auth_message_response', 'response': None}
            send(client, {'type': 'success'})
            request = receive(client)
            assert request['type'] == 'start_session'
            assert shlex.split(' '.join(request['cmd'])) == [
                '/usr/bin/true', '--name=space value', '$(not-a-command)']
            assert 'XDG_SESSION_TYPE=wayland' in request['env']
            assert not any('GREETD_SOCK' in value for value in request['env'])
            send(client, {'type': 'success'})
            marker.write_text('started')
print('Wrong answer, cancellation, information and selected session argv passed. No real login performed.')
