#!/usr/bin/env bash
set -euo pipefail

# Reverses the .pkg built by package/macos/package.sh. Run with sudo.

if [ "$(id -u)" -ne 0 ]; then
    # Double-clicking the .command file opens Terminal, so re-exec through sudo to prompt on the tty.
    script_path="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"
    exec sudo /bin/bash "$script_path"
fi

echo "Stopping snx-rs daemon"
launchctl bootout system/com.github.snx-rs 2>/dev/null || true

echo "Removing files"
rm -f /usr/local/bin/snx-rs /usr/local/bin/snxctl
rm -f /Library/LaunchDaemons/com.github.snx-rs.plist
rm -rf "/Library/Application Support/snx-rs"
rm -rf /Applications/SNX-RS.app
pkgutil --forget com.github.snx-rs 2>/dev/null || true

echo "Uninstall finished."
echo "If you installed the optional login item, also remove it with:"
echo "  launchctl bootout gui/\$(id -u) ~/Library/LaunchAgents/com.github.snx-rs.gui.plist"
echo "  rm ~/Library/LaunchAgents/com.github.snx-rs.gui.plist"
