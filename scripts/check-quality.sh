#!/bin/bash
set -euo pipefail
[[ $(uname -s) == Linux ]] || { printf '%s\n' 'Linux is required.' >&2; exit 1; }
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/native-env.sh
. "$repo/scripts/native-env.sh"
cd "$repo"
python3 scripts/check-source.py
bash scripts/compile-data.sh
bash scripts/update-po.sh --check
translation=$(LOCPATH="$repo/target/locale-data" LC_ALL=tr_TR.UTF-8 LANGUAGE=tr \
    TEXTDOMAINDIR="$repo/target/locale" gettext -d asena 'Pinned applications')
[[ $translation == 'Sabitlenmiş uygulamalar' ]] || {
    printf '%s\n' 'Turkish gettext lookup failed.' >&2
    exit 1
}
export GSETTINGS_SCHEMA_DIR=$repo/target/schemas
export ASENA_LOCALE_DIR=$repo/target/locale
export GSETTINGS_BACKEND=memory
shellcheck scripts/*.sh scripts/asena-session experiments/*/*.sh
xmllint --noout data/asena.gresource.xml data/schemas/*.xml
desktop-file-validate data/sessions/asena.desktop
cargo fmt --all -- --check
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest discover -s tests -p 'test_*.py' -v
