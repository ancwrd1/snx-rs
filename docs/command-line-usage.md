# Command Line Usage

Check the [Configuration Options](https://github.com/ancwrd1/snx-rs/blob/main/docs/options.md) section for a list of all available options. Options can be specified in the configuration file
and the path of the file given via `-c /path/to/custom.conf` command line parameter.

Alternatively, in standalone mode, they can be specified via the command line of the `snx-rs` executable, prefixed with `--` (double dash).

Before the client can establish a connection, it must know the login (authentication) method to use (`--login-type` or `-o` option).
To find the supported login types, run it with the `-m info` parameter:

```sh
# in standalone mode
snx-rs -m info -s remote.acme.com
# in command mode
snxctl info
```

This command will display the supported login types. Use the `vpn_XXX` identifier as the login type.
If a certificate error is returned, try adding the `-X true` command line parameter to ignore certificate errors.

Example output (may differ for your server):

```text
           Server address: remote.company.com
                Server IP: 1.2.3.4
           Client enabled: true
      Supported protocols: IPSec, SSL, L2TP
       Preferred protocol: IPSec
                TCPT port: 443
                NATT port: 4500
  Internal CA fingerprint: MATE FRED PEN RANK LIP HUGH BEAD WET CAGE DEW FULL EDIT
[Microsoft Authenticator]: vpn_Microsoft_Authenticator (password)
       [Emergency Access]: vpn_Emergency_Access (password)
      [Username Password]: vpn_Username_Password (password)
   [Azure Authentication]: vpn_Azure_Authentication (identity_provider)
```

There are two ways to use the application:

* **Command Mode**: Selected by the `-m command` parameter. In this mode, the application runs as a service without establishing a connection and awaits commands from the external client. Use the `snxctl` utility to send commands to the service. This mode is recommended for desktop usage. The following commands are accepted:
  - `connect`: Establish a connection. Parameters are taken from the `~/.config/snx-rs/snx-rs.conf` file.
  - `disconnect`: Disconnect a tunnel.
  - `reconnect`: Drop the connection and then reconnect.
  - `status`: Show connection status.
  - `info`: Show server authentication methods and supported tunnel types.
  - Run it with the `--help` option to get usage help.
* **Standalone Service Mode**: Selected by the `-m standalone` parameter. This is the default mode if no parameters are specified. Run `snx-rs --help` to get help with all command line parameters. In this mode, the application takes connection parameters either from the command line or from the specified configuration file. This mode is recommended for headless usage.
