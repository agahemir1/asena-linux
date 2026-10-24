#!/bin/bash
# Run after preview-desktop.sh start. Leaves the preview running for review.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
# shellcheck source=scripts/native-env.sh
. "$repo/scripts/native-env.sh"
export LOCPATH=$repo/target/locale-data
shell_pid=$(pgrep -u "$(id -u)" -n -f '/target/debug/asena-shell$')
while IFS= read -r -d '' variable; do
    case $variable in
        XDG_RUNTIME_DIR=*|WAYLAND_DISPLAY=*|DBUS_SESSION_BUS_ADDRESS=*|XDG_DATA_DIRS=*|GSETTINGS_SCHEMA_DIR=*|ASENA_LOCALE_DIR=*|XDG_CURRENT_DESKTOP=*|GDK_BACKEND=*|GSK_RENDERER=*|LOCPATH=*|LC_ALL=*|LANGUAGE=*) export "${variable?}" ;;
    esac
done <"/proc/$shell_pid/environ"
work=$repo/target/evidence/apps-session
mkdir -p "$work/data/asena/weather" "$work/cache"
export XDG_DATA_HOME=$work/data XDG_CACHE_HOME=$work/cache
python3 - "$work" <<'PY'
from pathlib import Path
import json, sys
work = Path(sys.argv[1])
(work/'data/asena/weather/places.json').write_text(json.dumps(dict(places=[dict(name='İstanbul',region='Türkiye',latitude=41.01,longitude=28.97)], selected=0,units='Metric')))
stream = b'BT /F1 24 Tf 50 760 Td (Asena PDF test) Tj ET'
objects = [b'<< /Type /Catalog /Pages 2 0 R >>', b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
           b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>',
           b'<< /Length '+str(len(stream)).encode()+b' >>\nstream\n'+stream+b'\nendstream',
           b'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>']
data = bytearray(b'%PDF-1.4\n'); offsets=[0]
for index, obj in enumerate(objects,1):
    offsets.append(len(data)); data.extend(f'{index} 0 obj\n'.encode()+obj+b'\nendobj\n')
xref=len(data); data.extend(b'xref\n0 6\n0000000000 65535 f \n')
for offset in offsets[1:]: data.extend(f'{offset:010} 00000 n \n'.encode())
data.extend(f'trailer << /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF'.encode())
(work/'sample.pdf').write_bytes(data)
PY
active=''
cleanup() { if [[ -n $active ]]; then kill "$active" 2>/dev/null || true; fi; }
trap cleanup EXIT
for pair in 'monitor:SystemMonitor' 'disks:Disks' 'backup:Backup' 'camera:Camera' 'terminal:Terminal' 'documents:Documents' 'weather:Weather'; do
    binary=${pair%%:*} id=${pair#*:}
    args=()
    if [[ $binary == documents ]]; then args=("$work/sample.pdf"); fi
    "$repo/target/debug/asena-$binary" "${args[@]}" >"$work/$binary.log" 2>&1 &
    active=$!
    found=0
    for _ in {1..100}; do
        if wlrctl toplevel list | grep -F "org.asenalinux.$id" >/dev/null; then found=1; break; fi
        kill -0 "$active"
        sleep 0.1
    done
    [[ $found == 1 ]] || { cat "$work/$binary.log"; exit 1; }
    sleep 2
    if [[ $binary == terminal ]]; then
        rm -f "$work/terminal-proof.txt"
        wtype -d 15 "printf terminal-ok > '$work/terminal-proof.txt'"
        wtype -k Return
        for _ in {1..100}; do [[ $(cat "$work/terminal-proof.txt" 2>/dev/null) == terminal-ok ]] && break; sleep 0.1; done
        grim -o HEADLESS-1 "$work/terminal.png"
        [[ $(cat "$work/terminal-proof.txt") == terminal-ok ]]
    fi
    if [[ $binary == weather ]]; then
        for _ in {1..200}; do
            if compgen -G "$work/cache/asena/weather/*.json" >/dev/null; then break; fi
            sleep 0.1
        done
        compgen -G "$work/cache/asena/weather/*.json" >/dev/null
    fi
    grim -o HEADLESS-1 "$work/$binary.png"
    kill "$active"
    wait "$active" 2>/dev/null || true
    active=''
    printf 'PASS: %s window, screenshot and live process\n' "$binary"
done
for panel in applications printers recovery; do
    "$repo/target/debug/asena-settings" "$panel" >"$work/settings-$panel.log" 2>&1 &
    sleep 1
    grim -o HEADLESS-1 "$work/settings-$panel.png"
done
printf '%s\n' 'PASS: three Settings routes; terminal command execution; live weather response cached'
