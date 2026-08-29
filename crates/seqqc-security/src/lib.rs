//! Encrypted local project storage for `SeqQC`.
//!
//! This crate owns the application-facing persistence boundary. `SQLCipher` and
//! SQLite types do not cross that boundary.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

const SCHEMA_VERSION: i64 = 3;
const PROJECT_KEY_BYTES: usize = 32;
const PROFILE_SALT_BYTES: usize = 16;
const XCHACHA_NONCE_BYTES: usize = 24;
const ARGON2_MEMORY_KIB: u32 = 65_536;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_LANES: u32 = 1;
const PROFILE_KEY_AAD: &[u8] = b"org.seqqc.profile-key.v1";
const LOCK_FILE_NAME: &str = "writer.lock";

/// Random database secret owned by the application and zeroized on drop.
pub struct ProjectKey([u8; PROJECT_KEY_BYTES]);

impl ProjectKey {
    /// Generates a project key from the operating system random source.
    ///
    /// # Errors
    ///
    /// Returns an error when the operating system random source is unavailable.
    pub fn generate() -> Result<Self, KeyStoreError> {
        let mut bytes = [0_u8; PROJECT_KEY_BYTES];
        getrandom::fill(&mut bytes).map_err(|_| KeyStoreError::RandomSourceUnavailable)?;
        Ok(Self(bytes))
    }

    fn from_secret(secret: &[u8]) -> Result<Self, KeyStoreError> {
        let bytes: [u8; PROJECT_KEY_BYTES] = secret
            .try_into()
            .map_err(|_| KeyStoreError::InvalidStoredKey)?;
        Ok(Self(bytes))
    }

    fn sqlcipher_secret(&self) -> Zeroizing<String> {
        Zeroizing::new(hex::encode(self.0))
    }

    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for ProjectKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProjectKey([REDACTED])")
    }
}

impl Drop for ProjectKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Versioned password-wrapped key material suitable for a non-sensitive outer
/// profile envelope. The salt and nonce are public; ciphertext is authenticated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PasswordKeyEnvelope {
    pub format: String,
    pub kdf: String,
    pub memory_kib: u32,
    pub iterations: u32,
    pub lanes: u32,
    pub salt_hex: String,
    pub cipher: String,
    pub nonce_hex: String,
    pub ciphertext_hex: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum KeyEnvelopeError {
    EmptyPassphrase,
    RandomSourceUnavailable,
    InvalidEnvelope,
    DerivationFailed,
    AuthenticationFailed,
}

impl fmt::Display for KeyEnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPassphrase => formatter.write_str("local passphrase cannot be empty"),
            Self::RandomSourceUnavailable => {
                formatter.write_str("operating system random source is unavailable")
            }
            Self::InvalidEnvelope => formatter.write_str("profile key envelope is invalid"),
            Self::DerivationFailed => formatter.write_str("profile key derivation failed"),
            Self::AuthenticationFailed => formatter
                .write_str("profile key could not be unlocked with the supplied local passphrase"),
        }
    }
}

impl std::error::Error for KeyEnvelopeError {}

