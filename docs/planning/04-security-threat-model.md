# Security and offline threat model

## 1. Security objectives

SeqQC Studio must protect confidentiality of sample/project metadata, integrity
of scientific records, availability/recoverability of projects, and
traceability of actions. It must also make the no-internet promise technically
testable.

This is a product threat model, not a certification. Host compromise, malicious
administrators, physical access to an unlocked machine, and compromised
third-party tools remain important environmental risks.

## 2. Assets

- Subject/sample identifiers and clinical/preclinical metadata
- Paths that disclose identifiers or institutional structure
- FASTQ/BAM/CRAM and derived artifacts
- Reference, target, and annotation resources
- Workflow settings and institutional thresholds
- Findings, reports, comments, and adjudications
- Local credentials, encryption keys, and portable-archive passwords
- Tool binaries, validation status, and scheduler profiles
- Audit log, completion manifests, and signatures/checksums

## 3. Trust boundaries

```text
User ↔ desktop UI ↔ trusted SeqQC core ↔ encrypted project store
                           │
                           ├── untrusted input files
                           ├── verified external executables
                           ├── local OS facilities/key store
                           └── local scheduler commands/shared storage
```

The UI, input files, filenames, sample sheets, annotations, third-party tool
output, scheduler output, imported archives, and report destinations are not
trusted merely because they are local.

## 4. Threat actors and failure sources

- Unauthorized local user
- Authorized user making an unsafe or mistaken action
- Malware or compromised host account
- Maliciously crafted sequencing/reference/archive file
- Replaced or compromised third-party executable
- Concurrent writer or stale process
- Misconfigured scheduler/shared filesystem
- Accidental disclosure through logs, exports, screenshots, or temporary files
- Supply-chain compromise during build or packaging
- Silent scientific corruption caused by incompatible resources or versions

## 5. Threats and required controls

| Threat | Required controls |
| --- | --- |
| Internet/data exfiltration | No network-capable runtime features; packaged-only assets; URI rejection; loopback API off by default; network-attempt tests |
| Unauthorized project access | Local authentication/session; OS key store; encrypted sensitive store; inactivity lock; fail-closed unlock |
| Project copied from disk | Encryption at rest for metadata and selected artifacts; portable archive encryption; no cleartext secrets |
| File/path injection | Argument-vector process launch; canonicalized paths; no shell interpolation; allow-listed task definitions |
| Malicious archive | Authenticated encryption; size/count limits; safe extraction; traversal/symlink defense; schema checks |
| Tool replacement | Executable digest, version probe, administrator approval, per-platform validation status, pre-run recheck |
| Tampered result | Atomic commit; artifact hashes; completion manifest; append-only/tamper-evident audit; verification UI |
| Concurrent modification | OS-level project lock plus renewable lease; single writer; read-only fallback; audited stale-lock recovery |
| Crash/power loss | Transactional database; staged artifacts; write-then-fsync-then-rename strategy where supported; recovery journal |
| Sensitive logs | Structured redaction; opaque IDs; classified fields; bounded retention; protected debug export |
| Insecure temporary data | Project-scoped permissions; configurable secure scratch; cleanup manifest; avoid sensitive OS-global temp by default |
| Overprivileged process | Run without elevation; narrow Tauri capabilities; scoped filesystem access; child-process environment allow-list |
| Scheduler impersonation/status confusion | Record scheduler profile, job ID, submission digest, owner, working path; verify returned result envelope |
| Resource exhaustion | Preflight estimates, quotas/limits, concurrency control, archive limits, responsive cancellation |
| Preset/rule manipulation | Versioned signed/import-verified preset packages; role/policy control; visible override history |
| Export disclosure | Audience/privacy preview; de-identification profile; destination confirmation; audit; no silent overwrite |
| Downgrade/migration attack | Signed release artifacts; schema minimums; no silent downgrade; backup and integrity verification |

## 6. Offline boundary

### Prohibited in release builds

- Telemetry, analytics, ads, remote error reporting, and crash uploading
- Automatic update checks or downloads
- Remote fonts, icons, JavaScript, CSS, documentation, or report dependencies
- HTTP(S)/FTP/S3/GS input and reference access
- DNS-based discovery, license activation, and online login
- Embedded browser navigation to external sites
- Package/environment installation during runtime

Documentation may display a plain-text external citation or URL, but the
application must not retrieve it. Opening external links should be absent in
controlled mode and, if ever introduced elsewhere, require an explicit policy
decision outside version 1.

