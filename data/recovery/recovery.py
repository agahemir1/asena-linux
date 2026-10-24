#!/usr/bin/python3
"""Privileged reset engine. Called only by org.asenalinux.Recovery.

Device identities and image hashes come from the root-owned installation
manifest, never from D-Bus arguments. No partition is formatted or repartitioned.
"""
import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys

CONFIG = Path('/etc/asena/recovery.json')
MOUNT = Path('/run/asena-recovery')
TARGET = Path('/run/asena-reset-target')
IMAGE = Path('/run/asena-reset-image')
ENTRY = 'asena-recovery'
UUID = re.compile(r'[0-9a-fA-F-]{8,64}\Z')
LOGIN = re.compile(r'[a-z_][a-z0-9_-]{0,30}\Z')
MODES = {'keep', 'remove'}
ACCOUNT_FILES = ('passwd', 'shadow', 'group', 'gshadow', 'subuid', 'subgid')
KEPT_FILES = ('hostname', 'hosts', 'locale.conf', 'default/locale', 'default/keyboard',
              'timezone', 'machine-id')


def run(*args, input=None):
    result = subprocess.run(args, input=input, text=True, capture_output=True, check=False,
                            env={**os.environ, 'PATH': '/usr/sbin:/usr/bin:/sbin:/bin',
                                 'LC_ALL': 'C', 'DEBIAN_FRONTEND': 'noninteractive'})
    if result.returncode:
        # Do not echo arguments, stdin, account files or maintainer output.
        raise RuntimeError(f'{Path(args[0]).name} failed ({result.returncode})')
    return result.stdout.strip()


