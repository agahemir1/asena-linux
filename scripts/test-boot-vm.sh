#!/bin/bash
# Development VM only. Builds a small UEFI test disk the way the image is
# built: Debian's signed shim, GRUB and kernel, XanMod LTS, the files of
# scripts/stage-system-files.sh and the chroot hooks of iso/config/hooks. It
# then boots the disk with OVMF twice, Secure Boot off and Secure Boot on with
# Microsoft keys, captures the screen during boot and reports the running
# kernel, Secure Boot, preemption, identity, swap and boot splash on the serial
# console. Covers K20, K23, K24 and the boot branding of K07.
# Uses sudo, a loop device and about 4 GB in target/boot-test.
# Results: target/evidence/boot-test.
# Tools: sudo apt-get install debootstrap qemu-system-x86 qemu-utils ovmf gdisk dosfstools \
#   librsvg2-bin python3-gi gir1.2-gdkpixbuf-2.0
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
[[ ${ASENA_BOOT_TEST_VM:-} == 1 ]] || {
    printf '%s\n' 'Set ASENA_BOOT_TEST_VM=1: this script uses sudo, loop devices and debootstrap.' >&2
    exit 1
}
# debootstrap, sgdisk and mkfs live in sbin, which a user's PATH may lack.
PATH=$PATH:/usr/sbin:/sbin
for program in debootstrap qemu-system-x86_64 sgdisk mkfs.vfat mkfs.ext4 losetup rsvg-convert python3 sudo; do
    command -v "$program" >/dev/null || { printf 'Missing %s; see the header of this script.\n' "$program" >&2; exit 1; }
done
ovmf=/usr/share/OVMF
for file in OVMF_CODE_4M.secboot.fd OVMF_VARS_4M.fd OVMF_VARS_4M.ms.fd; do
    [[ -f $ovmf/$file ]] || { printf 'Missing %s/%s; install ovmf.\n' "$ovmf" "$file" >&2; exit 1; }
