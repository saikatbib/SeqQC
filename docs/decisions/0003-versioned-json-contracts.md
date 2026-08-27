# ADR 0003: Versioned JSON contracts

- Status: Accepted
- Date: 2026-08-26

## Context

Projects must be portable across operating systems and versions. Scientific
workers, schedulers, exporters, and validation tooling require inspectable
contracts independent of the desktop UI.

## Decision

- Define public data contracts with JSON Schema Draft 2020-12.
- Give every root schema a stable `https://schemas.seqqc.org/` identifier even
  though schemas are resolved from packaged local files at runtime.
- Never retrieve a schema URI from the network.
- Use semantic versions for public contracts and explicit per-entity
  `schema_version` values.
- Reject unknown required enum values and unsupported major versions.
- Keep artifact distributions in checksummed typed files referenced by JSON,
  rather than embedding large arrays in project records.
- Generate language types from schemas only after schema validation; generated
  types never become the source of truth.

## Consequences

- Schema bundles are release artifacts and part of validation evidence.
- Migrations operate transactionally with a backup and retain original run
  manifests.
- A local schema-reference checker runs without third-party dependencies in the
  Milestone 0 scaffold.