def atomic_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    temporary = path.with_suffix('.new')
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW
    with os.fdopen(os.open(temporary, flags, 0o600), 'w') as stream:
        json.dump(value, stream)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)
    descriptor = os.open(path.parent, os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def read_json(path):
    if path.is_symlink() or not path.is_file():
        raise RuntimeError('Recovery configuration is missing')
    return json.loads(path.read_text())


def validate_config(config):
    if config.get('version') != 1:
        raise ValueError('Unsupported recovery manifest')
    for key in ('root_uuid', 'recovery_uuid'):
        if not UUID.fullmatch(config.get(key, '')):
            raise ValueError('Invalid filesystem identity')
    if config['root_uuid'] == config['recovery_uuid']:
        raise ValueError('Recovery and system must be separate partitions')
    if config.get('esp_uuid') and not UUID.fullmatch(config['esp_uuid']):
        raise ValueError('Invalid EFI identity')
    if config.get('esp_uuid') in (config['root_uuid'], config['recovery_uuid']):
        raise ValueError('EFI must be a separate partition')
    return config


def device(uuid):
    if not UUID.fullmatch(uuid):
        raise ValueError('Invalid filesystem identity')
    path = Path('/dev/disk/by-uuid') / uuid
    if not stat.S_ISBLK(path.stat().st_mode):
        raise RuntimeError('Recovery device is not a block device')
    return str(path.resolve(strict=True))


def filesystem(path):
    return run('blkid', '-s', 'TYPE', '-o', 'value', str(path))


def mounted_targets(path):
    result = subprocess.run(['findmnt', '-rn', '-S', str(path), '-o', 'TARGET'],
                            text=True, capture_output=True, check=False)
    return result.stdout.splitlines()


def writable(target):
    # mount(8) falls back to read-only without failing when the device is
    # already mounted read-only elsewhere.
    options = run('findmnt', '-nro', 'OPTIONS', '--mountpoint', str(target)).split(',')
    return 'rw' in options


@contextlib.contextmanager
def mounted(source, target, options):
    target.mkdir(parents=True, exist_ok=True, mode=0o700)
    if target.is_symlink() or os.path.ismount(target):
        raise RuntimeError('Recovery mount point is already in use')
    run('mount', '-o', options, str(source), str(target))
    try:
        if 'rw' in options.split(',') and not writable(target):
            raise RuntimeError('The partition could not be mounted for writing')
        yield target
    finally:
        run('umount', str(target))


@contextlib.contextmanager
def remounted_writable(target):
    # live-boot mounts the recovery partition read-only and keeps it busy with
    # the running image; a second mount would share that read-only state.
    run('mount', '-o', 'remount,rw', str(target))
    try:
        if not writable(target):
            raise RuntimeError('The recovery partition could not be made writable')
        yield target
    finally:
        subprocess.run(['mount', '-o', 'remount,ro', str(target)], capture_output=True, check=False)


def offline():
    return 'asena.recovery=1' in Path('/proc/cmdline').read_text().split()


@contextlib.contextmanager
def recovery(write=True):
    if offline():
        # live-boot found this filesystem by UUID from our GRUB entry.
        medium = Path('/run/live/medium')
        config = validate_config(read_json(medium / 'asena/recovery.json'))
        device(config['recovery_uuid'])
        actual = run('findmnt', '-nro', 'UUID', '--target', str(medium))
        if actual != config['recovery_uuid']:
            raise RuntimeError('The booted recovery partition does not match')
        if not write:
            yield config, medium
            return
        with remounted_writable(medium) as partition:
            yield config, partition
    else:
        config = validate_config(read_json(CONFIG))
        root_uuid = run('findmnt', '-nro', 'UUID', '--target', '/')
        if root_uuid != config['root_uuid'] or filesystem(device(root_uuid)) != 'ext4':
            raise RuntimeError('This system layout is not supported for reset')
        if run('findmnt', '-nro', 'UUID', '--target', '/home') != root_uuid:
            raise RuntimeError('A separate home filesystem is not supported for reset')
        source = device(config['recovery_uuid'])
        if filesystem(source) != 'ext4' or mounted_targets(source):
            raise RuntimeError('The recovery partition is unavailable')
        # Status only reads; it must not mount the partition for writing.
        options = 'rw,nosuid,nodev,noexec' if write else 'ro,nosuid,nodev,noexec'
        with mounted(source, MOUNT, options) as partition:
            installed = validate_config(read_json(partition / 'asena/recovery.json'))
            if installed != config:
                raise RuntimeError('Recovery manifests do not match')
            yield config, partition


def verify_image(partition):
    manifest = read_json(partition / 'asena/image.json')
    expected = {'live/filesystem.squashfs', 'live/vmlinuz', 'live/initrd.img'}
    if set(manifest) != expected:
        raise RuntimeError('Incomplete recovery image')
    for name, digest in manifest.items():
        path = partition / name
        if path.is_symlink() or not path.is_file() or not re.fullmatch(r'[0-9a-f]{64}', digest):
            raise RuntimeError('Invalid recovery image')
        with path.open('rb') as stream:
            actual = hashlib.file_digest(stream, 'sha256').hexdigest()
        if actual != digest:
            raise RuntimeError('The recovery image is damaged')


def rows(path):
    if not path.exists():
        return []
    if path.is_symlink():
        raise RuntimeError('Account files must not be symbolic links')
    return [line.split(':') for line in path.read_text().splitlines() if line]


def preserved_accounts(root):
    passwd = rows(root / 'etc/passwd')
    users = [row for row in passwd if len(row) == 7 and 1000 <= int(row[2]) < 65534]
    if not users:
        raise RuntimeError('There are no accounts to preserve')
    for row in users:
        home = Path(row[5])
        if (not LOGIN.fullmatch(row[0]) or home.parent != Path('/home')
                or home.name in ('', '.', '..') or (root / home.relative_to('/')).is_symlink()):
            raise RuntimeError('A user has an unsupported home folder')
    names = {row[0] for row in users}
    gids = {row[3] for row in users}
    groups = [row for row in rows(root / 'etc/group') if len(row) == 4]
    # Keep group identity plus only the human memberships of system groups.
    kept_groups = []
    for row in groups:
        members = [name for name in row[3].split(',') if name in names]
        if row[2] in gids or int(row[2]) >= 1000 or members:
            kept_groups.append([*row[:3], ','.join(members)])
    group_names = {row[0] for row in kept_groups}
    result = {'passwd': users, 'group': kept_groups}
    for name in ('shadow', 'subuid', 'subgid'):
        result[name] = [row for row in rows(root / 'etc' / name) if row[0] in names]
    result['gshadow'] = [row for row in rows(root / 'etc/gshadow') if row[0] in group_names]
    if {row[0] for row in result['shadow']} != names:
        raise RuntimeError('Account password records are incomplete')
    return result


def merge_accounts(base, preserved, kind):
    result = {row[0]: list(row) for row in base}
    for row in preserved:
        prior = result.get(row[0])
        if kind in ('passwd', 'group'):
            if prior and prior[2] != row[2]:
                raise RuntimeError('Account names collide with the recovery image')
            if any(other[0] != row[0] and other[2] == row[2] for other in base):
                raise RuntimeError('Account IDs collide with the recovery image')
        if kind == 'group' and prior:
            row = [*row[:3], ','.join(sorted(set(filter(None, (prior[3]+','+row[3]).split(',')))))]
        if kind == 'gshadow' and prior:
            row = [*row[:2], prior[2], ','.join(sorted(set(filter(None, (prior[3]+','+row[3]).split(',')))))]
        result[row[0]] = row
    return '\n'.join(':'.join(row) for row in result.values()) + '\n'


def safe_remove(path):
    if path.is_symlink() or not path.is_dir():
        path.unlink(missing_ok=True)
    else:
        shutil.rmtree(path)


def clear_system(root, keep):
    # The caller mounted and validated the ext4 partition. Keep GRUB available
    # through a power failure; its recovery entry can resume the same operation.
    for item in root.iterdir():
        if item.name in ('boot', 'lost+found') or (keep and item.name == 'home'):
            continue
        safe_remove(item)


def validate_layout(root):
    fstab = root / 'etc/fstab'
    if fstab.is_symlink():
        raise RuntimeError('Unsupported filesystem table')
    if fstab.is_file():
        for line in fstab.read_text().splitlines():
            fields = line.split()
            if fields and not fields[0].startswith('#'):
                if len(fields) < 4 or fields[1] not in ('/', '/boot/efi', 'none'):
                    raise RuntimeError('Separate system or home filesystems are not supported')


def remove_obsolete_kernels(root, image):
    # Leaving a newer third-party kernel in /boot would make GRUB select it
    # after its matching modules have been removed by the factory restore.
    for pattern in ('vmlinuz-*', 'initrd.img-*', 'System.map-*', 'config-*'):
        for path in (root / 'boot').glob(pattern):
            if not (image / 'boot' / path.name).exists():
                if path.is_dir() and not path.is_symlink():
                    raise RuntimeError('Unexpected directory in the boot files')
                path.unlink()


def remove_user_flatpaks(root, accounts):
    for user in accounts['passwd']:
        home = root / user[5].lstrip('/')
        path = home / '.local/share/flatpak'
        # Never follow a user-controlled ancestor out of the mounted root.
        for ancestor in (home, home / '.local', home / '.local/share'):
            if ancestor.is_symlink():
                raise RuntimeError('Flatpak data has a symbolic link ancestor')
        safe_remove(path)


def copy_config(source, target):
    if source.is_symlink():
        raise RuntimeError('A preserved configuration is a symbolic link')
    if source.is_dir():
        if any(path.is_symlink() for path in source.rglob('*')):
            raise RuntimeError('A preserved configuration contains symbolic links')
        shutil.copytree(source, target, dirs_exist_ok=True)
    elif source.is_file():
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)


