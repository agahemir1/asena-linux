#!/bin/bash
# VM-only disposable devices. Restore default devices and unload our modules.
set -euo pipefail
[[ ${ASENA_AUDIO_TEST_VM:-0} == 1 ]] || { printf '%s\n' 'Set ASENA_AUDIO_TEST_VM=1 only inside the development VM.' >&2; exit 2; }
command -v pactl >/dev/null
previous_sink=$(pactl get-default-sink)
previous_source=$(pactl get-default-source)
sink_module='' source_module='' playback_pid='' record_pid=''
cleanup() {
    trap - EXIT
    for child in "$playback_pid" "$record_pid"; do [[ -z $child ]] || kill "$child" 2>/dev/null || true; done
    for child in "$playback_pid" "$record_pid"; do [[ -z $child ]] || wait "$child" 2>/dev/null || true; done
    [[ -z $source_module ]] || pactl unload-module "$source_module" || true
    [[ -z $sink_module ]] || pactl unload-module "$sink_module" || true
    [[ -z $previous_sink ]] || pactl set-default-sink "$previous_sink" 2>/dev/null || true
    [[ -z $previous_source ]] || pactl set-default-source "$previous_source" 2>/dev/null || true
}
trap cleanup EXIT
sink_module=$(pactl load-module module-null-sink sink_name=asena_test sink_properties=device.description=Asena_Test)
source_module=$(pactl load-module module-remap-source master=asena_test.monitor source_name=asena_test_mic)
pactl set-default-sink asena_test
pactl set-default-source asena_test_mic
pacat --playback --device=asena_test --rate=48000 --channels=2 --format=s16le /dev/zero >/dev/null 2>&1 &
playback_pid=$!
pacat --record --device=asena_test_mic >/dev/null 2>&1 &
record_pid=$!
sleep 0.3
if ! target/debug/examples/audio_probe; then pactl get-sink-volume asena_test; exit 1; fi
if (( $# > 0 )); then "$@"; fi
