#!/usr/bin/bash

if grep -q '^ID=nixos' /etc/os-release 2>/dev/null; then
    echo "Running on NixOS, installation aborted"
    exit 1
fi

systemctl stop snx-rs 2>/dev/null

# Versions before 6.4.2 installed the unit into /etc/systemd/system. Once the unit lives in
# /usr/lib, a file in /etc is an admin override and must be left alone.
if [ -f /etc/systemd/system/snx-rs.service ] && [ ! -f /usr/lib/systemd/system/snx-rs.service ]; then
    systemctl disable snx-rs 2>/dev/null
    rm -f /etc/systemd/system/snx-rs.service
fi

echo "Installing application"
install -m 755 ./snx-rs ./snx-rs-gui ./snxctl /usr/bin/
install -D -m 644 ./snx-rs.service /usr/lib/systemd/system/snx-rs.service
install ./snx-rs-gui.desktop /usr/share/applications/
for svg in ./*.svg ; do install -m 644 $svg /usr/share/icons/hicolor/symbolic/apps/ ; done
gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor 2>/dev/null || true

echo "Starting service"
systemctl daemon-reload
systemctl enable --now snx-rs

pkill -USR1 -x snx-rs-gui 2>/dev/null || true

echo "Installation finished."
