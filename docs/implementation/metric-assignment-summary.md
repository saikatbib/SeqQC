# Metric implementation assignment summary

All 128 planning metric IDs are covered by first-match rules in
`config/metric-implementation-rules.json`. The repository checker fails if a
metric is duplicated, unmatched, or assigned by an incomplete rule.

| Assignment family | Metrics | Primary direction |
| --- | ---: | --- |
| Input validation | 15 | Native Rust format and metadata validation |
| FASTQ QC | 32 | Native streaming accumulator |
| Alignment accounting | 24 | Native SAM/BAM/CRAM accumulator |
| Coverage | 7 | Native interval/depth event accumulator |
| WGS core | 6 | Native aggregation |
| WGS contamination | 1 | Conditional Linux external adapter |
| WGS fingerprint | 1 | Conditional Linux external adapter |
| WES | 11 | Native target/coverage aggregation |
| Bulk RNA-seq | 20 | Native annotation and RNA library accumulators |
| Cohort | 6 | Native deterministic statistics |
| Workflow observability | 5 | Native executor/provider accounting |

Core native methods target Windows, macOS, and Linux. The two resource-dependent
human identity/contamination metrics are extended and may correctly return
`not_evaluated`. This assignment is an implementation plan, not validation
approval.

Independent reference candidates include format specifications, hand-derived
fixtures, fastp, SAMtools/HTSlib, mosdepth, Picard, RNA-SeQC, and controlled
statistical calculations. Exact versions and tolerances remain validation work.

