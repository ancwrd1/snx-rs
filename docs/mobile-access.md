# Mobile Access Authentication

Mobile Access authentication is a feature that replaces the official `cshell` connection method using the web access.
It is helpful in cases when a DynamicID MFA code is required via the web portal login and the normal VPN connection via the advertised
authentication types does not work.
To use it, the snx-rs-gui application must be built with the `mobile-access` feature enabled. It requires the `webkit6` package to be installed.
Depending on the distribution, it may be called `libwebkitgtk-6.0-dev`, `webkit2gtk4-devel` or similar.
Additionally, it may require to install `libsoup-3.0-dev` and `libjavascriptcoregtk-6.0-dev`.

A new login type called "Mobile Access" will be visible from the dropdown list in the GUI settings. When connecting using that method,
the application will open a browser window and navigate to the VPN web portal. After successful authentication, the application will attempt
to find the password cookie in the web page. Then the browser will be closed and the tunnel will be established.
