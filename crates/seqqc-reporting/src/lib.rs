//! Canonical, standalone report model and Milestone 0 exporters for `SeqQC`.

use std::fmt::{self, Write as _};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Evidence status used consistently by every report representation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    WithinExpectedRange,
    NeedsReview,
    NotEvaluated,
}

impl EvidenceStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::WithinExpectedRange => "Within expected range",
            Self::NeedsReview => "Needs review",
            Self::NotEvaluated => "Not evaluated",
        }
    }
}

/// One normalized metric shown in every export format.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReportMetric {
    pub schema_version: String,
    #[serde(rename = "metric_id")]
    pub id: String,
    pub scope: ReportScope,
    pub label: String,
    pub value: f64,
    pub value_type: String,
    pub unit: String,
    pub method: VersionedId,
    pub source_artifact_ids: Vec<String>,
    pub applicability: String,
    pub quality: String,
    pub computed_at: String,
    pub status: EvidenceStatus,
    pub explanation: String,
}

/// Versioned identifier shared by methods, rules, and threshold sources.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VersionedId {
    pub id: String,
    pub version: String,
}

/// Scope attached to a canonical metric or finding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReportScope {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
}

/// One normalized report finding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReportFinding {
    pub schema_version: String,
    pub finding_id: String,
    pub rule: VersionedId,
    pub scope: ReportScope,
    pub title: String,
    pub status: EvidenceStatus,
    pub severity: u8,
    pub metric_ids: Vec<String>,
    pub threshold_source: VersionedId,
    pub evidence_quality: String,
    pub content_key: String,
    pub content_version: String,
    pub generated_at: String,
    pub observation: String,
    pub next_action: String,
}

/// Application-owned canonical report used by the dashboard and exporters.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CanonicalReport {
    pub schema_version: String,
    pub report_id: String,
    pub source_run_id: String,
    pub source_manifest_digest: String,
    pub audience: String,
    pub project_context: String,
    pub title: String,
    pub assay: String,
    pub input_root: String,
    pub findings: Vec<ReportFinding>,
    pub metrics: Vec<ReportMetric>,
    pub methods: Vec<VersionedId>,
    pub limitations: Vec<String>,
    pub generated_at: String,
    pub content_digest: String,
}

/// Paths written by [`export_all`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportPaths {
    pub json: PathBuf,
    pub csv: PathBuf,
    pub tsv: PathBuf,
    pub html: PathBuf,
    pub pdf: PathBuf,
}

/// Writes JSON, CSV, TSV, standalone HTML, and PDF from one report model.
///
/// # Errors
///
/// Returns an I/O or serialization error when the output directory or an
/// export cannot be written.
pub fn export_all(
    report: &CanonicalReport,
    directory: impl AsRef<Path>,
) -> io::Result<ExportPaths> {
    let directory = directory.as_ref();
    fs::create_dir_all(directory)?;
    let paths = ExportPaths {
        json: directory.join("report.json"),
        csv: directory.join("metrics.csv"),
        tsv: directory.join("metrics.tsv"),
        html: directory.join("report.html"),
        pdf: directory.join("report.pdf"),
    };

    let json = serde_json::to_vec_pretty(report).map_err(invalid_data)?;
    fs::write(&paths.json, json)?;
    fs::write(&paths.csv, delimited_metrics(report, ','))?;
    fs::write(&paths.tsv, delimited_metrics(report, '\t'))?;
    fs::write(&paths.html, standalone_html(report))?;
    fs::write(&paths.pdf, simple_pdf(report))?;
    Ok(paths)
}

/// Calculates the canonical SHA-256 content digest with the digest field
/// normalized to 64 zeroes, avoiding a self-referential value.
///
/// # Errors
///
/// Returns a serialization error if the report cannot be encoded as JSON.
pub fn content_digest(report: &CanonicalReport) -> Result<String, serde_json::Error> {
    let mut normalized = report.clone();
    normalized.content_digest = format!("sha256:{}", "0".repeat(64));
    let bytes = serde_json::to_vec(&normalized)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

/// Calculates a prefixed SHA-256 digest for source-manifest evidence.
#[must_use]
pub fn sha256_digest(bytes: impl AsRef<[u8]>) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes.as_ref()))
}

