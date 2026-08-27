import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";

type Theme = "light" | "dark" | "contrast";
type Audience = "guided" | "standard" | "expert";
type Assay = "WGS" | "WES" | "Bulk RNA-seq";

type ScanSummary = {
  supportedFiles: number;
  ignoredFiles: number;
  fastqFiles: number;
  bamFiles: number;
  cramFiles: number;
  groupsNeedingReview: number;
};

type ScanFile = {
  relativePath: string;
  kind: "FASTQ" | "BAM" | "CRAM";
  sizeBytes: number;
  sample: string;
  mate: number | null;
  confidence: string;
};

type ScanGroup = {
  sample: string;
  status: string;
  fileCount: number;
  lane: string | null;
};

type ScanResult = {
  root: string;
  summary: ScanSummary;
  files: ScanFile[];
  groups: ScanGroup[];
  limitations: string[];
};

type InspectionResult = {
  relativePath: string;
  format: string;
  compression: string;
  inputBytes: number;
  recordCount: number;
  totalBases: number;
  minimumReadLength: number | null;
  maximumReadLength: number | null;
  mappedRecords: number;
  unmappedRecords: number;
  pairedRecords: number;
  referenceSequenceCount: number;
  evidenceLabel: string;
};

type ExportResult = { directory: string; files: string[] };

const themeLabels: Record<Theme, string> = {
  light: "Clinical light",
  dark: "Clinical night",
  contrast: "High contrast",
};

