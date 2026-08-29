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
- Use a random local-profile key, wrapped by a local application passphrase
  through a versioned Argon2id + XChaCha20-Poly1305 envelope.
- Store independent random project keys only inside the SQLCipher-encrypted
  profile key vault. A human password is never a project database key.
- Treat the OS key store as an optional future convenience, not a required
  runtime service: desktop-less and service sessions may not expose one.
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
- profile-key wrapping, key rotation/recovery, documented limitations, and
  optional OS key-store integration; and
- independent security review of portable-archive design.

## Engineering-preview evidence (2026-08-28)

The `0.1.0-alpha.3` implementation adds the cross-platform profile-vault path:

- Argon2id v19 derives a 256-bit wrapping key using 65,536 KiB, three
  iterations, and one lane.
- XChaCha20-Poly1305 wraps the random 256-bit profile key using a fresh OS-random
  192-bit nonce and fixed format-specific associated data.
- The encrypted profile holds random 256-bit per-project keys. Project names,
  contexts, input paths, and recent-project entries remain inside SQLCipher.
- A cleartext outer project manifest contains only the format version, random
  project ID, crypto profile, opaque key locator, and creating app version.
- Password-envelope tampering, wrong-password rejection, encrypted key-vault
  round trips, SQLCipher integrity, one-writer locking, and persistent audit
  verification have automated tests.

A real Windows Credential Manager probe returned `NoStorageAccess` in a
non-interactive logon session. That result is why the OS key store is not a hard
dependency for the cross-platform application.

The ADR remains Proposed. The envelope format and parameters require an
independent cryptographic review, macOS/Linux runtime evidence, recovery and key
rotation design, and portable-archive review before a regulated-readiness claim.

If SQLCipher fails a gate, alternatives include an application-layer encrypted
record store or a different reviewed encrypted database. Plain SQLite plus
unencrypted sensitive columns is not an acceptable fallback.
