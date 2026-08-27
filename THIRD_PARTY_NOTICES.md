# Third-party software notices

SeqQC source is licensed under Apache-2.0. It depends on third-party software
under its own licenses. This file is a review aid, not a substitute for the
license files in released artifacts or the generated SBOM.

## Direct application and build dependencies

| Component | Pinned version | License signal |
| --- | ---: | --- |
| Tauri / Tauri build / CLI / JavaScript API | 2.11.5 / 2.6.3 / 2.11.4 / 2.11.1 | MIT or Apache-2.0 |
| Tauri dialog plugin | 2.7.2 | MIT or Apache-2.0 |
| React / React DOM | 19.2.8 | MIT |
| TypeScript | 7.0.2 | Apache-2.0 |
| Vite | 8.2.2 | MIT |
| noodles | 0.115.0 | MIT |
| flate2 | 1.1.9 | MIT or Apache-2.0 |
| rusqlite | 0.40.2 | MIT |
| SQLCipher Community | 4.14.0 | BSD-style |
| OpenSSL | 3.6.3 | Apache-2.0 |
| keyring | 4.1.6 | MIT or Apache-2.0 |
| getrandom | 0.3.4 | MIT or Apache-2.0 |
| sha2 | 0.10.9 | MIT or Apache-2.0 |
| zeroize | 1.9.0 | Apache-2.0 or MIT |
| hex | 0.4.3 | MIT or Apache-2.0 |
| serde / serde_json | 1.0.229 / 1.0.151 | MIT or Apache-2.0 |
| time | 0.3.55 | MIT or Apache-2.0 |

The exact transitive dependency graph is defined by `Cargo.lock` and
`pnpm-lock.yaml`. Release CI generates `artifacts/sbom/seqqc.cdx.json`; release
review must retain all license texts required by those components.

## External tools

BWA-MEM2, STAR, samtools/HTSlib, fastp, mosdepth, Picard, RNA-SeQC, and
VerifyBamID2 are evaluation candidates only. They are not bundled in the
Milestone 0 preview. A future bundle requires separate artifact, license,
checksum, provenance, and scientific-method approval.
