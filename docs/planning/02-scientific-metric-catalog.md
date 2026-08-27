# Scientific metric catalog

## 1. Purpose and rules

This catalog defines the version 1 metric *scope*. It does not freeze numerical
cutoffs. Numerical thresholds require literature review, dataset validation,
assay context, and versioned preset approval.

The companion [`02a-metric-registry.csv`](02a-metric-registry.csv) is the
initial machine-readable inventory of stable IDs, value types, units, tiers,
and applicability. It is a planning registry—not yet an implemented schema or
claim that every listed method has passed validation.

Every normalized metric must have:

- a stable metric ID and schema version;
- display name, definition, unit, directionality, and precision;
- sample/run/cohort scope;
- applicable assay, library, layout, platform, and input type;
- required reference/annotation and exclusions;
- calculation/tool ID, version, and parameters;
- observation count or denominator;
- missing/not-applicable reason;
- source-artifact fingerprint; and
- validation status for the active platform.

Metrics and findings are different. A metric is evidence; a finding is a
versioned interpretation of one or more metrics in context.

## 2. Evaluation vocabulary

Analytical findings use:

| Status | Meaning |
| --- | --- |
| `within_expected_range` | Evidence is consistent with the selected contextual range. |
| `borderline` | Evidence is near or modestly beyond a boundary and deserves awareness. |
| `needs_review` | Evidence is unusual enough to require human review before downstream use. |
| `critical_issue` | Evidence suggests the declared use may be seriously compromised. |
| `not_evaluated` | Required evidence, rule, or validated implementation is unavailable. |
| `not_applicable` | The metric/rule does not apply to this sample or workflow. |

File corruption, missing inputs, failed commands, invalid indexes, and schema
errors are operational errors and use separate error codes.

## 3. Threshold precedence

The effective rule is selected in this order unless a locked institutional
policy forbids overrides:

1. locked institutional policy;
2. explicit project override;
3. selected institutional preset;
4. selected published/SeqQC-curated preset;
5. descriptive cohort context with no classification.

The dashboard must show the effective rule and every overridden source.
Published presets must cite the publication/guideline, population, assay, and
limitations. “Common practice” is not an acceptable anonymous source.

## 4. Input and metadata validation

| ID family | Version 1 measures/checks | Applies to |
| --- | --- | --- |
| `input.file.*` | readability, size, modification stability, full/fast fingerprint, duplicate content, supported compression | All |
| `input.fastq.*` | record structure, sequence/quality length agreement, encoding plausibility, truncated gzip/records, sampled/full validation mode | FASTQ |
| `input.pair.*` | mate count agreement, read-name concordance, orientation hints, lane grouping, orphan fraction | Paired FASTQ |
| `input.sam.*` | header validity, EOF marker where relevant, sort declaration/evidence, index presence/usability, read groups, sample-name consistency | BAM/CRAM |
| `input.reference.*` | FASTA readability, index/dictionary consistency, sequence fingerprints, contig dictionary | Alignment/BAM/CRAM |
| `input.annotation.*` | BED/GTF/GFF syntax, coordinate bounds, duplicates, overlaps, contig compatibility, feature availability | WES/RNA-seq |
| `input.metadata.*` | required fields, uniqueness, allowed values, sample-sheet/file reconciliation, sensitive-field classification | All |

Fast validation may be used for exploration, but a completed controlled run must
record the chosen validation level. Clinical-context presets may require full
integrity validation.

## 5. Universal FASTQ metrics

### 5.1 Yield and structure

- `fastq.reads.total`: total reads per file/mate/sample
- `fastq.bases.total`: total sequenced bases
- `fastq.read_length.min`, `.max`, `.mean`, `.median`, `.distribution`
- `fastq.empty_read.fraction`
- `fastq.layout`: single, paired, or unresolved
- `fastq.lanes.count` and lane contribution fractions
- `fastq.mate.read_count.delta` and orphan fraction
- estimated uncompressed size and compression ratio

### 5.2 Base-call quality

- per-cycle quality distribution and median/quantiles
- per-read mean-quality distribution
- Q20/Q30 base fractions (threshold names remain explicit)
- low-quality tail prevalence
- quality-score encoding and maximum observed score
- mate- and lane-specific quality differences

