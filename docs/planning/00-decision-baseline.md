# Decision baseline

## Identity and ownership

- Application name: **SeqQC Studio**.
- Analysis engine and repository name: **SeqQC**.
- Intended repository owner: GitHub user `saikatbib`.
- Intended repository location: `https://github.com/saikatbib/SeqQC`.
- Proposed project license: Apache License 2.0, pending third-party review.

## Users and scientific scope

- The interface must serve wet-lab researchers, sequencing-core personnel,
  clinicians, students, bioinformaticians, and validation administrators.
- Version 1 must support WGS, WES, and bulk RNA-seq.
- Version 1 must accept FASTQ/FQ, gzip-compressed FASTQ/FQ, BAM, and CRAM.
- BAM/CRAM inputs are normally aligned. Alignment from FASTQ is an optional,
  explicitly selected workflow.
- Alignment requires user- or institution-supplied references and associated
  files. SeqQC must not download reference data.
- QC of FASTQ/BAM/CRAM and report review target Windows, macOS, and Linux.
  Optional aligner execution is Linux-only for the initial release. Windows and
  macOS must disable aligner launch with a clear capability explanation rather
  than attempting an unvalidated compatibility path.
- Human clinical, human research, preclinical, and unspecified project contexts
  must be represented distinctly.

## Deployment and privacy

- Windows, macOS, and Linux desktop applications must be released.
- The user experience must be a native desktop application; no separately
  opened browser interface is required.
- Runtime operation must remain offline. The application must not contain
  telemetry, advertising, cloud services, remote fonts, remote scripts, update
  polling, or automatic downloads.
- Local and mounted network paths are permitted. Network access to the public
  internet is not.
- A local API may exist but must be disabled by default and restricted to the
  loopback interface when enabled.

## Projects and security

- Projects retain metadata, prior runs, results, reports, configuration,
  provenance, and audit history locally.
- Authentication is local. No online identity provider is required.
- A project may be modified by one user/process at a time.
- Project metadata and designated sensitive fields must support encryption at
  rest.
- Secrets should be protected by the operating-system key store when available.
- A portable project may be exported as a password-protected encrypted archive.
- Completed runs must have tamper-evident manifests and checksums.

## Execution

- The same versioned workflow definition must drive laptop, workstation, and HPC
  execution.
- CPU, RAM, free disk space, temporary space, and relevant instruction-set
  capabilities must be detected before execution.
- SeqQC must recommend safe concurrency and refuse only when continuing is
  likely to corrupt results, exhaust required storage, or violate a required
  control.
- Local execution, generic job-script export, Slurm, PBS Pro, LSF, and SGE are
  version 1 targets.
- Scheduler integrations are adapters; scientific modules must not contain
  scheduler-specific behavior.

## Tools and reports

- SeqQC uses a hybrid of native engine algorithms and established third-party
  tools.
- A beginner distribution should bundle redistributable, validated binaries.
- An institutional distribution must support an administrator-managed external
  tools directory.
- One curated default aligner workflow is exposed per assay initially.
- Initial alignment providers must run on validated Linux targets; native
  Windows and macOS alignment is deferred beyond the initial release.
- Required report targets are the in-app dashboard, standalone offline HTML,
  PDF, JSON, CSV, and TSV. A versioned local machine API is also required.

## Interpretation and presentation

- Analytical labels are `within_expected_range`, `borderline`, `needs_review`,
  `critical_issue`, `not_evaluated`, and `not_applicable`.
- Objective file or workflow errors use a separate error state and are never
  disguised as analytical findings.
- Thresholds may come from published, SeqQC-curated, institutional, project, or
  user sources. Precedence and provenance must be visible.
- The default visual style is restrained scientific/clinical. Light,
  high-contrast, and alternate futuristic themes must be available.
- Guided, Standard, and Expert presentation levels share the same underlying
  results.

## Regulatory boundary

SeqQC is initially research-use software. Security, audit, provenance, and
validation features are intended to support—not replace—an organization's own
validation, procedures, risk management, and jurisdictional obligations. No
diagnostic, clinical-validation, or regulatory-approval claim may be made
without a separate documented process.
