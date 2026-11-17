# Additional Usage Notes

* If identity provider SSO authentication is used in standalone mode, the browser URL will be printed to the console. In command mode, the browser will be opened automatically.
* User passwords can be stored in the OS keychain. This is controlled by the `keychain` option. The entered username is saved in the profile. In the GUI, the "Save credentials" checkbox controls this; without `keychain` the password is saved in the profile instead. Keychain integration is provided only in command mode or in the GUI frontend. The `password-factor` option controls which MFA factor to consider as a password.