impl ProjectKey {
    /// Wraps a random key using Argon2id-derived key material and
    /// XChaCha20-Poly1305 authenticated encryption.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty passphrase, unavailable OS randomness,
    /// failed key derivation, or authenticated-encryption failure.
    pub fn wrap_with_passphrase(
        &self,
        passphrase: &str,
    ) -> Result<PasswordKeyEnvelope, KeyEnvelopeError> {
        if passphrase.is_empty() {
            return Err(KeyEnvelopeError::EmptyPassphrase);
        }
        let mut salt = [0_u8; PROFILE_SALT_BYTES];
        let mut nonce = [0_u8; XCHACHA_NONCE_BYTES];
        getrandom::fill(&mut salt)
            .and_then(|()| getrandom::fill(&mut nonce))
            .map_err(|_| KeyEnvelopeError::RandomSourceUnavailable)?;
        let mut wrapping_key = Zeroizing::new([0_u8; PROJECT_KEY_BYTES]);
        derive_wrapping_key(passphrase, &salt, &mut wrapping_key)?;
        let cipher = XChaCha20Poly1305::new_from_slice(wrapping_key.as_ref())
            .map_err(|_| KeyEnvelopeError::DerivationFailed)?;
        let nonce =
            XNonce::try_from(nonce.as_slice()).map_err(|_| KeyEnvelopeError::InvalidEnvelope)?;
        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: self.as_bytes(),
                    aad: PROFILE_KEY_AAD,
                },
            )
            .map_err(|_| KeyEnvelopeError::AuthenticationFailed)?;
        Ok(PasswordKeyEnvelope {
            format: "org.seqqc.password-wrapped-key.v1".to_owned(),
            kdf: "argon2id-v19".to_owned(),
            memory_kib: ARGON2_MEMORY_KIB,
            iterations: ARGON2_ITERATIONS,
            lanes: ARGON2_LANES,
            salt_hex: hex::encode(salt),
            cipher: "xchacha20-poly1305".to_owned(),
            nonce_hex: hex::encode(nonce),
            ciphertext_hex: hex::encode(ciphertext),
        })
    }

    /// Unwraps a key after strictly validating the supported cryptographic
    /// profile. Authentication errors deliberately do not distinguish a wrong
    /// passphrase from a modified envelope.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported or malformed envelope, empty or
    /// incorrect passphrase, failed derivation, or failed authentication.
    pub fn unwrap_with_passphrase(
        envelope: &PasswordKeyEnvelope,
        passphrase: &str,
    ) -> Result<Self, KeyEnvelopeError> {
        if passphrase.is_empty() {
            return Err(KeyEnvelopeError::EmptyPassphrase);
        }
        if envelope.format != "org.seqqc.password-wrapped-key.v1"
            || envelope.kdf != "argon2id-v19"
            || envelope.memory_kib != ARGON2_MEMORY_KIB
            || envelope.iterations != ARGON2_ITERATIONS
            || envelope.lanes != ARGON2_LANES
            || envelope.cipher != "xchacha20-poly1305"
        {
            return Err(KeyEnvelopeError::InvalidEnvelope);
        }
        let salt =
            hex::decode(&envelope.salt_hex).map_err(|_| KeyEnvelopeError::InvalidEnvelope)?;
        let nonce =
            hex::decode(&envelope.nonce_hex).map_err(|_| KeyEnvelopeError::InvalidEnvelope)?;
        let ciphertext = Zeroizing::new(
            hex::decode(&envelope.ciphertext_hex).map_err(|_| KeyEnvelopeError::InvalidEnvelope)?,
        );
        if salt.len() != PROFILE_SALT_BYTES
            || nonce.len() != XCHACHA_NONCE_BYTES
            || ciphertext.len() != PROJECT_KEY_BYTES + 16
        {
            return Err(KeyEnvelopeError::InvalidEnvelope);
        }
        let mut wrapping_key = Zeroizing::new([0_u8; PROJECT_KEY_BYTES]);
        derive_wrapping_key(passphrase, &salt, &mut wrapping_key)?;
        let cipher = XChaCha20Poly1305::new_from_slice(wrapping_key.as_ref())
            .map_err(|_| KeyEnvelopeError::DerivationFailed)?;
        let nonce =
            XNonce::try_from(nonce.as_slice()).map_err(|_| KeyEnvelopeError::InvalidEnvelope)?;
        let plaintext = Zeroizing::new(
            cipher
                .decrypt(
                    &nonce,
                    Payload {
                        msg: ciphertext.as_ref(),
                        aad: PROFILE_KEY_AAD,
                    },
                )
                .map_err(|_| KeyEnvelopeError::AuthenticationFailed)?,
        );
        Self::from_secret(&plaintext).map_err(|_| KeyEnvelopeError::AuthenticationFailed)
    }
}

/// Platform-native credential-store adapter.
#[derive(Clone, Debug)]
pub struct NativeKeyStore {
    service: String,
}

impl NativeKeyStore {
    #[must_use]
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    /// Stores a project key under one non-secret project identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform credential store is unavailable or
    /// refuses the write.
    pub fn store(&self, project_id: &str, key: &ProjectKey) -> Result<(), KeyStoreError> {
        validate_project_id(project_id)?;
        keyring::Entry::new(&self.service, project_id)?
            .set_secret(key.as_bytes())
            .map_err(KeyStoreError::from)
    }

    /// Loads a project key from the platform credential store.
    ///
    /// # Errors
    ///
    /// Returns an error when the identifier is invalid, the credential is
    /// missing or inaccessible, or its stored length is invalid.
    pub fn load(&self, project_id: &str) -> Result<ProjectKey, KeyStoreError> {
        validate_project_id(project_id)?;
        let mut secret = keyring::Entry::new(&self.service, project_id)?.get_secret()?;
        let result = ProjectKey::from_secret(&secret);
        secret.zeroize();
        result
    }

