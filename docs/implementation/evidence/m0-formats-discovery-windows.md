# M0 format facade and discovery-scale evidence: Windows

**Evidence date:** 2026-08-26  
**Platform:** Windows x86-64  
**Status:** Initial spike passed with listed limitations

## Questions under test

1. Can SeqQC stream the initial FASTQ, gzip FASTQ, BAM, and CRAM inputs behind
   one application-owned summary contract?
2. Do independent, deliberately small comparison readers agree on the fixed
   fixture evidence?
3. Can recursive discovery group 10,000 local FASTQ paths without silently
   resolving ambiguity?
4. Do important path forms behave correctly on the Windows development host?

## Parser selection and boundary

The spike pins [noodles 0.115.0](https://docs.rs/crate/noodles/0.115.0)
with only BAM, core, CRAM, FASTA, FASTQ, and SAM features. Upstream describes
the API as experimental, so no noodles type crosses `seqqc-formats`' public
boundary. Gzip support uses flate2 1.1.9 with its Rust backend. Both exact
resolutions are retained in `Cargo.lock`.

The current facade returns structural evidence only: format/compression, file
size, record/base counts, read-length range, mapped/unmapped/paired counts, and
reference-sequence count. These are parser-spike results, not validated assay
QC metrics.

## Synthetic fixtures

The fixtures under `test-data/synthetic/tiny-formats` contain two artificial
four-base reads and one artificial eight-base reference. They have no human,
animal, clinical, public-dataset, or sequencer origin.

| Fixture | Bytes | SHA-256 |
| --- | ---: | --- |
| `reference.fa` | 34 | `33d2896ae28823014e8676a2626404a86d1d0e155bad4a5e123a6f1decf5602f` |
| `reference.fa.fai` | 13 | `ee4a59a35bc17fa6efed71348dc0a71cc333a6e3f94ac5910dfc4ad575c8f8d4` |
| `tiny.fastq` | 66 | `c5d6ab69e7abdc8abdb655c32fd668b10960bef5996422af435497c1126fabd6` |
| `tiny.fastq.gz` | 69 | `148552a54efbb77dfc58b2780f06ac7f1a911489d2e0c8486635cdf0b0dca6b4` |
| `tiny.bam` | 241 | `6a785d943746bf8947802d1b57ac871e28f729de7e6149c996e1cc6735b98987` |
| `tiny.cram` | 1,059 | `1323b7dcc7e5c073dac04188e949de1530b4cc4ea4ad04787be6f698962333fa` |

The noodles CRAM writer emitted logically identical decoded content but a
different byte stream when the generator was repeated. The committed CRAM is a
frozen regression input; its checksum must not be advertised as a reproducible
encoding result.

## Comparison results

Three focused Rust tests passed:

- the facade produced 2 records, 8 bases, and 4-base min/max lengths for plain
  and gzip FASTQ, BAM, and CRAM;
- BAM and CRAM each produced 1 mapped, 1 unmapped, 0 paired records, and 1
  reference-sequence header entry;
- a separate four-line FASTQ reader, manual BAM binary-core reader, and manual
  CRAM 3.0 container-header reader agreed with the facade;
- malformed FASTQ sequence/quality length and unsupported extensions failed
  with typed I/O categories.

The BAM comparison independently parses decompressed BAM binary fields. The
CRAM comparison independently verifies its file definition and container record
counts, but it does not independently decode CRAM slices. A future staged
`samtools`/HTSlib comparison remains required for full independent CRAM
validation.

## Discovery-scale results

The optimized benchmark creates 50 batch directories containing 5,000 paired
samples represented by 10,000 empty `.fastq.gz` entries. File creation is
excluded from the timed interval. Five discovery measurements were:

```text
80.629 ms
81.714 ms
87.188 ms
69.915 ms
73.034 ms
```

Median: **80.629 ms**. Range: **69.915–87.188 ms**. Every run returned exactly
10,000 supported files and 5,000 paired candidates with no ignored entries.
This is a local metadata benchmark, not a guarantee for network storage,
antivirus configurations, slow disks, or UI rendering.

## Path and robustness coverage

Passed on Windows:

- nested directories;
- Unicode directory and filenames;
- spaces and leading dashes;
- mixed-case FASTQ/CRAM extensions;
- missing and duplicate mate reporting;
- malformed FASTQ rejection.

Not yet evidenced:

- Windows symbolic-link creation was denied for this account, so the symlink
  assertion reported a skip;
- UNC/mapped network-drive latency, disconnects, and permission changes;
- peak-memory measurement and cancellation during a 10,000-file scan;
- corrupt/truncated BAM and CRAM corpus;
- macOS/Linux equivalence;
- independent `samtools`/HTSlib CRAM decoding.

## Reproduction

```powershell
& .\tools\invoke-rust.ps1 test --locked --package seqqc-formats
& .\tools\invoke-rust.ps1 test --locked --package seqqc-discovery
& .\tools\invoke-rust.ps1 bench --locked --package seqqc-discovery --bench discovery_10000
& .\tools\invoke-rust.ps1 clippy --locked --package seqqc-formats --all-targets '--' '-Dwarnings'
& .\tools\invoke-rust.ps1 clippy --locked --package seqqc-discovery --all-targets '--' '-Dwarnings'
```

