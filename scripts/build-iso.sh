#!/bin/bash
# Builds the Asena live ISO from iso/ with live-build (Phase 6C).
# Development VM only: lb build runs as root through sudo, downloads Debian
# packages and needs about 10 GB in target/iso. Output and build-info.txt
# stay in target/iso. Tools: sudo apt-get install live-build
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for program in lb sha256sum sudo; do
    command -v "$program" >/dev/null || { printf 'Missing %s; see the header of this script.\n' "$program" >&2; exit 1; }
done
work=$repo/target/iso
# live-build keeps its chroot and caches in the working directory; every build
# starts from a clean copy of iso/.
if [[ -d $work ]]; then
    (cd "$work" && sudo lb clean --purge >/dev/null 2>&1) || true
    sudo rm -rf -- "$work"
fi
mkdir -p "$work"
cp -r "$repo/iso/auto" "$repo/iso/config" "$work/"
chmod +x "$work"/auto/*
# Locally built runtime packages: scripts/build-wlroots.sh now, the Asena
# packages after Phase 6A. Development, debug and example packages stay out.
mkdir -p "$work/config/packages.chroot"
if [[ -d $repo/target/packages ]]; then
    find "$repo/target/packages" -name '*.deb' ! -name '*-dbgsym_*' ! -name '*-dev_*' \
        ! -name '*-examples_*' -exec cp -t "$work/config/packages.chroot/" {} +
fi
# XanMod archive for the chroot stage (K20). live-build trusts .key.chroot
# only inside the build chroot and removes it with the source and pin.
cp "$repo/data/apt/keyrings/xanmod-archive-keyring.asc" "$work/config/archives/xanmod.key.chroot"
cp "$repo/data/apt/preferences.d/asena-xanmod.pref" "$work/config/archives/xanmod.pref.chroot"
# System files of the image until Asena packages own them (Phase 6A).
include=$work/config/includes.chroot_after_packages
install -D -m 0644 "$repo/data/apt/sources.list.d/xanmod.sources" "$include/etc/apt/sources.list.d/xanmod.sources"
install -D -m 0644 "$repo/data/apt/keyrings/xanmod-archive-keyring.asc" \
    "$include/usr/share/keyrings/xanmod-archive-keyring.asc"
install -D -m 0644 "$repo/data/apt/preferences.d/asena-xanmod.pref" "$include/etc/apt/preferences.d/asena-xanmod.pref"
install -D -m 0644 "$repo/data/grub/default/asena.cfg" "$include/etc/default/grub.d/asena.cfg"
install -D -m 0755 "$repo/data/grub/grub.d/42_asena_kernel" "$include/etc/grub.d/42_asena_kernel"
install -D -m 0644 "$repo/data/zram/zram-generator.conf" "$include/etc/systemd/zram-generator.conf"
install -D -m 0644 "$repo/data/sysctl/90-asena-zram.conf" "$include/etc/sysctl.d/90-asena-zram.conf"
install -D -m 0755 "$repo/data/live/0125-asena-greetd" "$include/usr/lib/live/config/0125-asena-greetd"
cd "$work"
lb config
sudo lb build
image=$(find . -maxdepth 1 -name '*.iso' -print -quit)
[[ -n $image ]] || { printf '%s\n' 'live-build produced no image; see target/iso/build.log.' >&2; exit 1; }
# The VM copy has no .git; pass ASENA_SOURCE_COMMIT from the host checkout.
commit=${ASENA_SOURCE_COMMIT:-$(git -C "$repo" rev-parse HEAD 2>/dev/null || printf unknown)}
{
    printf 'image=%s\n' "${image#./}"
    printf 'sha256=%s\n' "$(sha256sum "$image" | cut -d ' ' -f 1)"
    printf 'source=%s\n' "$commit"
    printf 'live-build=%s\n' "$(dpkg-query -W -f='${Version}' live-build)"
    printf 'built=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'local-packages=%s\n' "$(find config/packages.chroot -name '*.deb' -printf '%f ' 2>/dev/null)"
} >build-info.txt
cat build-info.txt
