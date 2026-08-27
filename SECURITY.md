# Security policy

## Supported versions

SeqQC is pre-release software. Security fixes are applied to the current main
branch and the most recent published preview only.

## Reporting a vulnerability

Please do not open a public issue for a suspected vulnerability. Use GitHub's
private vulnerability reporting for `saikatbib/SeqQC`. If that feature is not
available, contact the repository owner through the private contact method on
the GitHub profile and include “SeqQC security” in the subject.

Include the affected version, operating system, reproduction steps, expected
impact, and whether sample information or credentials may have been exposed.
Do not attach real clinical data or secrets. You should receive an
acknowledgement within seven days.

## Security boundary

SeqQC Studio is designed to run without internet access. The local API is off,
there is no updater, telemetry, HTTP, shell, or general frontend filesystem
capability, and the packaged content security policy denies connections.

The current Windows engineering preview nevertheless fails the strict-offline
runtime gate: the operating-system WebView2 process family made non-loopback
HTTPS connections during an observed launch. Disabling WebView background
features and routing the webview through a fail-closed proxy did not eliminate
the behavior. Do not use the preview on a network-connected endpoint containing
sensitive data. An OS-enforced network-deny policy is required for evaluation;
the production desktop shell must pass independent zero-network testing.

The `seqqc-security` foundation crate uses SQLCipher Community with
application-generated keys held through the operating-system credential-store
facade. That crate is not yet connected to the preview application's project
management screen, so the current desktop installer must not be described as
providing encrypted project persistence.

These controls do not make the preview clinically validated, FIPS validated,
or suitable for diagnosis. Institutions remain responsible for endpoint,
account, backup, network isolation, access-control, and regulated-workflow
validation.
