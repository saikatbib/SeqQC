# Milestone 1 app review

Review date: 2026-10-31. Scope: native desktop and its shared project/security
boundaries. The retained Tauri UI is a reference implementation.

## Findings and implementation

| Finding | Change |
| --- | --- |
| Sample review stopped at the first 250 groups | Searchable pages expose every matching group, including the last page of 10,000 samples |
| Scan results could arrive after project close/switch | Drop the result channel and cooperatively cancel traversal when replacing the project or input context |
| Unsaved project drafts remained after session lock | Clear draft names, paths, search filters, references, and simulator state |
| Failed input metadata writes still started a scan | Start discovery only after successful persistence |
| Opening the current project conflicted with its own lock | Treat reopening the same directory as navigation |
| Recovery existed only as a core method | Guided project/profile recovery retains evidence, requires explicit confirmation and a 15-minute minimum age |
| Age-only recovery could displace a live writer | Hold an OS file lease throughout the writer lifetime; recovery cannot take that lease from an active current-build writer |
| Readiness was only an overview label | Add a dedicated screen, built-in tool inventory, encrypted FASTA/GTF/GFF/BED path registry, background accessibility checks, and a path-free text report |
| No simulated local workflow | Add queue/run/complete/cancel states, encrypted state persistence, audit transitions, and interrupted-work identification on reopen |
| Discovery benchmark represented 5,000 samples | Expand to 10,000 paired samples (20,000 files) |

Reference accessibility is an operational check, not scientific validation of
format, reference identity, annotation compatibility, or target coordinates.
The tool inventory currently describes built-in providers; external provider
installation, version/checksum approval, and actual QC execution remain future
implementation work. Simulation produces no biological findings or reports.

## Local verification

Regression coverage includes worker cancellation, lock-time draft clearing,
10,000-sample egui rendering in all themes, reference registry persistence,
interrupted task persistence, supported database migrations, future-schema
rejection, active-writer recovery rejection, and stale-evidence preservation.

| Check | Result |
| --- | --- |
| `cargo test --workspace --locked` via `tools/invoke-rust.ps1` | 45 passed, 0 failed, 1 ignored interactive Windows Credential Manager test |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all --check` | Passed |
| `node tools/check-all.mjs` | 9 schemas, 8 valid examples, 4 invalid examples, 128 metric assignments, 30 dependency candidates, 2 desktop capability permissions |
| `node --test` | 3 passed |
| `cargo bench --package seqqc-discovery --bench discovery_10000 --locked` | 20,000 files, 10,000 paired groups; discovery 142.756 ms on this Windows host (fixture creation excluded) |
| Native release build | Passed |
| Updated Windows startup observer, 10 seconds | Process remained running; no TCP/UDP endpoints observed in its process family |

The observer now rejects early process exit instead of claiming the full
interval from a partial observation. Endpoint polling cannot exclude brief
connection attempts between samples and is not packet-level offline assurance.

Executable: `target/release/seqqc-native-desktop.exe`.
SHA-256: `927D0E5DFC7982B122030662A77DCFF9BDE60DABC1C339911CD31B50F7B1097F`.

The initial Windows desktop capture helper timed out twice. The follow-up
qualification uses native framebuffer captures instead. Neither headless egui
tests nor framebuffer images substitute for representative-user testing.

## Follow-up qualification

The [qualification protocol](m1-qualification-protocol.md) and CI matrix now
cover synthetic rendered scenes in all themes at 100%/200% scale and runtime
network observation on Windows, macOS, and Linux. The qualification feature is
off by default and its fixtures cannot use an unmarked user-profile directory.
The qualification binary must not be distributed as the production app.

Live Windows UI Automation inspection succeeded without desktop screenshots.
Tab reached the theme controls. The initial review found an unnamed display-name
field, insufficient light-theme semantic contrast, overlapping header controls
at 200%, and a locked screen with unreachable lower controls. Changes add
explicit field labels, theme-specific semantic and selection colors, palette
contrast regression checks, wrapping header controls, explicit text-size
options, compact navigation, a scrollable lock screen, wrapped input paths, and
horizontal table scrolling. Native captures are inspected after each iteration.

`config/provider-approvals.json` records all eight external candidates as pending.
The SAMtools/fastp upstream provenance review is prepared; no binary execution or
redistribution is approved. Policy tests reject enabling unapproved artifacts.
Independent reviewer names and representative-user observations remain unfilled
in the protocol, rather than being inferred from automated testing.

## Milestone acceptance still pending

Milestone 1 is **not release-qualified** solely by implementation and unit tests.

- Clean-host Windows, macOS, and Linux installation and offline workflow evidence,
  including failure handling and recovery. The new startup observation above
  covers this binary only; it does not qualify full workflows or other platforms.
- Representative network-mount lock semantics and crash behavior. A filesystem
  that cannot support the OS lease must fail closed. Older binaries do not hold
  the new OS lease, so close all versions before recovery.
- Formative sample-mapping usability evidence against the serious-use-error
  threshold; keyboard, screen-reader, scaling, and contrast review.
- Security review of recovery, migrations, and the existing encrypted-store design.
- External tool provider registry/version/checksum approval when providers are
  introduced; the foundation inventory must not imply those providers exist.

Portable encrypted archives remain Milestone 6 scope, and biological metrics
remain Milestone 2 and later scope. Neither blocks this foundation implementation
by itself, and neither is represented as delivered here.
