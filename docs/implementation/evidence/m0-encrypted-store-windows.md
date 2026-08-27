# Milestone 0 encrypted-store evidence: Windows

**Evidence date:** 2026-08-26  
**Host:** Windows x86-64, MSVC toolchain  
**Scope:** local encrypted metadata store spike; not production key management

## Outcome

The `seqqc-security` crate builds a statically linked SQLCipher store behind an
application-owned Rust API. The Windows spike passes five tests covering
encrypted create/reopen, wrong-key rejection, transactional rollback,
plaintext artifact scanning, and deliberate page tampering.

This advances ADR 0004 but does not accept it. macOS/Linux builds, random
per-project keys, OS key-store integration, rekey, forced-process crash
recovery, backup/restore, migration, and portable-archive cryptography remain
open gates.

## Pinned implementation boundary

| Component | Pinned evidence version | Role |
| --- | --- | --- |
| `rusqlite` | 0.40.2 | Rust database binding |
| `libsqlite3-sys` | 0.38.2 | Bundled native build boundary |
| SQLCipher Community | 4.14.0 | Encrypted SQLite implementation |
| `openssl-src` | 300.6.1+3.6.3 | Vendored OpenSSL source wrapper |
| OpenSSL | 3.6.3 | SQLCipher cryptographic provider |

The exact Cargo feature is `bundled-sqlcipher-vendored-openssl`. The native
libraries are compiled and statically linked; a system OpenSSL installation is
not a runtime dependency. The Windows source build required Perl for OpenSSL's
configure phase. Strawberry Perl 5.42.2.1 was used on this evidence host. The
application still performs no runtime network access.

The crate is intentionally a facade: public SeqQC types contain no
`rusqlite`, SQLite, or SQLCipher types. This keeps provider replacement and
regulated builds possible without changing callers.

## Security behavior exercised

| Behavior | Evidence |
| --- | --- |
| Key order | `PRAGMA key` is the first database operation after open. |
| Unlock proof | `SELECT count(*) FROM sqlite_master` forces an immediate page read. |
| Cipher proof | `cipher_version`, `cipher_provider`, and `cipher_status` must all return valid values. |
| Wrong secret | Rejected as the normalized `UnlockFailed` error; the supplied secret and raw database error are absent from the diagnostic. |
| At-rest inspection | Main database, WAL, and SHM files present during the test are scanned for the SQLite header, test passphrase, and synthetic clinical marker. None occur in plaintext. |
| Transaction atomicity | A two-write batch with a deliberate constraint failure rolls back its earlier write; absence is confirmed again after reopen. |
| Recovery path | Committed WAL content is checkpointed and successfully reopened; rollback state is also preserved after close/reopen. |
| Tampering | One bit is flipped near the end of a multi-page database; `cipher_integrity_check` returns one or more findings. |
| Overwrite safety | Create refuses an existing store and rejects an empty secret. |

Test command:

```powershell
& .\tools\invoke-rust.ps1 test --locked --package seqqc-security
```

Result:

```text
running 5 tests
5 passed; 0 failed
```

## Findings and limitations

- This spike passes a human test passphrase to SQLCipher's documented key API.
  That is test scaffolding, not the proposed production key hierarchy. The
  production design still calls for a random per-project database key protected
  by the operating-system key store.
- SQLCipher Community is not a FIPS-validated cryptographic module. A regulated
  deployment requiring FIPS must use an approved provider/build and undergo
  organizational validation; this result must not be represented as clinical
  certification.
- SQLCipher's optional `cipher_memory_security=ON` mode was evaluated and is
  not enabled in this build. On this Windows host it emitted `VirtualLock`
  error 1453 and caused repeat/parallel test stack overflows. SQLCipher's
  default sanitization of internal cryptographic allocations remains active.
  This decision must be repeated on macOS/Linux and revisited for an approved
  regulated build.
- The vendored OpenSSL debug link emits missing `ossl_static.pdb` warnings.
  Runtime tests pass, but release-symbol and reproducible-build policy is still
  open.
- Clean reopen and transaction rollback are proven. Abrupt process termination,
  disk-full behavior, torn-write simulation, backup/restore, schema migration,
  and stale writer-lock recovery are not yet proven.
- Searching synthetic artifacts for known plaintext is a useful regression
  check, not a cryptographic proof or independent security audit.

## Primary references

- [SQLCipher API](https://www.zetetic.net/sqlcipher/sqlcipher-api/)
- [rusqlite 0.40.2 source manifest and feature definitions](https://docs.rs/crate/rusqlite/0.40.2/source/Cargo.toml.orig)
- [SQLCipher source and releases](https://github.com/sqlcipher/sqlcipher)
