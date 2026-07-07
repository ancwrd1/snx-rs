# Implemented Features

* Browser-based identity provider authentication
* Username/password authentication with MFA support
* Certificate authentication via the provided client certificate (PFX, PEM, or HSM device)
* Hybrid authentication using machine certificate and user credentials
* Certificate enrollment and renewal using the command line interface
* Mobile Access authentication using VPN web portal
* HSM token authentication
* GUI frontend with tray icon (Linux, Windows, macOS)
* IPsec tunnel via Linux native kernel XFRM interface, or userspace TUN/ESP transport (used on Windows and macOS, where kernel XFRM is not available)
* Automatic IPsec tunnel reconnection without authentication, via optional parameter
* SSL tunnel via TUN device
* Store a password factor in the OS keychain using Secret Service API on Linux, or Keychain (Security.framework) on macOS
* Multiple connection profiles
