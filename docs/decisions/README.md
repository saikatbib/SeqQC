# Architecture decision records

Architecture decision records (ADRs) capture choices that constrain SeqQC
Studio implementation. Accepted records may be superseded only by a later ADR;
they are not edited to hide a changed decision.

| ADR | Status | Decision |
| --- | --- | --- |
| [0001](0001-desktop-and-core-stack.md) | Superseded | Tauri 2 + React/TypeScript prototype + Rust core |
| [0002](0002-offline-capability-boundary.md) | Accepted | No runtime network capability |
| [0003](0003-versioned-json-contracts.md) | Accepted | JSON Schema Draft 2020-12 contracts |
| [0004](0004-encrypted-project-storage.md) | Proposed | SQLCipher metadata and password-wrapped profile/project key-vault evaluation |
| [0005](0005-analysis-process-boundary.md) | Accepted | Typed adapters and argument-vector execution |
| [0006](0006-linux-only-alignment.md) | Accepted | Initial aligner execution is Linux-only |
| [0007](0007-native-first-qc-engine.md) | Accepted | Portable native QC with independent references |
| [0008](0008-canonical-report-model.md) | Accepted | All exports derive from one report model |
| [0009](0009-strict-offline-desktop-shell.md) | Accepted | Production shell may not depend on an uncontained system webview |
| [0010](0010-native-egui-desktop-shell.md) | Accepted for Milestone 1 spike | Evaluate a native-only egui presentation shell |

## Status definitions

- **Proposed:** Evidence or review is still required before implementation is
  allowed to depend on the choice.
- **Accepted:** The decision is approved for implementation.
- **Superseded:** A later ADR replaces the decision.
- **Rejected:** The option was considered and intentionally not selected.