function initialTheme(): Theme {
  const stored = localStorage.getItem("seqqc-theme");
  if (stored === "light" || stored === "dark" || stored === "contrast") return stored;
  return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`;
}

function readableError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function App() {
  const [theme, setTheme] = useState<Theme>(initialTheme);
  const [audience, setAudience] = useState<Audience>("guided");
  const [assay, setAssay] = useState<Assay>("WGS");
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [inspection, setInspection] = useState<InspectionResult | null>(null);
  const [referenceFasta, setReferenceFasta] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [exportResult, setExportResult] = useState<ExportResult | null>(null);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("seqqc-theme", theme);
  }, [theme]);

  const reviewGroups = useMemo(
    () => scan?.groups.filter((group) => !["Paired candidate", "Aligned input"].includes(group.status)) ?? [],
    [scan],
  );

  async function selectInputFolder() {
    setError(null);
    const selection = await open({ directory: true, multiple: false, title: "Select sequencing input folder" });
    if (typeof selection !== "string") return;
    setBusy(true);
    setInspection(null);
    setExportResult(null);
    try {
      setScan(await invoke<ScanResult>("scan_folder", { root: selection }));
    } catch (caught) {
      setError(readableError(caught));
    } finally {
      setBusy(false);
    }
  }

  async function selectReference() {
    const selection = await open({
      directory: false,
      multiple: false,
      title: "Select local reference FASTA",
      filters: [{ name: "Reference FASTA", extensions: ["fa", "fasta", "fna"] }],
    });
    if (typeof selection === "string") setReferenceFasta(selection);
  }

  async function inspectFile(file: ScanFile) {
    if (!scan) return;
    setBusy(true);
    setError(null);
    try {
      setInspection(
        await invoke<InspectionResult>("inspect_input", {
          root: scan.root,
          relativePath: file.relativePath,
          referenceFasta,
        }),
      );
    } catch (caught) {
      setError(`${file.relativePath}: ${readableError(caught)}`);
    } finally {
      setBusy(false);
    }
  }

  async function exportReport() {
    if (!scan) return;
    setError(null);
    const selection = await open({ directory: true, multiple: false, title: "Select report output folder" });
    if (typeof selection !== "string") return;
    setBusy(true);
    try {
      setExportResult(
        await invoke<ExportResult>("export_discovery_report", {
          root: scan.root,
          assay,
          outputDirectory: selection,
        }),
      );
    } catch (caught) {
      setError(readableError(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="Primary navigation">
        <div className="brand" aria-label="SeqQC Studio">
          <span className="brand-mark" aria-hidden="true"><i /><i /><i /></span>
          <span><strong>SeqQC</strong><small>STUDIO</small></span>
        </div>
        <nav>
          <button className="nav-item active" type="button"><span aria-hidden="true">⌂</span> Analysis</button>
          <button className="nav-item" type="button" disabled><span aria-hidden="true">◫</span> References</button>
          <button className="nav-item" type="button" disabled><span aria-hidden="true">◈</span> Tool library</button>
        </nav>
        <div className="privacy-card">
          <span className="status-dot" />
          <div><strong>Engineering preview</strong><small>Use an isolated host</small></div>
        </div>
      </aside>

      <main>
        <header className="topbar">
          <div>
            <span className="eyebrow">LOCAL WORKSPACE · ARCHITECTURE PREVIEW</span>
            <h1>{scan ? "Input readiness" : "Start a local analysis"}</h1>
          </div>
          <div className="toolbar">
            <label>View
              <select value={audience} onChange={(event) => setAudience(event.target.value as Audience)}>
                <option value="guided">Guided</option><option value="standard">Standard</option><option value="expert">Expert</option>
              </select>
            </label>
            <label>Theme
              <select value={theme} onChange={(event) => setTheme(event.target.value as Theme)}>
                {Object.entries(themeLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
              </select>
            </label>
          </div>
        </header>

        {error && <div className="notice error" role="alert"><strong>Action could not be completed</strong><span>{error}</span></div>}
        {busy && <div className="progress" role="status"><span /> Reading local evidence…</div>}

        {!scan ? (
          <>
            <section className="hero" aria-labelledby="hero-title">
              <div className="hero-copy">
                <span className="pill">LOCAL FILES · PREVIEW BUILD</span>
                <h2 id="hero-title">Turn sequencing inputs into understandable readiness evidence.</h2>
                <p>Select a local folder. SeqQC Studio discovers FASTQ, BAM, and CRAM files, proposes sample relationships, and lets you inspect streaming structural evidence without copying inputs into a project.</p>
                <button className="primary-action" type="button" onClick={selectInputFolder} disabled={busy}>Select input folder <span aria-hidden="true">→</span></button>
              </div>
              <div className="sequence-art" aria-hidden="true"><div className="orbit orbit-one" /><div className="orbit orbit-two" /><div className="helix">{Array.from({ length: 8 }, (_, index) => <span key={index} style={{ "--index": index } as React.CSSProperties} />)}</div></div>
            </section>
            <section className="workflow-section" aria-labelledby="workflow-title">
              <div className="section-heading"><div><span className="eyebrow">DECLARED WORKFLOWS</span><h2 id="workflow-title">Choose the assay after discovery</h2></div><span className="platform-note">Alignment: Linux only · QC shell: all platforms</span></div>
              <div className="workflow-grid">
                {["WGS", "WES", "Bulk RNA-seq"].map((workflow, index) => <article className="workflow-card" key={workflow}><span className={`assay-icon assay-${index + 1}`}>{workflow[0]}</span><div><h3>{workflow}</h3><p>Milestone 0 input and report preview</p></div><span className="preview-state">PREVIEW</span></article>)}
              </div>
            </section>
          </>
        ) : (
          <div className="dashboard">
            <section className="project-strip">
              <div><span className="eyebrow">SELECTED LOCAL FOLDER</span><h2>{scan.root}</h2><p>Inputs are read in place and are not copied into the preview.</p></div>
              <div className="project-actions">
                <label>Assay<select value={assay} onChange={(event) => setAssay(event.target.value as Assay)}><option>WGS</option><option>WES</option><option>Bulk RNA-seq</option></select></label>
                <button className="secondary-action" onClick={selectInputFolder} disabled={busy}>Change folder</button>
                <button className="primary-action compact" onClick={exportReport} disabled={busy}>Export report</button>
              </div>
            </section>

            <section className="metric-grid" aria-label="Discovery summary">
              <MetricCard label="Supported files" value={scan.summary.supportedFiles} detail="FASTQ, BAM, and CRAM" />
              <MetricCard label="FASTQ" value={scan.summary.fastqFiles} detail="Compressed or plain" />
              <MetricCard label="BAM / CRAM" value={scan.summary.bamFiles + scan.summary.cramFiles} detail={`${scan.summary.bamFiles} BAM · ${scan.summary.cramFiles} CRAM`} />
              <MetricCard label="Needs review" value={scan.summary.groupsNeedingReview} detail="Pairing or layout ambiguity" attention={scan.summary.groupsNeedingReview > 0} />
            </section>

            <section className="finding-panel">
              <div className={`finding-badge ${reviewGroups.length ? "review" : "ready"}`}>{reviewGroups.length ? "Needs review" : "Within expected range"}</div>
              <div><h2>{reviewGroups.length ? "Confirm input relationships" : "Input structure is ready for review"}</h2><p>{reviewGroups.length ? `${reviewGroups.length} group(s) have missing mates, duplicate mates, or unresolved layout.` : "No pairing ambiguity was detected. This is operational evidence—not a biological QC result."}</p></div>
            </section>

            {exportResult && <div className="notice success" role="status"><strong>Five standalone reports created</strong><span>{exportResult.directory}</span></div>}

            <section className="content-grid">
              <article className="panel">
                <div className="panel-heading"><div><span className="eyebrow">INPUT GROUPS</span><h2>Proposed samples</h2></div><span>{scan.groups.length} groups</span></div>
                <div className="group-list">
                  {scan.groups.slice(0, 100).map((group, index) => <div className="group-row" key={`${group.sample}-${index}`}><span className={`group-status ${["Paired candidate", "Aligned input"].includes(group.status) ? "ok" : "review"}`} aria-hidden="true" /><div><strong>{group.sample}</strong><small>{group.status}{group.lane ? ` · ${group.lane}` : ""}</small></div><b>{group.fileCount}</b></div>)}
                </div>
              </article>

              <article className="panel inspection-panel">
                <div className="panel-heading"><div><span className="eyebrow">STREAMING INSPECTION</span><h2>File evidence</h2></div></div>
                {inspection ? <InspectionView evidence={inspection} audience={audience} /> : <div className="empty-state"><span>◎</span><h3>Select “Inspect” beside a file</h3><p>SeqQC will stream records locally and summarize basic structural evidence.</p></div>}
                <div className="reference-row"><span>{referenceFasta ? `Reference: ${referenceFasta}` : "CRAM may require a local reference FASTA."}</span><button onClick={selectReference} type="button">Choose reference</button></div>
              </article>
            </section>

            <section className="panel files-panel">
              <div className="panel-heading"><div><span className="eyebrow">DISCOVERED FILES</span><h2>Local inputs</h2></div><span>Showing {Math.min(scan.files.length, 200)} of {scan.files.length}</span></div>
              <div className="table-wrap"><table><thead><tr><th>File</th><th>Type</th><th>Proposed sample</th><th>Size</th><th>Confidence</th><th><span className="sr-only">Action</span></th></tr></thead><tbody>{scan.files.slice(0, 200).map((file) => <tr key={file.relativePath}><td title={file.relativePath}>{file.relativePath}</td><td><span className="file-kind">{file.kind}</span></td><td>{file.sample}{file.mate ? ` · R${file.mate}` : ""}</td><td>{formatBytes(file.sizeBytes)}</td><td>{file.confidence}</td><td><button className="table-action" onClick={() => inspectFile(file)} disabled={busy}>Inspect</button></td></tr>)}</tbody></table></div>
            </section>

            <section className="limitations"><strong>Current evidence boundary</strong>{scan.limitations.map((item) => <p key={item}>{item}</p>)}{audience === "expert" && <p>{scan.summary.ignoredFiles} unsupported or symlink entries were ignored. Exports are derived from one canonical model.</p>}</section>
          </div>
        )}
      </main>
    </div>
  );
}

function MetricCard({ label, value, detail, attention = false }: { label: string; value: number; detail: string; attention?: boolean }) {
  return <article className={`metric-card ${attention ? "attention" : ""}`}><span>{label}</span><strong>{value.toLocaleString()}</strong><small>{detail}</small></article>;
}

function InspectionView({ evidence, audience }: { evidence: InspectionResult; audience: Audience }) {
  const rows = [
    ["Records", evidence.recordCount.toLocaleString()], ["Total bases", evidence.totalBases.toLocaleString()],
    ["Read length", evidence.minimumReadLength === null ? "Not available" : `${evidence.minimumReadLength}–${evidence.maximumReadLength} bp`],
    ["Mapped / unmapped", `${evidence.mappedRecords.toLocaleString()} / ${evidence.unmappedRecords.toLocaleString()}`],
  ];
  return <div className="inspection"><span className="evidence-label">{evidence.evidenceLabel}</span><h3>{evidence.relativePath}</h3><p>{evidence.format} · {evidence.compression} · {formatBytes(evidence.inputBytes)}</p><dl>{rows.map(([term, value]) => <div key={term}><dt>{term}</dt><dd>{value}</dd></div>)}</dl>{audience === "expert" && <p className="expert-note">Paired records: {evidence.pairedRecords.toLocaleString()} · Reference sequences: {evidence.referenceSequenceCount}</p>}</div>;
}
