#!/bin/bash

basedir="$(cd $(dirname $0)/.. && pwd -P)"
exefile="$basedir/target/release/snx-rs"
conffile="$basedir/assets/snx-rs.conf"
unitfile="$basedir/assets/snx-rs.service"
installdir="/opt/snx-rs"
confdir="/etc/snx-rs"

if [ -x "/opt/snx-rs/snx-rs" ]; then
    echo "Error: looks like the app is already installed under /opt/snx-rs!"
    exit 1
fi

if [ "$EUID" != "0" ]; then
    echo "Error: please run me as a root!"
    exit 1
fi

if [ ! -x "$exefile" ]; then
    echo "Error: no $exefile executable!"
    exit 1
fi

echo "Copying executable: $installdir/snx-rs"
mkdir -p "$installdir"
cp "$exefile" "$installdir/"

echo "Creating config file: /etc/snx-rs/snx-rs.conf, please edit it as a root user"
mkdir -p "$confdir"
install -m 0600 "$conffile" "$confdir/"

echo "Creating systemd service"
cp "$unitfile" /etc/systemd/system/

echo "Reloading systemd daemon"
systemctl daemon-reload

echo "Installation completed. Run 'systemctl start snx-rs' to start the client"
exit 0
