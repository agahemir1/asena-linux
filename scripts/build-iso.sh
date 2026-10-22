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
