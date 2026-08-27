# Milestone 0 status

**Started:** 2026-08-26  
**Evidence refreshed:** 2026-08-27  
**Deliverable status:** Working Windows engineering preview produced  
**Milestone exit status:** Blocked by strict-offline runtime gate

## Outcome

Milestone 0 produced a usable SeqQC Studio preview, reusable Rust foundations,
versioned contracts, tests, release metadata, CI/package workflows, and Windows
installers. A user can select an input folder, review conservatively proposed
samples, structurally inspect FASTQ/BAM/CRAM inputs, select a local CRAM
reference, and export one canonical readiness report as HTML, PDF, JSON, CSV,
and TSV.

The milestone cannot be signed off as a strict-offline release. On Windows,
the packaged Tauri application launched descendant WebView2 processes that
established non-loopback HTTPS connections. Application CSP and capability
checks pass, but those controls do not govern all system-webview behavior. ADR
0009 restricts this build to engineering evaluation and requires a production
shell without an uncontained system webview.

## Delivered scope

| Area | Delivered evidence |
| --- | --- |
| Product and architecture | Product, scientific, UX, threat, data, execution, validation, and roadmap plans; nine ADRs |
| Contracts | Eight local JSON Schema 2020-12 contracts; seven valid and three invalid examples |
| Scientific registry | All 128 planned metric IDs assigned; interpretations remain investigational |
| Input handling | Recursive FASTQ/FQ/GZ, BAM, and CRAM discovery; nested folders; pairing/lane proposals; explicit ambiguities; cancellation |
| Structural inspection | Streamed FASTQ, gzip FASTQ, BAM, and CRAM summaries; truncation/corruption failures; local CRAM FASTA selection |
| Desktop UX | Native folder/file dialogs, guided/standard/expert density, light/night/high-contrast themes, accessible status language |
| Reporting | One schema-backed model with UUIDs, timestamps, source digest, method/provenance fields, and five reconciled exports |
| Security foundations | SQLCipher store, generated keys, credential-store facade, single-writer lease, stale evidence retention, audit hash chain |
| Execution foundations | Offline packages and conservative state mapping for Generic, Slurm, PBS Pro, LSF, and SGE |
| Platform policy | QC/reporting cross-platform; optional alignment rejected on Windows/macOS and allowed only on Linux |
| Release engineering | Apache-2.0, notices, contribution/security/support policy, SBOM, issue forms, three-OS CI/package workflows, installer checklist |
| Windows packaging | Release executable, MSI, and NSIS installer produced; unsigned evaluation artifacts |

## Local Windows verification

```text
Repository checks:
  8 schemas
  7 valid examples
  3 deliberately invalid examples
  128 assigned metrics
  23 dependency candidates
  2 desktop capability permissions

Node tests:
  3 passed
  0 failed

Frontend:
  TypeScript check passed
  Vite production build passed

Rust:
  29 passed
  0 failed
  1 ignored interactive credential-store test
  rustfmt --check passed
  strict all-target Clippy passed

Packaging:
  Windows release executable, MSI, and NSIS build passed

Runtime network observation:
  FAILED — WebView2 descendants established four observed IPv6 TCP/443 connections
```

The Windows Credential Manager round trip is an explicit ignored integration
test in the default suite because headless or restricted runners may have no
interactive credential logon session. It must be run in a representative user
session as a release gate. The SQLCipher/key-store foundation is not yet wired
to desktop project persistence.

## Release blockers

1. Replace or independently contain the Tauri/WebView2 presentation shell and
   demonstrate zero attempted DNS, TCP, and UDP activity for its full process
   family.
2. Run the package/test workflows on clean macOS and Linux hosts after the
   repository is published; retain equivalent evidence and installers.
3. Connect encrypted storage, credential management, project locking, and audit
   persistence to the desktop project lifecycle.
4. Complete independent scientific, security/cryptography, accessibility,
   licensing, and regulated-workflow reviews before making validated claims.
5. Sign/notarize release artifacts or label every preview prominently as
   unsigned and unsuitable for deployment.

## Publication decision

The source repository is suitable for public alpha development with the known
limitations visible. The current installers are suitable only as unsigned
engineering previews on an OS-enforced isolated host. They must not be labeled
strictly offline, clinically validated, diagnostic, or deployment-ready.

Detailed results and hashes are retained in
`docs/implementation/evidence/m0-working-preview-windows.md`.
