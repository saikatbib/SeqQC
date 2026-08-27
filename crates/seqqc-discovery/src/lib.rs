//! Deterministic, recursive discovery of local sequencing inputs.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputKind {
    Fastq,
    Bam,
    Cram,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PairConfidence {
    High,
    Medium,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredFile {
    pub relative_path: PathBuf,
    pub kind: InputKind,
    pub extension: &'static str,
    pub size_bytes: u64,
    pub mate: Option<u8>,
    pub pair_key: Option<String>,
    pub proposed_sample: String,
    pub lane: Option<String>,
    pub chunk: Option<String>,
    pub confidence: PairConfidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IgnoredEntry {
    pub relative_path: PathBuf,
    pub reason: IgnoreReason,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IgnoreReason {
    SymbolicLink,
    UnsupportedExtension,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupStatus {
    PairedCandidate,
    AmbiguousDuplicateMate,
    MissingMate,
    AlignedInput,
    UnresolvedLayout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputGroup {
    pub kind: InputKind,
    pub pair_key: Option<String>,
    pub proposed_sample: Option<String>,
    pub lane: Option<String>,
    pub r1: Vec<PathBuf>,
    pub r2: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
    pub confidence: PairConfidence,
    pub status: GroupStatus,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DiscoverySummary {
    pub supported_files: usize,
    pub ignored_files: usize,
    pub fastq_files: usize,
    pub bam_files: usize,
    pub cram_files: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryResult {
    pub root: PathBuf,
    pub files: Vec<DiscoveredFile>,
    pub ignored: Vec<IgnoredEntry>,
    pub groups: Vec<InputGroup>,
    pub summary: DiscoverySummary,
}

#[derive(Clone, Debug)]
struct FastqIdentity {
    mate: Option<u8>,
    pair_key: Option<String>,
    proposed_sample: String,
    lane: Option<String>,
    chunk: Option<String>,
    confidence: PairConfidence,
}

/// Recursively discovers supported inputs without following symbolic links.
///
/// # Errors
/// Returns an I/O error when the root is not a directory or cannot be read.
pub fn discover(root_path: impl AsRef<Path>) -> io::Result<DiscoveryResult> {
    discover_with_cancel(root_path, || false)
}

/// Recursively discovers inputs while consulting a cooperative cancellation
/// callback before each directory entry.
///
/// # Errors
///
/// Returns [`io::ErrorKind::Interrupted`] when cancellation is requested, or an
/// I/O error when the root or an entry cannot be read.
pub fn discover_with_cancel<F>(
    root_path: impl AsRef<Path>,
    mut is_cancelled: F,
) -> io::Result<DiscoveryResult>
where
    F: FnMut() -> bool,
{
    let root = fs::canonicalize(root_path)?;
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "discovery root must be a directory",
        ));
    }

    let mut files = Vec::new();
    let mut ignored = Vec::new();
    walk(&root, &root, &mut files, &mut ignored, &mut is_cancelled)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    ignored.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let groups = build_groups(&files);
    let summary = DiscoverySummary {
        supported_files: files.len(),
        ignored_files: ignored.len(),
        fastq_files: files
            .iter()
            .filter(|file| file.kind == InputKind::Fastq)
            .count(),
        bam_files: files
            .iter()
            .filter(|file| file.kind == InputKind::Bam)
            .count(),
        cram_files: files
            .iter()
            .filter(|file| file.kind == InputKind::Cram)
            .count(),
    };

    Ok(DiscoveryResult {
        root,
        files,
        ignored,
        groups,
        summary,
    })
}

fn walk(
    directory: &Path,
    root: &Path,
    files: &mut Vec<DiscoveredFile>,
    ignored: &mut Vec<IgnoredEntry>,
    is_cancelled: &mut impl FnMut() -> bool,
) -> io::Result<()> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        if is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "sequencing file discovery was cancelled",
            ));
        }
        let path = entry.path();
        let relative_path = path
            .strip_prefix(root)
            .map_or_else(|_| path.clone(), Path::to_path_buf);
        let file_type = entry.file_type()?;

        if file_type.is_symlink() {
            ignored.push(IgnoredEntry {
                relative_path,
                reason: IgnoreReason::SymbolicLink,
            });
        } else if file_type.is_dir() {
            walk(&path, root, files, ignored, is_cancelled)?;
        } else if file_type.is_file() {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                ignored.push(IgnoredEntry {
                    relative_path,
                    reason: IgnoreReason::UnsupportedExtension,
                });
                continue;
            };
            let Some((kind, extension, stem)) = classify_file(name) else {
                ignored.push(IgnoredEntry {
                    relative_path,
                    reason: IgnoreReason::UnsupportedExtension,
                });
                continue;
            };

            let identity = if kind == InputKind::Fastq {
                fastq_identity(
                    stem,
                    relative_path.parent().unwrap_or_else(|| Path::new("")),
                )
            } else {
                FastqIdentity {
                    mate: None,
                    pair_key: None,
                    proposed_sample: stem.to_owned(),
                    lane: None,
                    chunk: None,
                    confidence: PairConfidence::Unresolved,
                }
            };
            files.push(DiscoveredFile {
                relative_path,
                kind,
                extension,
                size_bytes: entry.metadata()?.len(),
                mate: identity.mate,
                pair_key: identity.pair_key,
                proposed_sample: identity.proposed_sample,
                lane: identity.lane,
                chunk: identity.chunk,
                confidence: identity.confidence,
            });
        }
    }
    Ok(())
}

