#!/bin/bash
# Development VM only (K20). Builds a small UEFI test disk with Debian's
# signed shim, GRUB and kernel, XanMod LTS and the Asena kernel files from
# data/apt and data/grub, then boots it with OVMF twice: Secure Boot off and
# Secure Boot on with Microsoft keys. Each boot reports the running kernel,
# the Secure Boot state and the preemption model on the serial console and
# powers off. Uses sudo, a loop device and about 4 GB in target/kernel-test.
# Results: target/evidence/kernel-boot.
# Tools: sudo apt-get install debootstrap qemu-system-x86 qemu-utils ovmf gdisk dosfstools
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
[[ ${ASENA_KERNEL_TEST_VM:-} == 1 ]] || {
    printf '%s\n' 'Set ASENA_KERNEL_TEST_VM=1: this script uses sudo, loop devices and debootstrap.' >&2
    exit 1
}
# debootstrap, sgdisk and mkfs live in sbin, which a user's PATH may lack.
PATH=$PATH:/usr/sbin:/sbin
for program in debootstrap qemu-system-x86_64 sgdisk mkfs.vfat mkfs.ext4 losetup sudo; do
    command -v "$program" >/dev/null || { printf 'Missing %s; see the header of this script.\n' "$program" >&2; exit 1; }
done
ovmf=/usr/share/OVMF
for file in OVMF_CODE_4M.secboot.fd OVMF_VARS_4M.fd OVMF_VARS_4M.ms.fd; do
    [[ -f $ovmf/$file ]] || { printf 'Missing %s/%s; install ovmf.\n' "$ovmf" "$file" >&2; exit 1; }
done

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
work=$repo/target/kernel-test
evidence=$repo/target/evidence/kernel-boot
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
    --include=systemd-sysv,udev,kmod,e2fsprogs,initramfs-tools,ca-certificates,mokutil \
    trixie "$root" "$mirror" >"$evidence/debootstrap.log" 2>&1

put() { sudo install -D -m "$1" "$2" "$root$3"; }
# The Asena kernel files exactly as the image installs them.
put 0644 "$repo/data/apt/sources.list.d/xanmod.sources" /etc/apt/sources.list.d/xanmod.sources
put 0644 "$repo/data/apt/keyrings/xanmod-archive-keyring.asc" /usr/share/keyrings/xanmod-archive-keyring.asc
put 0644 "$repo/data/apt/preferences.d/asena-xanmod.pref" /etc/apt/preferences.d/asena-xanmod.pref
put 0644 "$repo/data/grub/default/asena.cfg" /etc/default/grub.d/asena.cfg
put 0755 "$repo/data/grub/grub.d/42_asena_kernel" /etc/grub.d/42_asena_kernel

# Test-only settings: serial console, short menu timeout, report service.
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
sudo tee "$root/etc/default/grub.d/zz-kernel-test.cfg" >/dev/null <<'EOF'
GRUB_TIMEOUT=1
GRUB_TERMINAL="console serial"
GRUB_SERIAL_COMMAND="serial --unit=0 --speed=115200"
GRUB_CMDLINE_LINUX_DEFAULT="console=ttyS0,115200"
EOF
sudo tee "$root/usr/local/sbin/kernel-test-report" >/dev/null <<'EOF'
#!/bin/sh
sb=$(mokutil --sb-state 2>&1 | head -n 1)
preempt=$(dmesg | grep -o 'Dynamic Preempt: [a-z]*' | head -n 1)
printf 'ASENA-KERNEL-TEST kernel=%s sb="%s" preempt="%s" cmdline="%s"\n' \
    "$(uname -r)" "$sb" "$preempt" "$(cat /proc/cmdline)" >/dev/ttyS0
systemctl poweroff
EOF
sudo chmod 0755 "$root/usr/local/sbin/kernel-test-report"
sudo tee "$root/etc/systemd/system/kernel-test-report.service" >/dev/null <<'EOF'
[Unit]
Description=Report the booted kernel and power off
After=multi-user.target

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/kernel-test-report

[Install]
WantedBy=multi-user.target
EOF
root_uuid=$(sudo blkid -s UUID -o value "${loop}p2")
efi_uuid=$(sudo blkid -s UUID -o value "${loop}p1")
printf 'UUID=%s / ext4 errors=remount-ro 0 1\nUUID=%s /boot/efi vfat umask=0077 0 1\n' \
    "$root_uuid" "$efi_uuid" | sudo tee "$root/etc/fstab" >/dev/null
printf '%s\n' asena-kernel-test | sudo tee "$root/etc/hostname" >/dev/null

for fs in proc sys dev dev/pts run; do
    sudo mount --bind "/$fs" "$root/$fs"
done
in_root() { sudo chroot "$root" env DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8 "$@"; }
in_root apt-get update >"$evidence/apt-update.log" 2>&1
in_root apt-cache policy "$xanmod_package" clang-21 dkms libbpf1 >"$evidence/apt-policy.txt"
in_root apt-get install -y --no-install-recommends linux-image-amd64 grub-efi-amd64 \
    grub-efi-amd64-signed shim-signed "$xanmod_package" >"$evidence/apt-install.log" 2>&1
in_root systemctl enable kernel-test-report.service >/dev/null 2>&1
in_root grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=debian \
    --uefi-secure-boot --force-extra-removable --no-nvram >"$evidence/grub-install.log" 2>&1
in_root update-grub >"$evidence/update-grub.log" 2>&1
sudo cp "$root/boot/grub/grub.cfg" "$evidence/grub.cfg"
sudo chown "$(id -u):$(id -g)" "$evidence/grub.cfg"
ls "$root/boot" >"$evidence/boot-files.txt"
sudo umount -R "$root"
sudo losetup -d "$loop"
loop=

status=0
for mode in off on; do
    vars=$ovmf/OVMF_VARS_4M.fd
    [[ $mode == on ]] && vars=$ovmf/OVMF_VARS_4M.ms.fd
    cp "$vars" "$work/vars-$mode.fd"
    log=$evidence/serial-secure-boot-$mode.log
    timeout 1200 qemu-system-x86_64 -machine q35,smm=on -accel kvm -accel tcg -cpu max -m 2048 -smp 2 \
        -global driver=cfi.pflash01,property=secure,value=on \
        -drive "if=pflash,format=raw,unit=0,readonly=on,file=$ovmf/OVMF_CODE_4M.secboot.fd" \
        -drive "if=pflash,format=raw,unit=1,file=$work/vars-$mode.fd" \
        -drive "file=$disk,format=raw,if=virtio" \
        -display none -serial "file:$log" -net none -no-reboot || true
    report=$(grep -a -m 1 'ASENA-KERNEL-TEST' "$log" | tr -d '\r' || true)
    printf 'secure-boot=%s %s\n' "$mode" "${report:-no report}" | tee -a "$evidence/summary.txt"
    case $mode:$report in
        off:*kernel=*xanmod*) ;;
        on:*kernel=*-amd64*'SecureBoot enabled'*) ;;
        *) status=1 ;;
    esac
done
exit "$status"