/// Produces a self-contained HTML report with no remote assets or scripts.
#[must_use]
pub fn standalone_html(report: &CanonicalReport) -> String {
    let mut metrics = String::new();
    for metric in &report.metrics {
        let _ = write!(
            metrics,
            "<tr><td><code>{}</code><strong>{}</strong></td><td>{} {}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&metric.id),
            escape_html(&metric.label),
            metric.value,
            escape_html(&metric.unit),
            escape_html(metric.status.label()),
            escape_html(&metric.explanation)
        );
    }
    let mut findings = String::new();
    for finding in &report.findings {
        let _ = write!(
            findings,
            "<article><span>{}</span><h3>{}</h3><p>{}</p><small>Next: {}</small></article>",
            escape_html(finding.status.label()),
            escape_html(&finding.title),
            escape_html(&finding.observation),
            escape_html(&finding.next_action)
        );
    }
    let mut limitations = String::new();
    for item in &report.limitations {
        let _ = write!(limitations, "<li>{}</li>", escape_html(item));
    }

    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src data:\"><title>{title}</title><style>{styles}</style></head><body><header><small>SEQQC STUDIO · STANDALONE REPORT</small><h1>{title}</h1><p>{assay} · {root}</p></header><main><section><h2>Findings</h2><div class=\"findings\">{findings}</div></section><section><h2>Metrics</h2><table><thead><tr><th>Metric</th><th>Value</th><th>Interpretation</th><th>Explanation</th></tr></thead><tbody>{metrics}</tbody></table></section><section><h2>Limitations</h2><ul>{limitations}</ul></section></main><footer>Generated {generated} · Schema {schema} · {report_id}</footer></body></html>",
        title = escape_html(&report.title),
        assay = escape_html(&report.assay),
        root = escape_html(&report.input_root),
        generated = escape_html(&report.generated_at),
        schema = escape_html(&report.schema_version),
        report_id = escape_html(&report.report_id),
        styles = HTML_STYLES,
    )
}

fn delimited_metrics(report: &CanonicalReport, delimiter: char) -> String {
    let mut output = String::new();
    let separator = delimiter.to_string();
    let _ = writeln!(
        output,
        "metric_id{delimiter}label{delimiter}value{delimiter}unit{delimiter}status{delimiter}explanation"
    );
    for metric in &report.metrics {
        let fields = [
            metric.id.clone(),
            metric.label.clone(),
            metric.value.to_string(),
            metric.unit.clone(),
            metric.status.label().to_owned(),
            metric.explanation.clone(),
        ];
        let row = fields
            .iter()
            .map(|field| quote_delimited(field, delimiter))
            .collect::<Vec<_>>()
            .join(&separator);
        let _ = writeln!(output, "{row}");
    }
    output
}

fn quote_delimited(value: &str, delimiter: char) -> String {
    if value.contains([delimiter, '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn simple_pdf(report: &CanonicalReport) -> Vec<u8> {
    let mut lines = vec![
        "SeqQC Studio - Standalone QC Report".to_owned(),
        report.title.clone(),
        format!("Assay: {}", report.assay),
        format!("Input: {}", report.input_root),
        format!("Generated: {}", report.generated_at),
        String::new(),
        "Metrics".to_owned(),
    ];
    lines.extend(report.metrics.iter().take(24).map(|metric| {
        format!(
            "{} | {}: {} {} - {}",
            metric.id,
            metric.label,
            metric.value,
            metric.unit,
            metric.status.label()
        )
    }));
    lines.push(String::new());
    lines.push("Limitations".to_owned());
    lines.extend(report.limitations.iter().take(8).cloned());

    let mut stream = String::from("BT /F1 11 Tf 48 792 Td 14 TL ");
    for line in lines {
        let clean = ascii_pdf_text(&line);
        let _ = write!(stream, "({clean}) Tj T* ");
    }
    stream.push_str("ET");

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{stream}\nendstream", stream.len()),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer << /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn ascii_pdf_text(value: &str) -> String {
    value
        .chars()
        .map(|character| if character.is_ascii() { character } else { '?' })
        .collect::<String>()
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn invalid_data(error: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

const HTML_STYLES: &str = "body{margin:0;background:#07131a;color:#e7f5f4;font:14px system-ui,sans-serif}header,main,footer{max-width:1100px;margin:auto;padding:32px}header{background:linear-gradient(120deg,#0e2029,#0b3938);border-radius:0 0 24px 24px}small{color:#83f6e2}h1{font-size:36px;margin:8px 0}h2{margin-top:36px}.findings{display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:14px}.findings article,table{background:#0e2029;border:1px solid #24404a;border-radius:12px}.findings article{padding:18px}.findings span{color:#66ead6;font-size:11px;text-transform:uppercase}.findings h3{margin:8px 0}.findings p{color:#a9bdc1}table{width:100%;border-collapse:collapse;overflow:hidden}th,td{padding:12px;border-bottom:1px solid #24404a;text-align:left}code,strong{display:block}code{color:#83f6e2;font-size:10px}footer{color:#8ea8ae;font-size:11px}@media print{body{background:white;color:#102631}header,.findings article,table{background:white}}";

impl fmt::Display for EvidenceStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}