fn classify_file(name: &str) -> Option<(InputKind, &'static str, &str)> {
    const EXTENSIONS: [(&str, InputKind); 6] = [
        (".fastq.gz", InputKind::Fastq),
        (".fq.gz", InputKind::Fastq),
        (".fastq", InputKind::Fastq),
        (".fq", InputKind::Fastq),
        (".bam", InputKind::Bam),
        (".cram", InputKind::Cram),
    ];
    let lower = name.to_ascii_lowercase();
    EXTENSIONS.iter().find_map(|(extension, kind)| {
        lower
            .ends_with(extension)
            .then(|| (*kind, *extension, &name[..name.len() - extension.len()]))
    })
}

fn fastq_identity(stem: &str, relative_directory: &Path) -> FastqIdentity {
    if let Some((pair_stem, mate, chunk, confidence)) = split_mate(stem) {
        let lane = find_lane(pair_stem);
        let proposed_sample = sample_stem(pair_stem);
        let pair_path = relative_directory.join(pair_stem);
        return FastqIdentity {
            mate: Some(mate),
            pair_key: Some(pair_path.to_string_lossy().to_lowercase()),
            proposed_sample,
            lane,
            chunk,
            confidence,
        };
    }
    FastqIdentity {
        mate: None,
        pair_key: None,
        proposed_sample: stem.to_owned(),
        lane: None,
        chunk: None,
        confidence: PairConfidence::Unresolved,
    }
}

fn split_mate(stem: &str) -> Option<(&str, u8, Option<String>, PairConfidence)> {
    let lower = stem.to_ascii_lowercase();
    for mate in [1_u8, 2] {
        for separator in ['_', '-', '.'] {
            let marker = format!("{separator}r{mate}");
            if let Some(index) = lower.rfind(&marker) {
                let tail = &stem[index + marker.len()..];
                if tail.is_empty() {
                    return Some((&stem[..index], mate, None, PairConfidence::High));
                }
                if tail.len() == 4
                    && matches!(tail.as_bytes()[0], b'_' | b'-' | b'.')
                    && tail[1..].bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Some((
                        &stem[..index],
                        mate,
                        Some(tail[1..].to_owned()),
                        PairConfidence::High,
                    ));
                }
            }
        }
        for separator in ['_', '-', '.'] {
            let marker = format!("{separator}{mate}");
            if lower.ends_with(&marker) {
                return Some((
                    &stem[..stem.len() - marker.len()],
                    mate,
                    None,
                    PairConfidence::Medium,
                ));
            }
        }
    }
    None
}

