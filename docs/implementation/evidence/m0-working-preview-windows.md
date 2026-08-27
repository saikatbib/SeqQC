# M0 working preview and packaging evidence: Windows

**Evidence date:** 2026-08-27  
**Platform:** Windows x86-64  
**Application source version:** 0.1.0-alpha.1  
**Windows package version:** 0.1.0-1  
**Status:** Functional engineering preview; strict-offline gate failed

## Functional scope exercised

- Production TypeScript/Vite build of the desktop presentation layer.
- Native Rust command boundary and embedded local contract registry.
- Recursive synthetic FASTQ/BAM/CRAM discovery and structural inspection.
- Conservative pairing/ambiguity handling and Linux-only alignment policy.
- Canonical report export reconciliation across HTML, PDF, JSON, CSV, and TSV.
- SQLCipher integrity, generated project keys, writer lock, audit chain, and
  credential-store boundary tests in the standalone security crate.
- Safe offline job-package rendering for Generic, Slurm, PBS Pro, LSF, and SGE.
- Windows release executable plus MSI and NSIS packaging.

## Reproducible checks

```powershell
pnpm check
pnpm test
pnpm desktop:build
pnpm generate:sbom
& .\tools\invoke-rust.ps1 test --workspace --locked
& .\tools\invoke-rust.ps1 fmt --all --check
& .\tools\invoke-rust.ps1 clippy --workspace --all-targets --locked -- -D warnings
& .\tools\observe-no-network-windows.ps1 `
    -Executable .\target\release\seqqc-desktop.exe -Seconds 10
```

## Contract and test results

```text
Repository policy/contracts: PASS
  8 schemas; 7 valid examples; 3 expected-invalid examples
  128 assigned metrics; 23 dependency candidates
  2 desktop capability permissions

Node regression suite: PASS
  3 passed; 0 failed

Frontend production build: PASS
  TypeScript and Vite completed

Rust workspace tests: PASS
  29 passed; 0 failed; 1 ignored interactive credential-store test
Rust formatting: PASS
Rust strict all-target Clippy: PASS
```

The Windows Credential Manager round-trip test is intentionally marked ignored
in the default suite. Running under a process without an interactive credential
logon session returned Windows error 1312. Release validation must run that test
explicitly in a representative signed-in user session.

## Package artifacts

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/seqqc-desktop.exe` | 9,935,360 | `DB1DE7F0D85CF4ACA5197907F4D935FC78C15E0658549F07A2E3F0EDD11E0D18` |
| `target/release/bundle/msi/SeqQC Studio_0.1.0-1_x64_en-US.msi` | 3,358,720 | `EBB6842D8E4E14751A8C0AF1DA1C6E4E8832F9727E2909B289142D3E263837AE` |
| `target/release/bundle/nsis/SeqQC Studio_0.1.0-1_x64-setup.exe` | 2,265,967 | `C9E7A1C534F16A69FB61FE184EE87754AF091DAB3B31C511B339B0E3B1CBB5BB` |

These binaries are unsigned. They are retained locally and are not source
repository contents.

## Strict-offline runtime result: failed

The 10-second process-family observation detected four established non-loopback
IPv6 TCP/443 connections from WebView2 151.0.4129.107 descendants:

```text
msedgewebview2.exe -> [2603:1036:305:401a::2]:443 (two local ports)
msedgewebview2.exe network service -> [2001:558:feed:443::118]:443
msedgewebview2.exe network service -> [2001:558:feed:443::117]:443
```

The browser process command line identified `seqqc-desktop.exe` as the WebView
host. Application code has no HTTP/updater plugin or remote asset path, and CSP
uses `connect-src 'none'`; the observed processes belong to the system WebView2
runtime. Separate trials with documented background-feature disabling arguments
and a fail-closed proxy/host resolver still showed outbound connections.

This result is a release blocker, not an accepted limitation for production.
ADR 0009 supersedes the system-webview presentation-shell decision. The preview
may be evaluated only on an OS-enforced isolated host and may not be represented
as satisfying SeqQC's strict-offline requirement.

## Boundaries of this evidence

- No scientific accuracy or clinical validity is claimed.
- The SQLCipher/security crate is not connected to desktop project persistence.
- macOS and Linux packages have not been produced on this Windows host.
- Network observation detects endpoints visible during its window; production
  approval also requires event-based monitoring for attempted DNS and sockets.
- Code signing, installer signing, and notarization remain release work.
