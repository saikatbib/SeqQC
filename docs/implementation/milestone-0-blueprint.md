# Milestone 0 implementation blueprint

## Objective

Retire the highest architectural, security, platform, scientific-contract, and
licensing risks before production feature development. Milestone 0 produces
evidence and executable spikes, not validated sequencing claims.

## Work packages

### M0.1 Architecture and repository

- Accept/review ADRs under `docs/decisions/`.
- Establish workspace boundaries for desktop, core, scientific modules,
  contracts, tools, fixtures, packaging, and documentation.
- Pin Rust, Node, package-manager, and platform toolchains.
- Define contribution, review, release-signing, and change-control policy.

### M0.2 Contracts

- Draft JSON Schema 2020-12 contracts for project, artifact, workflow, metric,
  finding, completion manifest, and report.
- Add valid/invalid examples and offline reference validation.
- Specify semantic-version compatibility and migration rules.

### M0.3 Scientific method assignment

- Assign all metric registry entries to a primary implementation family.
- Name independent validation references and target milestone.
- Resolve the first supported core subset and mark the rest conditional or
  investigational until evidence exists.

### M0.4 Security spikes

- Build encrypted transaction/recovery prototype on each OS.
- Test OS key stores, project locks, archive candidate, temporary-file policy,
  and audit hash chain.
- Implement network-attempt harness for the empty packaged app.
- Threat-model review and crypto design review.

### M0.5 File and scale spikes

- Stream FASTQ and BAM/CRAM fixtures without loading whole files.
- Measure discovery and sample grouping on 10,000 synthetic file entries.
- Prototype path handling for Unicode, spaces, leading dashes, network mappings,
  and symlinks.
- Evaluate `noodles` correctness/performance and independently compare outputs.

### M0.6 Desktop and reporting spikes

- Package an empty Tauri app on three operating systems with minimum capability
  permissions and packaged-only CSP.
- Exercise Guided/Standard/Expert mode and theme/accessibility baseline.
- Render one canonical synthetic report to dashboard, HTML, PDF, JSON, CSV, TSV.

### M0.7 Linux execution spike

- Detect validated Linux provider and reject alignment elsewhere.
- Prototype typed sidecar/task launch without frontend shell permission.
- Evaluate BWA-MEM2 and STAR versions/builds/licenses/resources.
- Prove local and one scheduler-neutral job-package round trip using synthetic
  tasks; no biological production claim.

## Proposed repository topology

```text
SeqQC/
├── apps/
│   └── desktop/                 Tauri + React/TypeScript
├── crates/
│   ├── seqqc-contracts/         generated/validated Rust contract types
│   ├── seqqc-core/              project and application services
│   ├── seqqc-discovery/         file/sample discovery
│   ├── seqqc-engine/            workflows and task state
│   ├── seqqc-formats/           validated sequencing I/O facade
│   ├── seqqc-metrics/           normalized scientific methods
│   ├── seqqc-interpretation/    rules and findings
│   ├── seqqc-execution/         local and scheduler adapters
│   ├── seqqc-reporting/         canonical report model/exporters
│   └── seqqc-security/          keys, storage, audit, archive
├── schemas/v1/                  normative JSON contracts
├── config/                      planning registries and policies
├── prototypes/                  disposable Milestone 0 evidence spikes
├── test-data/                   synthetic non-sensitive fixtures
├── tools/                       repository validation tooling
├── docs/
├── packaging/
└── .github/workflows/           CI only after toolchain/scaffold approval
```

## Toolchain baseline to provision

- Rust stable pinned by `rust-toolchain.toml`
- Tauri 2 CLI and Rust crates pinned by lockfiles
- React 19 + TypeScript with exact package lock
- Node active LTS selected at scaffold time; Node is build-only
- Native platform build prerequisites documented per Tauri
- No runtime package manager or dependency resolution

The Windows development workstation now uses an ignored workspace-local Rust
1.98.0 MSVC toolchain plus Visual Studio 2022 C++ Build Tools. The repository
retains the toolchain manifest and exact Cargo/pnpm lockfiles; it does not retain
or redistribute the local compiler installation. macOS and Linux clean-build
evidence remains required.

## Evidence ledger

Each spike records hypothesis, fixture/tool versions, platform, commands,
results, limitations, decision affected, and retained/discarded code. Prototype
code is not promoted to production by copying it; the corresponding ADR must
first be accepted and production tests defined.

## Exit checklist

- Accepted ADRs for all choices production scaffold depends on
- Encryption ADR accepted or a replacement chosen
- All 128 metric IDs assigned and registry consistency checks passing
- Contract schemas and examples pass offline validation
- Linux alignment candidates and provider matrix approved for implementation
- Empty desktop packaging/no-network evidence on three OS targets
- Large-file/10,000-sample spike evidence meets targets
- Threat-model and dependency/license reviews have named approvers
- Milestone 1 backlog and acceptance tests are ready