### Local connectivity

- Scheduler commands execute on the same host where SeqQC is running; version 1
  does not use SSH or remote APIs.
- Mounted network storage is permitted because the user placed it in scope.
- The optional API binds only to `127.0.0.1`/`::1`, is disabled by default, and
  must never fall back to a wildcard bind.
- Local inter-process communication should prefer OS-native IPC over TCP where
  it materially reduces exposure.

### Verification

Automated tests run with a functional network and monitor DNS and connection
attempts during install, first launch, every workflow, report viewing, error
handling, and uninstall. The test fails on an attempted connection even when the
connection is blocked.

## 7. Authentication and authorization

### Local principal

Use the authenticated operating-system user as the primary local principal.
SeqQC may require a local application unlock/password for protected projects,
but it must not implement an online account. Secret material is stored in the
OS key store when supported.

### Roles

The one-writer rule is independent of roles. A controlled installation may
define local roles:

- Owner: project lifecycle, run, export, grant local access
- Analyst: configure/run/review under policy
- Reviewer: read, comment, adjudicate, export if permitted
- Administrator: tool, scheduler, security, and institutional preset policy

Version 1 may map these to OS groups/configuration. It must not claim strong
multi-user authorization on a filesystem that cannot enforce it.

### Sessions

- Configurable inactivity timeout with secure default for clinical context
- Explicit lock action
- Clear display of active local principal and project mode
- Reauthentication for key export, protected archive export, and selected
  administrative changes
- No password or key material in logs, command lines, or environment dumps

## 8. Encryption and key management requirements

- Use reviewed, modern authenticated encryption through a maintained library.
- Never invent cryptographic primitives or protocols.
- Separate data-encryption keys from password/OS-key-store wrapping keys.
- Use a memory-hard password KDF for portable archives with versioned parameters.
- Generate randomness through the operating system.
- Support key rotation and recovery policy before claiming regulated readiness.
- Clear sensitive buffers where the runtime/library provides a reliable method.
- Backups must not contain unencrypted project keys.
- Loss of an unrecoverable password/key must be explained before encryption is
  enabled; SeqQC cannot recover it through an internet service.

Exact algorithms and formats require a security architecture decision record
and independent review before implementation.

## 9. Project locking

The lock record contains project ID, opaque local principal ID, process ID,
host ID, application version, start time, and renewable lease time. The OS lock
is authoritative while supported; the record helps diagnose network filesystems.

Stale-lock recovery must:

1. show the recorded owner/host/time without exposing unnecessary identity;
2. confirm the previous writer is not active where determinable;
3. make a recovery snapshot;
4. require explicit user confirmation; and
5. append an audit event.

Network filesystems with unreliable locks are either read-only or unsupported
for active projects under controlled policy. Users may execute against inputs on
such storage while keeping the project database on a reliable local/shared
volume.

## 10. Audit requirements

Audit events include authentication, project unlock/lock, configuration and
preset changes, preflight, execution, retry/cancel, result commit, annotation,
report/export, archive/import, tool approval, migration, and recovery.

Audit logs must be append-only through the application and tamper-evident.
Events record actor, action, target, outcome, timestamp, prior-event digest, and
bounded non-sensitive detail. Administrators can verify and export the chain.

## 11. Clinical-context controls

- Privacy-first export defaults
- Stronger inactivity timeout and full input fingerprint policy
- Required audit/provenance checks before report finalization
- Locked approved tools, workflows, references, and presets where configured
- Clear “validated by organization” metadata separate from SeqQC support status
- Amendment workflow rather than editing completed evidence
- Electronic-signature capability is out of scope until legal/technical
  requirements are separately specified

## 12. Secure development and release

- Threat-model review at each milestone
- Dependency lockfiles, SBOM, license inventory, vulnerability review
- Reproducible or provenance-attested builds where feasible
- Signed commits/tags policy for releases and signed installers/packages
- Static analysis, secret scanning, fuzzing of parsers, archive and sample-sheet
  tests, dependency review, and code review
- Private vulnerability-reporting instructions in `SECURITY.md`
- No real patient data in source, tests, issues, telemetry, or crash artifacts

## 13. Security release gates

- Independent review of cryptography and archive design
- No critical/high unresolved findings in the defined threat model
- Successful no-network suite
- Successful malicious input/archive corpus tests
- Verified least-privilege installer/runtime behavior
- Recovery, key loss, backup/restore, lock, and audit verification tests
- Documented residual risks and administrator hardening guide

