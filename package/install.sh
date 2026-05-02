#!/usr/bin/bash

if grep -q '^ID=nixos' /etc/os-release 2>/dev/null; then
    echo "Running on NixOS, installation aborted"
    exit 1
fi

systemctl stop snx-rs 2>/dev/null

echo "Installing application"
install -m 755 ./snx-rs ./snx-rs-gui ./snxctl /usr/bin/
install ./snx-rs.service /etc/systemd/system/
install ./snx-rs-gui.desktop /usr/share/applications/
for svg in ./*.svg ; do install -m 644 $svg /usr/share/icons/hicolor/symbolic/apps/ ; done
gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor 2>/dev/null || true

echo "Starting service"
systemctl daemon-reload
systemctl enable --now snx-rs

pkill -USR1 -x snx-rs-gui 2>/dev/null || true

echo "Installation finished."
