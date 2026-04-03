# Quick Start Guide

## GUI

1. Run the GUI frontend from the application menu of the desktop manager.
2. Click on the application tray icon, choose "Settings."
3. In the opened dialog, type the server address and press "Fetch info" to retrieve a list of supported login types.
4. Select the login type and save settings. Username and password fields are optional.
5. Click on the application tray icon and choose "Connect."

If the desktop environment does not have a dbus SNI interface, use the `--no-tray` command line parameter to the snx-rs-gui application to show the status window instead of the notification icon.

## CLI, Standalone Mode

1. Get the list of supported login types: `snx-rs -m info -s remote.company.com`
2. Run the tunnel: `sudo snx-rs -o vpn_Microsoft_Authenticator -s remote.company.com`

> **Note about certificate errors**: if the connection fails with the certificate error, add the `-X true` parameter (insecure and not recommended).

## CLI, Command Mode

1. Get the list of supported login types: `snx-rs -m info -s remote.company.com`
2. Create a configuration file: `$HOME/.config/snx-rs/snx-rs.conf`, with desired [options](https://github.com/ancwrd1/snx-rs/blob/main/docs/options.md).
3. Connect the tunnel with `snxctl connect` command.
