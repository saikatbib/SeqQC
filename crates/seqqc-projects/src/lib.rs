#![cfg_attr(test, allow(linker_messages))]
#![allow(clippy::missing_errors_doc)]

//! Cross-platform, offline project lifecycle for `SeqQC` Studio.
//!
//! Sensitive project names, paths, and contexts live only in SQLCipher-backed
//! stores. The cleartext project manifest contains the minimum information
//! needed to locate a random key in the encrypted local profile vault or an
//! administrator-selected operating-system credential store.

use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use seqqc_security::{
    EncryptedProjectStore, KeyEnvelopeError, KeyStoreError, NativeKeyStore, PasswordKeyEnvelope,
    ProjectKey, ProjectLockError, ProjectWriterLock, StoreError,
};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const MANIFEST_FILE: &str = "project.json";
const DATABASE_FILE: &str = "project.db";
const PROFILE_DATABASE_FILE: &str = "profile.db";
const PROFILE_KEY_FILE: &str = "profile-key.json";
const PROFILE_KEY_LOCATOR: &str = "local-profile-v1";
const PROJECT_FORMAT: &str = "org.seqqc.project";
const PROJECT_FORMAT_VERSION: &str = "1.0.0";
const OS_KEY_STORE_CRYPTO_PROFILE: &str = "seqqc-project-v1-os-key-store";
const PROFILE_VAULT_CRYPTO_PROFILE: &str = "seqqc-project-v1-profile-key-vault";
const MAX_RECENT_PROJECTS: usize = 12;

/// Supported assay families for a `SeqQC` project.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Assay {
    #[default]
    Wgs,
    Wes,
    BulkRnaSeq,
}

impl Assay {
    pub const ALL: [Self; 3] = [Self::Wgs, Self::Wes, Self::BulkRnaSeq];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Wgs => "Whole-genome sequencing",
            Self::Wes => "Whole-exome sequencing",
            Self::BulkRnaSeq => "Bulk RNA-seq",
        }
    }

    const fn stored_value(self) -> &'static str {
        match self {
            Self::Wgs => "wgs",
            Self::Wes => "wes",
            Self::BulkRnaSeq => "bulk_rnaseq",
        }
    }

    fn from_stored(value: &str) -> Option<Self> {
        match value {
            "wgs" => Some(Self::Wgs),
            "wes" => Some(Self::Wes),
            "bulk_rnaseq" => Some(Self::BulkRnaSeq),
            _ => None,
        }
    }
}

/// Declared project context. This affects guidance and future policy defaults.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ProjectContext {
    HumanClinical,
    #[default]
    HumanResearch,
    Preclinical,
    Other,
    Unspecified,
}

impl ProjectContext {
    pub const ALL: [Self; 5] = [
        Self::HumanClinical,
        Self::HumanResearch,
        Self::Preclinical,
        Self::Other,
        Self::Unspecified,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::HumanClinical => "Human clinical",
            Self::HumanResearch => "Human research",
            Self::Preclinical => "Preclinical",
            Self::Other => "Other",
            Self::Unspecified => "Unspecified",
        }
    }

    const fn stored_value(self) -> &'static str {
        match self {
            Self::HumanClinical => "human_clinical",
            Self::HumanResearch => "human_research",
            Self::Preclinical => "preclinical",
            Self::Other => "other",
            Self::Unspecified => "unspecified",
        }
    }

    fn from_stored(value: &str) -> Option<Self> {
        match value {
            "human_clinical" => Some(Self::HumanClinical),
            "human_research" => Some(Self::HumanResearch),
            "preclinical" => Some(Self::Preclinical),
            "other" => Some(Self::Other),
            "unspecified" => Some(Self::Unspecified),
            _ => None,
        }
    }
}

/// Metadata required to create a project.
#[derive(Clone, Debug)]
pub struct CreateProjectRequest {
    pub parent_directory: PathBuf,
    pub display_name: String,
    pub assay: Assay,
    pub context: ProjectContext,
    pub input_root: Option<PathBuf>,
}

/// Decrypted project information safe to show only in an unlocked session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSummary {
    pub project_id: String,
    pub display_name: String,
    pub assay: Assay,
    pub context: ProjectContext,
    pub owner_local_principal_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub directory: PathBuf,
    pub input_root: Option<PathBuf>,
}

/// One encrypted recent-project catalog entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecentProject {
    pub project_id: String,
    pub display_name: String,
    pub assay: Assay,
    pub context: ProjectContext,
    pub directory: PathBuf,
    pub last_opened_at: String,
}

/// A local principal backed by the current OS login and credential store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalPrincipal {
    pub principal_id: String,
    pub display_name: String,
}

/// A loaded encrypted local profile. Dropping it closes the profile database
/// and releases its one-writer lock.
pub struct LocalProfile {
    store: EncryptedProjectStore,
    _lock: ProjectWriterLock,
    principal: LocalPrincipal,
    recents: Vec<RecentProject>,
}

impl LocalProfile {
    #[must_use]
    pub fn principal(&self) -> &LocalPrincipal {
        &self.principal
    }

    #[must_use]
    pub fn recent_projects(&self) -> &[RecentProject] {
        &self.recents
    }

    fn remember_project(&mut self, summary: &ProjectSummary) -> Result<(), ProjectError> {
        let recent = RecentProject {
            project_id: summary.project_id.clone(),
            display_name: summary.display_name.clone(),
            assay: summary.assay,
            context: summary.context,
            directory: summary.directory.clone(),
            last_opened_at: now_rfc3339()?,
        };
        self.recents
            .retain(|entry| entry.project_id != recent.project_id);
        self.recents.insert(0, recent);
        self.recents.truncate(MAX_RECENT_PROJECTS);
        persist_recents(&mut self.store, &self.recents)
    }

    /// Removes one project from the encrypted recent-project catalog. Project
    /// files and credential-store keys are not deleted.
    pub fn forget_recent(&mut self, project_id: &str) -> Result<(), ProjectError> {
        self.recents.retain(|entry| entry.project_id != project_id);
        persist_recents(&mut self.store, &self.recents)
    }

