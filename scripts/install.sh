#!/usr/bin/bash

if grep -q '^ID=nixos' /etc/os-release 2>/dev/null; then
    echo "Running on NixOS, installation aborted"
    exit 1
fi

killall snx-rs-gui 2>/dev/null
systemctl stop snx-rs 2>/dev/null

echo "Installing application"
install -m 755 ./snx-rs ./snx-rs-gui ./snxctl /usr/bin/
install ./snx-rs.service /etc/systemd/system/
install ./snx-rs-gui.desktop /usr/share/applications/

echo "Starting service"
systemctl daemon-reload
systemctl enable --now snx-rs

echo "Installation finished."
