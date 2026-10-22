#!/bin/bash
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
mode=${1:-}
[[ -z $mode || $mode == --check ]] || { printf '%s\n' 'Usage: update-po.sh [--check]' >&2; exit 2; }
mkdir -p target
pot=$(mktemp "$repo/target/asena-po.XXXXXX.pot")
prepared=$(mktemp -d "$repo/target/asena-gettext.XXXXXX")
trap 'rm -f "$pot"; rm -r "$prepared"' EXIT
# Debian 13 gettext 0.23 has no Rust parser. Our ordinary gettext("...")
# string literals are supported by the C scanner; do not use Rust raw strings.
python3 scripts/prepare-gettext.py "$prepared"
(
    cd "$prepared"
    xgettext --language=C --from-code=UTF-8 --keyword=gettext --keyword=ngettext:1,2 \
        --package-name=asena --package-version=0.1.0 \
        --files-from="$repo/po/POTFILES" --output="$pot"
)
# Blueprint marks translatable text as _("..."), which the C scanner reads.
xgettext --language=C --from-code=UTF-8 --keyword=_ --keyword=C_:1c,2 --join-existing \
    --output="$pot" data/ui/*.blp
# xgettext does not extract GSettings XML as Rust source.
xgettext --language=GSettings --from-code=UTF-8 --join-existing \
    --output="$pot" data/schemas/*.gschema.xml
while IFS= read -r language; do
    [[ -n $language && $language != \#* ]] || continue
    if [[ $mode == --check ]]; then
        msgcmp "po/$language.po" "$pot"
    else
        msgmerge --update --backup=none "po/$language.po" "$pot"
    fi
done < po/LINGUAS
if [[ -z $mode ]]; then
    cp "$pot" po/asena.pot
fi