    /// Removes a project key from the platform credential store.
    ///
    /// # Errors
    ///
    /// Returns an error when the identifier is invalid or deletion fails.
    pub fn delete(&self, project_id: &str) -> Result<(), KeyStoreError> {
        validate_project_id(project_id)?;
        keyring::Entry::new(&self.service, project_id)?
            .delete_credential()
            .map_err(KeyStoreError::from)
    }
}

#[derive(Debug)]
pub enum KeyStoreError {
    InvalidProjectId,
    RandomSourceUnavailable,
    InvalidStoredKey,
    Platform(keyring::Error),
}

impl fmt::Display for KeyStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProjectId => formatter.write_str("project key identifier is invalid"),
            Self::RandomSourceUnavailable => {
                formatter.write_str("operating system random source is unavailable")
            }
            Self::InvalidStoredKey => formatter.write_str("stored project key has invalid length"),
            Self::Platform(_) => formatter.write_str("platform credential store operation failed"),
        }
    }
}

impl std::error::Error for KeyStoreError {}

impl From<keyring::Error> for KeyStoreError {
    fn from(error: keyring::Error) -> Self {
        Self::Platform(error)
    }
}

/// Evidence retained for an existing writer lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockEvidence {
    pub process_id: u32,
    pub created_unix_millis: u128,
    pub token: String,
}

/// Exclusive single-writer lease for one project directory.
pub struct ProjectWriterLock {
    path: PathBuf,
    evidence: LockEvidence,
}

impl fmt::Debug for ProjectWriterLock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProjectWriterLock")
            .field("path", &self.path)
            .field("process_id", &self.evidence.process_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum ProjectLockError {
    AlreadyLocked(LockEvidence),
    InvalidLock,
    NotStale,
    Io(io::Error),
}

impl fmt::Display for ProjectLockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyLocked(evidence) => write!(
                formatter,
                "project is already locked by process {}",
                evidence.process_id
            ),
            Self::InvalidLock => formatter.write_str("project writer lock is malformed"),
            Self::NotStale => {
                formatter.write_str("project writer lock is not old enough to recover")
            }
            Self::Io(error) => write!(formatter, "project writer lock I/O error: {error}"),
        }
    }
}

impl std::error::Error for ProjectLockError {}

impl From<io::Error> for ProjectLockError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl ProjectWriterLock {
    /// Acquires a new exclusive writer lock using atomic file creation.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectLockError::AlreadyLocked`] when another lease exists,
    /// or an I/O/random-source error when the lock cannot be created.
    pub fn acquire(project_directory: impl AsRef<Path>) -> Result<Self, ProjectLockError> {
        let lock_directory = project_directory.as_ref().join("lock");
        fs::create_dir_all(&lock_directory)?;
        let path = lock_directory.join(LOCK_FILE_NAME);
        let mut token_bytes = [0_u8; 16];
        getrandom::fill(&mut token_bytes).map_err(|_| {
            ProjectLockError::Io(io::Error::other(
                "operating system random source unavailable",
            ))
        })?;
        let evidence = LockEvidence {
            process_id: std::process::id(),
            created_unix_millis: unix_millis(),
            token: hex::encode(token_bytes),
        };
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                write_lock(&mut file, &evidence)?;
                file.sync_all()?;
                Ok(Self { path, evidence })
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(ProjectLockError::AlreadyLocked(read_lock(&path)?))
            }
            Err(error) => Err(ProjectLockError::Io(error)),
        }
    }

    #[must_use]
    pub fn evidence(&self) -> &LockEvidence {
        &self.evidence
    }

    /// Preserves and renames a stale lock so a new lease can be acquired.
    ///
    /// Recovery never silently removes evidence and must be initiated by an
    /// explicit user action after the supplied minimum age.
    ///
    /// # Errors
    ///
    /// Returns an error when the lock is malformed, too recent, or cannot be
    /// moved to its recovery name.
    pub fn recover_stale(
        project_directory: impl AsRef<Path>,
        minimum_age: Duration,
    ) -> Result<PathBuf, ProjectLockError> {
        let path = project_directory.as_ref().join("lock").join(LOCK_FILE_NAME);
        let evidence = read_lock(&path)?;
        let age_millis = unix_millis().saturating_sub(evidence.created_unix_millis);
        if age_millis < minimum_age.as_millis() {
            return Err(ProjectLockError::NotStale);
        }
        let recovered = path.with_file_name(format!(
            "writer.lock.recovered-{}-{}",
            evidence.created_unix_millis, evidence.process_id
        ));
        fs::rename(path, &recovered)?;
        Ok(recovered)
    }
}

