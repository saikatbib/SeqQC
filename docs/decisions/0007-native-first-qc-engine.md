# ADR 0007: Native-first QC engine

- Status: Accepted
- Date: 2026-08-26

## Context

SeqQC must provide the same core QC meaning on Windows, macOS, and Linux and must
not become a presentation wrapper around tool-specific HTML. Several established
tools have uneven platform binary availability.

## Decision

- Implement core input validation, FASTQ metrics, alignment accounting,
  reference/annotation validation, depth/territory metrics, interpretation, and
  cohort statistics in Rust.
- Evaluate the Rust `noodles` format crates for specification-oriented FASTQ,
  SAM, BAM, CRAM, FASTA, BED, GFF, and GTF I/O. Its API maturity and performance
  must pass a Milestone 0 spike before dependency acceptance.
- Use trusted independent tools and hand-derived fixtures as validation
  references, not as the report model.
- External tools remain appropriate for optional extended evidence such as
  contamination/fingerprinting and for Linux alignment.
- Every normalized metric identifies its native method or external adapter and
  validation status.

## Consequences

- Core scientific implementation is larger but portable and controllable.
- Cross-validation against SAMtools/HTSlib, fastp, mosdepth, Picard, RNA-SeQC,
  and selected reference methods is mandatory where applicable.
- A metric remains investigational until its method record and validation matrix
  are approved.

