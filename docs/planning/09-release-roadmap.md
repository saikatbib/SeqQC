# Release roadmap and acceptance gates

This roadmap is capability-gated rather than date-driven. All three assays are
version 1 commitments, but implementation proceeds through smaller validated
increments.

## Milestone 0 — specification and risk retirement

### Deliverables

- Planning Baseline 1 (this document set)
- Metric registry with tier/algorithm/validation assignments
- Architecture decision records for desktop stack, storage encryption, archive
  cryptography, PDF rendering, process isolation, and UI framework
- Prototype spikes for large-file streaming, encrypted transactions, offline
  desktop hardening, report rendering, and scheduler contract
- Dependency/license/SBOM proof of concept
- Threat-model review and validation dataset inventory

### Exit gate

- No unresolved product ambiguity that changes the core data/workflow model
- Linux alignment distribution/architecture targets and validation experiments
  are approved
- Security reviewers accept the key-management direction for implementation
- UX wireframes pass initial review with beginner and expert representatives

## Milestone 1 — secure desktop foundation (`0.1`)

### Deliverables

- Cross-platform desktop shell and packaged local assets
- Project create/open/lock/recovery lifecycle
- Local principal/session and encrypted metadata proof
- Versioned schemas/migrations and append-only audit foundation
- Recursive discovery and sample-map review
- Tool/reference registries and system-readiness report
- Local workflow/task state engine with simulated modules
- Clinical night, clinical light, and high-contrast themes

### Exit gate

- Clean offline installs start without connection attempts
- Crash/migration/single-writer/encryption tests pass
- 10,000-sample synthetic project remains navigable
- Input mapping serious-use-error threshold is met in formative testing

## Milestone 2 — universal FASTQ and alignment QC (`0.2`)

### Deliverables

- FASTQ integrity, yield, quality, composition, adapter, and complexity modules
- BAM/CRAM integrity, header, mapping, pairing, duplicate, insert-size, clipping,
  and common coverage evidence
- Metric normalization and interpretation engine
- Overview/findings/sample dashboards
- Offline HTML, JSON, CSV, TSV export and first PDF path
- Local executor, resource estimator, resumability, atomic result commit

### Exit gate

- Core universal metrics pass scientific and cross-platform matrix
- Every finding traces to metrics/rules and handles missingness correctly
- All exports reconcile with the canonical report model
- No-network, malicious-path, interrupted-run, and recovery suites pass

## Milestone 3 — WGS and WES (`0.3`)

### Deliverables

- WGS coverage, breadth, uniformity, GC, callable territory, chromosome profile
- WES on/near/off-target, enrichment, coverage thresholds, uniformity/fold-80,
  target/gene aggregation
- Optional contamination/fingerprint evidence with local resources
- Target/reference compatibility checks
- Cohort distributions and robust outlier method
- Published, institutional, and project preset framework

### Exit gate

- WGS/WES metric validation records approved for supported provider matrix
- Contamination/identity safely reports `not_evaluated` without resources
- Multiple reference/target structures and perturbation series pass
- Threshold provenance/override behavior passes review

## Milestone 4 — bulk RNA-seq (`0.4`)

### Deliverables

- Alignment, strandedness, feature distribution, junction, rRNA/mitochondrial,
  gene-body/degradation, expression and complexity evidence
- GTF/GFF validation and explicit preprocessing assumptions
- Cohort correlation, PCA, replicate and batch-pattern views
- RNA-seq guided explanations and troubleshooting

### Exit gate

- RNA metric registry core tier passes validation
- Single/paired and stranded/unstranded matrix passes
- Cohort views expose transformation/filtering and do not overclaim outliers
- Annotation incompatibility and ambiguity cases fail safely

## Milestone 5 — optional alignment (`0.5`)

### Deliverables

- One curated DNA workflow for WGS/WES
- One curated splice-aware workflow for bulk RNA-seq
- Linux-only alignment capability with explicit provider detection and
  Windows/macOS gating
- Local reference-index creation with estimation/provenance
- FASTQ → aligned artifact → QC chaining
- Platform/provider capability UI and validated institutional-tool mode

### Exit gate

- Exact default aligners and versions pass scientific, licensing, platform, and
  performance review
- Output headers/read groups/sort/index/provenance pass golden tests
- Resource exhaustion/cancellation/restart behavior is safe
- Each declared Linux distribution/architecture/provider passes the alignment
  matrix; Windows/macOS reliably disable aligner execution while retaining QC

## Milestone 6 — HPC and controlled operations (`0.6`)

### Deliverables

- Generic script export/import
- Slurm, PBS Pro, LSF, and SGE adapters
- Arrays, retry/recovery, path mappings, accounting normalization
- Encrypted portable project archive
- Tool/preset policy locking, de-identified report workflow
- Deployment evidence bundle

### Exit gate

- Adapter contract/live fixtures pass supported scheduler matrix
- Local/HPC normalized-result equivalence passes
- Archive security review and portability tests pass
- Controlled workflow audit/recovery scenarios pass

## Milestone 7 — stabilization and public `1.0`

### Deliverables

- Signed installers/packages for declared OS/architectures
- Final license, third-party notices, SBOM, security policy, contribution guide,
  user/admin/developer/validation manuals
- Example synthetic projects and screenshots
- Performance report, known limitations, compatibility matrix
- Release/signing/change-control process

### Exit gate

- Product acceptance criteria in the PRD pass
- Independent security review has no unresolved high/critical defect
- Scientific reviewers approve core metric records for all three assays
- Accessibility/usability thresholds pass
- Upgrade, downgrade-protection, backup/restore, and clean-uninstall tests pass
- Repository contains no patient data, secrets, unlicensed assets, or
  unapproved binaries

## Post-1.0 candidates

- Additional validated aligners and QC modules
- Native Windows/macOS alignment providers if later justified and validated
- Signed declarative module SDK
- Additional sequencing assays and long-read-specific modules
- More scheduler versions/providers
- Institution-scale multi-reader service architecture
- Jurisdiction-specific controlled distributions and electronic signatures

These are not implied by the version 1 commitment and require their own product,
scientific, security, and regulatory decisions.

## Workstream ownership

The project should explicitly name owners/reviewers for:

- product and UX;
- WGS/WES science;
- RNA-seq science;
- security/privacy;
- desktop/platform engineering;
- execution/HPC;
- quality/validation;
- licensing/release; and
- controlled-workflow documentation.

No single reviewer should approve their own high-risk cryptography, scientific
threshold, or release-redistribution decision without independent review.
