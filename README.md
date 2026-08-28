# SeqQC Studio

**SeqQC** is the open-source engine behind **SeqQC Studio**, a local-first
desktop application for understandable sequencing input quality control. It is
designed for wet-lab researchers, students, sequencing cores, bioinformaticians,
and clinical or preclinical teams working with WGS, WES, or bulk RNA-seq data.

> **Native-shell engineering preview:** the public package uses the new
> webview-free Rust application path in `apps/native-desktop`. It provides local
> folder selection, background FASTQ/BAM/CRAM discovery, searchable sample-map
> review, and clinical night/light/high-contrast themes. It does **not** yet
> calculate validated assay-specific biological QC metrics and must not be used
> for diagnosis or patient care. The dependency boundary and Windows runtime
> observation are clean; live macOS and Linux network observation remains a
> strict-offline release gate.

## Download the engineering preview

Download the newest engineering preview and `SHA256SUMS.txt` from
[SeqQC Studio Releases](https://github.com/saikatbib/SeqQC/releases):

- Windows x86-64: download the `Setup` executable for a normal per-user
  installation with Start-menu and Installed Apps entries. The ZIP remains
  available as a no-install portable option.
- macOS Apple silicon: extract the ZIP and move `SeqQC Studio.app` to
  Applications. The preview is unsigned, so use Control-click, then **Open**, on
  first launch.
- Linux x86-64: extract the tarball and run `./seqqc-studio` from a graphical
  desktop session.

Verify the downloaded archive against `SHA256SUMS.txt` before opening it. Intel
macOS and platform-signed packages are not included in this first prerelease.

## What works now

- Webview-free native portable packages built on Windows, macOS, and Linux by
  the tagged-release workflow.
- No application telemetry, updater, remote assets, HTTP client, local browser
  server, or runtime download path in the native shell.
- Recursive FASTQ/FQ/GZ, BAM, and CRAM discovery in nested folders and network
  mappings exposed by the operating system.
- Conservative paired-end and lane proposals that disclose missing or
  duplicate mates instead of silently choosing.
- Streamed structural inspection without loading full sequencing files into
  memory; CRAM accepts a user-selected local reference FASTA.
- Guided, Standard, and Expert views with clinical light, clinical night, and
  high-contrast themes.
- One schema-backed, investigational report exported to standalone HTML, PDF,
  JSON, CSV, and TSV.
- Cross-platform QC policy with optional alignment restricted to Linux.
- Security foundations for SQLCipher metadata, random project keys in native
  credential stores, a single-writer lock, and a chained audit log.
- Offline job-package contracts for Generic scripts, Slurm, PBS Pro, LSF, and
  SGE.

## Run the retained Milestone 0 reference UI

Install Node.js 22 or newer, pnpm 11.19.0, Rust 1.98.0, and the
[Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your
operating system. Windows SQLCipher builds also require Strawberry Perl.

```text
pnpm install --frozen-lockfile
pnpm check
pnpm test
pnpm desktop:build
cargo test --workspace --locked
pnpm --filter @seqqc/desktop tauri dev
```

On the provisioned Windows development host, replace `cargo ...` with:

```powershell
& .\tools\invoke-rust.ps1 test --workspace --locked
```

This older Tauri/WebView implementation remains as a UX and reporting reference
and is not the application distributed on the GitHub Releases page. Use an
operating-system network-deny policy for any sensitive-data evaluation of that
retained reference UI.

## Try the native-shell spike

The native shell does not use Node.js, React, Tauri, WebView2, WKWebView, or
WebKitGTK at runtime. Install the Rust and native graphics prerequisites for
your operating system, then run:

```text
cargo run --locked --package seqqc-native-desktop
```

On the provisioned Windows development host:

```powershell
& .\tools\invoke-rust.ps1 run --locked --package seqqc-native-desktop
```

This slice discovers and reviews input structure only. It does not yet run
validated biological QC or persist an encrypted project.

## Build the native release binary

```text
cargo build --release --locked --package seqqc-native-desktop
```

Tagged builds use `.github/workflows/package.yml` to create a Windows installer,
a Windows portable archive, macOS and Linux packages, calculate SHA-256
checksums, and publish them on the GitHub Releases page. The workflow verifies
that the Windows installer registers in Installed Apps and uninstalls cleanly.
See `packaging/release-checklist.md` before distributing binaries. Preview
packages are unsigned unless platform signing is configured, and they must not
be represented as validated clinical or regulated-workflow software.

## Safety and scientific scope

SeqQC uses cautious interpretations such as **Within expected range**,
**Needs review**, and **Not evaluated**. Every eventual scientific finding must
trace to a versioned method, threshold source, evidence quality, and known
limitations. Published, institutional, and user-defined presets are planned,
but are not validated in this preview.

Human clinical data is a design use case, not a regulatory claim. SeqQC Studio
is currently research-use, investigational software. Institutions must perform
their own security, privacy, scientific, deployment, and regulated-workflow
validation.

## Repository map

| Path | Purpose |
| --- | --- |
| `apps/native-desktop` | Webview-free native Rust production-shell spike |
| `apps/desktop` | Retained Tauri + React Milestone 0 UX/reference preview |
| `crates/seqqc-*` | Contracts, discovery, formats, workflow, execution, reporting, and security boundaries |
| `schemas/v1` | Offline JSON Schema 2020-12 contracts |
| `config` | Metric implementation and dependency review registries |
| `test-data/synthetic` | Small, deterministic, non-human fixtures |
| `docs/planning` | Product, science, UX, security, data, execution, validation, and roadmap baseline |
| `docs/decisions` | Architecture decision records |
| `docs/implementation` | Milestone status and retained evidence |
| `packaging` | Release and signing controls |

## Quality gates

```text
pnpm check
pnpm test
pnpm desktop:build
pnpm generate:sbom
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The repository policies verify eight schemas and examples, all 128 planned
metric assignments, exact dependency candidates, the Linux-only alignment
gate, and the desktop offline capability boundary. Synthetic native tests cover
format truncation, cancellation, scheduler escaping, exporter reconciliation,
SQLCipher integrity, credential-store boundary validation, writer locking, and
audit tamper detection. The live Windows Credential Manager round trip is an
explicit interactive release test because headless runners may have no usable
credential logon session.

The source-level capability gate passes. The packaged Windows runtime gate does
not: descendant `msedgewebview2.exe` processes were observed with non-loopback
HTTPS connections. See the [Milestone 0 status](docs/implementation/milestone-0-status.md)
and retained [Windows evidence](docs/implementation/evidence/m0-working-preview-windows.md).

## Contributing and security

Read `CONTRIBUTING.md`, `SECURITY.md`, and `CODE_OF_CONDUCT.md`. Never upload
real sequencing data, protected health information, credentials, project keys,
or encrypted project archives to an issue or pull request.

SeqQC is licensed under [Apache License 2.0](LICENSE). Third-party components
retain their own licenses; see `THIRD_PARTY_NOTICES.md` and the CycloneDX SBOM.

The intended public repository is
[github.com/saikatbib/SeqQC](https://github.com/saikatbib/SeqQC).