    /// Records a bounded, non-sensitive local-session event.
    pub fn record_event(&mut self, action: &str, outcome: &str) -> Result<(), ProjectError> {
        self.store
            .append_audit_event(&self.principal.principal_id, action, outcome, "")?;
        Ok(())
    }
}

/// An open project and its exclusive writer lease.
pub struct ProjectWorkspace {
    store: EncryptedProjectStore,
    _lock: ProjectWriterLock,
    summary: ProjectSummary,
}

impl ProjectWorkspace {
    #[must_use]
    pub fn summary(&self) -> &ProjectSummary {
        &self.summary
    }

    /// Updates mutable project metadata in one encrypted transaction.
    pub fn update_metadata(
        &mut self,
        display_name: &str,
        assay: Assay,
        context: ProjectContext,
        input_root: Option<&Path>,
    ) -> Result<(), ProjectError> {
        validate_display_name(display_name)?;
        let updated_at = now_rfc3339()?;
        let input = input_root.map_or_else(String::new, |path| path.to_string_lossy().into_owned());
        self.store.set_metadata_batch(&[
            ("display_name", display_name),
            ("assay", assay.stored_value()),
            ("context", context.stored_value()),
            ("input_root", input.as_str()),
            ("updated_at", updated_at.as_str()),
        ])?;
        self.store.append_audit_event(
            &self.summary.owner_local_principal_id,
            "project.metadata_updated",
            "success",
            "",
        )?;
        self.store.checkpoint()?;
        display_name.clone_into(&mut self.summary.display_name);
        self.summary.assay = assay;
        self.summary.context = context;
        self.summary.input_root = input_root.map(Path::to_path_buf);
        self.summary.updated_at = updated_at;
        Ok(())
    }

    /// Records a bounded, non-sensitive project lifecycle event.
    pub fn record_event(&mut self, action: &str, outcome: &str) -> Result<(), ProjectError> {
        self.store.append_audit_event(
            &self.summary.owner_local_principal_id,
            action,
            outcome,
            "",
        )?;
        Ok(())
    }
}

impl Drop for ProjectWorkspace {
    fn drop(&mut self) {
        let _ = self.store.checkpoint();
    }
}

/// Project-key operations hidden behind the project lifecycle boundary.
pub trait ProjectProtector {
    fn create_store(
        &self,
        key_locator: &str,
        database_path: &Path,
    ) -> Result<EncryptedProjectStore, ProtectionError>;

    fn open_store(
        &self,
        key_locator: &str,
        database_path: &Path,
    ) -> Result<EncryptedProjectStore, ProtectionError>;

    fn delete_key(&self, key_locator: &str) -> Result<(), ProtectionError>;
}

/// Production protector using a random per-database key in the signed-in
/// user's platform-native credential store.
#[derive(Clone, Debug)]
pub struct OsCredentialProtector {
    key_store: NativeKeyStore,
}

impl Default for OsCredentialProtector {
    fn default() -> Self {
        Self {
            key_store: NativeKeyStore::new("org.seqqc.studio.projects"),
        }
    }
}

impl ProjectProtector for OsCredentialProtector {
    fn create_store(
        &self,
        key_locator: &str,
        database_path: &Path,
    ) -> Result<EncryptedProjectStore, ProtectionError> {
        let key = ProjectKey::generate()?;
        self.key_store.store(key_locator, &key)?;
        match EncryptedProjectStore::create_with_project_key(database_path, &key) {
            Ok(store) => Ok(store),
            Err(error) => {
                let _ = self.key_store.delete(key_locator);
                Err(error.into())
            }
        }
    }

    fn open_store(
        &self,
        key_locator: &str,
        database_path: &Path,
    ) -> Result<EncryptedProjectStore, ProtectionError> {
        let key = self.key_store.load(key_locator)?;
        EncryptedProjectStore::open_with_project_key(database_path, &key).map_err(Into::into)
    }

    fn delete_key(&self, key_locator: &str) -> Result<(), ProtectionError> {
        self.key_store.delete(key_locator).map_err(Into::into)
    }
}

#[derive(Debug)]
pub enum ProtectionError {
    CredentialStore(KeyStoreError),
    EncryptedStore(StoreError),
}

impl fmt::Display for ProtectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CredentialStore(error) => write!(formatter, "credential store: {error}"),
            Self::EncryptedStore(error) => write!(formatter, "encrypted store: {error}"),
        }
    }
}

impl std::error::Error for ProtectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CredentialStore(error) => Some(error),
            Self::EncryptedStore(error) => Some(error),
        }
    }
}

impl From<KeyStoreError> for ProtectionError {
    fn from(error: KeyStoreError) -> Self {
        Self::CredentialStore(error)
    }
}

impl From<StoreError> for ProtectionError {
    fn from(error: StoreError) -> Self {
        Self::EncryptedStore(error)
    }
}

/// Cross-platform coordinator for one signed-in local profile.
pub struct ProjectManager<P> {
    protector: P,
    profile_path: PathBuf,
}

impl ProjectManager<OsCredentialProtector> {
    /// Creates a manager at the current OS user's application-data location.
    pub fn for_current_user() -> Result<Self, ProjectError> {
        Ok(Self::new(
            OsCredentialProtector::default(),
            default_profile_path()?,
        ))
    }
}

impl<P: ProjectProtector> ProjectManager<P> {
    #[must_use]
    pub fn new(protector: P, profile_path: PathBuf) -> Self {
        Self {
            protector,
            profile_path,
        }
    }

    #[must_use]
    pub fn profile_path(&self) -> &Path {
        &self.profile_path
    }

    #[must_use]
    pub fn profile_exists(&self) -> bool {
        self.profile_path.is_file()
    }