impl Drop for ProjectWriterLock {
    fn drop(&mut self) {
        if read_lock(&self.path).is_ok_and(|current| current.token == self.evidence.token) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// One append-only event with a chained SHA-256 digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRecord {
    pub sequence: u64,
    pub event: String,
    pub previous_digest: String,
    pub digest: String,
}

/// Creates a deterministic append-only audit hash chain.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuditChain {
    records: Vec<AuditRecord>,
}

impl AuditChain {
    /// Appends an event and returns its immutable chained record.
    ///
    /// # Panics
    ///
    /// Panics only if the record appended immediately beforehand is absent,
    /// which would indicate an internal logic error.
    pub fn append(&mut self, event: impl Into<String>) -> &AuditRecord {
        let event = event.into();
        let sequence = u64::try_from(self.records.len()).unwrap_or(u64::MAX);
        let previous_digest = self
            .records
            .last()
            .map_or_else(|| "0".repeat(64), |record| record.digest.clone());
        let digest = audit_digest(sequence, &previous_digest, &event);
        self.records.push(AuditRecord {
            sequence,
            event,
            previous_digest,
            digest,
        });
        self.records.last().expect("record was just appended")
    }

    #[must_use]
    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    #[must_use]
    pub fn verify(&self) -> bool {
        verify_audit_records(&self.records)
    }
}

/// Verifies an exported sequence of audit records.
#[must_use]
pub fn verify_audit_records(records: &[AuditRecord]) -> bool {
    let mut previous = "0".repeat(64);
    for record in records {
        if record.previous_digest != previous
            || record.digest != audit_digest(record.sequence, &previous, &record.event)
        {
            return false;
        }
        previous.clone_from(&record.digest);
    }
    true
}

/// Non-secret diagnostic information about the active `SQLCipher` connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CipherInfo {
    pub version: String,
    pub provider: String,
    pub provider_version: String,
    pub active: bool,
}

/// One append-only audit event persisted inside an encrypted store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredAuditEvent {
    pub sequence: u64,
    pub occurred_unix_millis: u128,
    pub actor_id: String,
    pub action: String,
    pub outcome: String,
    pub details: String,
    pub previous_digest: String,
    pub digest: String,
}

/// An application-owned handle to one encrypted, single-user project store.
pub struct EncryptedProjectStore {
    connection: Connection,
    path: PathBuf,
    cipher_info: CipherInfo,
}

impl fmt::Debug for EncryptedProjectStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncryptedProjectStore")
            .field("path", &self.path)
            .field("cipher_info", &self.cipher_info)
            .finish_non_exhaustive()
    }
}

