# Changelog

All notable changes are documented here. The project follows Semantic
Versioning while pre-release interfaces may still change.

## 0.1.0-alpha.3 - 2026-08-28

### Added

- Cross-platform encrypted local profile sign-in with explicit and 15-minute
  inactivity locking.
- Argon2id + XChaCha20-Poly1305 wrapping of a random profile key; human
  passphrases are never project database keys.
- Independent random project keys stored inside the SQLCipher-encrypted profile
  vault.
- Native create/open/edit/close project workflow, encrypted recent-project
  catalog, opaque project folders, versioned outer manifests, and single-writer
  leases.
- Persisted tamper-evident audit events and profile/project integrity tests.
- macOS disk-image packaging alongside the existing `.app` ZIP.

### Safety boundary

The cryptographic profile remains an engineering-preview format pending
independent review, recovery/key-rotation design, and three-OS runtime evidence.
The release does not calculate validated biological QC metrics and is not
validated for diagnostic, clinical-decision, or regulated production use.

## 0.1.0-alpha.2 - 2026-08-27

### Added

- Webview-free native desktop release packages for Windows x86-64, macOS Apple
  silicon, and Linux x86-64.
- Per-user Windows installer with Start-menu registration, Installed Apps
  metadata, an optional desktop shortcut, and an uninstaller.
- Tagged GitHub Release automation with SHA-256 checksums and installer
  install/uninstall smoke testing.

### Safety boundary

The packages remain unsigned engineering previews and are not validated for
clinical or diagnostic use. The native shell discovers local sequencing inputs;
the complete assay-specific QC pipeline is not yet connected.

## 0.1.0-alpha.1 - 2026-08-26

### Added

- Offline-native Tauri desktop preview for Windows, macOS, and Linux.
- Guided, Standard, and Expert views with clinical light, night, and
  high-contrast themes.
- Native local folder/file selection with a narrowly scoped open-dialog
  permission.
- Recursive FASTQ/FQ/GZ, BAM, and CRAM discovery, pairing proposals, ambiguity
  disclosure, streamed structural inspection, and CRAM reference selection.
- One canonical investigational report exported as standalone HTML, PDF, JSON,
  CSV, and TSV.
- SQLCipher project-store, operating-system key-store, single-writer lock, and
  audit-chain foundations.
- Generic, Slurm, PBS Pro, LSF, and SGE offline job-package generation.
- Versioned schemas, a 128-metric registry, synthetic fixtures, and repository
  policy checks.

### Safety boundary

This release is Milestone 0 architecture evidence. It is not a validated
scientific QC pipeline, medical device, diagnostic product, or substitute for
organizational validation.

### Known limitations

- The Windows Tauri/WebView2 preview fails the strict-offline runtime gate:
  descendant WebView2 processes made non-loopback HTTPS connections during
  launch observation. The installers are evaluation artifacts only.
- SQLCipher/key-store foundations are tested as a separate core crate but are
  not yet wired into desktop project persistence.
- macOS and Linux packaging evidence requires the public CI workflow to run.
