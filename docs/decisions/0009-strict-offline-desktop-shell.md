# ADR 0009: Strict-offline production desktop shell

- Status: Accepted
- Date: 2026-08-27
- Owners: Project owner and SeqQC architecture
- Supersedes: ADR 0001 for the production presentation shell

## Context

SeqQC Studio has a non-negotiable requirement to run on disconnected compute
nodes and never connect to the internet. The application-level Tauri boundary
contains no HTTP/updater plugin, denies frontend connections through CSP, and
uses only packaged assets.

During the Windows package test, descendant `msedgewebview2.exe` processes
opened non-loopback TCP connections on port 443. Additional WebView2 arguments
to disable background networking, component updates, experiments, sync, and
related features did not remove the connections. A fail-closed proxy and host
resolver mapping also did not eliminate them.

Application-level permissions are therefore insufficient evidence for a strict
offline claim when the presentation runtime is an independently serviced system
webview.

## Decision

- Keep the current Tauri/React application only as a Milestone 0 engineering
  preview and UX/reference implementation.
- Do not publish it as a strict-offline, clinical, or regulated-workflow build.
- The production presentation shell must not depend on an uncontained system
  webview. The next implementation milestone will select and spike a native
  Rust GUI that preserves the existing Rust contracts and analysis crates.
- A system-webview option may be reconsidered only if an OS-enforced containment
  profile is supported on all target platforms and independent observation
  demonstrates zero attempted DNS, TCP, or UDP activity.
- Network-deny controls remain defense in depth; they do not excuse runtime
  connection attempts in a release candidate.

## Consequences

- The working preview remains useful for validating discovery, inspection,
  report exports, visual language, and application contracts.
- The React presentation layer will not be treated as the production shell.
- UI work must be separated from the reusable Rust engine, reporting, security,
  and scheduler crates.
- Windows, macOS, and Linux packages must each pass source-capability review and
  live process-family network observation before release.

## Alternatives considered

- Document the behavior and continue with Tauri: rejected because it conflicts
  with the explicit product requirement.
- Rely only on WebView2 command-line flags: rejected by the observed test.
- Require endpoint firewall isolation while retaining WebView2: acceptable for
  evaluating this preview, but not as the production architecture.
- Electron or another browser runtime: rejected for the same independently
  networked runtime concern and a larger packaged surface.

## Evidence

See `docs/implementation/evidence/m0-working-preview-windows.md` and
`tools/observe-no-network-windows.ps1`.
