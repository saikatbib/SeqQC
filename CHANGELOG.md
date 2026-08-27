# Changelog

All notable changes are documented here. The project follows Semantic
Versioning while pre-release interfaces may still change.

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
