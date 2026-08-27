# ADR 0005: Analysis process boundary

- Status: Accepted
- Date: 2026-08-26

## Context

Third-party tools and user-controlled paths are important attack and reliability
boundaries. A general shell or concatenated command string would make safe path
handling and auditability difficult.

## Decision

- The React UI never invokes a tool.
- The Rust core resolves a versioned workflow task to an approved typed adapter.
- Launch executables with an argument vector and restricted environment; do not
  interpolate user data into shell commands.
- A tool manifest pins executable identity, supported versions, checksum, output
  parser, platform, license status, and validation status.
- Use project-scoped working/scratch directories and staged outputs.
- Normalize stdout/stderr into bounded redacted logs and stable events.
- Commit outputs only after schema, completeness, and checksum verification.
- Scheduler scripts serialize an already-resolved task package; scheduler
  directives remain distinct from scientific arguments.

## Consequences

- Arbitrary command plugins are out of scope for version 1.
- A new tool requires an adapter, fixtures, parser tests, license review, and
  scientific validation.
- Direct Tauri shell permission is not exposed to the frontend.