    /// Creates the encrypted profile for the current signed-in OS account.
    pub fn create_profile(&self, display_name: &str) -> Result<LocalProfile, ProjectError> {
        validate_display_name(display_name)?;
        if self.profile_exists() {
            return Err(ProjectError::ProfileAlreadyExists);
        }
        let directory = self.profile_directory()?;
        fs::create_dir_all(&directory)?;
        protect_directory(&directory)?;
        let lock = ProjectWriterLock::acquire(&directory)?;
        let mut store = self
            .protector
            .create_store(PROFILE_KEY_LOCATOR, &self.profile_path)?;
        let principal = LocalPrincipal {
            principal_id: Uuid::new_v4().to_string(),
            display_name: display_name.trim().to_owned(),
        };
        let created_at = now_rfc3339()?;
        let result = store.set_metadata_batch(&[
            ("profile_format", "org.seqqc.local-profile"),
            ("profile_format_version", "1.0.0"),
            ("principal_id", principal.principal_id.as_str()),
            ("principal_display_name", principal.display_name.as_str()),
            ("created_at", created_at.as_str()),
            ("recent_count", "0"),
        ]);
        if let Err(error) = result {
            drop(store);
            drop(lock);
            let _ = self.protector.delete_key(PROFILE_KEY_LOCATOR);
            let _ = fs::remove_file(&self.profile_path);
            return Err(error.into());
        }
        store.append_audit_event(
            &principal.principal_id,
            "local_profile.created",
            "success",
            "",
        )?;
        store.verify_integrity()?;
        Ok(LocalProfile {
            store,
            _lock: lock,
            principal,
            recents: Vec::new(),
        })
    }

    /// Opens the encrypted profile using the current OS account's credential
    /// store. No network account or online authentication is involved.
    pub fn open_profile(&self) -> Result<LocalProfile, ProjectError> {
        if !self.profile_exists() {
            return Err(ProjectError::ProfileMissing);
        }
        let directory = self.profile_directory()?;
        let lock = ProjectWriterLock::acquire(&directory)?;
        let mut store = self
            .protector
            .open_store(PROFILE_KEY_LOCATOR, &self.profile_path)?;
        store.verify_integrity()?;
        let principal = LocalPrincipal {
            principal_id: required_metadata(&store, "principal_id")?,
            display_name: required_metadata(&store, "principal_display_name")?,
        };
        let recents = load_recents(&store)?;
        store.append_audit_event(
            &principal.principal_id,
            "local_session.unlocked",
            "success",
            "",
        )?;
        Ok(LocalProfile {
            store,
            _lock: lock,
            principal,
            recents,
        })
    }

    /// Creates an opaque-named project directory and encrypted project store.
    #[allow(clippy::too_many_lines)]
    pub fn create_project(
        &self,
        profile: &mut LocalProfile,
        request: &CreateProjectRequest,
    ) -> Result<ProjectWorkspace, ProjectError> {
        validate_display_name(&request.display_name)?;
        if !request.parent_directory.is_dir() {
            return Err(ProjectError::InvalidRequest(
                "Choose an existing parent folder for the project.".to_owned(),
            ));
        }

        let project_id = Uuid::new_v4().to_string();
        let short_id = &project_id[..8];
        let directory = request
            .parent_directory
            .join(format!("SeqQC-{short_id}.seqqc"));
        fs::create_dir(&directory).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                ProjectError::ProjectAlreadyExists(directory.clone())
            } else {
                ProjectError::Io(error)
            }
        })?;
        protect_directory(&directory)?;

        let key_locator = format!("project-{project_id}");
        let result = (|| {
            let lock = ProjectWriterLock::acquire(&directory)?;
            let database_path = directory.join(DATABASE_FILE);
            let mut store = self.protector.create_store(&key_locator, &database_path)?;
            let created_at = now_rfc3339()?;
            let input_root = request
                .input_root
                .as_deref()
                .map_or_else(String::new, |path| path.to_string_lossy().into_owned());
            store.set_metadata_batch(&[
                ("schema_version", "1.0.0"),
                ("project_id", project_id.as_str()),
                ("display_name", request.display_name.trim()),
                ("assay", request.assay.stored_value()),
                ("context", request.context.stored_value()),
                ("presentation_profile", "guided"),
                (
                    "owner_local_principal_id",
                    profile.principal.principal_id.as_str(),
                ),
                ("single_writer_policy", "true"),
                ("created_at", created_at.as_str()),
                ("updated_at", created_at.as_str()),
                ("input_root", input_root.as_str()),
            ])?;
            store.append_audit_event(
                &profile.principal.principal_id,
                "project.created",
                "success",
                "",
            )?;
            store.verify_integrity()?;
            store.checkpoint()?;
            write_manifest(
                &directory,
                &ProjectManifest {
                    format: PROJECT_FORMAT.to_owned(),
                    format_version: PROJECT_FORMAT_VERSION.to_owned(),
                    project_id: project_id.clone(),
                    crypto_profile: OS_KEY_STORE_CRYPTO_PROFILE.to_owned(),
                    key_locator: key_locator.clone(),
                    created_by_app: env!("CARGO_PKG_VERSION").to_owned(),
                },
            )?;
            let summary = ProjectSummary {
                project_id: project_id.clone(),
                display_name: request.display_name.trim().to_owned(),
                assay: request.assay,
                context: request.context,
                owner_local_principal_id: profile.principal.principal_id.clone(),
                created_at: created_at.clone(),
                updated_at: created_at,
                directory: directory.clone(),
                input_root: request.input_root.clone(),
            };
            Ok(ProjectWorkspace {
                store,
                _lock: lock,
                summary,
            })
        })();

        match result {
            Ok(workspace) => {
                profile.remember_project(workspace.summary())?;
                Ok(workspace)
            }
            Err(error) => {
                let _ = self.protector.delete_key(&key_locator);
                let _ = fs::remove_dir_all(&directory);
                Err(error)
            }
        }
    }

    /// Opens an existing project and acquires its exclusive writer lock.
    pub fn open_project(
        &self,
        profile: &mut LocalProfile,
        directory: impl AsRef<Path>,
    ) -> Result<ProjectWorkspace, ProjectError> {
        let directory = directory.as_ref().to_path_buf();
        let manifest = read_manifest(&directory)?;
        validate_manifest(&manifest, OS_KEY_STORE_CRYPTO_PROFILE)?;
        let lock = ProjectWriterLock::acquire(&directory)?;
        let database_path = directory.join(DATABASE_FILE);
        let mut store = self
            .protector
            .open_store(&manifest.key_locator, &database_path)?;
        store.verify_integrity()?;
        let summary = load_summary(&store, directory, &manifest.project_id)?;
        store.append_audit_event(
            &profile.principal.principal_id,
            "project.opened",
            "success",
            "",
        )?;
        let workspace = ProjectWorkspace {
            store,
            _lock: lock,
            summary,
        };
        profile.remember_project(workspace.summary())?;
        Ok(workspace)
    }

    /// Persists the active project's latest summary in the encrypted profile.
    pub fn refresh_recent(
        &self,
        profile: &mut LocalProfile,
        workspace: &ProjectWorkspace,
    ) -> Result<(), ProjectError> {
        profile.remember_project(workspace.summary())
    }

    /// Preserves a sufficiently old lock as evidence before a user-initiated
    /// recovery attempt.
    pub fn recover_stale_project_lock(
        &self,
        directory: impl AsRef<Path>,
        minimum_age: Duration,
    ) -> Result<PathBuf, ProjectError> {
        ProjectWriterLock::recover_stale(directory, minimum_age).map_err(Into::into)
    }

    fn profile_directory(&self) -> Result<PathBuf, ProjectError> {
        self.profile_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| ProjectError::InvalidApplicationDataPath(self.profile_path.clone()))
    }
}

