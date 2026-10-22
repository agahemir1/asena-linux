#!/bin/bash
pid=$(pgrep -n asena-shell || true)
if [[ -z "$pid" ]]; then
    echo "asena-shell not found" >&2
    exit 1
fi
XDG_RUNTIME_DIR=$(tr '\0' '\n' < /proc/"$pid"/environ | grep '^XDG_RUNTIME_DIR=' | cut -d= -f2-)
export XDG_RUNTIME_DIR
WAYLAND_DISPLAY=$(tr '\0' '\n' < /proc/"$pid"/environ | grep '^WAYLAND_DISPLAY=' | cut -d= -f2-)
export WAYLAND_DISPLAY
export GSETTINGS_SCHEMA_DIR=/home/asena/asena/target/schemas
cd /home/asena/asena || exit 1
./target/debug/asena-settings
echo "Exit code: $?"
