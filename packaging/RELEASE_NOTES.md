## SeqQC Studio native-shell engineering preview

This prerelease introduces the webview-free native SeqQC Studio desktop shell for Windows, macOS, and Linux.

### Included

- Local folder selection and asynchronous discovery of FASTQ, BAM, and CRAM inputs
- Searchable sample grouping and input-readiness summaries
- Clinical night, light, and high-contrast themes
- Linux-only alignment capability messaging
- No network client or webview dependency in the native executable tree
- SHA-256 checksums for every downloadable package
- Per-user Windows installer with Start-menu, Installed Apps, optional desktop shortcut, and clean uninstall integration

### Choose a package

- **Windows x86-64 installer (recommended):** `Setup-...-windows-x86_64.exe`
- **Windows x86-64 portable:** `windows-x86_64.zip`
- **macOS Apple silicon:** `macos-arm64.zip`
- **Linux x86-64:** `linux-x86_64.tar.gz`

These packages are unsigned engineering previews. Read `PORTABLE_PACKAGE_README.txt` inside the archive before launching.

> SeqQC Studio is not yet validated for clinical or diagnostic use. This release demonstrates the offline native shell and input discovery; the full QC analysis pipeline remains under active development.
