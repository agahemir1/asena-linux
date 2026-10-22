#!/bin/bash
# Installs the Asena system files of the image under ROOT: the ISO build's
# includes.chroot_after_packages directory or the root of a test disk. Asena
# packages take these files over in Phase 6A. The hooks in iso/config/hooks
# then divert os-release and select the boot splash inside the chroot.
# Tools: sudo apt-get install librsvg2-bin
set -euo pipefail
root=${1:?usage: stage-system-files.sh ROOT}
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v rsvg-convert >/dev/null || { printf '%s\n' 'Missing rsvg-convert; see the header of this script.' >&2; exit 1; }
put() { install -D -m "$1" "$repo/$2" "$root$3"; }

# Kernel archive and boot menu (K20)
put 0644 data/apt/sources.list.d/xanmod.sources /etc/apt/sources.list.d/xanmod.sources
put 0644 data/apt/keyrings/xanmod-archive-keyring.asc /usr/share/keyrings/xanmod-archive-keyring.asc
put 0644 data/apt/preferences.d/asena-xanmod.pref /etc/apt/preferences.d/asena-xanmod.pref
put 0644 data/grub/default/asena.cfg /etc/default/grub.d/asena.cfg
put 0755 data/grub/grub.d/42_asena_kernel /etc/grub.d/42_asena_kernel

# Swap in RAM (K24) and the live session (K26)
put 0644 data/zram/zram-generator.conf /etc/systemd/zram-generator.conf
put 0644 data/sysctl/90-asena-zram.conf /etc/sysctl.d/90-asena-zram.conf
put 0755 data/live/0125-asena-greetd /usr/lib/live/config/0125-asena-greetd

# Identity and branding (K07, K23)
put 0644 data/branding/os-release /usr/share/asena/os-release
put 0644 data/branding/issue /etc/issue
put 0644 data/branding/issue.net /etc/issue.net
put 0644 data/fastfetch/config.jsonc /etc/xdg/fastfetch/config.jsonc
put 0644 data/branding/asena-logo.txt /usr/share/asena/fastfetch/logo.txt
for size in scalable 16x16 24x24 32x32; do
    put 0644 "data/icons/hicolor/$size/apps/asena-logo.svg" "/usr/share/icons/hicolor/$size/apps/asena-logo.svg"
done
put 0644 data/icons/Asena/symbolic/actions/asena-logo-symbolic.svg \
    /usr/share/icons/hicolor/symbolic/apps/asena-logo-symbolic.svg
theme=/usr/share/plymouth/themes/asena
put 0644 data/plymouth/asena/asena.plymouth "$theme/asena.plymouth"
put 0644 data/plymouth/asena/asena.script "$theme/asena.script"
rsvg-convert -w 160 -h 160 -o "$root$theme/logo.png" "$repo/data/branding/asena-emblem.svg"
rsvg-convert -w 8 -h 8 -o "$root$theme/dot.png" "$repo/data/plymouth/asena/dot.svg"
