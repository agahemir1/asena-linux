#!/bin/bash
# Isolated install/upgrade/remove and APT authentication checks in the dev VM.
set -euo pipefail
[[ $(uname -s) == Linux && $EUID == 0 ]] || { printf '%s\n' 'Run as root in the Debian development VM.' >&2; exit 1; }
operation=${1:?usage: test-debian-packages.sh install|upgrade|purge|negative REPOSITORY}
archive=$(realpath -- "${2:?Local test repository is required}")
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
[[ $archive == "$repo/target/"* && -f $archive/TEST-ARCHIVE.txt ]] || {
    printf '%s\n' 'Expected a local test repository below target.' >&2; exit 1;
}
if [[ ${ASENA_TEST_NAMESPACE:-0} != 1 ]]; then
    exec unshare --mount --propagation private env ASENA_TEST_NAMESPACE=1 bash "$0" "$operation" "$archive"
fi
root=$repo/target/debian-test-root
evidence=$repo/target/evidence/debian
mkdir -p "$evidence"
if [[ ! -f $root/var/lib/dpkg/status ]]; then
    debootstrap --variant=minbase --include=ca-certificates \
        --cache-dir="$repo/target/debian-build-root/var/cache/apt/archives" \
        trixie "$root" https://deb.debian.org/debian
fi
mkdir -p "$root"/{proc,dev,sys,opt/asena-repo,opt/asena-trust}
mount -t proc proc "$root/proc"
mount --rbind /dev "$root/dev"
mount --make-rslave "$root/dev"
mount --rbind /sys "$root/sys"
mount --make-rslave "$root/sys"
mount --bind "$archive" "$root/opt/asena-repo"
mount -o remount,bind,ro "$root/opt/asena-repo"
mount --bind "${archive}-trust" "$root/opt/asena-trust"
mount -o remount,bind,ro "$root/opt/asena-trust"
cp --dereference /etc/resolv.conf "$root/etc/resolv.conf"
printf '#!/bin/sh\nexit 101\n' > "$root/usr/sbin/policy-rc.d"
chmod 0755 "$root/usr/sbin/policy-rc.d"
inside() { chroot "$root" env LC_ALL=C DEBIAN_FRONTEND=noninteractive "$@"; }
preference() {
    inside runuser -u asena-package-test -- dbus-run-session -- \
        gsettings "$@" org.asenalinux.desktop accent-color
}
case $operation in
    install)
        inside dpkg -i /opt/asena-trust/asena-test-archive-keyring_0.1.0_all.deb \
            /opt/asena-trust/asena-test-sources_0.1.0_all.deb
        mkdir -p "$root/etc/greetd"
        printf 'existing-display-manager-config\n' > "$root/etc/greetd/config.toml"
        inside apt-get -o APT::Update::Error-Mode=any update
        inside apt-get install -y --no-install-recommends asena-desktop
        : > "$evidence/installed-libraries.log"
        while IFS= read -r binary; do
            inside ldd "${binary#"$root"}" >> "$evidence/installed-libraries.log"
        done < <(find "$root/usr/bin" "$root/usr/libexec" -maxdepth 1 -type f \
            \( -name 'asena-*' -o -name asenactl \))
        if grep -F 'not found' "$evidence/installed-libraries.log"; then
            printf '%s\n' 'An installed runtime library is missing.' >&2; exit 1
        fi
        if ! inside id asena-package-test >/dev/null 2>&1; then
            inside useradd --create-home asena-package-test
        fi
        inside runuser -u asena-package-test -- dbus-run-session -- \
            gsettings set org.asenalinux.desktop accent-color purple
        mkdir -p "$root/home/asena-package-test/Documents"
        printf 'preserve-personal-file\n' > "$root/home/asena-package-test/Documents/keep.txt"
        inside chown -R asena-package-test:asena-package-test /home/asena-package-test
        inside dpkg-query -W --showformat="\${Package} \${Version}\n" 'asena-*' asenactl > "$evidence/installed-v1.txt"
        # The first update precedes installing AppStream and its APT hooks.
        inside apt-get -o APT::Update::Error-Mode=any update
        inside appstreamcli refresh-cache --force
        inside appstreamcli get org.asenalinux.Calculator > "$evidence/catalog-calculator.txt"
        ;;
    upgrade)
        inside apt-get -o APT::Update::Error-Mode=any update
        inside apt-get install -y --no-install-recommends --only-upgrade asena-desktop
        inside apt-get upgrade -y --no-install-recommends
        [[ $(preference get) == "'purple'" ]]
        grep -qx preserve-personal-file "$root/home/asena-package-test/Documents/keep.txt"
        grep -qx existing-display-manager-config "$root/etc/greetd/config.toml"
        inside dpkg-query -W --showformat="\${Package} \${Version}\n" 'asena-*' asenactl > "$evidence/installed-v2.txt"
        ! cmp -s "$evidence/installed-v1.txt" "$evidence/installed-v2.txt"
        ;;
    purge)
        mapfile -t packages < <(inside dpkg-query -W --showformat="\${Package}\n" 'asena-*' asenactl)
        inside apt-get purge -y "${packages[@]}"
        grep -qx preserve-personal-file "$root/home/asena-package-test/Documents/keep.txt"
        grep -qx existing-display-manager-config "$root/etc/greetd/config.toml"
        [[ -f $root/home/asena-package-test/.config/dconf/user ]]
        inside dpkg -i /opt/asena-trust/asena-test-archive-keyring_0.1.0_all.deb \
            /opt/asena-trust/asena-test-sources_0.1.0_all.deb
        inside apt-get -o APT::Update::Error-Mode=any update
        inside apt-get install -y --no-install-recommends asena-calculator
        [[ $(preference get) == "'purple'" ]]
        inside test ! -e /usr/bin/asena-shell
        ;;
    negative)
        python3 - "$root" "$archive" <<'PY'
