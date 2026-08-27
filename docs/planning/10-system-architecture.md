# System architecture

## 1. Recommended technical direction

Use a Tauri 2 desktop shell, a TypeScript component interface, and a Rust core
for project control, workflow orchestration, security-sensitive operations, and
local inter-process communication. Scientific calculations may be implemented
in Rust or delegated to pinned, validated external tools through typed adapters.

This direction is preferred over embedding a general-purpose web server:

- the product remains a desktop application;
- OS webviews can render a modern professional interface;
- the shell can enforce a small capability surface;
- Rust is suitable for file discovery, checksumming, process isolation, and
  large-file streaming; and
- third-party analysis remains replaceable and independently validated.

This is an architectural recommendation, not permission to scaffold code before
the planning gates in the roadmap are approved.

## 2. Logical components

```text
SeqQC Studio desktop process
├── Presentation
│   ├── project wizard
│   ├── run monitor
│   ├── dashboards and explanations
│   └── report/export builder
├── Application services
│   ├── local authentication and session
│   ├── project lifecycle and single-writer lock
│   ├── reference/tool registry
│   ├── preset and policy registry
│   └── audit service
├── SeqQC engine
│   ├── discovery and preflight
│   ├── workflow planner and state machine
│   ├── scheduler adapters
│   ├── metric normalization
│   ├── interpretation rules
│   └── report model and exporters
├── Analysis workers
│   ├── native SeqQC modules
│   └── verified third-party executables
└── Storage
    ├── encrypted metadata database
    ├── content-addressed manifests
    ├── staged/committed run artifacts
    └── encrypted portable archives
```

## 3. Process boundaries

- The UI process must not execute arbitrary tools directly.
- The core validates requests and converts them to a versioned workflow graph.
- Each task launches with an argument vector, restricted environment, explicit
  working directory, resource limits where supported, and captured standard
  output/error.
- Parsing happens through a tool adapter that converts output to the normalized
  metric schema.
- A task result is committed only after schema, checksum, and completeness
  checks pass.
- HPC tasks use the same serialized task contract and return signed/checksummed
  result envelopes through shared storage.

## 4. Storage layout

The project is a directory during active use and an encrypted archive only for
transport. A proposed logical layout is:

```text
project.seqc/
├── project.db                 encrypted metadata, state, findings, audit
├── manifest.json              non-sensitive format/version locator
├── lock/                      single-writer lease information
├── runs/<run-id>/
│   ├── plan.json
│   ├── state.json
│   ├── inputs.manifest.json
│   ├── metrics/
│   ├── artifacts/
│   ├── logs/
│   └── completion.manifest.json
├── reports/<report-id>/
├── cache/                     disposable derived data
└── recovery/                  migration and transaction recovery
```

Large input files remain external and are referenced by normalized path plus
identity evidence. A project must report when an external file has moved or
changed. Importing/copying inputs into the project is not a default behavior.

## 5. State machines

### Run state

`draft → preflighted → queued → running → verifying → completed`

Exceptional terminal states are `cancelled` and `failed`. `interrupted` is a
recoverable state and does not imply scientific failure. Completed records are
immutable; reanalysis creates a new run.

### Task state

`pending → ready → submitted → running → staged → verified → committed`

Retries create attempts under one task identity. The successful attempt and all
failed attempt metadata remain auditable.

## 6. Interfaces

| Interface | Contract |
| --- | --- |
| UI ↔ core | Typed commands/events; no direct database access |
| Core ↔ module | Versioned module manifest, input contract, metric schema |
| Core ↔ external tool | Pinned adapter with executable fingerprint and parser version |
| Core ↔ scheduler | Scheduler-neutral job and status contract |
| Metric ↔ interpretation | Stable metric IDs, units, applicability, evidence quality |
| Report ↔ result store | Read-only versioned report model |
| Local API ↔ core | Disabled-by-default, loopback-only, authenticated contract |

## 7. Cross-platform strategy

- Build and sign each desktop target on its native operating system.
- Maintain a platform/tool capability matrix; do not pretend a missing external
  binary is available.
- Prefer native, deterministic parsers for common validation and metadata tasks.
- Use per-platform verified tool bundles where legal and scientifically
  equivalent.
- Restrict initial-release aligner tasks to validated Linux providers. Continue
  to support QC and report review through native Windows/macOS/Linux builds.
- The UI must explain execution availability before project configuration is
  committed.

Supporting a Windows or macOS UI does not justify silently running an aligner
through a compatibility layer. Any future WSL/container execution mode would be
a separately declared and validated execution provider.

## 8. Offline enforcement

- Do not include updater, telemetry, analytics, crash-upload, remote font, CDN,
  or URL-fetch capabilities in release builds.
- Apply a content-security policy that permits packaged assets only.
- Treat HTTP(S), FTP, S3, GS, and other remote input URIs as unsupported.
- Keep the local API feature absent or inert until explicitly enabled.
- Test with network access available and assert that no connection is attempted;
  testing only in an air-gapped VM would miss attempted connections.

## 9. Extensibility boundary

Version 1 uses built-in signed/approved modules rather than arbitrary user code.
A future plugin SDK may expose declarative modules, but clinical-mode projects
must be able to restrict execution to an allow-list of module IDs, versions,
checksums, and validation statuses.

## 10. Architecture decisions still requiring evidence

The following are implementation investigations, not new product questions:

- encrypted database library and key-rotation behavior;
- UI framework choice within the TypeScript layer;
- exact portable-archive cryptographic format;
- native versus external implementation for each metric;
- supported CPU architectures per operating system;
- QC tool equivalence across platform builds and alignment equivalence across
  supported Linux providers; and
- whether PDF rendering uses a bundled offline renderer or print subsystem.

Each choice must receive a decision record, prototype evidence, and the relevant
security/licensing review.