def prepare(root, image, state, mode):
    state.mkdir(mode=0o700)
    accounts = preserved_accounts(root) if mode == 'keep' else {}
    if mode == 'keep':
        for name in ACCOUNT_FILES:
            merge_accounts(rows(image / 'etc' / name), accounts[name], name)
        # Check these before changing the installed system.
        for user in accounts['passwd']:
            home = root / user[5].lstrip('/')
            if any(path.is_symlink() for path in (home / '.local', home / '.local/share')):
                raise RuntimeError('Flatpak data has a symbolic link ancestor')
        for name in (*KEPT_FILES, 'NetworkManager/system-connections'):
            copy_config(root / 'etc' / name, state / 'etc' / name)
        localtime = root / 'etc/localtime'
        if localtime.exists():
            # localtime is usually an absolute symlink; dereference only zoneinfo.
            resolved = localtime.resolve()
            if localtime.is_symlink():
                link = os.readlink(localtime)
                if not link.startswith('/usr/share/zoneinfo/') or '..' in Path(link).parts:
                    raise RuntimeError('Unsupported timezone link')
                resolved = root / link.lstrip('/')
            shutil.copyfile(resolved, state / 'etc/localtime')
    # Only what the user added; the image brings the rest back.
    packages = sorted(installed_packages(root) - installed_packages(image))
    flatpaks = []
    for user in accounts.get('passwd', []):
        applications = root / user[5].lstrip('/') / '.local/share/flatpak/app'
        if applications.is_dir() and not applications.is_symlink():
            flatpaks.extend(path.name for path in applications.iterdir())
    atomic_json(state / 'accounts.json', accounts)
    atomic_json(state / 'removed-applications.json', {'packages': packages, 'flatpaks': sorted(flatpaks)})
    atomic_json(state / 'transaction.json', {'mode': mode, 'prepared': True})
    return accounts


