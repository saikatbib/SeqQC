# Packaging SeqQC Studio

SeqQC has no runtime dependency downloader. Build dependencies are resolved
only while producing an installer; the resulting application has no updater or
network capability.

## Local package

```text
pnpm install --frozen-lockfile
pnpm check
pnpm --filter @seqqc/desktop tauri build
```

Artifacts are written below `target/release/bundle`. The required Tauri native
prerequisites differ by operating system. Windows additionally needs Perl for
the vendored SQLCipher/OpenSSL build.

## Cross-platform evidence

`.github/workflows/package.yml` creates Windows x86-64, macOS arm64, and Linux
x86-64 preview artifacts. It deliberately does not create a public GitHub
release, sign binaries, or upload signing credentials. A maintainer must review
the CI run, attach the SBOM and notices, complete `release-checklist.md`, sign
with institutionally controlled credentials, and then publish the release.

Milestone 0 packages contain only SeqQC-owned code and locked application
dependencies. Candidate aligners and external validation tools are not bundled.