/// OS-neutral project manager using a local application passphrase to unwrap a
/// random profile key. Random per-project keys are kept inside that encrypted
/// profile vault, so the human passphrase is never used as a project DB key.
pub struct PassphraseProjectManager {
    profile_path: PathBuf,
}

impl PassphraseProjectManager {
    pub fn for_current_user() -> Result<Self, ProjectError> {
        Ok(Self::new(default_profile_path()?))
    }

    #[must_use]
    pub fn new(profile_path: PathBuf) -> Self {
        Self { profile_path }
    }

    #[must_use]
    pub fn profile_path(&self) -> &Path {
        &self.profile_path
    }

    #[must_use]
    pub fn profile_exists(&self) -> bool {
        self.profile_path.is_file()
    }

    pub fn create_profile(
        &self,
        display_name: &str,
        passphrase: &str,
    ) -> Result<LocalProfile, ProjectError> {
        validate_display_name(display_name)?;
        validate_local_passphrase(passphrase)?;
        if self.profile_exists() || self.profile_key_path()?.exists() {
            return Err(ProjectError::ProfileAlreadyExists);
        }
        let directory = self.profile_directory()?;
        fs::create_dir_all(&directory)?;
        protect_directory(&directory)?;
        let lock = ProjectWriterLock::acquire(&directory)?;
        let profile_key = ProjectKey::generate().map_err(ProtectionError::from)?;
        let envelope = profile_key.wrap_with_passphrase(passphrase)?;
        let mut store =
            EncryptedProjectStore::create_with_project_key(&self.profile_path, &profile_key)?;
        protect_file(&self.profile_path)?;
        let principal = LocalPrincipal {
            principal_id: Uuid::new_v4().to_string(),
            display_name: display_name.trim().to_owned(),
        };
        let created_at = now_rfc3339()?;
        let setup = (|| {
            store.set_metadata_batch(&[
                ("profile_format", "org.seqqc.local-profile"),
                ("profile_format_version", "1.0.0"),
                ("principal_id", principal.principal_id.as_str()),
                ("principal_display_name", principal.display_name.as_str()),
                ("created_at", created_at.as_str()),
                ("recent_count", "0"),
            ])?;
            store.append_audit_event(
                &principal.principal_id,
                "local_profile.created",
                "success",
                "",
            )?;
            store.verify_integrity()?;
            store.checkpoint()?;
            write_profile_key_file(&self.profile_key_path()?, &envelope)?;
            Ok::<(), ProjectError>(())
        })();
        if let Err(error) = setup {
            drop(store);
            drop(lock);
            let _ = fs::remove_file(&self.profile_path);
            let _ = fs::remove_file(self.profile_key_path()?);
            return Err(error);
        }
        Ok(LocalProfile {
            store,
            _lock: lock,
            principal,
            recents: Vec::new(),
        })
    }

    pub fn open_profile(&self, passphrase: &str) -> Result<LocalProfile, ProjectError> {
        if !self.profile_exists() {
            return Err(ProjectError::ProfileMissing);
        }
        let directory = self.profile_directory()?;
        let lock = ProjectWriterLock::acquire(&directory)?;
        let envelope = read_profile_key_file(&self.profile_key_path()?)?;
        let profile_key = ProjectKey::unwrap_with_passphrase(&envelope, passphrase)?;
        let mut store =
            EncryptedProjectStore::open_with_project_key(&self.profile_path, &profile_key)?;
        store.verify_integrity()?;
        let principal = LocalPrincipal {
            principal_id: required_metadata(&store, "principal_id")?,
            display_name: required_metadata(&store, "principal_display_name")?,
        };
        let recents = load_recents(&store)?;
        store.append_audit_event(
            &principal.principal_id,
            "local_session.unlocked",
            "success",
            "",
        )?;
        Ok(LocalProfile {
            store,
            _lock: lock,
            principal,
            recents,
        })
    }

