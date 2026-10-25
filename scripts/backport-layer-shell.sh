#!/bin/bash
# Build the approved native backport without replacing Debian libraries.
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
version=1.3.0
checksum=1ebb01ab14e98afd1727f68f64981c37bd23305b1f131f5667c02b94cf593192
directory=$repo/target/backports
prefix=$repo/target/native/gtk4-layer-shell-$version
mkdir -p "$directory" "$repo/target/evidence"
archive=$directory/gtk4-layer-shell-$version.tar.gz
if [[ ! -f $archive ]]; then
    curl --fail --location --proto '=https' --tlsv1.2 \
        "https://codeload.github.com/wmww/gtk4-layer-shell/tar.gz/refs/tags/v$version" \
        --output "$archive.part"
    mv "$archive.part" "$archive"
fi
printf '%s  %s\n' "$checksum" "$archive" | sha256sum --check
tar -xzf "$archive" -C "$directory"
for patch in "$repo"/packaging/native/*.patch; do
    [[ -f $patch ]] || continue
    # The archive checksum stays upstream's; these patches only repair its tests.
    if ! patch -p1 --reverse --dry-run -d "$directory/gtk4-layer-shell-$version" < "$patch" >/dev/null 2>&1; then
        patch -p1 --forward -d "$directory/gtk4-layer-shell-$version" < "$patch"
    fi
done
meson setup --reconfigure "$directory/build-layer-shell-$version" \
    "$directory/gtk4-layer-shell-$version" --prefix="$prefix" --libdir=lib \
    -Dexamples=false -Ddocs=false -Dintrospection=false -Dvapi=false -Dtests=true -Dsmoke-tests=false
meson compile -C "$directory/build-layer-shell-$version"
meson test -C "$directory/build-layer-shell-$version" --print-errorlogs
meson install -C "$directory/build-layer-shell-$version"
printf '%s\n' 'Native backport is isolated. Source scripts/native-env.sh before building or launching.'
