#!/bin/bash
# Stop/restart only the disposable VM's user audio service, restoring it on exit.
set -euo pipefail
[[ ${ASENA_AUDIO_TEST_VM:-0} == 1 ]] || exit 2
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
ready=$repo/target/evidence/audio-reconnect-ready
rm -f "$ready"
probe_pid=''
cleanup(){
    trap - EXIT
    systemctl --user start pipewire-pulse.socket pipewire-pulse.service
    [[ -z $probe_pid ]] || kill "$probe_pid" 2>/dev/null || true
    rm -f "$ready"
}
trap cleanup EXIT
ASENA_AUDIO_READY_FILE="$ready" target/debug/examples/audio_probe --reconnect &
probe_pid=$!
for _ in {1..100}; do [[ -f $ready ]] && break; sleep 0.05; done
[[ -f $ready ]]
systemctl --user stop pipewire-pulse.socket pipewire-pulse.service
for _ in {1..100}; do [[ $(cat "$ready") == lost ]] && break; sleep 0.05; done
[[ $(cat "$ready") == lost ]]
systemctl --user start pipewire-pulse.socket pipewire-pulse.service
wait "$probe_pid"
probe_pid=''
