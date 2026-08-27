# Tiny sequencing-format fixtures

These files contain deterministic, wholly synthetic Milestone 0 content. They do
not originate from a person, animal, clinical system, sequencer, or public
dataset and contain no protected or identifying data.

The generator is
`crates/seqqc-formats/examples/generate_tiny_fixtures.rs`. It writes:

- two four-base FASTQ records, both plain and gzip-compressed;
- one artificial eight-base reference named `sq0` and its FAI;
- one BAM and one CRAM containing the same mapped and unmapped records.

Expected normalized evidence for every logical two-record input is 2 records,
8 total bases, and a fixed read length of 4. BAM and CRAM additionally contain
one mapped and one unmapped record and one reference-sequence header entry.

Regenerate logical fixture content only with the pinned lockfile:

```powershell
& .\tools\invoke-rust.ps1 run --locked --package seqqc-formats --example generate_tiny_fixtures
```

The current noodles CRAM writer does not produce a byte-identical CRAM encoding
on repeated generation, although the decoded records and independent container
counts agree. The committed CRAM is therefore a frozen regression artifact;
regeneration requires review and an intentional checksum update. Binary fixture
checksums and independent comparison results are recorded in the corresponding
implementation evidence document and are not treated as biological truth.