### 5.3 Composition

- overall and per-cycle A/C/G/T/N fractions
- GC fraction and GC distribution
- per-cycle N fraction
- sequence-length-stratified composition when variable length is present
- poly-G and other poly-X tail prevalence
- homopolymer summaries where validated

### 5.4 Contamination and technical sequence signals

- adapter/primer sequence evidence and affected-read fraction
- overrepresented sequences with local database match when a curated offline
  database is installed
- enriched k-mers and positional enrichment
- tile/lane quality signals when instrument read names provide the fields
- index-read metrics only when index reads are explicitly supplied

### 5.5 Complexity

- exact/estimated duplicate fraction with method and sampling model
- unique-sequence fraction
- complexity curve/estimated library complexity when method assumptions hold

Complexity measured from unaligned reads must be labeled as sequence redundancy,
not interchangeable with alignment/PCR duplicate metrics.

### 5.6 FASTQ comparisons

- mate imbalance
- lane-to-lane distribution shifts
- per-sample deviation from cohort median and robust spread
- read-yield outliers
- quality/GC/adapter outliers

## 6. Universal BAM/CRAM metrics

### 6.1 Alignment accounting

- total records, primary reads, secondary and supplementary records
- mapped/unmapped reads and bases
- properly paired, singleton, mate-unmapped, discordant-pair fractions
- read 1/read 2 counts
- duplicate and optical-duplicate counts when distinguishable
- QC-fail records
- read-group, lane, library, and platform-unit breakdowns

All fractions must declare their denominator and flag filters.

### 6.2 Alignment quality and structure

- mapping-quality distribution and thresholded fractions
- alignment score/edit-distance summaries when tags exist
- mismatch rate with declared calculation method
- insertion/deletion event and base rates
- soft/hard-clipped base and read fractions
- supplementary/chimeric alignment summaries
- insert-size distribution, orientation, median, robust spread, and tails
- mate-reference and chromosome-pair anomalies

### 6.3 Reference and genome distribution

- contig-level aligned counts and fractions
- autosome/sex-chromosome/mitochondrial summaries when the registered reference
  declares those roles
- unmapped and unlocalized contig contribution
- unexpected contig evidence
- reference-build compatibility evidence and confidence

SeqQC must not infer biological sex, gender, or sample identity as a fact. It may
show sex-chromosome coverage patterns as reviewable evidence when enabled and
appropriate, with privacy-aware wording.

### 6.4 Coverage

- mean, median, minimum, and maximum depth over a declared territory
- depth distribution and breadth at configurable thresholds
- zero-coverage fraction
- coverage uniformity and robust dispersion
- GC-stratified normalized coverage
- callable-territory fraction only when a complete callable definition is
  selected
- per-contig/per-target summaries

Every coverage metric must identify territory, filters, duplicate handling,
mapping/base-quality cutoffs, overlap handling, and reference build.

### 6.5 Integrity and metadata

- header/read-group consistency
- coordinate-sort evidence
- index compatibility
- mate/tag consistency checks
- RG/SM/LB/PL/PU field completeness under selected policy
- program-record provenance summary

## 7. WGS-specific metrics

- autosomal coverage depth and breadth
- genome-wide coverage distribution
- uniformity and fold-deviation summaries
- GC bias curve and normalized coverage by GC bin
- duplicate rate and estimated library size
- insert-size and orientation profile
- mapping and usable-read fractions
- callable genome fraction under a named definition
- chromosome-level coverage outliers
- mitochondrial fraction
- contamination estimate when a validated, locally provisioned resource and
  compatible method are available
- fingerprint/sample concordance only when the necessary local marker resource
  and expected fingerprints are supplied

Contamination and identity checks must become `not_evaluated`, not silently
absent, when population/reference resources are unavailable.

## 8. WES-specific metrics

- target-territory size and validated target interval count
- reads/bases on target, near target, and off target
- on-target fraction and fold enrichment
- mean/median target coverage
- target breadth at configurable coverage thresholds
- target uniformity, fold-80 penalty or selected equivalent
- zero/low-coverage targets and genes with declared aggregation method
- coverage distribution across targets
- GC bias over target intervals
- duplicate rate, insert size, mapping, usable bases
- target padding sensitivity where configured
- contamination and fingerprint checks under the same availability rules as WGS

