# Contribution, review, signing, and change-control policy

SeqQC uses protected pull-request review as the intended public workflow.
Ordinary source changes require one maintainer approval. Scientific methods,
thresholds, cryptography, key management, schema compatibility, new runtime
capabilities, external binary redistribution, and release signing require an
independent domain reviewer in addition to the change author.

Release credentials never enter the repository or CI logs. Platform signing
keys should live in the operating-system or institutional secret service and be
available only to a named release owner. Current Milestone 0 artifacts are
unsigned engineering previews; source publication does not imply installer
trust or clinical validation.

Accepted ADRs may be changed only through a superseding ADR. Contract changes
must state compatibility and migration effects. Scientific changes must update
the metric registry and validation evidence. Every release retains its exact
lockfiles, SBOM, checksums, notices, test results, limitations, and signing
record.
