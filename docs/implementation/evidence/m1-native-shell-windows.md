# Milestone 1 native-shell spike evidence: Windows

**Date:** 2026-08-27

**Platform:** Windows x86-64

**Rust:** 1.98.0 MSVC

**Shell candidate:** eframe/egui 0.36.1 with native OpenGL rendering

**Dialog candidate:** rfd 0.17.2

## Question

Can SeqQC replace the Milestone 0 system-webview presentation process with a
native Rust executable that reuses the existing discovery and policy crates,
remains responsive during folder discovery, and opens no network endpoint at
startup?

## Implemented slice

- Native Windows process with no Tauri, WebView2, browser, or local web server
- Clinical night, clinical light, and high-contrast themes
- Native local-folder picker
- Background recursive sequencing-input discovery
- FASTQ/BAM/CRAM counts and proposed-group summary
- Searchable, bounded sample-map table with explicit review status
- Linux-only alignment policy surfaced in the navigation area
- AccessKit platform accessibility integration

The native application depends only on `seqqc-discovery` and `seqqc-engine`
from the SeqQC workspace in this slice. It does not yet inspect file contents,
persist an encrypted project, or export a report.

## Dependency boundary

`eframe` default features are disabled. The enabled features are `accesskit`,
`default_fonts`, `glow`, `wayland`, and `x11`. Browser-link, inspection-server,
framework persistence, web, updater, and HTTP capabilities are not enabled.

The initial `wgpu` renderer attempt was rejected after the locked Windows build
resolved incompatible `windows` interfaces inside `wgpu-hal` 30.0.1 and
`gpu-allocator` 0.28.0. Switching the spike to eframe's native OpenGL renderer
removed that path. The accepted dependency tree contains no Tauri, WebView2,
`reqwest`, `hyper`, `ureq`, `curl`, `webbrowser`, or WebSocket transport.

CI now checks this forbidden set against the reachable dependency tree for
`seqqc-native-desktop` on Windows, macOS, and Linux.

## Verification

```text
& .\tools\invoke-rust.ps1 check --package seqqc-native-desktop
& .\tools\invoke-rust.ps1 test --locked --package seqqc-native-desktop
& .\tools\invoke-rust.ps1 clippy --locked --package seqqc-native-desktop --all-targets
& .\tools\invoke-rust.ps1 fmt --all -- --check
& .\tools\invoke-rust.ps1 build --release --locked --package seqqc-native-desktop
& .\tools\observe-no-network-windows.ps1 -Executable .\target\release\seqqc-native-desktop.exe -Seconds 10
```

Results:

```text
Native unit tests: 2 passed, 0 failed
Clippy: no warnings
Release build: passed
Observed process family: seqqc-native-desktop.exe (PID 13012)
No TCP or UDP endpoints were observed in the SeqQC process family for 10 seconds.
```

The observer was strengthened during this spike. It now runs hidden, fails
closed if CIM process enumeration or TCP/UDP enumeration is unavailable, fails
if no process is observed, records the process family, and terminates the test
process after observation.

Release binary:

```text
Path: target/release/seqqc-native-desktop.exe
Size: 7,493,120 bytes
SHA-256: FAC2803DF2C0428F76739AEAEAAF3D602171D2B64100D106FB1644C4D8B9D923
```

## Interpretation and limitations

This result retires the observed Windows WebView2 startup-network blocker for
the new native executable. It does not close the SeqQC strict-offline gate.

Required follow-up evidence includes:

- interactive folder selection and long-running discovery under observation;
- packet-level or independently implemented process-family corroboration;
- equivalent clean-host macOS and Linux build/runtime observation;
- crash, accessibility, scaling, GPU fallback, and large-project navigation;
- encrypted project lifecycle and report parity; and
- signed platform packages.

No clinical, diagnostic, regulated-workflow, or complete strict-offline claim
is supported by this spike alone.
