#!/bin/bash
# Optional test services for the disposable Debian VM, not the user's host.
set -euo pipefail
[[ $(uname -s) == Linux && ${ASENA_SERVICE_TEST_VM:-0} == 1 ]] || {
    printf '%s\n' 'Run only inside the development VM with ASENA_SERVICE_TEST_VM=1.' >&2; exit 2;
}
# Keep the cloud VM's existing SSH network under its current network manager.
sudo install -d -m 755 /etc/NetworkManager/conf.d
printf '[keyfile]\nunmanaged-devices=interface-name:*\n' |
    sudo tee /etc/NetworkManager/conf.d/90-asena-dev-vm.conf >/dev/null
sudo apt-get install -y --no-install-recommends libpulse-dev pipewire-pulse wireplumber \
    pulseaudio-utils upower power-profiles-daemon network-manager bluez
systemctl --user start pipewire.socket pipewire-pulse.socket wireplumber.service
