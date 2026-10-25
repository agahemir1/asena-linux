#!/bin/bash
# Run as root in the development VM. All mounts live in a private namespace.
set -euo pipefail
[[ $(uname -s) == Linux && $EUID == 0 ]] || { printf '%s\n' 'Run as root in the Debian development VM.' >&2; exit 1; }
source_dir=$(realpath -- "${1:?usage: build-debs-clean.sh PREPARED_SOURCE_DIRECTORY}")
mode=${2:-complete}
[[ $mode == complete || $mode == --binary-only ]] || { printf '%s\n' 'Unknown build mode.' >&2; exit 2; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
[[ $source_dir == "$repo/target/packages/asena/"* && -f $source_dir/debian/control ]] || {
    printf '%s\n' 'Expected a prepared Asena source directory under target/packages/asena.' >&2; exit 1;
}
if [[ ${ASENA_BUILD_NAMESPACE:-0} != 1 ]]; then
    exec unshare --mount --propagation private env ASENA_BUILD_NAMESPACE=1 bash "$0" "$source_dir" "$mode"
fi
root=$repo/target/debian-build-root
if [[ ! -f $root/var/lib/dpkg/status ]]; then
    debootstrap --variant=minbase --include=ca-certificates trixie "$root" https://deb.debian.org/debian
fi
mkdir -p "$root"/{proc,dev,sys,build}
mount -t proc proc "$root/proc"
mount --rbind /dev "$root/dev"
mount --make-rslave "$root/dev"
mount --rbind /sys "$root/sys"
mount --make-rslave "$root/sys"
mount --bind "$repo/target/packages/asena" "$root/build"
cp --dereference /etc/resolv.conf "$root/etc/resolv.conf"
printf '#!/bin/sh\nexit 101\n' > "$root/usr/sbin/policy-rc.d"
chmod 0755 "$root/usr/sbin/policy-rc.d"
chroot "$root" apt-get update -qq
chroot "$root" env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends build-essential
chroot "$root" env DEBIAN_FRONTEND=noninteractive apt-get build-dep -y --no-install-recommends "/build/${source_dir##*/}"
if ! chroot "$root" id builder >/dev/null 2>&1; then
    chroot "$root" useradd --uid "$(stat -c %u "$source_dir")" --create-home builder
fi
# This is an actual network namespace with no external interface. Neither
# Cargo nor build scripts can download undeclared sources during the build.
unshare --net chroot "$root" runuser -u builder -- \
    bash "/build/${source_dir##*/}/scripts/build-debs.sh" "/build/${source_dir##*/}" "$mode"