Gene-level summaries require a versioned mapping from target intervals to genes;
the BED name column alone must not be assumed to be a gene identifier.

## 9. Bulk RNA-seq-specific metrics

### 9.1 Alignment and library structure

- overall, unique, multi-mapping, and unmapped fractions
- splice-junction and chimeric alignment summaries
- mismatch, insertion, deletion, clipping, and mapping-quality metrics
- insert-size/fragment-length summaries when meaningful
- duplicate/sequence redundancy metrics with method-specific interpretation
- inferred/declared strandedness with confidence and annotation version

### 9.2 Genomic distribution

- exonic, intronic, intergenic, and ambiguous fractions
- coding, UTR, and other feature fractions when annotation supports them
- ribosomal RNA fraction using a declared local annotation/resource
- mitochondrial transcript/read fraction
- chromosome/contig contribution

### 9.3 Coverage and degradation

- gene-body coverage profile
- 5′/3′ bias and coverage skew
- transcript integrity/degradation proxy with assumptions
- junction saturation and discovery curve where supported
- read distribution along annotated features

### 9.4 Expression and complexity

- detected genes at configurable count/abundance thresholds
- count/abundance distribution
- library complexity/saturation curve
- top-expressed gene contribution
- biotype contribution when annotation contains biotypes
- zero-count and low-count feature fractions

### 9.5 Cohort-level RNA-seq

- library-size and detected-gene outliers
- sample correlation using declared transformation/filtering
- PCA or equivalent low-dimensional view using declared transformation,
  features, scaling, and missing-data rules
- replicate concordance
- known metadata association visualization
- batch-pattern evidence without causal claims
- sample-swap/fingerprint evidence only with suitable local resources

PCA and clustering are descriptive. SeqQC must not label a sample biologically
wrong solely because it is separated from other samples.

## 10. Alignment workflow metrics

Alignment QC must retain both input FASTQ evidence and output alignment evidence:

Initial-release aligner execution is Linux-only. These metrics remain readable
and reportable on Windows and macOS after a completed project is transferred.

- selected aligner workflow ID/version and executable fingerprint
- reference/index fingerprints and index-build provenance
- effective command arguments without secrets
- task CPU, peak memory where available, wall time, exit status, and I/O sizes
- alignment throughput
- output integrity, sort, index, and header checks
- intermediate cleanup record
- comparison of expected versus actual storage use

## 11. Cohort and batch statistics

For compatible samples SeqQC should calculate:

- median, interquartile range, median absolute deviation, quantiles
- robust standardized deviation with minimum cohort-size guard
- missingness and not-applicable counts
- distribution shift by declared batch metadata
- rank and percentile with warnings for small cohorts
- pairwise/correlation measures only for appropriate metric families

An outlier flag must name its method, cohort definition, cohort size, exclusions,
and whether a fixed threshold also applies. Cohort-derived thresholds must not be
presented as published quality standards.

## 12. Findings model

Each finding contains:

- finding ID/version, scope, status, severity, title, and concise summary;
- contributing metric IDs and observed values;
- expected interval/rule and threshold source;
- assay/project context and applicability conditions;
- confidence/evidence-quality indicator;
- plain-language significance;
- plausible causes framed as possibilities;
- prioritized next actions;
- limitations and links to local help;
- original status plus any user adjudication/comment; and
- timestamp and engine/ruleset versions.

Findings must be deterministic for a given normalized result and ruleset.

## 13. Metric tiers and release gating

| Tier | Definition | Release behavior |
| --- | --- | --- |
| Core | Required for assay readiness and supported on all declared execution profiles | Blocks assay release if invalid |
| Extended | High-value metric requiring extra annotation/tool/resource | Visible as available/not evaluated |
| Investigational | Method not yet sufficiently validated for a quality claim | Expert view only, explicitly labeled |

Before implementation, each metric listed above must receive a registry row
assigning its tier, algorithm/tool candidate, validation dataset, tolerance,
estimated cost, and supported platform matrix. No metric is “supported” merely
because an external tool prints a value.
