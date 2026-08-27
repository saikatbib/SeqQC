# Dependencies, tooling, and licensing plan

**Review date:** 2026-08-25  
**Status:** Candidate inventory; no item is approved for redistribution yet

## 1. Policy

SeqQC source is proposed under Apache-2.0. That choice does not relicense
third-party software. Every shipped component must have:

- exact name, source repository, version/commit, and artifact origin;
- license/SPDX expression and complete license/notice text;
- transitive native/runtime dependencies and their licenses;
- redistribution, modification, attribution, patent, trademark, and data/model
  terms reviewed;
- platform/architecture and build provenance;
- cryptographic digest and signature verification where available;
- vulnerability and maintenance assessment;
- scientific validation status; and
- an owner and re-review date.

An apparent permissive license is necessary but not sufficient. Precompiled
binaries may incorporate libraries with additional obligations, and supporting
resources may have licenses different from the software.

## 2. Distribution modes

### Beginner bundle

- Contains only approved redistributable binaries for the exact platform.
- Ships all required notices, citations, SBOM, checksums, and version manifest.
- Never downloads a missing tool.
- Displays unavailable modules honestly when no validated binary exists.

### Institution-managed tools

- Administrator registers one or more external tool roots.
- A signed/versioned local policy maps module → executable/version/checksum.
- SeqQC verifies the executable before each controlled run.
- “Found on PATH” is not equivalent to approved.
- Institutions remain responsible for local licenses and validated deployment.

### Development environment

Build and test automation may retrieve dependencies, but release runtime must
not. Development convenience must not leak an installer/update/package manager
into the application.

## 3. Application-stack candidates

| Component | Candidate role | Upstream license signal | Current disposition |
| --- | --- | --- | --- |
| [Tauri 2](https://github.com/tauri-apps/tauri) | Native desktop shell/capability boundary | MIT or Apache-2.0 | Preferred; prototype capability and offline hardening |
| Rust | Core, streaming parsers, process/storage control | Mixed per component; standard toolchain terms | Preferred language; crate-by-crate review required |
| TypeScript UI | Desktop presentation | Framework-dependent | UI framework decision pending prototype/accessibility evidence |
| SQLite-compatible store | Transactional project metadata | Implementation-dependent | Encryption choice pending security ADR |
| Local PDF renderer | Offline PDF export | Implementation-dependent | Must support reproducible layout and packaging |

No updater or networking plugin should be included in the desktop build. Tauri's
availability of such features is not a reason to ship them.

## 4. Scientific-tool shortlist

The following are candidates behind typed adapters, not final selections.

| Candidate | Potential use | Upstream license signal | Key release questions |
| --- | --- | --- | --- |
| [fastp](https://github.com/OpenGene/fastp) | FASTQ QC evidence; preprocessing only if separately enabled later | MIT | Official prebuilt distribution is primarily Linux; build and cross-platform equivalence need proof |
| [HTSlib](https://github.com/samtools/htslib) / [SAMtools](https://github.com/samtools/samtools) | BAM/CRAM validation, indexing, statistics | MIT/BSD-style / MIT | Windows packaging, CRAM reference behavior, linked libraries, security update process |
| [mosdepth](https://github.com/brentp/mosdepth) | WGS/WES depth and breadth | MIT | Official binary architecture coverage; Windows/macOS builds; territory/filter equivalence |
| [Picard](https://github.com/broadinstitute/picard) | Alignment/library metrics | MIT | Java runtime size/license/notices; startup/memory; metric overlap and normalization |
| [BWA-MEM2](https://github.com/bwa-mem2/bwa-mem2) | Default DNA alignment candidate | MIT | Linux CPU-architecture coverage and build provenance require validation |
| [STAR](https://github.com/alexdobin/STAR) | Default bulk RNA-seq alignment candidate | MIT | Linux architecture coverage plus large index/RAM needs require validation |
| [RNA-SeQC 2](https://github.com/getzlab/rnaseqc) | Bulk RNA-seq metrics | BSD-3-Clause plus documented third-party licenses | GTF preprocessing assumptions; Linux/macOS binary coverage; normalization and Windows gap |
| [VerifyBamID2](https://github.com/Griffan/VerifyBamID) | Optional contamination estimation | MIT | Required population/resource licensing and build compatibility; applicability safeguards |

The shortlist intentionally does not include MultiQC as the product's report or
interpretation layer. SeqQC may validate against its source tools but should
normalize evidence directly and produce its own report model.

## 5. Tool-selection rubric

Score each candidate on:

1. scientific validity and peer-reviewed/field evidence;
2. exact metric definition and stable machine-readable output;
3. deterministic behavior and version stability;
4. performance and resource predictability;
5. Windows/macOS/Linux and CPU-architecture availability;
6. active maintenance and security response;
7. redistribution and transitive-license clarity;
8. ability to operate entirely offline;
9. safe invocation with arbitrary local paths;
10. testability with small synthetic fixtures; and
11. feasibility of an institutional external-tool mode.

A high scientific score does not override an unsolved licensing or platform
gap. SeqQC may use different implementations only if their normalized outputs
meet declared equivalence tolerances and provenance remains explicit.

## 6. Linux-only alignment boundary

The initial release deliberately restricts aligner execution to validated Linux
providers. This removes native Windows/macOS aligner builds from the first
release gate while preserving cross-platform FASTQ QC, BAM/CRAM QC, project
review, and reporting.

Alignment candidates still require validation for supported Linux distributions,
CPU architectures, linked libraries, and local/HPC providers. Windows/macOS
must not invoke WSL, containers, emulation, or remote execution as an implicit
fallback. Any future expansion is a separately reviewed product capability with
scientific-equivalence, security, packaging, and licensing evidence.

## 7. Reference and auxiliary data

SeqQC never downloads reference bundles. It may ship small, openly licensed
technical sequence lists (for example adapter sequences) only after recording:

- source and version;
- license and redistribution permission;
- scientific purpose and applicable platforms;
- fingerprint; and
- update/import procedure.

Human population panels, fingerprints, transcript annotations, genome builds,
and capture targets are user/institution supplied by default. Software license
approval does not imply permission to redistribute these data.

## 8. Fonts, icons, and report assets

All UI/report assets are packaged locally. Font and icon licenses must permit
application embedding and report redistribution. Product identity must not copy
the Tauri or any tool logo. Reports include third-party citations/notices only
as required and useful; trademark names must not imply endorsement.

## 9. SBOM and notices

Each installer produces:

- SPDX or CycloneDX SBOM;
- `THIRD_PARTY_NOTICES` covering source and binary components;
- machine-readable tool manifest;
- checksums and build provenance; and
- a list of excluded optional tools/resources.

The About/System Readiness screen displays these files offline.

## 10. Approval gates

No binary enters `third_party/` or an installer until legal/maintainer review is
recorded. Approval is specific to version, artifact, operating system,
architecture, and build. Changes trigger scientific smoke tests, license/SBOM
regeneration, vulnerability review, and reproducibility checks.
