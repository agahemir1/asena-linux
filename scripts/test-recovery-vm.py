#!/usr/bin/python3
"""Destructive reset tests on disposable loop devices, inside the Debian VM.

Real ext4 mounts, squashfs, rsync, useradd and chpasswd are used. Bootloader
generation and initramfs generation are excluded; ISO boot acceptance is separate.
"""
import contextlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

REPO = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('recovery', REPO / 'data/recovery/recovery.py')
engine = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(engine)


def run(*args):
    return subprocess.check_output(args, text=True).strip()


def copy_binary(path, root):
    source = Path(path)
    target = root / source.relative_to('/')
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    for library in re.findall(r'(/[^\s()]+)', run('ldd', str(source))):
        source = Path(library)
        if source.is_file():
            target = root / source.relative_to('/')
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)


@contextlib.contextmanager
def loop(path):
    with path.open('wb') as stream:
        stream.truncate(256 * 1024 * 1024)
    device = run('losetup', '--find', '--show', str(path))
    try:
        run('mkfs.ext4', '-q', '-F', device)
        yield device
    finally:
        run('losetup', '-d', device)


def write(root, name, text):
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def fixture(image):
    write(image, 'etc/passwd', 'root:x:0:0:root:/root:/bin/bash\n')
    write(image, 'etc/shadow', 'root:!:20000:0:99999:7:::\n')
    write(image, 'etc/group', 'root:x:0:\nsudo:x:27:\n')
    write(image, 'etc/gshadow', 'root:*::\nsudo:*::\n')
    write(image, 'etc/hostname', 'asena\n')
    write(image, 'etc/login.defs', 'UID_MIN 1000\nUID_MAX 60000\nGID_MIN 1000\nGID_MAX 60000\n')
    write(image, 'usr/bin/asena-recovery', 'fixture\n')
    write(image, 'usr/share/asena-factory', 'restored\n')
    # Run in a chroot by the reset; the fixture replaces them with no-ops.
    write(image, 'usr/sbin/update-initramfs', '#!/bin/sh\n')
    write(image, 'usr/sbin/update-grub', '#!/bin/sh\n')
    (image/'etc/skel').mkdir()
    (image/'boot/grub').mkdir(parents=True)
    (image/'home').mkdir()
    for binary in ('/usr/sbin/useradd', '/usr/sbin/chpasswd', '/bin/bash'):
        copy_binary(binary, image)


def main():
    if os.geteuid() != 0:
        raise SystemExit('Run with sudo inside the Debian development VM.')
    with tempfile.TemporaryDirectory(prefix='asena-reset-test-') as temporary:
        work = Path(temporary)
        source = work/'source'
        source.mkdir()
        fixture(source)
        with contextlib.ExitStack() as stack:
            system = stack.enter_context(loop(work/'system.ext4'))
            recovery = stack.enter_context(loop(work/'recovery.ext4'))
            root_uuid = run('blkid', '-s', 'UUID', '-o', 'value', system)
            recovery_uuid = run('blkid', '-s', 'UUID', '-o', 'value', recovery)
            partition = stack.enter_context(engine.mounted(recovery, work/'recovery', 'rw'))
            (partition/'live').mkdir()
            (partition/'asena').mkdir()
            run('mksquashfs', str(source), str(partition/'live/filesystem.squashfs'), '-noappend', '-processors', '2', '-quiet')
            (partition/'live/vmlinuz').write_bytes(b'fixture signed kernel')
            (partition/'live/initrd.img').write_bytes(b'fixture initrd')
            import hashlib
            manifest = {name: hashlib.sha256((partition/name).read_bytes()).hexdigest()
                        for name in ('live/filesystem.squashfs', 'live/vmlinuz', 'live/initrd.img')}
            engine.atomic_json(partition/'asena/image.json', manifest)
            config = dict(version=1, root_uuid=root_uuid, recovery_uuid=recovery_uuid, esp_uuid='')
            engine.device = lambda uuid: {root_uuid: system, recovery_uuid: recovery}[uuid]
            engine.offline = lambda: True
            engine.TARGET, engine.IMAGE = work/'target', work/'image'
            actual_run = engine.run

            def fixture_run(*args, **kwargs):
                if args[0] == 'chroot' and args[2] in ('update-initramfs', 'update-grub'):
                    return ''
                return actual_run(*args, **kwargs)

            engine.run = fixture_run
            with engine.mounted(system, work/'setup', 'rw') as root:
                shutil.copytree(source, root, dirs_exist_ok=True)
                write(root, 'etc/passwd', (source/'etc/passwd').read_text()+'emir:x:1000:1000:Emir:/home/emir:/bin/bash\n')
                write(root, 'etc/shadow', (source/'etc/shadow').read_text()+'emir:$6$test$preserved:20000:0:99999:7:::\n')
                write(root, 'etc/group', 'root:x:0:\nsudo:x:27:emir\nemir:x:1000:\n')
                write(root, 'etc/gshadow', 'root:*::\nsudo:*::emir\nemir:!::\n')
                write(root, 'etc/hostname', 'my-laptop\n')
                write(root, 'etc/custom-change', 'remove')
                write(root, 'home/emir/Documents/keep.txt', 'personal file')
                write(root, 'home/emir/.local/share/flatpak/app/org.example.App/installed', 'remove')
                write(root, 'home/emir/.var/app/org.example.App/settings', 'keep application data')
                write(root, 'var/lib/flatpak/app/installed', 'remove')
                run('grub-editenv', str(root/'boot/grub/grubenv'), 'create')
            engine.reset(config, partition, dict(mode='keep'))
            with engine.mounted(system, work/'check', 'rw') as root:
                assert (root/'home/emir/Documents/keep.txt').read_text() == 'personal file'
                assert (root/'home/emir/.var/app/org.example.App/settings').is_file()
                assert not (root/'home/emir/.local/share/flatpak').exists()
                assert not (root/'var/lib/flatpak/app/installed').exists()
                assert not (root/'etc/custom-change').exists()
                assert 'emir:$6$test$preserved' in (root/'etc/shadow').read_text()
                assert 'sudo:x:27:emir' in (root/'etc/group').read_text()
                assert (root/'etc/hostname').read_text() == 'my-laptop\n'
                assert (root/'usr/share/asena-factory').is_file()
                assert (root/'home/emir/Asena-removed-applications.json').is_file()
            print('PASS: keep files, account/password/sudo/hostname; remove system and user Flatpaks')
            engine.reset(config, partition, dict(mode='remove', username='newuser', password='fixture-password'))
            with engine.mounted(system, work/'check', 'rw') as root:
                assert not (root/'home/emir').exists()
                assert (root/'home/newuser').is_dir()
                assert 'newuser:' in (root/'etc/passwd').read_text()
                assert 'emir:' not in (root/'etc/passwd').read_text()
                assert 'newuser:$6$' in (root/'etc/shadow').read_text()
                assert 'sudo:x:27:newuser' in (root/'etc/group').read_text()
            print('PASS: remove all, create a real account with hashed password and sudo membership')


if __name__ == '__main__':
    main()