from pathlib import Path
import shutil, sys
root, archive = map(Path, sys.argv[1:])
for mode in ('wrong-key', 'unsigned', 'changed-release', 'changed-catalog'):
    fixture = root/'opt'/('asena-negative-'+mode)
    fixture.mkdir(exist_ok=True)
    shutil.copytree(archive/'dists', fixture/'dists', dirs_exist_ok=True)
    release = fixture/'dists/asena-test'
    key = '/usr/share/keyrings/asena-test-archive-keyring.asc'
    if mode == 'wrong-key': key = '/usr/share/keyrings/debian-archive-keyring.gpg'
    if mode == 'unsigned':
        (release/'InRelease').unlink()
        (release/'Release.gpg').unlink()
    if mode == 'changed-release':
        p=release/'InRelease'
        p.write_bytes(p.read_bytes().replace(b'Origin: asena', b'Origin: changed'))
    if mode == 'changed-catalog':
        files=list((release/'main/dep11').glob('Components-amd64.yml*'))
        assert files, 'No catalog to tamper with'
        for p in files: p.write_bytes(b'altered catalog data')
    (root/'tmp'/('negative-'+mode+'.sources')).write_text(
        f'Types: deb\nURIs: file:/opt/asena-negative-{mode}\nSuites: asena-test\nComponents: main\nSigned-By: {key}\n')
PY
        for mode in wrong-key unsigned changed-release changed-catalog; do
            mkdir -p "$root/var/lib/apt/lists-$mode/partial"
            if inside apt-get -o APT::Update::Error-Mode=any \
                -o "Dir::Etc::sourcelist=/tmp/negative-$mode.sources" \
                -o Dir::Etc::sourceparts=- -o "Dir::State::lists=/var/lib/apt/lists-$mode" \
                update > "$evidence/rejected-$mode.log" 2>&1; then
                printf 'APT accepted invalid archive: %s\n' "$mode" >&2; exit 1
            fi
            printf 'PASS: APT rejected %s\n' "$mode"
        done
        ;;
    *) printf '%s\n' 'Unknown test operation.' >&2; exit 2 ;;
esac
printf 'PASS: %s in isolated Debian root\n' "$operation"
