# ADR 0004: Encrypted project storage

- Status: Proposed
- Date: 2026-08-26

## Context

SeqQC must store sensitive metadata locally, support transactional recovery, and
protect projects copied from disk. The exact encryption choice affects all
desktop platforms, backups, key rotation, and regulated-environment guidance.

## Proposed decision

- Store transactional metadata in SQLite through Rust `rusqlite`.
- Evaluate SQLCipher Community Edition as the full-database encryption layer.
- Use a random per-project data key; never use a human password directly as the
  database key.
- Wrap the project key with the OS key store for ordinary local use.
- For portable archives derive a key-encryption key from a password using a
  reviewed memory-hard KDF and authenticated encryption selected in a separate
  crypto-format review.
- Keep large input data external. Encrypt designated sensitive derived artifacts
  or require an encrypted project filesystem according to policy.

## Why this is not accepted yet

Milestone 0 must demonstrate:

- reproducible SQLCipher builds for Windows, macOS, and Linux;
- license and transitive cryptographic-provider approval;
- correct key/rekey behavior and memory/error handling;
- crash-safe transactions, backup, restore, and migration;
- OS key-store integration and documented recovery limitations; and
- independent security review of portable-archive design.

If SQLCipher fails a gate, alternatives include an application-layer encrypted
record store or a different reviewed encrypted database. Plain SQLite plus
unencrypted sensitive columns is not an acceptable fallback.