/// Storage failures with password and low-level unlock details intentionally
/// excluded from diagnostics.
#[derive(Debug)]
pub enum StoreError {
    AlreadyExists(PathBuf),
    Missing(PathBuf),
    EmptyPassphrase,
    UnlockFailed,
    CipherUnavailable,
    UnsupportedSchema(i64),
    IntegrityCheckFailed(Vec<String>),
    Database(rusqlite::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists(path) => {
                write!(
                    formatter,
                    "project store already exists: {}",
                    path.display()
                )
            }
            Self::Missing(path) => {
                write!(
                    formatter,
                    "project store does not exist: {}",
                    path.display()
                )
            }
            Self::EmptyPassphrase => formatter.write_str("project passphrase cannot be empty"),
            Self::UnlockFailed => formatter
                .write_str("project store could not be unlocked with the supplied credentials"),
            Self::CipherUnavailable => {
                formatter.write_str("the database connection is not protected by SQLCipher")
            }
            Self::UnsupportedSchema(version) => {
                write!(
                    formatter,
                    "encrypted store schema version {version} is unsupported"
                )
            }
            Self::IntegrityCheckFailed(messages) => write!(
                formatter,
                "encrypted project integrity check failed with {} finding(s)",
                messages.len()
            ),
            Self::Database(error) => write!(formatter, "encrypted project database error: {error}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl EncryptedProjectStore {
    /// Creates a store using an application-generated project key.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::create`]. The temporary `SQLCipher`
    /// secret representation is zeroized after use.
    pub fn create_with_project_key(
        path: impl AsRef<Path>,
        key: &ProjectKey,
    ) -> Result<Self, StoreError> {
        let secret = key.sqlcipher_secret();
        Self::create(path, &secret)
    }

    /// Opens a store using an application-generated project key.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::open`]. The temporary `SQLCipher`
    /// secret representation is zeroized after use.
    pub fn open_with_project_key(
        path: impl AsRef<Path>,
        key: &ProjectKey,
    ) -> Result<Self, StoreError> {
        let secret = key.sqlcipher_secret();
        Self::open(path, &secret)
    }

    /// Creates a new encrypted project store. Existing files are never opened
    /// or overwritten by this operation.
    ///
    /// # Errors
    ///
    /// Returns an error when the passphrase is empty, the path already exists,
    /// the file cannot be created, encryption is unavailable, or schema
    /// initialization fails.
    pub fn create(path: impl AsRef<Path>, passphrase: &str) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        validate_passphrase(passphrase)?;
        if path.exists() {
            return Err(StoreError::AlreadyExists(path));
        }

        let connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        let mut store = Self::unlock(connection, &path, passphrase)?;
        store.initialize_schema()?;
        Ok(store)
    }

    /// Opens an existing encrypted project store.
    ///
    /// # Errors
    ///
    /// Returns an error when the passphrase is empty, the path is missing, the
    /// supplied credentials cannot unlock the store, encryption is unavailable,
    /// or connection configuration fails.
    pub fn open(path: impl AsRef<Path>, passphrase: &str) -> Result<Self, StoreError> {
        let path = path.as_ref().to_path_buf();
        validate_passphrase(passphrase)?;
        if !path.is_file() {
            return Err(StoreError::Missing(path));
        }

        let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        let mut store = Self::unlock(connection, &path, passphrase)?;
        store.migrate_schema()?;
        Ok(store)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn cipher_info(&self) -> &CipherInfo {
        &self.cipher_info
    }

    /// Applies all metadata changes in one transaction. A constraint failure
    /// rolls back every change in the batch.
    ///
    /// # Errors
    ///
    /// Returns a database error when a value violates the schema, a statement
    /// cannot be executed, or the transaction cannot be committed.
    pub fn set_metadata_batch<K, V>(&mut self, entries: &[(K, V)]) -> Result<(), StoreError>
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let transaction = self.connection.transaction()?;
        {
            let mut statement = transaction.prepare_cached(
                "INSERT INTO project_metadata (metadata_key, metadata_value)\
                 VALUES (?1, ?2)\
                 ON CONFLICT(metadata_key) DO UPDATE SET metadata_value = excluded.metadata_value",
            )?;
            for (key, value) in entries {
                statement.execute(params![key.as_ref(), value.as_ref()])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Returns the current value for one metadata key, if present.
    ///
    /// # Errors
    ///
    /// Returns a database error when the encrypted query cannot be prepared or
    /// executed, or when the stored value has an unexpected type.
    pub fn metadata(&self, key: &str) -> Result<Option<String>, StoreError> {
        self.connection
            .query_row(
                "SELECT metadata_value FROM project_metadata WHERE metadata_key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(StoreError::from)
    }

    /// Stores or replaces one random project key inside this encrypted store.
    /// This is used by the encrypted local profile as a project-key vault.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid locator or failed encrypted transaction.
    pub fn store_project_key(
        &mut self,
        key_locator: &str,
        key: &ProjectKey,
    ) -> Result<(), StoreError> {
        validate_project_id(key_locator).map_err(|_| {
            StoreError::Database(rusqlite::Error::InvalidParameterName(
                "invalid protected key locator".to_owned(),
            ))
        })?;
        self.connection.execute(
            "INSERT INTO protected_keys (key_locator, key_material)
             VALUES (?1, ?2)
             ON CONFLICT(key_locator) DO UPDATE SET key_material = excluded.key_material",
            params![key_locator, key.as_bytes()],
        )?;
        Ok(())
    }

    /// Loads a random project key from this encrypted profile vault.
    ///
    /// # Errors
    ///
    /// Returns an error when the locator is invalid, absent, corrupt, or the
    /// encrypted query fails.
    pub fn project_key(&self, key_locator: &str) -> Result<ProjectKey, StoreError> {
        validate_project_id(key_locator).map_err(|_| {
            StoreError::Database(rusqlite::Error::InvalidParameterName(
                "invalid protected key locator".to_owned(),
            ))
        })?;
        let mut secret = self
            .connection
            .query_row(
                "SELECT key_material FROM protected_keys WHERE key_locator = ?1",
                [key_locator],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(|_| StoreError::UnlockFailed)?;
        let result = ProjectKey::from_secret(&secret).map_err(|_| StoreError::UnlockFailed);
        secret.zeroize();
        result
    }

    /// Removes a project key from this encrypted profile vault.
    ///
    /// # Errors
    ///
    /// Returns an error when the encrypted delete cannot be completed.
    pub fn delete_project_key(&mut self, key_locator: &str) -> Result<(), StoreError> {
        self.connection.execute(
            "DELETE FROM protected_keys WHERE key_locator = ?1",
            [key_locator],
        )?;
        Ok(())
    }

    /// Runs `SQLCipher`'s page-HMAC integrity check. A successful check returns
    /// no findings.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::IntegrityCheckFailed`] when one or more encrypted
    /// pages fail verification, or a database error if the check cannot run.
    pub fn verify_integrity(&self) -> Result<(), StoreError> {
        let mut statement = self.connection.prepare("PRAGMA cipher_integrity_check")?;
        let findings = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        if findings.is_empty() {
            Ok(())
        } else {
            Err(StoreError::IntegrityCheckFailed(findings))
        }
    }

    /// Flushes committed WAL pages into the encrypted main database.
    ///
    /// # Errors
    ///
    /// Returns a database error when the WAL checkpoint cannot be completed.
    pub fn checkpoint(&self) -> Result<(), StoreError> {
        self.connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(())
    }

    /// Appends a tamper-evident audit event in one encrypted transaction.
    /// Actor IDs and details must already be bounded, non-secret identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when the prior event cannot be read or the new event
    /// cannot be committed atomically.
    pub fn append_audit_event(
        &mut self,
        actor_id: &str,
        action: &str,
        outcome: &str,
        details: &str,
    ) -> Result<StoredAuditEvent, StoreError> {
        let transaction = self.connection.transaction()?;
        let previous = transaction
            .query_row(
                "SELECT sequence, digest FROM audit_events ORDER BY sequence DESC LIMIT 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let (sequence, previous_digest) = previous.map_or_else(
            || (0_i64, "0".repeat(64)),
            |(sequence, digest)| (sequence.saturating_add(1), digest),
        );
        let occurred_unix_millis = i64::try_from(unix_millis()).unwrap_or(i64::MAX);
        let digest = stored_audit_digest(
            sequence,
            occurred_unix_millis,
            &previous_digest,
            actor_id,
            action,
            outcome,
            details,
        );
        transaction.execute(
            "INSERT INTO audit_events (
                 sequence, occurred_unix_millis, actor_id, action, outcome, details,
                 previous_digest, digest
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                sequence,
                occurred_unix_millis,
                actor_id,
                action,
                outcome,
                details,
                previous_digest,
                digest
            ],
        )?;
        transaction.commit()?;
        Ok(StoredAuditEvent {
            sequence: u64::try_from(sequence).unwrap_or(u64::MAX),
            occurred_unix_millis: u128::try_from(occurred_unix_millis).unwrap_or_default(),
            actor_id: actor_id.to_owned(),
            action: action.to_owned(),
            outcome: outcome.to_owned(),
            details: details.to_owned(),
            previous_digest,
            digest,
        })
    }

    /// Returns the encrypted store's audit chain in sequence order.
    ///
    /// # Errors
    ///
    /// Returns an error when an audit row cannot be queried or decoded.
    pub fn audit_events(&self) -> Result<Vec<StoredAuditEvent>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT sequence, occurred_unix_millis, actor_id, action, outcome, details,
                    previous_digest, digest
             FROM audit_events ORDER BY sequence",
        )?;
        statement
            .query_map([], |row| {
                let sequence = row.get::<_, i64>(0)?;
                let occurred = row.get::<_, i64>(1)?;
                Ok(StoredAuditEvent {
                    sequence: u64::try_from(sequence).unwrap_or(u64::MAX),
                    occurred_unix_millis: u128::try_from(occurred).unwrap_or_default(),
                    actor_id: row.get(2)?,
                    action: row.get(3)?,
                    outcome: row.get(4)?,
                    details: row.get(5)?,
                    previous_digest: row.get(6)?,
                    digest: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    /// Verifies every digest and link in the persisted audit chain.
    ///
    /// # Errors
    ///
    /// Returns an error when the encrypted audit events cannot be read.
    pub fn verify_audit_events(&self) -> Result<bool, StoreError> {
        let events = self.audit_events()?;
        let mut previous = "0".repeat(64);
        for event in events {
            let sequence = i64::try_from(event.sequence).unwrap_or(i64::MAX);
            let occurred = i64::try_from(event.occurred_unix_millis).unwrap_or(i64::MAX);
            if event.previous_digest != previous
                || event.digest
                    != stored_audit_digest(
                        sequence,
                        occurred,
                        &previous,
                        &event.actor_id,
                        &event.action,
                        &event.outcome,
                        &event.details,
                    )
            {
                return Ok(false);
            }
            previous = event.digest;
        }
        Ok(true)
    }

    fn unlock(connection: Connection, path: &Path, passphrase: &str) -> Result<Self, StoreError> {
        // SQLCipher requires the key to be the first operation after opening.
        connection
            .pragma_update(None, "key", passphrase)
            .map_err(|_| StoreError::UnlockFailed)?;
        connection
            .pragma_update(None, "cipher_log_level", "NONE")
            .map_err(|_| StoreError::CipherUnavailable)?;

        // Force a page read now so an incorrect key is rejected at the
        // boundary, rather than surfacing later as an arbitrary query error.
        connection
            .query_row("SELECT count(*) FROM sqlite_master", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|_| StoreError::UnlockFailed)?;

        let version = connection
            .query_row("PRAGMA cipher_version", [], |row| row.get::<_, String>(0))
            .map_err(|_| StoreError::CipherUnavailable)?;
        if version.trim().is_empty() {
            return Err(StoreError::CipherUnavailable);
        }
        let provider = connection
            .query_row("PRAGMA cipher_provider", [], |row| row.get::<_, String>(0))
            .map_err(|_| StoreError::CipherUnavailable)?;
        let provider_version = connection
            .query_row("PRAGMA cipher_provider_version", [], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|_| StoreError::CipherUnavailable)?;

        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "secure_delete", "ON")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.busy_timeout(Duration::from_secs(5))?;
        let journal_mode = connection.query_row("PRAGMA journal_mode = WAL", [], |row| {
            row.get::<_, String>(0)
        })?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(StoreError::Database(rusqlite::Error::InvalidQuery));
        }

        let active = connection
            .query_row("PRAGMA cipher_status", [], |row| row.get::<_, String>(0))
            .map_err(|_| StoreError::CipherUnavailable)?
            == "1";
        if !active {
            return Err(StoreError::CipherUnavailable);
        }

        Ok(Self {
            connection,
            path: path.to_path_buf(),
            cipher_info: CipherInfo {
                version,
                provider,
                provider_version,
                active,
            },
        })
    }

    fn initialize_schema(&mut self) -> Result<(), StoreError> {
        let transaction = self.connection.transaction()?;
        transaction.execute_batch(
            "CREATE TABLE project_metadata (\
                 metadata_key TEXT PRIMARY KEY CHECK (length(metadata_key) > 0),\
                 metadata_value TEXT NOT NULL\
             ) STRICT;\
             CREATE TABLE schema_information (\
                 singleton INTEGER PRIMARY KEY CHECK (singleton = 1),\
                 schema_version INTEGER NOT NULL CHECK (schema_version > 0)\
             ) STRICT;\
             CREATE TABLE audit_events (\
                 sequence INTEGER PRIMARY KEY CHECK (sequence >= 0),\
                 occurred_unix_millis INTEGER NOT NULL CHECK (occurred_unix_millis >= 0),\
                 actor_id TEXT NOT NULL CHECK (length(actor_id) > 0),\
                 action TEXT NOT NULL CHECK (length(action) > 0),\
                 outcome TEXT NOT NULL CHECK (length(outcome) > 0),\
                 details TEXT NOT NULL,\
                 previous_digest TEXT NOT NULL CHECK (length(previous_digest) = 64),\
                 digest TEXT NOT NULL CHECK (length(digest) = 64)\
             ) STRICT;\
             CREATE TABLE protected_keys (\
                 key_locator TEXT PRIMARY KEY CHECK (length(key_locator) > 0),\
                 key_material BLOB NOT NULL CHECK (length(key_material) = 32)\
             ) STRICT;",
        )?;
        transaction.execute(
            "INSERT INTO schema_information (singleton, schema_version) VALUES (1, ?1)",
            [SCHEMA_VERSION],
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn migrate_schema(&mut self) -> Result<(), StoreError> {
        let version = self.connection.query_row(
            "SELECT schema_version FROM schema_information WHERE singleton = 1",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        match version {
            SCHEMA_VERSION => Ok(()),
            1 => {
                let transaction = self.connection.transaction()?;
                transaction.execute_batch(
                    "CREATE TABLE audit_events (\
                         sequence INTEGER PRIMARY KEY CHECK (sequence >= 0),\
                         occurred_unix_millis INTEGER NOT NULL CHECK (occurred_unix_millis >= 0),\
                         actor_id TEXT NOT NULL CHECK (length(actor_id) > 0),\
                         action TEXT NOT NULL CHECK (length(action) > 0),\
                         outcome TEXT NOT NULL CHECK (length(outcome) > 0),\
                         details TEXT NOT NULL,\
                         previous_digest TEXT NOT NULL CHECK (length(previous_digest) = 64),\
                         digest TEXT NOT NULL CHECK (length(digest) = 64)\
                     ) STRICT;\
                     CREATE TABLE protected_keys (\
                         key_locator TEXT PRIMARY KEY CHECK (length(key_locator) > 0),\
                         key_material BLOB NOT NULL CHECK (length(key_material) = 32)\
                     ) STRICT;\
                     UPDATE schema_information SET schema_version = 3 WHERE singleton = 1;",
                )?;
                transaction.commit()?;
                Ok(())
            }
            2 => {
                let transaction = self.connection.transaction()?;
                transaction.execute_batch(
                    "CREATE TABLE protected_keys (\
                         key_locator TEXT PRIMARY KEY CHECK (length(key_locator) > 0),\
                         key_material BLOB NOT NULL CHECK (length(key_material) = 32)\
                     ) STRICT;\
                     UPDATE schema_information SET schema_version = 3 WHERE singleton = 1;",
                )?;
                transaction.commit()?;
                Ok(())
            }
            unsupported => Err(StoreError::UnsupportedSchema(unsupported)),
        }
    }
}

fn validate_passphrase(passphrase: &str) -> Result<(), StoreError> {
    if passphrase.is_empty() {
        Err(StoreError::EmptyPassphrase)
    } else {
        Ok(())
    }
}

fn derive_wrapping_key(
    passphrase: &str,
    salt: &[u8],
    output: &mut [u8; PROJECT_KEY_BYTES],
) -> Result<(), KeyEnvelopeError> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_LANES,
        Some(PROJECT_KEY_BYTES),
    )
    .map_err(|_| KeyEnvelopeError::DerivationFailed)?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, output)
        .map_err(|_| KeyEnvelopeError::DerivationFailed)
}

fn validate_project_id(project_id: &str) -> Result<(), KeyStoreError> {
    if project_id.is_empty()
        || project_id.len() > 128
        || !project_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Err(KeyStoreError::InvalidProjectId)
    } else {
        Ok(())
    }
}

fn unix_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

fn write_lock(file: &mut fs::File, evidence: &LockEvidence) -> io::Result<()> {
    writeln!(file, "process_id={}", evidence.process_id)?;
    writeln!(file, "created_unix_millis={}", evidence.created_unix_millis)?;
    writeln!(file, "token={}", evidence.token)
}

fn read_lock(path: &Path) -> Result<LockEvidence, ProjectLockError> {
    let text = fs::read_to_string(path)?;
    let mut process_id = None;
    let mut created_unix_millis = None;
    let mut token = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("process_id=") {
            process_id = value.parse().ok();
        } else if let Some(value) = line.strip_prefix("created_unix_millis=") {
            created_unix_millis = value.parse().ok();
        } else if let Some(value) = line.strip_prefix("token=") {
            token = Some(value.to_owned());
        }
    }
    match (process_id, created_unix_millis, token) {
        (Some(process_id), Some(created_unix_millis), Some(token)) if token.len() == 32 => {
            Ok(LockEvidence {
                process_id,
                created_unix_millis,
                token,
            })
        }
        _ => Err(ProjectLockError::InvalidLock),
    }
}

fn audit_digest(sequence: u64, previous_digest: &str, event: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sequence.to_be_bytes());
    hasher.update(previous_digest.len().to_be_bytes());
    hasher.update(previous_digest.as_bytes());
    hasher.update(event.len().to_be_bytes());
    hasher.update(event.as_bytes());
    hex::encode(hasher.finalize())
}

fn stored_audit_digest(
    sequence: i64,
    occurred_unix_millis: i64,
    previous_digest: &str,
    actor_id: &str,
    action: &str,
    outcome: &str,
    details: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sequence.to_be_bytes());
    hasher.update(occurred_unix_millis.to_be_bytes());
    for value in [previous_digest, actor_id, action, outcome, details] {
        hasher.update(value.len().to_be_bytes());
        hasher.update(value.as_bytes());
    }
    hex::encode(hasher.finalize())
}
