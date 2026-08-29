//! `SeqQC` Studio's local native application boundary.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use seqqc_discovery::{GroupStatus, InputKind, PairConfidence};
use seqqc_formats::{Compression, SequencingFormat};
use seqqc_reporting::{
    CanonicalReport, EvidenceStatus, ReportFinding, ReportMetric, ReportScope, VersionedId,
    content_digest, export_all, sha256_digest,
};
use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimePolicy {
    offline_only: bool,
    local_api_enabled: bool,
    alignment_platform: &'static str,
    supported_contracts: usize,
    release_stage: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanSummary {
    supported_files: usize,
    ignored_files: usize,
    fastq_files: usize,
    bam_files: usize,
    cram_files: usize,
    groups_needing_review: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanFile {
    relative_path: String,
    kind: &'static str,
    size_bytes: u64,
    sample: String,
    mate: Option<u8>,
    confidence: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanGroup {
    sample: String,
    status: &'static str,
    file_count: usize,
    lane: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResult {
    root: String,
    summary: ScanSummary,
    files: Vec<ScanFile>,
    groups: Vec<ScanGroup>,
    limitations: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectionResult {
    relative_path: String,
    format: &'static str,
    compression: &'static str,
    input_bytes: u64,
    record_count: u64,
    total_bases: u64,
    minimum_read_length: Option<usize>,
    maximum_read_length: Option<usize>,
    mapped_records: u64,
    unmapped_records: u64,
    paired_records: u64,
    reference_sequence_count: usize,
    evidence_label: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportResult {
    directory: String,
    files: Vec<String>,
}

#[tauri::command]
fn runtime_policy() -> RuntimePolicy {
    RuntimePolicy {
        offline_only: true,
        local_api_enabled: false,
        alignment_platform: "linux",
        supported_contracts: seqqc_contracts::V1_SCHEMAS.len(),
        release_stage: "Milestone 0 architecture preview",
    }
}

#[tauri::command]
fn scan_folder(root: String) -> Result<ScanResult, String> {
    let root = PathBuf::from(root);
    let result = seqqc_discovery::discover(&root).map_err(display_error)?;
    let groups_needing_review = result
        .groups
        .iter()
        .filter(|group| {
            matches!(
                group.status,
                GroupStatus::AmbiguousDuplicateMate
                    | GroupStatus::MissingMate
                    | GroupStatus::UnresolvedLayout
            )
        })
        .count();
    let files = result
        .files
        .into_iter()
        .map(|file| ScanFile {
            relative_path: file.relative_path.to_string_lossy().into_owned(),
            kind: input_kind(file.kind),
            size_bytes: file.size_bytes,
            sample: file.proposed_sample,
            mate: file.mate,
            confidence: confidence(file.confidence),
        })
        .collect();
    let groups = result
        .groups
        .into_iter()
        .map(|group| ScanGroup {
            sample: group
                .proposed_sample
                .unwrap_or_else(|| "Unresolved sample".to_owned()),
            status: group_status(group.status),
            file_count: group.files.len() + group.r1.len() + group.r2.len(),
            lane: group.lane,
        })
        .collect();
    Ok(ScanResult {
        root: result.root.to_string_lossy().into_owned(),
        summary: ScanSummary {
            supported_files: result.summary.supported_files,
            ignored_files: result.summary.ignored_files,
            fastq_files: result.summary.fastq_files,
            bam_files: result.summary.bam_files,
            cram_files: result.summary.cram_files,
            groups_needing_review,
        },
        files,
        groups,
        limitations: vec![
            "Milestone 0 inspects file structure and basic streaming evidence only.",
            "No assay-specific scientific QC or clinical interpretation has run.",
        ],
    })
}

#[tauri::command]
fn inspect_input(
    root: String,
    relative_path: String,
    reference_fasta: Option<String>,
) -> Result<InspectionResult, String> {
    let root = PathBuf::from(root);
    let path = resolve_input(&root, &relative_path)?;
    let reference = reference_fasta.map(PathBuf::from);
    let summary =
        seqqc_formats::inspect_path(&path, reference.as_deref()).map_err(display_error)?;
    Ok(InspectionResult {
        relative_path,
        format: format_name(summary.format),
        compression: compression_name(summary.compression),
        input_bytes: summary.input_bytes,
        record_count: summary.record_count,
        total_bases: summary.total_bases,
        minimum_read_length: summary.minimum_read_length,
        maximum_read_length: summary.maximum_read_length,
        mapped_records: summary.mapped_records,
        unmapped_records: summary.unmapped_records,
        paired_records: summary.paired_records,
        reference_sequence_count: summary.reference_sequence_count,
        evidence_label: "Investigational structural evidence",
    })
}

#[tauri::command]
fn export_discovery_report(
    root: String,
    assay: String,
    output_directory: String,
) -> Result<ExportResult, String> {
    let root = PathBuf::from(root);
    let discovery = seqqc_discovery::discover(&root).map_err(display_error)?;
    let report = discovery_report(&discovery, assay)?;
    let paths = export_all(&report, &output_directory).map_err(display_error)?;
    let files = [paths.json, paths.csv, paths.tsv, paths.html, paths.pdf]
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    Ok(ExportResult {
        directory: output_directory,
        files,
    })
}

fn discovery_report(
    discovery: &seqqc_discovery::DiscoveryResult,
    assay: String,
) -> Result<CanonicalReport, String> {
    if !matches!(assay.as_str(), "WGS" | "WES" | "Bulk RNA-seq") {
        return Err("assay must be WGS, WES, or Bulk RNA-seq".to_owned());
    }
    let generated_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(display_error)?;
    let report_id = random_uuid()?;
    let source_run_id = random_uuid()?;
    let scope = ReportScope {
        kind: "run".to_owned(),
        id: source_run_id.clone(),
    };
    let discovery_method = VersionedId {
        id: "seqqc.input-discovery".to_owned(),
        version: "0.1.0".to_owned(),
    };
    let mut source_manifest = String::new();
    for file in &discovery.files {
        let _ = writeln!(
            source_manifest,
            "{}\t{}\t{}",
            file.relative_path.display(),
            input_kind(file.kind),
            file.size_bytes
        );
    }
    let review_groups = discovery
        .groups
        .iter()
        .filter(|group| {
            matches!(
                group.status,
                GroupStatus::AmbiguousDuplicateMate
                    | GroupStatus::MissingMate
                    | GroupStatus::UnresolvedLayout
            )
        })
        .count();
    let structure_status = if discovery.summary.supported_files == 0 || review_groups > 0 {
        EvidenceStatus::NeedsReview
    } else {
        EvidenceStatus::WithinExpectedRange
    };
    let finding = readiness_finding(
        discovery.summary.supported_files,
        review_groups,
        structure_status,
        &scope,
        &generated_at,
    )?;
    let mut report = CanonicalReport {
        schema_version: "1.0.0".to_owned(),
        report_id,
        source_run_id,
        source_manifest_digest: sha256_digest(source_manifest),
        audience: "guided".to_owned(),
        project_context: "unspecified".to_owned(),
        title: "SeqQC Studio input readiness report".to_owned(),
        assay,
        input_root: discovery.root.to_string_lossy().into_owned(),
        metrics: discovery_metrics(
            &discovery.summary,
            review_groups,
            structure_status,
            &scope,
            &discovery_method,
            &generated_at,
        ),
        findings: vec![finding],
        methods: vec![discovery_method],
        limitations: vec![
            "Milestone 0 architecture preview; no assay-specific scientific QC has run.".to_owned(),
            "Results are investigational and must not be used for diagnosis or patient care."
                .to_owned(),
            "Expected-range labels in this report apply only to file-layout readiness.".to_owned(),
        ],
        generated_at,
        content_digest: format!("sha256:{}", "0".repeat(64)),
    };
    report.content_digest = content_digest(&report).map_err(display_error)?;
    Ok(report)
}

fn readiness_finding(
    supported_files: usize,
    review_groups: usize,
    status: EvidenceStatus,
    scope: &ReportScope,
    generated_at: &str,
) -> Result<ReportFinding, String> {
    let needs_review = review_groups > 0;
    Ok(ReportFinding {
        schema_version: "1.0.0".to_owned(),
        finding_id: random_uuid()?,
        rule: VersionedId {
            id: "seqqc.input-readiness".to_owned(),
            version: "0.1.0".to_owned(),
        },
        scope: scope.clone(),
        title: if needs_review {
            "Input mapping needs review"
        } else {
            "Input structure discovered"
        }
        .to_owned(),
        status,
        severity: u8::from(needs_review),
        metric_ids: if needs_review {
            vec![
                "input.files.count".to_owned(),
                "input.groups_needing_review.count".to_owned(),
            ]
        } else {
            vec!["input.files.count".to_owned()]
        },
        threshold_source: VersionedId {
            id: "seqqc.operational-readiness".to_owned(),
            version: "0.1.0".to_owned(),
        },
        evidence_quality: "investigational".to_owned(),
        content_key: if needs_review {
            "finding.input_readiness.needs_review"
        } else {
            "finding.input_readiness.within_range"
        }
        .to_owned(),
        content_version: "1.0.0".to_owned(),
        generated_at: generated_at.to_owned(),
        observation: if needs_review {
            format!("{review_groups} input group(s) have unresolved pairing or layout.")
        } else {
            format!(
                "{supported_files} supported local file(s) were grouped without a detected layout ambiguity."
            )
        },
        next_action: if needs_review {
            "Review sample names and mate assignments before scientific analysis."
        } else {
            "Inspect file-level evidence and configure references before analysis."
        }
        .to_owned(),
    })
}

fn discovery_metrics(
    summary: &seqqc_discovery::DiscoverySummary,
    review_groups: usize,
    status: EvidenceStatus,
    scope: &ReportScope,
    method: &VersionedId,
    generated_at: &str,
) -> Vec<ReportMetric> {
    [
        (
            "input.files.count",
            "Supported input files",
            summary.supported_files,
            status,
        ),
        (
            "input.fastq_files.count",
            "FASTQ files",
            summary.fastq_files,
            EvidenceStatus::NotEvaluated,
        ),
        (
            "input.alignment_files.count",
            "BAM and CRAM files",
            summary.bam_files + summary.cram_files,
            EvidenceStatus::NotEvaluated,
        ),
        (
            "input.groups_needing_review.count",
            "Groups needing review",
            review_groups,
            status,
        ),
    ]
    .into_iter()
    .map(|(id, label, value, metric_status)| {
        count_metric(id, label, value, metric_status, scope, method, generated_at)
    })
    .collect()
}

fn count_metric(
    id: &str,
    label: &str,
    value: usize,
    status: EvidenceStatus,
    scope: &ReportScope,
    method: &VersionedId,
    generated_at: &str,
) -> ReportMetric {
    ReportMetric {
        schema_version: "1.0.0".to_owned(),
        id: id.to_owned(),
        scope: scope.clone(),
        label: label.to_owned(),
        // Report values use IEEE-754 numbers; discovery counts are practical
        // filesystem counts and remain far below the exact-integer boundary.
        #[allow(clippy::cast_precision_loss)]
        value: value as f64,
        value_type: "integer".to_owned(),
        unit: "files".to_owned(),
        method: method.clone(),
        source_artifact_ids: Vec::new(),
        applicability: "applicable".to_owned(),
        quality: "investigational".to_owned(),
        computed_at: generated_at.to_owned(),
        status,
        explanation: "Operational discovery evidence; not a biological QC measurement.".to_owned(),
    }
}

fn random_uuid() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| "operating system random source is unavailable".to_owned())?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    ))
}

fn resolve_input(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative.is_absolute() {
        return Err("input path must be relative to the selected folder".to_owned());
    }
    let root = root.canonicalize().map_err(display_error)?;
    let candidate = root.join(relative).canonicalize().map_err(display_error)?;
    if !candidate.starts_with(&root) || !candidate.is_file() {
        return Err("input path is outside the selected folder or is not a file".to_owned());
    }
    Ok(candidate)
}

const fn input_kind(kind: InputKind) -> &'static str {
    match kind {
        InputKind::Fastq => "FASTQ",
        InputKind::Bam => "BAM",
        InputKind::Cram => "CRAM",
    }
}

const fn confidence(value: PairConfidence) -> &'static str {
    match value {
        PairConfidence::High => "High",
        PairConfidence::Medium => "Medium",
        PairConfidence::Unresolved => "Unresolved",
    }
}

