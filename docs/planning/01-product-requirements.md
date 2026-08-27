# Product requirements document

## 1. Purpose

SeqQC Studio turns sequencing-quality evidence into understandable, traceable
decisions. It is not a clone of MultiQC and is not primarily a collection of
third-party HTML reports. Its core product value is a guided explanation layer
over assay-aware evidence while retaining full expert detail and provenance.

## 2. Outcomes

A user must be able to answer:

1. Are my input files valid and correctly associated with samples?
2. Is sequencing and alignment quality plausible for this assay?
3. Is coverage or RNA library performance sufficient for my declared purpose?
4. Which findings need attention, why, and what should I do next?
5. Are samples, lanes, batches, or replicates unusual relative to their cohort?
6. Can another person reproduce and audit exactly what was run?

## 3. Personas and access modes

| Persona | Primary need | Default presentation |
| --- | --- | --- |
| Student/new user | Safe setup, definitions, guided remediation | Guided |
| Wet-lab researcher | Library and sample-level interpretation | Guided/Standard |
| Sequencing core | Batch overview, outliers, throughput, export | Standard |
| Bioinformatician | Raw values, plots, logs, versions, overrides | Expert |
| Clinical reviewer | Traceability, locked results, audit evidence | Standard/Expert |
| Administrator | Tool registry, presets, security and scheduler policy | Administration |

Presentation levels do not remove data or change calculations. They only alter
information density and explanatory detail.

## 4. Supported project contexts

- Human clinical
- Human research
- Preclinical
- Other/unspecified

Context changes privacy defaults, terminology, required metadata, and report
notices. It must never silently change a scientific threshold.

## 5. Primary user journeys

### 5.1 QC of existing FASTQ files

1. Create or open a local project.
2. Choose FASTQ QC and one or more folders/sample sheets.
3. Discover files recursively and propose sample/read/lane relationships.
4. Resolve ambiguities instead of guessing.
5. Choose assay, library details, threshold preset, and output location.
6. Run preflight integrity, resource, and storage checks.
7. Execute, pause/cancel where safe, resume after interruption, and monitor.
8. Review prioritized findings and supporting plots.
9. Export selected report formats and a reproducibility manifest.

### 5.2 QC of aligned BAM/CRAM files

The flow is the same, with additional checks for sort order, index, reference
compatibility, read groups, target BED, and GTF/GFF as applicable. CRAM must not
be analyzed without resolving its reference requirements.

### 5.3 Optional alignment

Alignment is opt-in. The wizard collects assay-specific reference files,
verifies them, estimates index/output/scratch requirements, displays the chosen
default aligner and parameters, and creates new aligned artifacts. QC then runs
against those immutable artifacts.

For the initial release, aligner execution is available only through a validated
Linux execution provider. On Windows and macOS the desktop application continues
to provide FASTQ QC and QC of existing BAM/CRAM files, but it must disable the
alignment start action and explain that alignment requires Linux. It must not
silently invoke WSL, a container, or another compatibility layer.

### 5.4 Cohort review

Users select a run or compatible runs, inspect distributions and outliers,
filter by metadata, compare batches, annotate findings, and export a cohort
report. SeqQC must warn when incompatible assay, build, target, annotation, or
workflow versions are compared.

### 5.5 Offline project transfer

The owner creates an encrypted archive with an explicit content list. A second
installation verifies the archive and application/schema compatibility before
import. Import never overwrites an existing project silently.

## 6. Functional requirements

### FR-01 Input discovery

- Select one or more folders or sample sheets.
- Traverse nested folders with previewable include/exclude rules.
- Recognize case-insensitive supported extensions.
- Propose paired-end, lane, and sample grouping with a confidence and rationale.
- Detect duplicates, symlink cycles, unreadable files, changing files, and name
  collisions.
- Allow review and correction before any expensive analysis.
- Treat network shares as potentially unstable and verify file identity before
  and after execution.

### FR-02 Reference management

- Register, validate, name, and version local FASTA, FAI/dictionary, alignment
  indexes, target BED, and GTF/GFF resources.
- Store paths plus fingerprints; do not copy large references unless requested.
- Detect contig naming and build mismatches where evidence permits.
- Generate missing indexes only with explicit confirmation and a storage
  estimate.
- Never fetch a resource from the internet.

### FR-03 Workflow configuration

- Provide assay-aware guided defaults and an expert parameter view.
- Save versioned reusable presets.
- Distinguish scientific settings from performance settings.
- Record every effective parameter, including inherited defaults.
- Mark changes that invalidate comparability with prior runs.

### FR-04 Preflight

- Validate readable inputs, required associations, output writability, reference
  compatibility, tools, memory, CPU, scratch space, and expected output size.
- Explain blocking errors separately from advisory warnings.
- Produce a preflight record even if execution is not started.

### FR-05 Execution

- Support local workers and the scheduler adapters defined in the execution
  specification.
- Avoid oversubscription and expose resource recommendations.
- Preserve task state across application restarts.
- Use atomic result publication: partial outputs remain staged until verified.
- Support cancellation and resumability at declared safe checkpoints.
- Never invoke a shell with untrusted filenames interpolated into a command
  string.

