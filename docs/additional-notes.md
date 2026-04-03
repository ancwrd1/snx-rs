# Additional Usage Notes

* If identity provider SSO authentication is used in standalone mode, the browser URL will be printed to the console. In command mode, the browser will be opened automatically.
* User passwords can be stored in the OS keychain (KDE KWallet or GNOME Keyring). This is controlled by the `keychain` option. Keychain integration is provided only in command mode or in the GUI frontend. The `password-factor` option controls which MFA factor to consider as a password.
