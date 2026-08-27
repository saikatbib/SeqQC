# ADR 0006: Linux-only alignment in the initial release

- Status: Accepted
- Date: 2026-08-26
- Decision owner: Project owner

## Context

Validated native distributions of preferred DNA and RNA aligners do not have a
uniform Windows/macOS/Linux packaging story. Alignment is optional; QC of
existing FASTQ/BAM/CRAM is the primary product capability.

## Decision

- Initial-release aligner tasks may execute only on validated Linux providers.
- Windows and macOS retain FASTQ QC, existing BAM/CRAM QC, dashboards, project
  management, and reports.
- On non-Linux providers the workflow planner rejects alignment before execution
  and the UI explains the Linux requirement.
- Do not invoke WSL, containers, emulation, SSH, or remote APIs as an implicit
  fallback.
- Support one curated DNA workflow for WGS/WES and one curated splice-aware
  workflow for bulk RNA-seq after Linux scientific/tool validation.

## Consequences

- Version 1 validation focuses alignment on declared Linux distributions and CPU
  architectures.
- Adding Windows/macOS alignment requires a later ADR and new scientific,
  security, licensing, resource, and packaging evidence.

