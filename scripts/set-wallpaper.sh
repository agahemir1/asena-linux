#!/bin/bash
pid=$(pgrep -n asena-shell)
val_xdg=$(tr '\0' '\n' < "/proc/$pid/environ" | grep '^XDG_RUNTIME_DIR=' | cut -d= -f2-)
export XDG_RUNTIME_DIR="$val_xdg"
val_dbus=$(tr '\0' '\n' < "/proc/$pid/environ" | grep '^DBUS_SESSION_BUS_ADDRESS=' | cut -d= -f2-)
export DBUS_SESSION_BUS_ADDRESS="$val_dbus"
export GSETTINGS_SCHEMA_DIR=/home/asena/asena/target/schemas
gsettings set org.asenalinux.desktop background-uri 'file:///usr/share/backgrounds/asena/asena-gece.jpg'
gsettings set org.asenalinux.desktop background-mode 'zoom'
echo "GSettings updated:"
gsettings get org.asenalinux.desktop background-uri
gsettings get org.asenalinux.desktop background-mode
