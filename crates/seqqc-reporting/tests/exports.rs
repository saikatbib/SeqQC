use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use seqqc_reporting::{CanonicalReport, EvidenceStatus, ReportFinding, ReportMetric, export_all};
use seqqc_reporting::{ReportScope, VersionedId, content_digest};

#[test]
fn every_export_is_derived_from_the_same_metric_values() {
    let output = temporary_directory();
    let report = synthetic_report();
    let paths = export_all(&report, &output).expect("all exports must be written");

    let json: CanonicalReport =
        serde_json::from_slice(&fs::read(paths.json).expect("read JSON")).expect("parse JSON");
    assert_eq!(json, report);
    for path in [&paths.csv, &paths.tsv, &paths.html, &paths.pdf] {
        assert!(path.is_file(), "missing export {}", path.display());
        assert!(fs::metadata(path).expect("read metadata").len() > 50);
    }

    let csv = fs::read_to_string(paths.csv).expect("read CSV");
    let tsv = fs::read_to_string(paths.tsv).expect("read TSV");
    let html = fs::read_to_string(paths.html).expect("read HTML");
    let pdf = fs::read(paths.pdf).expect("read PDF");
    for expected in ["input.files.count", "3", "Needs review"] {
        assert!(csv.contains(expected));
        assert!(tsv.contains(expected));
        assert!(html.contains(expected));
        assert!(String::from_utf8_lossy(&pdf).contains(expected));
    }
    assert!(html.contains("default-src 'none'"));
    assert!(!html.contains("http://"));
    assert!(!html.contains("https://"));
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(pdf.ends_with(b"%%EOF\n"));

    fs::remove_dir_all(output).expect("remove export directory");
}

fn synthetic_report() -> CanonicalReport {
    let timestamp = "2026-08-26T00:00:00Z".to_owned();
    let scope = ReportScope {
        kind: "project".to_owned(),
        id: "11111111-1111-4111-8111-111111111111".to_owned(),
    };
    let method = VersionedId {
        id: "seqqc.input-discovery".to_owned(),
        version: "0.1.0".to_owned(),
    };
    let mut report = CanonicalReport {
        schema_version: "1.0.0".to_owned(),
        report_id: "22222222-2222-4222-8222-222222222222".to_owned(),
        source_run_id: "33333333-3333-4333-8333-333333333333".to_owned(),
        source_manifest_digest: format!("sha256:{}", "a".repeat(64)),
        audience: "guided".to_owned(),
        project_context: "unspecified".to_owned(),
        title: "Synthetic project QC".to_owned(),
        assay: "WGS".to_owned(),
        input_root: "synthetic/local inputs".to_owned(),
        metrics: vec![ReportMetric {
            schema_version: "1.0.0".to_owned(),
            id: "input.files.count".to_owned(),
            scope: scope.clone(),
            label: "Input files".to_owned(),
            value: 3.0,
            value_type: "integer".to_owned(),
            unit: "files".to_owned(),
            method: method.clone(),
            source_artifact_ids: Vec::new(),
            applicability: "applicable".to_owned(),
            quality: "investigational".to_owned(),
            computed_at: timestamp.clone(),
            status: EvidenceStatus::NeedsReview,
            explanation: "Discovery evidence only; scientific QC has not run.".to_owned(),
        }],
        findings: vec![ReportFinding {
            schema_version: "1.0.0".to_owned(),
            finding_id: "44444444-4444-4444-8444-444444444444".to_owned(),
            rule: VersionedId {
                id: "seqqc.input-readiness".to_owned(),
                version: "0.1.0".to_owned(),
            },
            scope,
            title: "Input mapping needs review".to_owned(),
            status: EvidenceStatus::NeedsReview,
            severity: 1,
            metric_ids: vec!["input.files.count".to_owned()],
            threshold_source: VersionedId {
                id: "seqqc.operational-readiness".to_owned(),
                version: "0.1.0".to_owned(),
            },
            evidence_quality: "investigational".to_owned(),
            content_key: "finding.input_readiness.needs_review".to_owned(),
            content_version: "1.0.0".to_owned(),
            generated_at: timestamp.clone(),
            observation: "One synthetic group is unresolved.".to_owned(),
            next_action: "Confirm sample pairing before analysis.".to_owned(),
        }],
        methods: vec![method],
        limitations: vec!["Milestone 0 synthetic evidence; not for diagnosis.".to_owned()],
        generated_at: timestamp,
        content_digest: format!("sha256:{}", "0".repeat(64)),
    };
    report.content_digest = content_digest(&report).expect("digest report");
    report
}

fn temporary_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock must be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("seqqc-reporting-{}-{nonce}", std::process::id()))
}
