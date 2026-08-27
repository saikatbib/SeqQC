# ADR 0002: Offline capability boundary

- Status: Accepted
- Date: 2026-08-26

## Context

SeqQC Studio will run on local and air-gapped systems and may process sensitive
human data. “We do not upload data” is insufficient if unused networking,
updater, remote asset, or URL-loading features remain in the binary.

## Decision

- Release runtime contains no intentional public-network client capability.
- Do not include updater, telemetry, analytics, crash upload, remote font,
  remote documentation, advertising, or remote-reference code.
- Reject non-file input schemes including HTTP(S), FTP, S3, and GS.
- Package all UI and report assets.
- Apply a packaged-assets-only content security policy.
- Keep the local API disabled by default and loopback-only when enabled.
- Scheduler integration invokes local command-line clients and shared paths; it
  does not use SSH or remote APIs in version 1.
- CI tests attempted DNS/socket activity while a functional network is present.

## Consequences

- References, annotations, tools, presets, and updates are transferred through
  explicit offline files/installers.
- Documentation may display citations but must not fetch or open them inside the
  controlled application.
- Build systems may retrieve dependencies; build provenance and vendoring are
  separate from runtime behavior.
- A future network capability requires a superseding ADR and threat-model review.

## Verification gate

Installation, first launch, project creation, analysis, report viewing, failure
handling, archive import/export, and uninstall must produce zero connection
attempts under network instrumentation.

