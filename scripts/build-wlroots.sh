#!/bin/bash
# Rebuilds Debian 13's wlroots 0.18 with the patches in packaging/wlroots and
# leaves the .deb files in target/packages/wlroots. Installing them is a
# separate, explicit step:
#   sudo apt-get install ./target/packages/wlroots/libwlroots-0.18_*_amd64.deb
# Build tools: sudo apt-get build-dep wlroots && sudo apt-get install devscripts
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# The Debian version these patches were checked against.
debian_version=0.18.2-3
for program in apt-get dpkg-buildpackage dch; do
    command -v "$program" >/dev/null || { printf 'Missing %s; see the header of this script.\n' "$program" >&2; exit 1; }
done
work=$repo/target/packages/wlroots
rm -rf -- "$work"
mkdir -p "$work"
cd "$work"
apt-get source --only-source "wlroots=$debian_version"
source_dir=$(find . -mindepth 1 -maxdepth 1 -type d -name 'wlroots-*' -print -quit)
[[ -n $source_dir ]] || { printf '%s\n' 'The wlroots source was not unpacked.' >&2; exit 1; }
cd "$source_dir"
[[ $(cat debian/source/format) == '3.0 (quilt)' ]] || { printf '%s\n' 'Unexpected source format.' >&2; exit 1; }
mkdir -p debian/patches
for patch in "$repo"/packaging/wlroots/*.patch; do
    cp -- "$patch" debian/patches/
    printf '%s\n' "${patch##*/}" >>debian/patches/series
done
DEBFULLNAME='agahemir1' DEBEMAIL='projectxurme@gmail.com' \
    dch --local +asena --distribution trixie \
    'Apply upstream output-management fix: filter mode resources by client.'
dpkg-checkbuilddeps
dpkg-buildpackage -us -uc -b -j"$(nproc)"
dpkg-buildpackage -us -uc -S
ls -1 "$work"/*.deb
