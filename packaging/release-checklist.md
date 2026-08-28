# Release checklist

## Required before a preview tag

- [ ] Version agrees in root `package.json`, desktop `package.json`, Cargo
  workspace, changelog, and citation metadata. Tauri package metadata maps the
  textual prerelease label to its Windows-safe numeric package version.
- [ ] Repository policy, Node tests, Rust workspace tests, rustfmt, strict
  Clippy, frontend production build, and three-OS package workflow pass.
- [ ] Generated CycloneDX SBOM is reviewed and attached.
- [ ] `THIRD_PARTY_NOTICES.md` and required dependency license texts are
  included and redistribution candidates are approved.
- [ ] No patient data, real sequencing data, keys, credentials, or unlicensed
  assets are present.
- [ ] Packaged binaries pass startup, folder selection, synthetic discovery,
  inspection, five-format export, and no-network observation on each OS.
- [ ] Known limitations and investigational/clinical disclaimers are visible.
- [ ] Installers are signed with maintainer-controlled platform credentials, or
  the release is explicitly marked unsigned and unsuitable for deployment.
- [ ] SHA-256 checksums are published for every artifact.
- [ ] A rollback/archive plan and responsible release owner are recorded.

## Change control

Schema, scientific method, threshold, cryptography, runtime capability, bundled
binary, and signing changes require a linked ADR or review record. A release
owner cannot be the sole approver for a high-risk cryptographic or scientific
validation decision.
