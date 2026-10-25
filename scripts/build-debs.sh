#!/bin/bash
# Build complete native source and Debian binaries from a prepared source tree.
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Debian 13 Linux is required.' >&2; exit 1; }
source_dir=${1:?usage: build-debs.sh PREPARED_SOURCE_DIRECTORY}
cd "$source_dir"
[[ -f debian/control && -d vendor/cargo && -d third-party/gtk4-layer-shell ]] || {
    printf '%s\n' 'Prepare complete source with scripts/prepare-debian-source.py first.' >&2; exit 1;
}
export PATH=/usr/bin:/bin:$PATH
export DEB_BUILD_OPTIONS=${DEB_BUILD_OPTIONS:-parallel=2}
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
# The source archive is produced before any build outputs exist.
source_options=()
while IFS= read -r package; do
    source_options+=("-Idebian/$package")
done < <(sed -n 's/^Package: //p' debian/control)
dpkg-source "${source_options[@]}" --build .
dpkg-buildpackage --no-sign -b -nc
