use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use seqqc_security::{
    AuditChain, EncryptedProjectStore, NativeKeyStore, ProjectKey, ProjectLockError,
    ProjectWriterLock, StoreError, verify_audit_records,
};

#[test]
fn generated_project_keys_encrypt_stores_and_are_redacted() {
    let directory = TestDirectory::create("project-key");
    let path = directory.0.join("project.db");
    let key = ProjectKey::generate().expect("generate project key");
    assert_eq!(format!("{key:?}"), "ProjectKey([REDACTED])");

    let mut store =
        EncryptedProjectStore::create_with_project_key(&path, &key).expect("create keyed store");
    store
        .set_metadata_batch(&[("project_name", "Synthetic project")])
        .expect("write metadata");
    drop(store);

    let reopened =
        EncryptedProjectStore::open_with_project_key(&path, &key).expect("reopen keyed store");
    assert_eq!(
        reopened.metadata("project_name").expect("read metadata"),
        Some("Synthetic project".to_owned())
    );
    let wrong_key = ProjectKey::generate().expect("generate second key");
    assert!(matches!(
        EncryptedProjectStore::open_with_project_key(&path, &wrong_key),
        Err(StoreError::UnlockFailed)
    ));
}

#[test]
fn writer_lock_blocks_a_second_writer_and_preserves_stale_evidence() {
    let directory = TestDirectory::create("writer-lock");
    let first = ProjectWriterLock::acquire(&directory.0).expect("acquire first lock");
    assert!(matches!(
        ProjectWriterLock::acquire(&directory.0),
        Err(ProjectLockError::AlreadyLocked(_))
    ));

    let recovered = ProjectWriterLock::recover_stale(&directory.0, Duration::ZERO)
        .expect("explicitly recover lock");
    assert!(recovered.is_file());
    let replacement = ProjectWriterLock::acquire(&directory.0).expect("acquire replacement lock");
    drop(first);
    assert!(directory.0.join("lock/writer.lock").is_file());
    drop(replacement);
    assert!(!directory.0.join("lock/writer.lock").exists());
    assert!(recovered.is_file());
}

#[test]
fn audit_chain_detects_mutation_and_reordering() {
    let mut chain = AuditChain::default();
    chain.append("project.created");
    chain.append("inputs.discovered");
    chain.append("report.exported");
    assert!(chain.verify());

    let mut modified = chain.records().to_vec();
    modified[1].event = "inputs.replaced".to_owned();
    assert!(!verify_audit_records(&modified));
    let mut reordered = chain.records().to_vec();
    reordered.swap(1, 2);
    assert!(!verify_audit_records(&reordered));
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "requires an interactive Windows Credential Manager logon session"]
fn windows_credential_manager_round_trips_a_project_key() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock must be after Unix epoch")
        .as_nanos();
    let project_id = format!("m0-test-{}-{nonce}", std::process::id());
    let key_store = NativeKeyStore::new("org.seqqc.studio.m0-tests");
    let key = ProjectKey::generate().expect("generate project key");
    key_store
        .store(&project_id, &key)
        .expect("store project key");
    let loaded = key_store.load(&project_id).expect("load project key");

    let directory = TestDirectory::create("credential-key");
    let path = directory.0.join("project.db");
    drop(
        EncryptedProjectStore::create_with_project_key(&path, &key)
            .expect("create with original key"),
    );
    EncryptedProjectStore::open_with_project_key(&path, &loaded)
        .expect("credential-loaded key must unlock the project");
    key_store
        .delete(&project_id)
        .expect("delete test credential");
}

#[test]
fn native_key_store_rejects_invalid_project_identifiers_before_platform_access() {
    let key_store = NativeKeyStore::new("org.seqqc.studio.m0-tests");
    let key = ProjectKey::generate().expect("generate project key");
    assert!(key_store.store("", &key).is_err());
    assert!(key_store.store("contains/slash", &key).is_err());
    assert!(key_store.store(&"a".repeat(129), &key).is_err());
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn create(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "seqqc-security-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if self.0.exists() {
            fs::remove_dir_all(&self.0).expect("remove test directory");
        }
    }
}