def installed_packages(root):
    status = root / 'var/lib/dpkg/status'
    if not status.is_file() or status.is_symlink():
        return set()
    names = set()
    for entry in status.read_text().split('\n\n'):
        name = re.search(r'^Package: (.+)$', entry, re.M)
        if name and re.search(r'^Status: .* installed$', entry, re.M):
            names.add(name.group(1))
    return names


# The reset rebuilds the initramfs and boot menu inside the restored image.
REQUIRED_TOOLS = ('usr/sbin/update-initramfs', 'usr/sbin/update-grub')


def check_image_tools(image):
    if any(not (image / tool).exists() for tool in REQUIRED_TOOLS):
        raise RuntimeError('The recovery image cannot rebuild the boot files')


def validate_new_account(username, password):
    if not LOGIN.fullmatch(username) or username in ('root', 'nobody', 'asena'):
        raise ValueError('Choose a valid new username other than root, nobody or asena')
    if len(password) < 8 or any(character in password for character in '\r\n\0:'):
        raise ValueError('Choose a password of at least eight characters')


def reset(config, partition, request):
    if not offline():
        raise RuntimeError('Reset is available only in Asena Recovery')
    mode = request.get('mode')
    if mode not in MODES:
        raise ValueError('Invalid reset mode')
    if mode == 'remove':
        validate_new_account(request.get('username', ''), request.get('password', ''))
    root_device = device(config['root_uuid'])
    if filesystem(root_device) != 'ext4' or mounted_targets(root_device):
        raise RuntimeError('The system partition is mounted or unsupported')
    verify_image(partition)
    state = partition / 'asena/state'
    with mounted(partition / 'live/filesystem.squashfs', IMAGE, 'loop,ro,nosuid,nodev') as image:
        if not (image / 'usr/bin/asena-recovery').is_file():
            raise RuntimeError('This image does not contain Asena Recovery')
        # Checked before anything is removed, so a missing tool cannot leave an
        # erased system that returns to recovery on every start.
        check_image_tools(image)
        with mounted(root_device, TARGET, 'rw') as root:
            mounts = json.loads(run('findmnt', '-J', '-R', str(root)))['filesystems']
            if len(mounts) != 1 or mounts[0].get('children'):
                raise RuntimeError('Other filesystems are mounted inside the target')
            for name in ('home', 'boot'):
                if (root / name).is_symlink():
                    raise RuntimeError('The target has an unsupported layout')
            transaction = state / 'transaction.json'
            if mode == 'remove' and any(row[0] == request['username'] for row in rows(image / 'etc/passwd')):
                raise RuntimeError('The new username is reserved by the recovery image')
            if transaction.exists():
                if read_json(transaction).get('mode') != mode:
                    raise RuntimeError('Resume the pending reset before changing its mode')
                accounts = read_json(state / 'accounts.json')
            else:
                validate_layout(root)
                # Incomplete preparation can be discarded; destruction has not begun.
                if state.exists():
                    shutil.rmtree(state)
                accounts = prepare(root, image, state, mode)
            grubenv = root / 'boot/grub/grubenv'
            if not grubenv.is_file() or grubenv.is_symlink():
                raise RuntimeError('The installed bootloader is not ready for recovery')
            run('grub-editenv', str(grubenv), 'set', f'next_entry={ENTRY}')
            # No destructive operation occurs above this line.
            clear_system(root, mode == 'keep')
            run('rsync', '-aHAX', '--numeric-ids', '--one-file-system', '--exclude=/home/***',
                '--exclude=/boot/efi/***', '--exclude=/boot/grub/***', str(image) + '/', str(root) + '/')
            remove_obsolete_kernels(root, image)
            if mode == 'keep':
                for name in ACCOUNT_FILES:
                    path = root / 'etc' / name
                    path.write_text(merge_accounts(rows(path), accounts[name], name))
                    if name in ('shadow', 'gshadow'):
                        path.chmod(0o640)
                for name in (*KEPT_FILES, 'localtime', 'NetworkManager/system-connections'):
                    destination = root / 'etc' / name
                    # Image localtime can be a symlink into the running environment.
                    if destination.is_symlink():
                        destination.unlink()
                    copy_config(state / 'etc' / name, destination)
                remove_user_flatpaks(root, accounts)
                for user in accounts['passwd']:
                    home = root / user[5].lstrip('/')
                    report = home / 'Asena-removed-applications.json'
                    if not report.is_symlink():
                        shutil.copyfile(state / 'removed-applications.json', report)
                        os.chown(report, int(user[2]), int(user[3]))
            else:
                login = request['username']
                run('chroot', str(root), 'useradd', '--create-home', '--groups', 'sudo', '--shell', '/bin/bash', login)
                run('chroot', str(root), 'chpasswd', '--crypt-method', 'SHA512', input=f"{login}:{request['password']}\n")
            (root / 'etc/fstab').write_text(
                f"UUID={config['root_uuid']} / ext4 defaults 0 1\n" +
                (f"UUID={config['esp_uuid']} /boot/efi vfat umask=0077 0 1\n" if config.get('esp_uuid') else ''))
            atomic_json(root / 'etc/asena/recovery.json', config)
            with contextlib.ExitStack() as stack:
                for folder in ('dev', 'proc', 'sys'):
                    target = root / folder
                    target.mkdir(exist_ok=True)
                    run('mount', '--rbind', '/' + folder, str(target))
                    run('mount', '--make-rslave', str(target))
                    stack.callback(run, 'umount', '-R', str(target))
                run('chroot', str(root), 'update-initramfs', '-u', '-k', 'all')
                run('chroot', str(root), 'update-grub')
            run('grub-editenv', str(grubenv), 'unset', 'next_entry')
            run('sync', '-f', str(root))
    safe_remove(partition / 'asena/request.json')
    shutil.rmtree(state)


