#!/bin/bash
# Called by the installer after unpacking the system and installing GRUB.
# Both arguments MUST be mounted partitions, never directories on the live root.
# Usage: sudo scripts/install-recovery.sh TARGET_ROOT RECOVERY_MOUNT LIVE_MEDIUM
set -euo pipefail
[[ $EUID == 0 ]] || { printf '%s\n' 'Run as root from the installer.' >&2; exit 1; }
target=$(realpath -- "${1:?target root required}")
recovery=$(realpath -- "${2:?recovery mount required}")
medium=$(realpath -- "${3:?live medium required}")
[[ $target != / && $recovery != / && $target != "$recovery" ]] || exit 1
mountpoint -q "$target"
mountpoint -q "$recovery"
[[ $(findmnt -nro FSTYPE --mountpoint "$target") == ext4 ]]
[[ $(findmnt -nro FSTYPE --mountpoint "$recovery") == ext4 ]]
root_uuid=$(findmnt -nro UUID --mountpoint "$target")
recovery_uuid=$(findmnt -nro UUID --mountpoint "$recovery")
[[ -n $root_uuid && -n $recovery_uuid && $root_uuid != "$recovery_uuid" ]]
[[ $(findmnt -nro UUID --target /) != "$root_uuid" ]]
[[ -f $target/boot/grub/grubenv && -x $target/usr/bin/asena-recovery ]]
[[ ! -e $recovery/asena ]] || { printf '%s\n' 'Recovery is already installed; refusing to replace it.' >&2; exit 1; }
# live-build names kernels by version when the image has more than one. The
# recovery entry must start Debian's signed kernel, the one Secure Boot accepts.
kernel=$(find "$medium/live" -maxdepth 1 -type f -name 'vmlinuz-*-amd64' -printf '%f\n' | sort -V | tail -n 1)
if [[ -n $kernel ]]; then
    initrd=initrd.img-${kernel#vmlinuz-}
else
    kernel=vmlinuz initrd=initrd.img
fi
for file in filesystem.squashfs "$kernel" "$initrd"; do
    [[ -f $medium/live/$file && ! -L $medium/live/$file ]] || { printf '%s\n' "Missing live/$file on the installation medium." >&2; exit 1; }
done
required=$(du -cb "$medium/live/filesystem.squashfs" "$medium/live/$kernel" "$medium/live/$initrd" | tail -1 | cut -f1)
available=$(df -B1 --output=avail "$recovery" | tail -1)
(( available > required + 512 * 1024 * 1024 )) || { printf '%s\n' 'The recovery partition needs space for the image plus 512 MiB.' >&2; exit 1; }
install -d -m 0700 "$recovery/asena"
install -d -m 0755 "$recovery/live"
install -m 0644 "$medium/live/filesystem.squashfs" "$recovery/live/filesystem.squashfs"
install -m 0644 "$medium/live/$kernel" "$recovery/live/vmlinuz"
install -m 0644 "$medium/live/$initrd" "$recovery/live/initrd.img"
esp_uuid=
if mountpoint -q "$target/boot/efi"; then
    esp_uuid=$(findmnt -nro UUID --mountpoint "$target/boot/efi")
fi
python3 -I - "$target" "$recovery" "$root_uuid" "$recovery_uuid" "$esp_uuid" <<'PY'
import hashlib, json, pathlib, sys
target, recovery = map(pathlib.Path, sys.argv[1:3])
config = dict(version=1, root_uuid=sys.argv[3], recovery_uuid=sys.argv[4], esp_uuid=sys.argv[5])
manifest = {}
for name in ('live/filesystem.squashfs', 'live/vmlinuz', 'live/initrd.img'):
    with (recovery/name).open('rb') as stream:
        manifest[name] = hashlib.file_digest(stream, 'sha256').hexdigest()
(target/'etc/asena').mkdir(parents=True, exist_ok=True)
for path in (target/'etc/asena/recovery.json', recovery/'asena/recovery.json'):
    path.write_text(json.dumps(config))
    path.chmod(0o600)
(recovery/'asena/image.json').write_text(json.dumps(manifest))
PY
chroot "$target" update-grub
sync -f "$recovery"
printf '%s\n' 'Recovery image and boot entry installed.'