    #[allow(clippy::too_many_lines)]
    pub fn create_project(
        &self,
        profile: &mut LocalProfile,
        request: &CreateProjectRequest,
    ) -> Result<ProjectWorkspace, ProjectError> {
        validate_display_name(&request.display_name)?;
        if !request.parent_directory.is_dir() {
            return Err(ProjectError::InvalidRequest(
                "Choose an existing parent folder for the project.".to_owned(),
            ));
        }
        let project_id = Uuid::new_v4().to_string();
        let directory = request
            .parent_directory
            .join(format!("SeqQC-{}.seqqc", &project_id[..8]));
        fs::create_dir(&directory).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                ProjectError::ProjectAlreadyExists(directory.clone())
            } else {
                ProjectError::Io(error)
            }
        })?;
        protect_directory(&directory)?;
        let key_locator = format!("project-{project_id}");
        let project_key = ProjectKey::generate().map_err(ProtectionError::from)?;
        profile
            .store
            .store_project_key(&key_locator, &project_key)?;
        profile.store.checkpoint()?;

        let result = (|| {
            let lock = ProjectWriterLock::acquire(&directory)?;
            let database_path = directory.join(DATABASE_FILE);
            let mut store =
                EncryptedProjectStore::create_with_project_key(&database_path, &project_key)?;
            protect_file(&database_path)?;
            let created_at = now_rfc3339()?;
            let input_root = request
                .input_root
                .as_deref()
                .map_or_else(String::new, |path| path.to_string_lossy().into_owned());
            store.set_metadata_batch(&[
                ("schema_version", "1.0.0"),
                ("project_id", project_id.as_str()),
                ("display_name", request.display_name.trim()),
                ("assay", request.assay.stored_value()),
                ("context", request.context.stored_value()),
                ("presentation_profile", "guided"),
                (
                    "owner_local_principal_id",
                    profile.principal.principal_id.as_str(),
                ),
                ("single_writer_policy", "true"),
                ("created_at", created_at.as_str()),
                ("updated_at", created_at.as_str()),
                ("input_root", input_root.as_str()),
            ])?;
            store.append_audit_event(
                &profile.principal.principal_id,
                "project.created",
                "success",
                "",
            )?;
            store.verify_integrity()?;
            store.checkpoint()?;
            write_manifest(
                &directory,
                &ProjectManifest {
                    format: PROJECT_FORMAT.to_owned(),
                    format_version: PROJECT_FORMAT_VERSION.to_owned(),
                    project_id: project_id.clone(),
                    crypto_profile: PROFILE_VAULT_CRYPTO_PROFILE.to_owned(),
                    key_locator: key_locator.clone(),
                    created_by_app: env!("CARGO_PKG_VERSION").to_owned(),
                },
            )?;
            let summary = ProjectSummary {
                project_id: project_id.clone(),
                display_name: request.display_name.trim().to_owned(),
                assay: request.assay,
                context: request.context,
                owner_local_principal_id: profile.principal.principal_id.clone(),
                created_at: created_at.clone(),
                updated_at: created_at,
                directory: directory.clone(),
                input_root: request.input_root.clone(),
            };
            Ok(ProjectWorkspace {
                store,
                _lock: lock,
                summary,
            })
        })();
        match result {
            Ok(workspace) => {
                profile.remember_project(workspace.summary())?;
                Ok(workspace)
            }
            Err(error) => {
                let _ = profile.store.delete_project_key(&key_locator);
                let _ = profile.store.checkpoint();
                let _ = fs::remove_dir_all(&directory);
                Err(error)
            }
        }
    }

    pub fn open_project(
        &self,
        profile: &mut LocalProfile,
        directory: impl AsRef<Path>,
    ) -> Result<ProjectWorkspace, ProjectError> {
        let directory = directory.as_ref().to_path_buf();
        let manifest = read_manifest(&directory)?;
        validate_manifest(&manifest, PROFILE_VAULT_CRYPTO_PROFILE)?;
        let lock = ProjectWriterLock::acquire(&directory)?;
        let project_key = profile.store.project_key(&manifest.key_locator)?;
        let database_path = directory.join(DATABASE_FILE);
        let mut store = EncryptedProjectStore::open_with_project_key(&database_path, &project_key)?;
        store.verify_integrity()?;
        let summary = load_summary(&store, directory, &manifest.project_id)?;
        store.append_audit_event(
            &profile.principal.principal_id,
            "project.opened",
            "success",
            "",
        )?;
        let workspace = ProjectWorkspace {
            store,
            _lock: lock,
            summary,
        };
        profile.remember_project(workspace.summary())?;
        Ok(workspace)
    }

    pub fn refresh_recent(
        &self,
        profile: &mut LocalProfile,
        workspace: &ProjectWorkspace,
    ) -> Result<(), ProjectError> {
        profile.remember_project(workspace.summary())
    }

    pub fn recover_stale_project_lock(
        &self,
        directory: impl AsRef<Path>,
        minimum_age: Duration,
    ) -> Result<PathBuf, ProjectError> {
        ProjectWriterLock::recover_stale(directory, minimum_age).map_err(Into::into)
    }

    fn profile_directory(&self) -> Result<PathBuf, ProjectError> {
        self.profile_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| ProjectError::InvalidApplicationDataPath(self.profile_path.clone()))
    }

    fn profile_key_path(&self) -> Result<PathBuf, ProjectError> {
        Ok(self.profile_directory()?.join(PROFILE_KEY_FILE))
    }
}