def main():
    if os.geteuid() != 0:
        raise PermissionError('Recovery requires the system service')
    command = sys.argv[1]
    if command not in ('status', 'schedule', 'cancel', 'reset'):
        raise ValueError('Unknown recovery operation')
    lock = Path('/run/asena-recovery.lock')
    with lock.open('w') as stream:
        fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        request = json.loads(sys.stdin.read(16384) or '{}')
        try:
            with recovery(write=command != 'status') as (config, partition):
                pending = partition / 'asena/request.json'
                state = partition / 'asena/state/transaction.json'
                if command == 'status':
                    print(json.dumps({'available': True, 'offline': offline(),
                                      'pending': read_json(pending).get('mode', '') if pending.exists() else '',
                                      'resuming': read_json(state).get('mode', '') if state.exists() else ''}))
                elif command == 'schedule':
                    if offline() or request.get('mode') not in MODES:
                        raise ValueError('Invalid scheduled reset')
                    verify_image(partition)
                    atomic_json(pending, {'mode': request['mode']})
                    try:
                        run('grub-reboot', ENTRY)
                    except Exception:
                        safe_remove(pending)
                        raise
                elif command == 'cancel':
                    if state.exists():
                        raise RuntimeError('An interrupted reset must be resumed')
                    environment = run('grub-editenv', '/boot/grub/grubenv', 'list')
                    if f'next_entry={ENTRY}' in environment.splitlines():
                        run('grub-editenv', '/boot/grub/grubenv', 'unset', 'next_entry')
                    safe_remove(pending)
                else:
                    reset(config, partition, request)
        except (OSError, RuntimeError, ValueError) as error:
            if command == 'status':
                print(json.dumps({'available': False, 'offline': offline(), 'pending': '', 'resuming': ''}))
            else:
                raise error


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