const fn group_status(status: GroupStatus) -> &'static str {
    match status {
        GroupStatus::PairedCandidate => "Paired candidate",
        GroupStatus::AmbiguousDuplicateMate => "Duplicate mate ambiguity",
        GroupStatus::MissingMate => "Missing mate",
        GroupStatus::AlignedInput => "Aligned input",
        GroupStatus::UnresolvedLayout => "Unresolved layout",
    }
}

const fn format_name(format: SequencingFormat) -> &'static str {
    match format {
        SequencingFormat::Fastq => "FASTQ",
        SequencingFormat::Bam => "BAM",
        SequencingFormat::Cram => "CRAM",
    }
}

const fn compression_name(compression: Compression) -> &'static str {
    match compression {
        Compression::None => "None",
        Compression::Gzip => "Gzip",
        Compression::Bgzf => "BGZF",
        Compression::CramInternal => "CRAM internal",
    }
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Starts the native desktop runtime.
///
/// # Panics
///
/// Panics when Tauri cannot initialize or the native event loop exits with an
/// unrecoverable runtime error.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            runtime_policy,
            scan_folder,
            inspect_input,
            export_discovery_report
        ])
        .run(tauri::generate_context!())
        .expect("SeqQC Studio runtime failed");
}

#[cfg(test)]
mod tests {
    use super::{discovery_report, resolve_input, runtime_policy};
    use std::path::Path;

    #[test]
    fn runtime_defaults_are_offline_and_local_api_is_disabled() {
        let policy = runtime_policy();
        assert!(policy.offline_only);
        assert!(!policy.local_api_enabled);
        assert_eq!(policy.alignment_platform, "linux");
        assert_eq!(policy.supported_contracts, 9);
    }

    #[test]
    fn synthetic_discovery_builds_an_investigational_report() {
        let fixture_root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../test-data/synthetic/tiny-formats");
        let discovery = seqqc_discovery::discover(&fixture_root).expect("discover fixtures");
        let report = discovery_report(&discovery, "WGS".to_owned()).expect("build report");
        assert_eq!(report.assay, "WGS");
        assert!(report.metrics.iter().all(|metric| {
            metric
                .explanation
                .contains("not a biological QC measurement")
        }));
        assert!(
            report
                .limitations
                .iter()
                .any(|item| item.contains("must not be used for diagnosis"))
        );
    }

    #[test]
    fn input_resolution_rejects_absolute_paths() {
        let fixture_root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../test-data/synthetic/tiny-formats");
        let absolute = fixture_root.join("tiny.fastq");
        assert!(resolve_input(&fixture_root, absolute.to_string_lossy().as_ref()).is_err());
    }
}
