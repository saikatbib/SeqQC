# Milestone 0 status

**Started:** 2026-08-26
**Evidence refreshed:** 2026-08-28
**Deliverable status:** Working cross-platform engineering alpha produced
**Milestone exit status:** Complete for unsigned alpha publication

## Outcome

Milestone 0 produced a usable SeqQC Studio preview, reusable Rust foundations,
versioned contracts, tests, release metadata, three-OS CI/package workflows,
and native deliverables. The production preview shell is a Rust-native egui
application with no embedded browser or network client. A user can create and
unlock a local encrypted profile, create/open/edit encrypted projects, retain
recent projects locally, lock the session, select input folders, review
conservatively proposed samples, and structurally inspect FASTQ/BAM/CRAM
inputs. The retained reporting engine exports its canonical readiness report
as HTML, PDF, JSON, CSV, and TSV.

## Delivered scope

| Area | Delivered evidence |
| --- | --- |
| Product and architecture | Product, scientific, UX, threat, data, execution, validation, and roadmap plans; nine ADRs |
| Contracts | Nine local JSON Schema 2020-12 contracts; eight valid and four invalid examples |
| Scientific registry | All 128 planned metric IDs assigned; interpretations remain investigational |
| Input handling | Recursive FASTQ/FQ/GZ, BAM, and CRAM discovery; nested folders; pairing/lane proposals; explicit ambiguities; cancellation |
| Structural inspection | Streamed FASTQ, gzip FASTQ, BAM, and CRAM summaries; truncation/corruption failures; local CRAM FASTA selection |
| Desktop UX | Native folder/file dialogs, guided/standard/expert density, light/night/high-contrast themes, accessible status language |
| Reporting | One schema-backed model with UUIDs, timestamps, source digest, method/provenance fields, and five reconciled exports |
| Security foundations | Passphrase-wrapped profile key, per-project random keys, SQLCipher project/profile stores, single-writer lease, encrypted recents, inactivity lock, and persistent audit hash chain |
| Execution foundations | Offline packages and conservative state mapping for Generic, Slurm, PBS Pro, LSF, and SGE |
| Platform policy | QC/reporting cross-platform; optional alignment rejected on Windows/macOS and allowed only on Linux |
| Release engineering | Apache-2.0, notices, contribution/security/support policy, SBOM, issue forms, three-OS CI/package workflows, installer checklist |
| Packaging | Windows installer/portable ZIP, macOS DMG/ZIP, and Linux tarball workflows; unsigned evaluation artifacts |

## Local Windows verification

```text
Repository checks:
  9 schemas
  8 valid examples
  4 deliberately invalid examples
  128 assigned metrics
  30 dependency candidates
  2 desktop capability permissions

Node tests:
  3 passed
  0 failed

Frontend:
  TypeScript check passed
  Vite production build passed

Rust workspace:
  40 passed
  0 failed
  1 ignored interactive credential-store test
  rustfmt --check passed
  strict all-target Clippy passed

Runtime architecture:
  Native egui shell contains no webview or network-client dependency

Windows packaging:
  Optimized executable and Inno Setup installer built successfully
  Silent install, Installed Apps registration, Start-menu shortcut, and uninstall passed

Runtime network observation:
  Only seqqc-native-desktop.exe was observed
  No TCP or UDP endpoints were observed for 10 seconds
```

The Windows Credential Manager round trip remains an explicit ignored
integration test because restricted runners may have no interactive credential
logon session. The application therefore uses the cross-platform passphrase
profile path in this alpha while retaining the credential-store facade for
future administrator-controlled integration.

## Remaining release constraints

1. Complete independent scientific, security/cryptography, accessibility,
   licensing, and regulated-workflow reviews before making validated claims.
2. Sign/notarize release artifacts or label every preview prominently as
   unsigned and unsuitable for deployment.
3. Validate packaged runtime behavior on representative clean Windows, macOS,
   and Linux systems; CI builds alone are not a deployment qualification.
4. Add documented key rotation/recovery and encrypted portable project-archive
   workflows before declaring the storage design production-ready.

## Publication decision

The source repository is suitable for private alpha development with the known
limitations visible. Current packages are unsigned engineering previews. They
must not be labeled clinically validated, diagnostic, or deployment-ready.

Detailed results and hashes are retained in
`docs/implementation/evidence/m0-working-preview-windows.md`.