fn find_lane(pair_stem: &str) -> Option<String> {
    pair_stem
        .split(['_', '-'])
        .find(|part| {
            part.len() == 4
                && part.starts_with(['L', 'l'])
                && part[1..].bytes().all(|byte| byte.is_ascii_digit())
        })
        .map(str::to_ascii_uppercase)
}

fn sample_stem(pair_stem: &str) -> String {
    let lower = pair_stem.to_ascii_lowercase();
    let Some(lane_index) = lower.rfind("_l") else {
        return pair_stem.to_owned();
    };
    let lane_tail = &pair_stem[lane_index + 2..];
    if lane_tail.len() != 3 || !lane_tail.bytes().all(|byte| byte.is_ascii_digit()) {
        return pair_stem.to_owned();
    }

    let before_lane = &pair_stem[..lane_index];
    let before_lower = before_lane.to_ascii_lowercase();
    if let Some(sample_number_index) = before_lower.rfind("_s") {
        let sample_number = &before_lane[sample_number_index + 2..];
        if !sample_number.is_empty() && sample_number.bytes().all(|byte| byte.is_ascii_digit()) {
            return before_lane[..sample_number_index].to_owned();
        }
    }
    before_lane.to_owned()
}

fn build_groups(files: &[DiscoveredFile]) -> Vec<InputGroup> {
    let mut fastq_groups: BTreeMap<&str, InputGroup> = BTreeMap::new();
    let mut remaining = Vec::new();

    for file in files {
        match file.kind {
            InputKind::Bam | InputKind::Cram => remaining.push(InputGroup {
                kind: file.kind,
                pair_key: None,
                proposed_sample: Some(file.proposed_sample.clone()),
                lane: None,
                r1: Vec::new(),
                r2: Vec::new(),
                files: vec![file.relative_path.clone()],
                confidence: PairConfidence::Unresolved,
                status: GroupStatus::AlignedInput,
            }),
            InputKind::Fastq => {
                let (Some(pair_key), Some(mate)) = (file.pair_key.as_deref(), file.mate) else {
                    remaining.push(InputGroup {
                        kind: InputKind::Fastq,
                        pair_key: None,
                        proposed_sample: Some(file.proposed_sample.clone()),
                        lane: file.lane.clone(),
                        r1: Vec::new(),
                        r2: Vec::new(),
                        files: vec![file.relative_path.clone()],
                        confidence: PairConfidence::Unresolved,
                        status: GroupStatus::UnresolvedLayout,
                    });
                    continue;
                };
                let group = fastq_groups.entry(pair_key).or_insert_with(|| InputGroup {
                    kind: InputKind::Fastq,
                    pair_key: Some(pair_key.to_owned()),
                    proposed_sample: Some(file.proposed_sample.clone()),
                    lane: file.lane.clone(),
                    r1: Vec::new(),
                    r2: Vec::new(),
                    files: Vec::new(),
                    confidence: file.confidence,
                    status: GroupStatus::MissingMate,
                });
                if mate == 1 {
                    group.r1.push(file.relative_path.clone());
                } else {
                    group.r2.push(file.relative_path.clone());
                }
            }
        }
    }

    let mut groups = fastq_groups
        .into_values()
        .map(|mut group| {
            group.status = if group.r1.len() == 1 && group.r2.len() == 1 {
                GroupStatus::PairedCandidate
            } else if group.r1.len() > 1 || group.r2.len() > 1 {
                GroupStatus::AmbiguousDuplicateMate
            } else {
                GroupStatus::MissingMate
            };
            group
        })
        .collect::<Vec<_>>();
    remaining.sort_by(|left, right| left.files.cmp(&right.files));
    groups.extend(remaining);
    groups
}