done

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
work=$repo/target/boot-test
evidence=$repo/target/evidence/boot-test
root=$work/root
disk=$work/disk.img
xanmod_package=${ASENA_XANMOD_PACKAGE:-linux-xanmod-lts-x64v2}
mirror=${ASENA_DEBIAN_MIRROR:-http://deb.debian.org/debian}
loop=

# shellcheck disable=SC2317 # Called through the EXIT trap.
cleanup() {
    trap - EXIT
    if mountpoint -q "$root"; then
        sudo umount -R "$root" || true
    fi
    if [[ -n $loop ]]; then
        sudo losetup -d "$loop" || true
    fi
}
trap cleanup EXIT

if [[ -d $work ]]; then
    sudo rm -rf -- "$work"
fi
rm -rf -- "$evidence"
mkdir -p "$work" "$evidence" "$root"

truncate -s 6G "$disk"
sgdisk -o -n 1:0:+256M -t 1:ef00 -n 2:0:0 -t 2:8304 "$disk" >/dev/null
loop=$(sudo losetup -P --show -f "$disk")
sudo mkfs.vfat -F 32 -n EFI "${loop}p1" >/dev/null
sudo mkfs.ext4 -q -L asena-test "${loop}p2"
sudo mount "${loop}p2" "$root"
sudo mkdir -p "$root/boot/efi"
sudo mount "${loop}p1" "$root/boot/efi"

# shellcheck disable=SC2024 # The log belongs to the calling user.
sudo debootstrap --variant=minbase \
    --include=systemd-sysv,udev,kmod,e2fsprogs,initramfs-tools,ca-certificates,mokutil,procps \
    trixie "$root" "$mirror" >"$evidence/debootstrap.log" 2>&1

# The image's system files, exactly as scripts/build-iso.sh stages them.
sudo bash "$repo/scripts/stage-system-files.sh" "$root"

# Test-only settings: Debian archive, serial console, short menu timeout and
# the report service.
sudo tee "$root/etc/apt/sources.list.d/debian.sources" >/dev/null <<EOF
Types: deb
URIs: $mirror
Suites: trixie trixie-updates
Components: main
Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg

Types: deb
URIs: http://security.debian.org/debian-security
Suites: trixie-security
Components: main
Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg
EOF
sudo rm -f "$root/etc/apt/sources.list"
sudo tee "$root/etc/default/grub.d/zz-boot-test.cfg" >/dev/null <<'EOF'
GRUB_TIMEOUT=1
GRUB_TERMINAL="console serial"
GRUB_SERIAL_COMMAND="serial --unit=0 --speed=115200"
GRUB_CMDLINE_LINUX_DEFAULT="quiet splash console=tty0 console=ttyS0,115200 plymouth.ignore-serial-consoles"
EOF
sudo tee "$root/usr/local/sbin/boot-test-report" >/dev/null <<'EOF'
#!/bin/sh
sb=$(mokutil --sb-state 2>&1 | head -n 1)
preempt=$(dmesg | grep -o 'Dynamic Preempt: [a-z]*' | head -n 1)
os=$(. /etc/os-release && echo "$PRETTY_NAME ($ID, like $ID_LIKE)")
swap=$(swapon --show=NAME,SIZE --noheadings | tr -s ' \n' ' ')
splash=$(plymouth-set-default-theme 2>/dev/null)
printf 'ASENA-BOOT-TEST kernel=%s sb="%s" lockdown="%s" preempt="%s" os="%s" swap="%s" swappiness=%s splash=%s cmdline="%s"\n' \
    "$(uname -r)" "$sb" "$(cat /sys/kernel/security/lockdown 2>/dev/null)" "$preempt" "$os" \
    "$swap" "$(sysctl -n vm.swappiness)" "$splash" "$(cat /proc/cmdline)" >/dev/ttyS0
systemctl poweroff
EOF
sudo chmod 0755 "$root/usr/local/sbin/boot-test-report"
sudo tee "$root/etc/systemd/system/boot-test-report.service" >/dev/null <<'EOF'
[Unit]
Description=Report the booted system and power off
After=multi-user.target

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/boot-test-report

[Install]
WantedBy=multi-user.target
EOF
root_uuid=$(sudo blkid -s UUID -o value "${loop}p2")
efi_uuid=$(sudo blkid -s UUID -o value "${loop}p1")
printf 'UUID=%s / ext4 errors=remount-ro 0 1\nUUID=%s /boot/efi vfat umask=0077 0 1\n' \
    "$root_uuid" "$efi_uuid" | sudo tee "$root/etc/fstab" >/dev/null
printf '%s\n' asena | sudo tee "$root/etc/hostname" >/dev/null

for fs in proc sys dev dev/pts run; do
    sudo mount --bind "/$fs" "$root/$fs"
done
in_root() { sudo chroot "$root" env DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8 "$@"; }
in_root apt-get update >"$evidence/apt-update.log" 2>&1
in_root apt-cache policy "$xanmod_package" clang-21 dkms libbpf1 >"$evidence/apt-policy.txt"
in_root apt-get install -y --no-install-recommends linux-image-amd64 grub-efi-amd64 \
    grub-efi-amd64-signed shim-signed "$xanmod_package" plymouth plymouth-label fontconfig \
    fonts-dejavu-core systemd-zram-generator fastfetch lsb-release locales \
    >"$evidence/apt-install.log" 2>&1
# The image's chroot hooks, in live-build's order.
for hook in "$repo"/iso/config/hooks/live/*.hook.chroot; do
    sudo install -m 0755 "$hook" "$root/tmp/hook"
    in_root /tmp/hook >"$evidence/hook-$(basename "$hook" .hook.chroot).log" 2>&1
    sudo rm -f "$root/tmp/hook"
done
in_root systemctl enable boot-test-report.service >/dev/null 2>&1
# No --bootloader-id: grub-install must pick EFI/debian itself, which
# Debian's signed GRUB needs (data/grub/default/asena.cfg).
in_root grub-install --target=x86_64-efi --efi-directory=/boot/efi --uefi-secure-boot \
    --force-extra-removable --no-nvram >"$evidence/grub-install.log" 2>&1
in_root update-grub >"$evidence/update-grub.log" 2>&1
{
    sudo find "$root/boot/efi" -type f -printf '%P\n' | sort
    printf '\n'
    in_root lsb_release -a 2>/dev/null
    printf '\n'
    in_root dpkg-divert --list /usr/lib/os-release
    printf '\n'
    sudo cat "$root/etc/issue"
} >"$evidence/identity.txt"
in_root fastfetch --pipe false >"$evidence/fastfetch.txt" 2>&1 || true
sudo cp "$root/boot/grub/grub.cfg" "$evidence/grub.cfg"
sudo chown "$(id -u):$(id -g)" "$evidence/grub.cfg"
sudo umount -R "$root"
sudo losetup -d "$loop"
loop=

status=0
if ! grep -qx 'EFI/debian/grub.cfg' "$evidence/identity.txt"; then
    printf '%s\n' 'grub-install did not write EFI/debian/grub.cfg' | tee -a "$evidence/summary.txt"
    status=1
fi
for mode in off on; do
    vars=$ovmf/OVMF_VARS_4M.fd
    [[ $mode == on ]] && vars=$ovmf/OVMF_VARS_4M.ms.fd
    cp "$vars" "$work/vars-$mode.fd"
    log=$evidence/serial-secure-boot-$mode.log
    monitor=$work/monitor-$mode.sock
    timeout 1500 qemu-system-x86_64 -machine q35,smm=on -accel kvm -accel tcg -cpu max -m 2048 -smp 2 \
        -global driver=cfi.pflash01,property=secure,value=on \
        -drive "if=pflash,format=raw,unit=0,readonly=on,file=$ovmf/OVMF_CODE_4M.secboot.fd" \
        -drive "if=pflash,format=raw,unit=1,file=$work/vars-$mode.fd" \
        -drive "file=$disk,format=raw,if=virtio" -vga std \
        -display none -serial "file:$log" -monitor "unix:$monitor,server,nowait" \
        -net none -no-reboot &
    qemu=$!
    # A screen every 10 seconds while the machine runs.
    python3 -I - "$monitor" "$work/screen-$mode" "$qemu" <<'PY' || true
import os, socket, sys, time
path, prefix, pid = sys.argv[1], sys.argv[2], int(sys.argv[3])
for _ in range(100):
    if os.path.exists(path):
        break
    time.sleep(0.2)
index = 0
while True:
    try:
        os.kill(pid, 0)
    except OSError:
        break
    try:
        with socket.socket(socket.AF_UNIX) as monitor:
            monitor.connect(path)
            monitor.sendall(f"screendump {prefix}-{index:03d}.ppm\n".encode())
            time.sleep(0.5)
    except OSError:
        break
    index += 1
    time.sleep(10)
PY
    wait "$qemu" || true
    python3 -I - "$evidence" "$work"/screen-"$mode"-*.ppm <<'PY' || true
import sys
from pathlib import Path
import gi
gi.require_version("GdkPixbuf", "2.0")
from gi.repository import GdkPixbuf
target = Path(sys.argv[1])
for name in sys.argv[2:]:
    path = Path(name)
    if path.exists() and path.stat().st_size:
        GdkPixbuf.Pixbuf.new_from_file(str(path)).savev(str(target / (path.stem + ".png")), "png", [], [])
PY
    report=$(grep -a -m 1 'ASENA-BOOT-TEST' "$log" | tr -d '\r' || true)
    printf 'secure-boot=%s %s\n' "$mode" "${report:-no report}" | tee -a "$evidence/summary.txt"
    case $mode:$report in
        off:*kernel=*xanmod*'os="Asena Linux'*'swap="/dev/zram0'*splash=asena*) ;;
        on:*kernel=*-amd64*'SecureBoot enabled'*'os="Asena Linux'*'swap="/dev/zram0'*splash=asena*) ;;
        *) status=1 ;;
    esac
done
exit "$status"