#[derive(Debug)]
pub enum ProjectError {
    ProfileAlreadyExists,
    ProfileMissing,
    ProjectAlreadyExists(PathBuf),
    InvalidRequest(String),
    InvalidManifest(String),
    UnsupportedManifest(String),
    InvalidMetadata(String),
    ApplicationDataUnavailable,
    InvalidApplicationDataPath(PathBuf),
    ClockUnavailable,
    Io(io::Error),
    Json(serde_json::Error),
    KeyEnvelope(KeyEnvelopeError),
    Protection(ProtectionError),
    Store(StoreError),
    Lock(ProjectLockError),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileAlreadyExists => formatter.write_str(
                "[SEQ-PROFILE-001] A local SeqQC profile already exists for this OS account.",
            ),
            Self::ProfileMissing => formatter.write_str(
                "[SEQ-PROFILE-002] No local SeqQC profile exists for this OS account.",
            ),
            Self::ProjectAlreadyExists(path) => write!(
                formatter,
                "[SEQ-PROJECT-001] A project directory already exists at {}.",
                path.display()
            ),
            Self::InvalidRequest(message) => {
                write!(formatter, "[SEQ-PROJECT-002] {message}")
            }
            Self::InvalidManifest(message) => {
                write!(formatter, "[SEQ-PROJECT-003] Invalid project manifest: {message}")
            }
            Self::UnsupportedManifest(version) => write!(
                formatter,
                "[SEQ-PROJECT-004] Project format {version} is not supported by this release."
            ),
            Self::InvalidMetadata(key) => write!(
                formatter,
                "[SEQ-PROJECT-005] Encrypted project metadata is missing or invalid: {key}."
            ),
            Self::ApplicationDataUnavailable => formatter.write_str(
                "[SEQ-PROFILE-003] The operating system did not provide a user application-data folder.",
            ),
            Self::InvalidApplicationDataPath(path) => write!(
                formatter,
                "[SEQ-PROFILE-004] Invalid profile path: {}.",
                path.display()
            ),
            Self::ClockUnavailable => {
                formatter.write_str("[SEQ-PROJECT-006] The system clock is unavailable.")
            }
            Self::Io(error) => write!(formatter, "[SEQ-PROJECT-007] Local file error: {error}"),
            Self::Json(error) => write!(
                formatter,
                "[SEQ-PROJECT-008] Project manifest JSON error: {error}"
            ),
            Self::KeyEnvelope(error) => write!(
                formatter,
                "[SEQ-SECURITY-003] Unable to unlock the encrypted local profile: {error}"
            ),
            Self::Protection(error) => write!(
                formatter,
                "[SEQ-SECURITY-001] Unable to protect or unlock the local encrypted store: {error}"
            ),
            Self::Store(error) => {
                write!(formatter, "[SEQ-SECURITY-002] Encrypted store error: {error}")
            }
            Self::Lock(error) => {
                write!(formatter, "[SEQ-PROJECT-009] Project writer lock error: {error}")
            }
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::KeyEnvelope(error) => Some(error),
            Self::Protection(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Lock(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for ProjectError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ProjectError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<KeyEnvelopeError> for ProjectError {
    fn from(error: KeyEnvelopeError) -> Self {
        Self::KeyEnvelope(error)
    }
}

impl From<ProtectionError> for ProjectError {
    fn from(error: ProtectionError) -> Self {
        Self::Protection(error)
    }
}

impl From<StoreError> for ProjectError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<ProjectLockError> for ProjectError {
    fn from(error: ProjectLockError) -> Self {
        Self::Lock(error)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ProjectManifest {
    format: String,
    format_version: String,
    project_id: String,
    crypto_profile: String,
    key_locator: String,
    created_by_app: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProfileKeyFile {
    format: String,
    kdf: String,
    memory_kib: u32,
    iterations: u32,
    lanes: u32,
    salt_hex: String,
    cipher: String,
    nonce_hex: String,
    ciphertext_hex: String,
}

impl From<&PasswordKeyEnvelope> for ProfileKeyFile {
    fn from(envelope: &PasswordKeyEnvelope) -> Self {
        Self {
            format: envelope.format.clone(),
            kdf: envelope.kdf.clone(),
            memory_kib: envelope.memory_kib,
            iterations: envelope.iterations,
            lanes: envelope.lanes,
            salt_hex: envelope.salt_hex.clone(),
            cipher: envelope.cipher.clone(),
            nonce_hex: envelope.nonce_hex.clone(),
            ciphertext_hex: envelope.ciphertext_hex.clone(),
        }
    }
}

impl From<ProfileKeyFile> for PasswordKeyEnvelope {
    fn from(file: ProfileKeyFile) -> Self {
        Self {
            format: file.format,
            kdf: file.kdf,
            memory_kib: file.memory_kib,
            iterations: file.iterations,
            lanes: file.lanes,
            salt_hex: file.salt_hex,
            cipher: file.cipher,
            nonce_hex: file.nonce_hex,
            ciphertext_hex: file.ciphertext_hex,
        }
    }
}

fn write_profile_key_file(path: &Path, envelope: &PasswordKeyEnvelope) -> Result<(), ProjectError> {
    let temporary = path.with_extension("json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    serde_json::to_writer_pretty(&mut file, &ProfileKeyFile::from(envelope))?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    protect_file(&temporary)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn read_profile_key_file(path: &Path) -> Result<PasswordKeyEnvelope, ProjectError> {
    if !path.is_file() {
        return Err(ProjectError::InvalidManifest(
            "local profile key envelope is missing".to_owned(),
        ));
    }
    let file: ProfileKeyFile = serde_json::from_slice(&fs::read(path)?)?;
    Ok(file.into())
}

fn write_manifest(directory: &Path, manifest: &ProjectManifest) -> Result<(), ProjectError> {
    let target = directory.join(MANIFEST_FILE);
    let temporary = directory.join(".project.json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    serde_json::to_writer_pretty(&mut file, manifest)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    protect_file(&temporary)?;
    fs::rename(&temporary, &target)?;
    Ok(())
}

fn read_manifest(directory: &Path) -> Result<ProjectManifest, ProjectError> {
    let path = directory.join(MANIFEST_FILE);
    if !path.is_file() {
        return Err(ProjectError::InvalidManifest(format!(
            "{} is missing",
            path.display()
        )));
    }
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(Into::into)
}

fn validate_manifest(
    manifest: &ProjectManifest,
    expected_crypto_profile: &str,
) -> Result<(), ProjectError> {
    if manifest.format != PROJECT_FORMAT {
        return Err(ProjectError::InvalidManifest(
            "unrecognized format identifier".to_owned(),
        ));
    }
    if manifest.format_version != PROJECT_FORMAT_VERSION {
        return Err(ProjectError::UnsupportedManifest(
            manifest.format_version.clone(),
        ));
    }
    if manifest.crypto_profile != expected_crypto_profile {
        return Err(ProjectError::InvalidManifest(
            "unrecognized encryption profile".to_owned(),
        ));
    }
    if Uuid::parse_str(&manifest.project_id).is_err()
        || manifest.key_locator != format!("project-{}", manifest.project_id)
    {
        return Err(ProjectError::InvalidManifest(
            "project identity or key locator is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn load_summary(
    store: &EncryptedProjectStore,
    directory: PathBuf,
    manifest_project_id: &str,
) -> Result<ProjectSummary, ProjectError> {
    let project_id = required_metadata(store, "project_id")?;
    if project_id != manifest_project_id {
        return Err(ProjectError::InvalidMetadata(
            "project_id does not match the outer manifest".to_owned(),
        ));
    }
    let assay_value = required_metadata(store, "assay")?;
    let context_value = required_metadata(store, "context")?;
    let input_root = store
        .metadata("input_root")?
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    Ok(ProjectSummary {
        project_id,
        display_name: required_metadata(store, "display_name")?,
        assay: Assay::from_stored(&assay_value)
            .ok_or_else(|| ProjectError::InvalidMetadata("assay".to_owned()))?,
        context: ProjectContext::from_stored(&context_value)
            .ok_or_else(|| ProjectError::InvalidMetadata("context".to_owned()))?,
        owner_local_principal_id: required_metadata(store, "owner_local_principal_id")?,
        created_at: required_metadata(store, "created_at")?,
        updated_at: required_metadata(store, "updated_at")?,
        directory,
        input_root,
    })
}

fn required_metadata(store: &EncryptedProjectStore, key: &str) -> Result<String, ProjectError> {
    store
        .metadata(key)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ProjectError::InvalidMetadata(key.to_owned()))
}

fn persist_recents(
    store: &mut EncryptedProjectStore,
    recents: &[RecentProject],
) -> Result<(), ProjectError> {
    let mut entries = Vec::with_capacity(1 + recents.len() * 7);
    entries.push(("recent_count".to_owned(), recents.len().to_string()));
    for (index, recent) in recents.iter().enumerate() {
        entries.push((
            format!("recent.{index}.project_id"),
            recent.project_id.clone(),
        ));
        entries.push((
            format!("recent.{index}.display_name"),
            recent.display_name.clone(),
        ));
        entries.push((
            format!("recent.{index}.assay"),
            recent.assay.stored_value().to_owned(),
        ));
        entries.push((
            format!("recent.{index}.context"),
            recent.context.stored_value().to_owned(),
        ));
        entries.push((
            format!("recent.{index}.directory"),
            recent.directory.to_string_lossy().into_owned(),
        ));
        entries.push((
            format!("recent.{index}.last_opened_at"),
            recent.last_opened_at.clone(),
        ));
    }
    store.set_metadata_batch(&entries)?;
    store.checkpoint()?;
    Ok(())
}

fn load_recents(store: &EncryptedProjectStore) -> Result<Vec<RecentProject>, ProjectError> {
    let count = store
        .metadata("recent_count")?
        .unwrap_or_else(|| "0".to_owned())
        .parse::<usize>()
        .map_err(|_| ProjectError::InvalidMetadata("recent_count".to_owned()))?
        .min(MAX_RECENT_PROJECTS);
    let mut recents = Vec::with_capacity(count);
    for index in 0..count {
        let assay_value = required_metadata(store, &format!("recent.{index}.assay"))?;
        let context_value = required_metadata(store, &format!("recent.{index}.context"))?;
        recents.push(RecentProject {
            project_id: required_metadata(store, &format!("recent.{index}.project_id"))?,
            display_name: required_metadata(store, &format!("recent.{index}.display_name"))?,
            assay: Assay::from_stored(&assay_value)
                .ok_or_else(|| ProjectError::InvalidMetadata(format!("recent.{index}.assay")))?,
            context: ProjectContext::from_stored(&context_value)
                .ok_or_else(|| ProjectError::InvalidMetadata(format!("recent.{index}.context")))?,
            directory: PathBuf::from(required_metadata(
                store,
                &format!("recent.{index}.directory"),
            )?),
            last_opened_at: required_metadata(store, &format!("recent.{index}.last_opened_at"))?,
        });
    }
    Ok(recents)
}

fn validate_display_name(display_name: &str) -> Result<(), ProjectError> {
    let trimmed = display_name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 200 {
        Err(ProjectError::InvalidRequest(
            "Name must contain 1 to 200 characters.".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn validate_local_passphrase(passphrase: &str) -> Result<(), ProjectError> {
    if passphrase.chars().count() < 12 {
        Err(ProjectError::InvalidRequest(
            "Local passphrase must contain at least 12 characters.".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn now_rfc3339() -> Result<String, ProjectError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| ProjectError::ClockUnavailable)
}

/// Returns a non-authoritative display name for the current OS principal. The
/// credential store, not this environment value, provides the security
/// boundary.
#[must_use]
pub fn detected_local_principal_name() -> String {
    ["USERNAME", "USER", "LOGNAME"]
        .into_iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
        .unwrap_or_else(|| "Local user".to_owned())
}

/// Resolves the per-user encrypted profile path without network access.
pub fn default_profile_path() -> Result<PathBuf, ProjectError> {
    let root = application_data_root_with(current_host_os(), |name| env::var_os(name))?;
    let directory = match current_host_os() {
        HostOs::Windows | HostOs::MacOs => root.join("SeqQC Studio"),
        HostOs::Linux => root.join("seqqc-studio"),
    };
    Ok(directory.join(PROFILE_DATABASE_FILE))
}

#[allow(dead_code)] // Each release target constructs one variant; tests exercise all three.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HostOs {
    Windows,
    MacOs,
    Linux,
}

#[cfg(target_os = "windows")]
const fn current_host_os() -> HostOs {
    HostOs::Windows
}

#[cfg(target_os = "macos")]
const fn current_host_os() -> HostOs {
    HostOs::MacOs
}

#[cfg(target_os = "linux")]
const fn current_host_os() -> HostOs {
    HostOs::Linux
}

fn application_data_root_with<F>(os: HostOs, lookup: F) -> Result<PathBuf, ProjectError>
where
    F: Fn(&str) -> Option<OsString>,
{
    match os {
        HostOs::Windows => lookup("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or(ProjectError::ApplicationDataUnavailable),
        HostOs::MacOs => lookup("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library").join("Application Support"))
            .ok_or(ProjectError::ApplicationDataUnavailable),
        HostOs::Linux => lookup("XDG_DATA_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                lookup("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".local").join("share"))
            })
            .ok_or(ProjectError::ApplicationDataUnavailable),
    }
}

#[cfg(unix)]
fn protect_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn protect_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn protect_file(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)]
fn protect_file(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use seqqc_security::EncryptedProjectStore;

    use super::{
        Assay, CreateProjectRequest, HostOs, LocalProfile, PassphraseProjectManager,
        ProjectContext, ProjectManager, ProjectProtector, ProtectionError,
        application_data_root_with,
    };

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct TestProtector;

    impl ProjectProtector for TestProtector {
        fn create_store(
            &self,
            key_locator: &str,
            database_path: &Path,
        ) -> Result<EncryptedProjectStore, ProtectionError> {
            EncryptedProjectStore::create(database_path, key_locator).map_err(Into::into)
        }

        fn open_store(
            &self,
            key_locator: &str,
            database_path: &Path,
        ) -> Result<EncryptedProjectStore, ProtectionError> {
            EncryptedProjectStore::open(database_path, key_locator).map_err(Into::into)
        }

        fn delete_key(&self, _key_locator: &str) -> Result<(), ProtectionError> {
            Ok(())
        }
    }

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn create() -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "seqqc-projects-{}-{timestamp}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test directory");
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

    fn create_profile(manager: &ProjectManager<TestProtector>) -> LocalProfile {
        manager
            .create_profile("Test scientist")
            .expect("create encrypted profile")
    }

    #[test]
    fn creates_reopens_and_updates_an_encrypted_project() {
        let directory = TestDirectory::create();
        let manager = ProjectManager::new(TestProtector, directory.0.join("profile/profile.db"));
        let mut profile = create_profile(&manager);
        let input = directory.0.join("inputs");
        fs::create_dir(&input).expect("create input directory");
        let mut project = manager
            .create_project(
                &mut profile,
                &CreateProjectRequest {
                    parent_directory: directory.0.clone(),
                    display_name: "Encrypted cohort".to_owned(),
                    assay: Assay::Wgs,
                    context: ProjectContext::HumanClinical,
                    input_root: Some(input.clone()),
                },
            )
            .expect("create project");
        assert_eq!(profile.recent_projects().len(), 1);
        assert_eq!(project.summary().display_name, "Encrypted cohort");
        assert!(project.summary().directory.join("project.json").is_file());
        assert!(project.summary().directory.join("project.db").is_file());

        let project_directory = project.summary().directory.clone();
        project
            .update_metadata(
                "Renamed cohort",
                Assay::Wes,
                ProjectContext::HumanResearch,
                None,
            )
            .expect("update metadata");
        manager
            .refresh_recent(&mut profile, &project)
            .expect("refresh encrypted recent entry");
        drop(project);
        drop(profile);

        let mut profile = manager.open_profile().expect("reopen profile");
        assert_eq!(profile.recent_projects()[0].display_name, "Renamed cohort");
        let reopened = manager
            .open_project(&mut profile, &project_directory)
            .expect("reopen project");
        assert_eq!(reopened.summary().display_name, "Renamed cohort");
        assert_eq!(reopened.summary().assay, Assay::Wes);
        assert_eq!(reopened.summary().input_root, None);
    }

    #[test]
    fn cleartext_manifest_and_directory_do_not_expose_sensitive_metadata() {
        let directory = TestDirectory::create();
        let manager = ProjectManager::new(TestProtector, directory.0.join("profile/profile.db"));
        let mut profile = create_profile(&manager);
        let sensitive = "CLINICAL-SUBJECT-ALPHA-9981";
        let project = manager
            .create_project(
                &mut profile,
                &CreateProjectRequest {
                    parent_directory: directory.0.clone(),
                    display_name: sensitive.to_owned(),
                    assay: Assay::BulkRnaSeq,
                    context: ProjectContext::HumanClinical,
                    input_root: None,
                },
            )
            .expect("create project");
        let manifest = fs::read_to_string(project.summary().directory.join("project.json"))
            .expect("read outer manifest");
        assert!(!manifest.contains(sensitive));
        assert!(
            !project
                .summary()
                .directory
                .file_name()
                .expect("project directory name")
                .to_string_lossy()
                .contains(sensitive)
        );
    }

    #[test]
    fn simulated_platform_paths_cover_windows_macos_and_linux() {
        let variables = HashMap::from([
            (
                "LOCALAPPDATA",
                OsString::from(r"C:\Users\Researcher\AppData\Local"),
            ),
            ("HOME", OsString::from("/Users/researcher")),
            ("XDG_DATA_HOME", OsString::from("/srv/user-data")),
        ]);
        let lookup = |name: &str| variables.get(name).cloned();
        assert_eq!(
            application_data_root_with(HostOs::Windows, lookup).expect("windows root"),
            PathBuf::from(r"C:\Users\Researcher\AppData\Local")
        );
        assert_eq!(
            application_data_root_with(HostOs::MacOs, lookup).expect("mac root"),
            PathBuf::from("/Users/researcher/Library/Application Support")
        );
        assert_eq!(
            application_data_root_with(HostOs::Linux, lookup).expect("linux root"),
            PathBuf::from("/srv/user-data")
        );
    }

    #[test]
    fn linux_falls_back_to_the_freedesktop_user_data_location() {
        let lookup = |name: &str| match name {
            "HOME" => Some(OsString::from("/home/researcher")),
            _ => None,
        };
        assert_eq!(
            application_data_root_with(HostOs::Linux, lookup).expect("linux fallback"),
            PathBuf::from("/home/researcher/.local/share")
        );
    }

    #[test]
    fn passphrase_profile_uses_random_project_keys_and_rejects_wrong_unlock() {
        let directory = TestDirectory::create();
        let manager = PassphraseProjectManager::new(directory.0.join("profile/profile.db"));
        let mut profile = manager
            .create_profile("Offline scientist", "a sufficiently long local passphrase")
            .expect("create passphrase profile");
        let project = manager
            .create_project(
                &mut profile,
                &CreateProjectRequest {
                    parent_directory: directory.0.clone(),
                    display_name: "Vault-backed project".to_owned(),
                    assay: Assay::Wgs,
                    context: ProjectContext::HumanResearch,
                    input_root: None,
                },
            )
            .expect("create profile-vault project");
        let project_directory = project.summary().directory.clone();
        drop(project);
        drop(profile);

        assert!(manager.open_profile("wrong local passphrase").is_err());
        let mut reopened_profile = manager
            .open_profile("a sufficiently long local passphrase")
            .expect("unlock with correct passphrase");
        let reopened_project = manager
            .open_project(&mut reopened_profile, project_directory)
            .expect("open project with encrypted profile-vault key");
        assert_eq!(
            reopened_project.summary().display_name,
            "Vault-backed project"
        );
    }
}
