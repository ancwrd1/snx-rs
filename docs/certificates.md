# Certificate Authentication

## Certificate Validation

The following parameters control certificate validation during TLS and IKE exchanges:

* `ca-cert`: Comma-separated list of paths to PEM or DER files which contain custom CA root certificates
* `ignore-server-cert`: true|false. Disable all TLS certificate checks. Insecure and not recommended. Default is false.

Note that enabling the insecure option may compromise the channel security.

## Client Certificate Authentication

The following parameters control certificate-based authentication:

* `cert-type`: One of `none`, `pkcs12`, `pkcs8` or `pkcs11`. Choose `pkcs12` to read the certificate from an external PFX file. Choose `pkcs8` to read the certificate from an external PEM file (containing both private key and x509 cert). Private key must come first in this file. Choose `pkcs11` to use a hardware token via a PKCS11 driver.
* `cert-path`: Path to the PFX, PEM, or custom PKCS11 driver file, depending on the selected cert type. The default PKCS11 driver is `opensc-pkcs11.so`, which requires the opensc package to be installed.
* `cert-password`: Password for PKCS12 or PIN for PKCS11. Must be provided for those types.
* `cert-id`: Optional hexadecimal ID of the certificate for the PKCS11 type. Could be in the form of `xx:xx:xx` or `xxxxxx`.

Certificate authentication should be used with the appropriate vpn_XXX login type which has a "certificate" as its authentication factor.

## Machine Certificate Authentication

With the machine certificate authentication it is possible to combine the certificate with the normal authentication methods.
To enable it, specify the certificate authentication options as described in the previous section and use one of the normal
vpn_XXX login types. The machine certificate authentication must be enabled on the VPN server side.
The certificate subject must have an entry for the machine name: `CN=<machinename>`. It does not have to match the Linux hostname.

When using a GUI frontend, there is a switch in the settings dialog to enable this option.

## Certificate Enrollment

`snx-rs` supports certificate enrollment and renewal for those configurations which require certificate-based authentication.
It is implemented as a command-line interface, with two additional operation modes of the `snx-rs` application: `enroll` and `renew`.
Enrollment operation requires a registration key which the user should receive from the IT department. Renewal requires an existing certificate in PKCS12 format.

Usage:

```bash
# Enrollment into identity.p12 file using registration key 12345678
snx-rs --mode enroll \
       --reg-key 12345678 \
       --cert-path identity.p12 \
       --cert-password password \
       --server-name remote.company.com
```

```bash
# Renewal using existing identity.p12
snx-rs --mode renew \
       --cert-path identity.p12 \
       --cert-password password \
       --server-name remote.company.com
```

After enrollment or renewal, the obtained PKCS12 keystore can be used for tunnel authentication.
