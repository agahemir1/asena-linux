"""Exercise reset behavior on temporary trees; never mount or alter real disks."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('recovery', Path(__file__).resolve().parents[1] / 'data/recovery/recovery.py')
recovery = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(recovery)


class RecoveryTests(unittest.TestCase):
    def test_manifest_rejects_same_device_and_bad_uuid(self):
        config = dict(version=1, root_uuid='12345678-abcd', recovery_uuid='87654321-abcd')
        self.assertEqual(recovery.validate_config(config), config)
        with self.assertRaises(ValueError):
            recovery.validate_config({**config, 'recovery_uuid': config['root_uuid']})
        with self.assertRaises(ValueError):
            recovery.validate_config({**config, 'root_uuid': '../../dev/sda'})

    def test_keep_removes_installed_apps_but_preserves_documents_and_app_data(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ('etc', 'var/lib/flatpak/app', 'usr/bin', 'home/user/Documents',
                         'home/user/.local/share/flatpak/app', 'home/user/.var/app/org.example.App', 'boot/grub'):
                (root/name).mkdir(parents=True, exist_ok=True)
            for name in ('home/user/Documents/note.txt', 'home/user/.var/app/org.example.App/preferences',
                         'boot/grub/grub.cfg', 'etc/old-config', 'usr/bin/third-party'):
                (root/name).write_text('keep or remove')
            recovery.clear_system(root, True)
            recovery.remove_user_flatpaks(root, {'passwd': [['user', 'x', '1000', '1000', '', '/home/user', '/bin/bash']]})
            self.assertTrue((root/'home/user/Documents/note.txt').exists())
            self.assertTrue((root/'home/user/.var/app/org.example.App/preferences').exists())
            self.assertTrue((root/'boot/grub/grub.cfg').exists())
            self.assertFalse((root/'home/user/.local/share/flatpak').exists())
            self.assertFalse((root/'var').exists())
            self.assertFalse((root/'etc').exists())
            self.assertFalse((root/'usr').exists())

    def test_remove_everything_removes_homes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root/'home/user').mkdir(parents=True)
            (root/'home/user/file').write_text('personal')
            (root/'boot').mkdir()
            recovery.clear_system(root, False)
            self.assertFalse((root/'home').exists())
            self.assertTrue((root/'boot').exists())

    def test_symlink_cleanup_does_not_follow_target(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root/'outside').mkdir()
            (root/'outside/safe').write_text('personal')
            (root/'link').symlink_to(root/'outside', target_is_directory=True)
            recovery.safe_remove(root/'link')
            self.assertEqual((root/'outside/safe').read_text(), 'personal')

    def test_account_restore_keeps_sudo_members_and_rejects_uid_collisions(self):
        base = [['root', 'x', '0', ''], ['sudo', 'x', '27', 'service']]
        kept = [['sudo', 'x', '27', 'user'], ['user', 'x', '1000', '']]
        merged = recovery.merge_accounts(base, kept, 'group')
        self.assertIn('sudo:x:27:service,user', merged)
        with self.assertRaises(RuntimeError):
            recovery.merge_accounts([['someone', 'x', '1000', '']], [['user', 'x', '1000', '']], 'group')

    def test_image_corruption_is_rejected_before_reset(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root/'live').mkdir()
            (root/'asena').mkdir()
            manifest = {}
            for name in ('live/filesystem.squashfs', 'live/vmlinuz', 'live/initrd.img'):
                (root/name).write_bytes(b'image')
                manifest[name] = hashlib.sha256(b'image').hexdigest()
            (root/'asena/image.json').write_text(json.dumps(manifest))
            recovery.verify_image(root)
            (root/'live/filesystem.squashfs').write_bytes(b'damaged')
            with self.assertRaises(RuntimeError):
                recovery.verify_image(root)

    def test_new_account_validation(self):
        recovery.validate_new_account('emir', 'a secure password')
        for login, password in [('root', 'password'), ('../user', 'password'), ('emir', 'short'), ('emir', 'password\n')]:
            with self.assertRaises(ValueError):
                recovery.validate_new_account(login, password)

    def test_separate_home_is_rejected_and_obsolete_kernels_are_removed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)/'root'
            image = Path(temporary)/'image'
            (root/'etc').mkdir(parents=True)
            (root/'etc/fstab').write_text('UUID=root / ext4 defaults 0 1\nUUID=home /home ext4 defaults 0 2\n')
            with self.assertRaises(RuntimeError):
                recovery.validate_layout(root)
            (root/'etc/fstab').write_text('UUID=root / ext4 defaults 0 1\n')
            recovery.validate_layout(root)
            (root/'boot/grub').mkdir(parents=True)
            (image/'boot').mkdir(parents=True)
            for path in (root/'boot/vmlinuz-old', root/'boot/vmlinuz-factory', image/'boot/vmlinuz-factory'):
                path.write_bytes(b'kernel')
            recovery.remove_obsolete_kernels(root, image)
            self.assertFalse((root/'boot/vmlinuz-old').exists())
            self.assertTrue((root/'boot/vmlinuz-factory').exists())
            self.assertTrue((root/'boot/grub').is_dir())

    def test_image_without_boot_tools_is_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            image = Path(temporary)
            with self.assertRaises(RuntimeError):
                recovery.check_image_tools(image)
            for tool in recovery.REQUIRED_TOOLS:
                (image/tool).parent.mkdir(parents=True, exist_ok=True)
                (image/tool).write_text('#!/bin/sh\n')
            recovery.check_image_tools(image)

    def test_removed_packages_exclude_the_image(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)/'root'
            image = Path(temporary)/'image'
            for tree, entries in ((root, 'base extra gone'), (image, 'base')):
                (tree/'var/lib/dpkg').mkdir(parents=True)
                (tree/'var/lib/dpkg/status').write_text(''.join(
                    f'Package: {name}\nStatus: '
                    + ('deinstall ok config-files' if name == 'gone' else 'install ok installed')
                    + '\n\n' for name in entries.split()))
            self.assertEqual(recovery.installed_packages(root) - recovery.installed_packages(image), {'extra'})


if __name__ == '__main__':
    unittest.main()