#[cfg(test)]
mod tests {
    use super::{GroupStatus, InputKind, PairConfidence, discover, discover_with_cancel};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock must be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "seqqc-discovery-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test directory must be created");
            Self(path)
        }

        fn touch(&self, relative: &str) {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture parent must be created");
            }
            fs::write(path, b"synthetic fixture").expect("fixture must be written");
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn discovers_nested_inputs_and_proposes_common_pairs() {
        let root = TestDirectory::new("paired");
        root.touch("run/sample_S1_L001_R1_001.fastq.gz");
        root.touch("run/sample_S1_L001_R2_001.fastq.gz");
        root.touch("aligned/tumor.BAM");
        root.touch("notes.txt");

        let result = discover(&root.0).expect("discovery must succeed");
        assert_eq!(result.summary.supported_files, 3);
        assert_eq!(result.summary.fastq_files, 2);
        assert_eq!(result.summary.bam_files, 1);
        assert_eq!(result.summary.ignored_files, 1);

        let pair = result
            .groups
            .iter()
            .find(|group| group.kind == InputKind::Fastq)
            .expect("FASTQ group must exist");
        assert_eq!(pair.status, GroupStatus::PairedCandidate);
        assert_eq!(pair.proposed_sample.as_deref(), Some("sample"));
        assert_eq!(pair.lane.as_deref(), Some("L001"));
        assert_eq!(pair.confidence, PairConfidence::High);
    }

    #[test]
    fn reports_missing_mates_and_unresolved_layouts() {
        let root = TestDirectory::new("missing");
        root.touch("sample_R1.fq");
        root.touch("mystery.fastq");

        let result = discover(&root.0).expect("discovery must succeed");
        let statuses = result
            .groups
            .iter()
            .map(|group| group.status)
            .collect::<Vec<_>>();
        assert!(statuses.contains(&GroupStatus::MissingMate));
        assert!(statuses.contains(&GroupStatus::UnresolvedLayout));
    }

    #[test]
    fn never_silently_resolves_duplicate_mates() {
        let root = TestDirectory::new("duplicate");
        root.touch("sample_R1.fastq");
        root.touch("sample_r1.fq.gz");
        root.touch("sample_R2.fastq");

        let result = discover(&root.0).expect("discovery must succeed");
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].status, GroupStatus::AmbiguousDuplicateMate);
        assert_eq!(result.groups[0].r1.len(), 2);
        assert_eq!(result.groups[0].r2.len(), 1);
    }

    #[test]
    fn handles_unicode_spaces_leading_dashes_and_case_insensitive_extensions() {
        let root = TestDirectory::new("path-edges");
        root.touch("run α/--contrôle sample_R1.FASTQ.GZ");
        root.touch("run α/--contrôle sample_R2.fq.gz");
        root.touch("aligned data/病例.CRAM");

        let result = discover(&root.0).expect("discovery must succeed");
        assert_eq!(result.summary.supported_files, 3);
        assert_eq!(result.summary.fastq_files, 2);
        assert_eq!(result.summary.cram_files, 1);
        assert_eq!(result.groups[0].status, GroupStatus::PairedCandidate);
        assert_eq!(
            result.groups[0].proposed_sample.as_deref(),
            Some("--contrôle sample")
        );
    }

    #[test]
    fn ignores_symbolic_links_when_the_platform_allows_creating_them() {
        let root = TestDirectory::new("symlink");
        root.touch("real.bam");
        let link = root.0.join("linked.bam");
        if !create_file_symlink(&root.0.join("real.bam"), &link) {
            eprintln!("symlink creation is unavailable on this host; assertion not exercised");
            return;
        }
        eprintln!("symlink creation succeeded; ignore assertion exercised");

        let result = discover(&root.0).expect("discovery must succeed");
        assert_eq!(result.summary.supported_files, 1);
        assert_eq!(result.summary.ignored_files, 1);
        assert_eq!(result.ignored[0].reason, super::IgnoreReason::SymbolicLink);
    }

    #[test]
    fn cooperative_cancellation_stops_a_large_walk() {
        let root = TestDirectory::new("cancel");
        for index in 0..100 {
            root.touch(&format!("batch-{index:03}/sample_R1.fastq"));
        }
        let mut checks = 0_u32;
        let result = discover_with_cancel(&root.0, || {
            checks += 1;
            checks > 12
        });
        assert!(matches!(
            result,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted
        ));
        assert_eq!(checks, 13);
    }

    #[cfg(unix)]
    fn create_file_symlink(target: &std::path::Path, link: &std::path::Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    #[cfg(windows)]
    fn create_file_symlink(target: &std::path::Path, link: &std::path::Path) -> bool {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
}
