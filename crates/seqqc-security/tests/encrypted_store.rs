#![allow(linker_messages)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use seqqc_security::{EncryptedProjectStore, StoreError};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const PASSPHRASE: &str = "local-only example passphrase";
const PRIVATE_MARKER: &str = "CLINICAL-SAMPLE-ALPHA-0123456789";

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn create() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "seqqc-security-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("test directory must be created");
        Self(path)
    }

    fn database_path(&self) -> PathBuf {
        self.0.join("project.sqqc")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if self.0.exists() {
            fs::remove_dir_all(&self.0).expect("test directory must be removable");
        }
    }
}

#[test]
fn encrypted_store_round_trips_without_plaintext_artifacts() {
    let directory = TestDirectory::create();
    let path = directory.database_path();

    let mut store = EncryptedProjectStore::create(&path, PASSPHRASE).expect("create store");
    assert!(store.cipher_info().active);
    assert_eq!(store.cipher_info().version, "4.14.0 community");
    assert_eq!(store.cipher_info().provider, "openssl");
    assert!(
        store
            .cipher_info()
            .provider_version
            .starts_with("OpenSSL 3.6.3")
    );
    store
        .set_metadata_batch(&[("sample_identifier", PRIVATE_MARKER)])
        .expect("write encrypted metadata");
    store.verify_integrity().expect("integrity check");

    assert_no_plaintext_in_database_family(&path, &[PASSPHRASE, PRIVATE_MARKER]);
    store.checkpoint().expect("checkpoint encrypted WAL");
    drop(store);

    assert_no_plaintext_in_database_family(&path, &[PASSPHRASE, PRIVATE_MARKER]);
    let reopened = EncryptedProjectStore::open(&path, PASSPHRASE).expect("reopen store");
    assert_eq!(
        reopened
            .metadata("sample_identifier")
            .expect("read metadata"),
        Some(PRIVATE_MARKER.to_owned())
    );
    reopened.verify_integrity().expect("integrity after reopen");
}

#[test]
fn wrong_passphrase_is_rejected_without_sensitive_diagnostics() {
    let directory = TestDirectory::create();
    let path = directory.database_path();
    drop(EncryptedProjectStore::create(&path, PASSPHRASE).expect("create store"));

    let error = EncryptedProjectStore::open(&path, "incorrect passphrase")
        .expect_err("wrong passphrase must fail");
    assert!(matches!(error, StoreError::UnlockFailed));
    let diagnostic = error.to_string();
    assert!(!diagnostic.contains(PASSPHRASE));
    assert!(!diagnostic.contains("incorrect passphrase"));
}

#[test]
fn failed_batch_rolls_back_every_metadata_change() {
    let directory = TestDirectory::create();
    let path = directory.database_path();
    let mut store = EncryptedProjectStore::create(&path, PASSPHRASE).expect("create store");

    let error = store
        .set_metadata_batch(&[("will_rollback", "value"), ("", "invalid key")])
        .expect_err("empty metadata key must violate the database constraint");
    assert!(matches!(error, StoreError::Database(_)));
    assert_eq!(
        store.metadata("will_rollback").expect("query metadata"),
        None
    );
    drop(store);

    let reopened = EncryptedProjectStore::open(&path, PASSPHRASE).expect("reopen after rollback");
    assert_eq!(
        reopened
            .metadata("will_rollback")
            .expect("query reopened metadata"),
        None
    );
}

#[test]
fn page_tampering_is_reported_by_cipher_integrity_check() {
    let directory = TestDirectory::create();
    let path = directory.database_path();
    let mut store = EncryptedProjectStore::create(&path, PASSPHRASE).expect("create store");
    let values = (0..64)
        .map(|index| {
            (
                format!("evidence_{index:03}"),
                format!("{PRIVATE_MARKER}-{}", "x".repeat(1_024)),
            )
        })
        .collect::<Vec<_>>();
    store
        .set_metadata_batch(&values)
        .expect("write multiple encrypted pages");
    store.checkpoint().expect("checkpoint before tampering");
    drop(store);

    let mut bytes = fs::read(&path).expect("read encrypted store for tampering");
    assert!(bytes.len() > 8_192, "fixture must span several pages");
    let tamper_offset = bytes.len() - 128;
    bytes[tamper_offset] ^= 0x01;
    fs::write(&path, bytes).expect("write deliberately tampered test store");

    let reopened = EncryptedProjectStore::open(&path, PASSPHRASE)
        .expect("page one remains valid, so unlock should succeed");
    assert!(matches!(
        reopened.verify_integrity(),
        Err(StoreError::IntegrityCheckFailed(findings)) if !findings.is_empty()
    ));
}

#[test]
fn create_never_overwrites_and_empty_passphrases_are_rejected() {
    let directory = TestDirectory::create();
    let path = directory.database_path();
    assert!(matches!(
        EncryptedProjectStore::create(&path, ""),
        Err(StoreError::EmptyPassphrase)
    ));

    drop(EncryptedProjectStore::create(&path, PASSPHRASE).expect("create store"));
    assert!(matches!(
        EncryptedProjectStore::create(&path, PASSPHRASE),
        Err(StoreError::AlreadyExists(existing)) if existing == path
    ));
}

#[test]
fn persisted_audit_chain_survives_reopen_and_verifies() {
    let directory = TestDirectory::create();
    let path = directory.database_path();
    let mut store = EncryptedProjectStore::create(&path, PASSPHRASE).expect("create store");
    let first = store
        .append_audit_event("principal-1", "project.created", "success", "")
        .expect("append first event");
    let second = store
        .append_audit_event("principal-1", "project.opened", "success", "")
        .expect("append second event");
    assert_eq!(first.sequence, 0);
    assert_eq!(second.sequence, 1);
    assert_eq!(second.previous_digest, first.digest);
    assert!(store.verify_audit_events().expect("verify live chain"));
    drop(store);

    let reopened = EncryptedProjectStore::open(&path, PASSPHRASE).expect("reopen store");
    let events = reopened.audit_events().expect("load audit events");
    assert_eq!(events.len(), 2);
    assert!(
        reopened
            .verify_audit_events()
            .expect("verify reopened chain")
    );
}

fn assert_no_plaintext_in_database_family(database_path: &Path, secrets: &[&str]) {
    let file_name = database_path
        .file_name()
        .expect("database must have a file name")
        .to_string_lossy();
    let parent = database_path.parent().expect("database must have parent");
    let mut inspected = 0;

    for entry in fs::read_dir(parent).expect("read test directory") {
        let path = entry.expect("read directory entry").path();
        let candidate_name = path
            .file_name()
            .expect("candidate must have name")
            .to_string_lossy();
        if !candidate_name.starts_with(file_name.as_ref()) || !path.is_file() {
            continue;
        }

        inspected += 1;
        let bytes = fs::read(&path).expect("read database artifact");
        assert!(
            !contains(&bytes, b"SQLite format 3\0"),
            "unencrypted SQLite header in {}",
            path.display()
        );
        for secret in secrets {
            assert!(
                !contains(&bytes, secret.as_bytes()),
                "plaintext secret found in {}",
                path.display()
            );
        }
    }
    assert!(
        inspected > 0,
        "at least the main database must be inspected"
    );
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