### FR-06 Interpretation

- Convert normalized metrics into findings using a versioned rule engine.
- Preserve observed value, units, comparator, expected range, preset/rule ID,
  evidence source, applicability, confidence, explanation, causes, and action.
- Identify conflicts between selected threshold sources.
- Allow comments and adjudication without altering original results.
- Never generate unsupported biological or diagnostic conclusions.

### FR-07 Dashboard

- Show project readiness, top findings, run status, assay context, and cohort
  overview first.
- Provide sample, metric, cohort, provenance, and audit drill-down.
- Use color plus text/icon/shape; color alone must not encode status.
- Maintain Guided, Standard, and Expert views.
- Render all assets locally.

### FR-08 Reports and data export

- Export standalone offline HTML, PDF, JSON, CSV, and TSV.
- Keep HTML functional without network access or remote dependencies.
- Embed or package fonts and assets according to their licenses.
- Include report ID, generation time, source run, version, threshold preset,
  limitations, and integrity verification instructions.
- Allow de-identified exports with a preview of removed fields.
- Do not claim that CSV/TSV alone preserves the complete record.

### FR-09 Project lifecycle

- Create, open, lock, clone, archive, back up, restore, import, and export.
- Enforce a single writer with safe stale-lock recovery.
- Preserve completed run records as immutable; corrections create amendments or
  new runs.
- Support schema migrations with backup and rollback-on-failure.

### FR-10 Local API

- Disabled by default and enabled by an authorized local user.
- Bind to loopback only, use short-lived credentials, and expose its state.
- Publish an OpenAPI or equivalent versioned contract locally.
- Reject browser-origin requests by default and record mutating calls.
- Provide no remote or cloud mode in version 1.

### FR-11 Administration

- Register external tool directories and verify approved versions/checksums.
- Import/export signed institutional presets through files.
- Configure scheduler profiles, storage policies, session timeout, retention,
  approved export locations, and clinical-project controls.
- Produce an offline system-readiness report.

## 7. Non-functional requirements

### NFR-01 Offline assurance

Release testing must demonstrate zero attempted network connections during
installation, first launch, ordinary workflows, reports, failure handling, and
uninstall. Build-time dependency retrieval is separate from runtime behavior.

### NFR-02 Cross-platform behavior

Project files and exported results must be portable across supported operating
systems. Path handling must not embed platform-specific assumptions. Scientific
equivalence tolerances must be defined where bundled tool builds differ.
Cross-platform support applies to QC, project management, dashboards, and
reports; optional alignment is Linux-only in the initial release.

### NFR-03 Accessibility

Target WCAG 2.2 AA principles for keyboard use, contrast, focus, labels,
reflow, non-color status, and reduced motion, adapted to desktop controls.

### NFR-04 Reliability

- No completed run is visible before output verification.
- Application crashes must not corrupt the last committed project state.
- Reopening a project must identify interrupted tasks and offer safe recovery.
- Every error includes a stable error code and actionable message.

### NFR-05 Performance and scale

“Any scale” means a consistent workflow from laptop to supported HPC systems,
not unlimited work on finite hardware. The UI must remain responsive with at
least 10,000 samples represented through pagination/virtualization. Scientific
performance targets are established by benchmark tier during validation.

### NFR-06 Reproducibility

The same input fingerprints, workflow version, reference fingerprints, tool
versions, and settings must resolve to the same declared workflow graph. Any
nondeterminism or numerical tolerance must be documented.

### NFR-07 Maintainability

Scientific modules, presentation, storage, and execution adapters must have
versioned interfaces. A module can be tested without launching the UI or an HPC
scheduler.

### NFR-08 Privacy and logs

Logs use project-scoped opaque identifiers by default. Raw read names, sample
identifiers, patient identifiers, sequences, and filesystem locations must not
appear unless explicitly required and classified.

## 8. Out of scope for version 1

- Variant calling, diagnostic interpretation, or treatment recommendation
- Single-cell, spatial, metagenomic, ChIP-seq, ATAC-seq, and long-read-specific
  QC claims
- Internet-based reference acquisition or account synchronization
- Concurrent collaborative editing
- Browser-hosted or cloud-hosted execution
- Automatic submission to external repositories
- Native Windows or macOS aligner execution in the initial release
- A claim of regulatory approval or universal clinical suitability

## 9. Version 1 product acceptance

Version 1 is acceptable only when:

- all required formats and three assays pass the validation matrix;
- supported installers work offline on the declared OS/architecture matrix;
- QC works on Windows, macOS, and Linux while aligner execution passes its
  declared Linux provider/architecture matrix;
- local plus four scheduler adapters pass contract tests;
- every displayed finding is traceable to normalized evidence and a rule;
- all required report formats are mutually reconcilable;
- project recovery, encryption, portability, and single-writer locking pass;
- the no-network test suite passes;
- accessibility and usability studies meet their release thresholds;
- third-party redistribution and notices have been approved; and
- documentation states limitations and research-use status prominently.
