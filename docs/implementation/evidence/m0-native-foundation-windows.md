# M0 native foundation evidence: Windows

**Evidence date:** 2026-08-26  
**Platform:** Windows x86-64  
**Status:** Historical foundation evidence; packaged runtime later failed the strict-offline gate

> Superseded evidence: native selection, packaging, and runtime observation were
> completed in the later
> [working-preview evidence](m0-working-preview-windows.md). That observation
> found outbound WebView2 connections and resulted in ADR 0009.

## Scope

This evidence covers the first native SeqQC Studio foundation on Windows:

- pinned Rust and frontend toolchains;
- an asset-backed Tauri 2 desktop shell;
- embedded local v1 contract registry;
- deterministic FASTQ/BAM/CRAM folder discovery;
- Linux-only alignment policy;
- a minimal Tauri capability and deny-all connection CSP;
- production frontend compilation and strict Rust linting.

It does not claim that a Windows installer has passed release signing, that
macOS/Linux packages build, or that a running packaged binary has passed live
DNS/socket instrumentation.

## Toolchain under test

| Component | Exact version/evidence |
| --- | --- |
| Rust | 1.98.0, `x86_64-pc-windows-msvc`, minimal profile |
| Cargo | 1.98.0 |
| rustfmt / Clippy | Rust 1.98.0 components |
| Visual Studio Build Tools | 2022 17.14.39, C++ workload |
| WebView2 Runtime | 151.0.4129.107 |
| Node.js | 24.19.0 build runtime |
| pnpm | 11.19.0 |
| Tauri Rust crate / build crate | 2.11.5 / 2.6.3 |
| Tauri CLI / JavaScript API | 2.11.4 / 2.11.1 |
| React / React DOM | 19.2.8 / 19.2.8 |
| TypeScript / Vite | 7.0.2 / 8.2.2 |

Exact dependency resolution is retained in `Cargo.lock` and `pnpm-lock.yaml`.
The Rust installation is development-only under the ignored `.toolchains/`
directory; it is not a runtime dependency or bundled application directory.

## Reproducible checks

Run from the repository root on the provisioned Windows host:

```powershell
node tools/check-all.mjs
node --test
pnpm --filter '@seqqc/desktop' build
& .\tools\invoke-rust.ps1 test --workspace --locked
& .\tools\invoke-rust.ps1 clippy --workspace --all-targets --locked '--' '-Dwarnings'
```

## Results

```text
Repository policy/contracts:
  8 schemas
  7 valid examples
  3 deliberately invalid examples
  128 assigned metrics
  15 dependency candidates
  1 desktop capability permission

Node prototype tests:
  3 passed
  0 failed

Rust tests:
  8 passed
  0 failed

Frontend production build:
  TypeScript project build passed
  Vite production build passed

Rust quality gate:
  rustfmt --check passed
  Clippy --all-targets with -D warnings passed
```

## Offline architecture assertions

The repository checker currently enforces all of the following:

- CSP includes `connect-src 'none'`;
- the Tauri configuration contains no `devUrl`;
- updater artifact creation is false;
- the asset protocol and global Tauri injection are disabled;
- only `desktop-base` is active, with only `core:default` permission;
- SeqQC declares no Tauri HTTP, updater, shell, or filesystem plugin;
- application-owned frontend source contains no HTTP(S) URL.

Tauri's transitive dependency graph can include generic networking-capable
libraries. Their presence is not evidence of a network attempt, and manifest
checks are not a substitute for observation. A packaged-binary DNS/socket test
under an operating-system monitor remains required before the no-network exit
criterion can pass.

## Known limitations and next evidence

- The Select input folder control is intentionally disabled; native folder
  selection and command authorization are not wired in this foundation slice.
- The generated app mark is provisional branding.
- Run equivalent builds and policy checks on macOS and Linux.
- Build the Windows installer, verify contents, and instrument the running
  binary for attempted DNS, TCP, UDP, and updater access.
- Add signing/notarization evidence later in the release-engineering milestone.
