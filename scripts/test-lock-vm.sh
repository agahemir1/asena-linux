#!/bin/bash
# Only the disposable development VM: temporary credential and PAM policy.
set -euo pipefail
[[ ${ASENA_LOCK_TEST_VM:-0} == 1 && $(id -un) == asena ]] || exit 2
[[ $(sudo passwd -S asena | cut -d ' ' -f 2) == L ]] || {
    printf '%s\n' 'The test requires the initially locked disposable VM account.' >&2; exit 2;
}
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
password_file=$HOME/.local/state/asena-lock-test-password
[[ ! -e $password_file && ! -L $password_file && ! -e /etc/pam.d/asena-lock ]] || exit 2
policy_created=0
cleanup() {
    local result=$?
    trap - EXIT
    if [[ -f $password_file ]]; then
        local logs=()
        mapfile -d '' -t logs < <(find target/evidence/session-smoke-cairo -maxdepth 1 -type f -name '*.log' -print0)
        if [[ ${#logs[@]} -gt 0 ]]; then
            local scan=0
            grep -F -q -f "$password_file" "${logs[@]}" || scan=$?
            if [[ $scan == 0 ]]; then printf '%s\n' 'Credential found in test logs.' >&2; result=1;
            elif [[ $scan != 1 ]]; then printf '%s\n' 'Credential log scan failed.' >&2; result=1; fi
        fi
    fi
    sudo passwd -l asena >/dev/null
    rm -f -- "$password_file"
    if [[ $policy_created == 1 ]] && cmp -s data/pam.d/asena-lock /etc/pam.d/asena-lock; then
        sudo rm -- /etc/pam.d/asena-lock
    fi
    exit "$result"
}
trap cleanup EXIT
umask 077
mkdir -p "$HOME/.local/state"
python3 - "$password_file" <<'PY'
import secrets, sys
from pathlib import Path
with Path(sys.argv[1]).open('xb') as output:
    output.write(secrets.token_urlsafe(32).encode())
PY
sudo install -m 0644 data/pam.d/asena-lock /etc/pam.d/asena-lock
policy_created=1
python3 - "$password_file" <<'PY' | sudo chpasswd
import sys
from pathlib import Path
sys.stdout.buffer.write(b'asena:' + Path(sys.argv[1]).read_bytes() + b'\n')
PY
ASENA_SMOKE_LOCK=1 ASENA_POWER_TEST_VM=1 ASENA_LOG_STDERR=1 GSK_RENDERER=cairo bash scripts/smoke-session.sh
