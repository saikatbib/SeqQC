# Project data model

## 1. Goals

The data model must make results portable, inspectable, migratable, and
traceable without exposing protected metadata. JSON examples below are
conceptual; exact JSON Schema and database migrations will be produced during
the foundation milestone.

## 2. Identifier rules

- Internal IDs are random UUIDs and never derived from patient/sample names.
- User-facing aliases are separate, classified fields.
- IDs are immutable after creation.
- Each entity includes `schema_version`, `created_at`, and provenance as
  appropriate.
- Timestamps are UTC with explicit offsets at display boundaries.

## 3. Core entities

| Entity | Purpose |
| --- | --- |
| `Project` | Security, context, ownership, policy, and collections |
| `Subject` | Optional sensitive subject linkage; separated from sample identity |
| `Sample` | Biological sample and non-sensitive operational alias |
| `Library` | Library preparation, assay, layout, capture, strandedness |
| `InputArtifact` | FASTQ/BAM/CRAM/sample-sheet identity and location evidence |
| `ReferenceBundle` | FASTA, dictionaries/indexes, BED, GTF/GFF fingerprints |
| `ToolInstallation` | Executable identity, version, platform, validation state |
| `Preset` | Versioned scientific/performance/policy configuration |
| `Run` | Immutable execution intent and outcome |
| `Task`/`Attempt` | Workflow state, scheduler identity, resources, logs |
| `Metric` | Normalized scientific observation |
| `Finding` | Ruleset-derived interpretation of metrics |
| `Annotation` | User comment/adjudication that does not rewrite evidence |
| `Report` | Versioned projection of a completed run |
| `AuditEvent` | Append-only security and lifecycle record |

## 4. Project manifest

The cleartext outer manifest contains only information required to recognize and
unlock a project. It must not contain sample identifiers, paths, findings, or
clinical context.

```json
{
  "format": "org.seqqc.project",
  "format_version": "1.0.0",
  "project_id": "uuid",
  "crypto_profile": "seqqc-project-v1",
  "key_locator": "opaque-os-key-id",
  "created_by_app": "1.0.0"
}
```

## 5. Project record

```json
{
  "project_id": "uuid",
  "display_name": "classified field",
  "context": "human_research",
  "presentation_profile": "guided",
  "owner_local_principal_id": "uuid",
  "single_writer_policy": true,
  "retention_policy_id": "uuid",
  "active_threshold_preset": {"id": "uuid", "version": "1.2.0"},
  "created_at": "RFC3339 UTC",
  "archived_at": null
}
```

## 6. Artifact identity

```json
{
  "artifact_id": "uuid",
  "kind": "fastq_r1",
  "path": {"value": "encrypted", "classification": "sensitive"},
  "size_bytes": 123,
  "mtime_observed": "RFC3339 UTC",
  "fingerprints": [
    {"algorithm": "sha256", "mode": "full", "value": "hex"}
  ],
  "stability_check": {"before": "...", "after": "...", "stable": true},
  "sample_id": "uuid",
  "library_id": "uuid",
  "read": 1,
  "lane": "L001"
}
```

Fast fingerprints may accelerate discovery but cannot replace the full integrity
fingerprint required by a controlled preset.

## 7. Run and workflow plan

```json
{
  "run_id": "uuid",
  "run_number": 7,
  "workflow": {"id": "bulk-rnaseq-qc", "version": "1.0.0"},
  "state": "completed",
  "input_artifact_ids": ["uuid"],
  "reference_bundle": {"id": "uuid", "version": "2026-01"},
  "effective_presets": [
    {"kind": "scientific", "id": "uuid", "version": "1.0.0"},
    {"kind": "performance", "id": "uuid", "version": "1.0.0"}
  ],
  "resolved_parameters": {},
  "workflow_graph_digest": "sha256:...",
  "started_at": "RFC3339 UTC",
  "completed_at": "RFC3339 UTC",
  "completion_manifest_digest": "sha256:..."
}
```

## 8. Normalized metric

```json
{
  "metric_id": "alignment.mapped.fraction",
  "metric_schema_version": "1.0.0",
  "scope": {"type": "sample", "id": "uuid"},
  "value": 0.974,
  "value_type": "number",
  "unit": "fraction",
  "numerator": 974,
  "denominator": 1000,
  "method": {"id": "seqqc-alignment-accounting", "version": "1.0.0"},
  "filters": {},
  "source_artifact_ids": ["uuid"],
  "applicability": "applicable",
  "quality": "validated",
  "computed_at": "RFC3339 UTC"
}
```

Large distributions use typed artifact files with shape, column, compression,
and checksum metadata rather than oversized database JSON fields.

## 9. Finding

```json
{
  "finding_id": "uuid",
  "rule": {"id": "wgs.mapping-rate", "version": "1.0.0"},
  "scope": {"type": "sample", "id": "uuid"},
  "status": "needs_review",
  "severity": 3,
  "metric_refs": ["alignment.mapped.fraction"],
  "observations": [{"value": 0.80, "unit": "fraction"}],
  "expected": {"operator": ">=", "value": 0.95},
  "threshold_source": {"preset_id": "uuid", "version": "1.0.0"},
  "evidence_quality": "validated",
  "content_key": "finding.wgs.mapping_rate.low",
  "content_version": "1.0.0",
  "generated_at": "RFC3339 UTC"
}
```

Localized prose is resolved from versioned packaged content. Exported reports
retain the exact rendered text or its content bundle fingerprint.

## 10. Audit event

```json
{
  "event_id": "uuid",
  "sequence": 184,
  "previous_event_digest": "sha256:...",
  "occurred_at": "RFC3339 UTC",
  "actor": {"local_principal_id": "uuid"},
  "action": "report.exported",
  "object": {"type": "report", "id": "uuid"},
  "outcome": "success",
  "details": {"format": "pdf", "destination_class": "user_selected"},
  "event_digest": "sha256:..."
}
```

The hash chain is tamper-evident, not magically tamper-proof. Verification and
key-protection limitations must be documented honestly.

## 11. Report model

All exporters consume one immutable report model containing:

- identity and scope;
- project/run context permitted by the export policy;
- executive findings and metrics;
- sample and cohort detail;
- methods, references, thresholds, limitations, and citations;
- tool/workflow/application versions;
- report generation environment;
- de-identification transform;
- source completion-manifest digest; and
- report-content digest.

JSON is the canonical machine-readable report projection. CSV/TSV are flattened
views and must include join keys and data dictionaries.

## 12. Portable archive

An archive must include an authenticated encrypted payload, versioned crypto
metadata, salt/KDF parameters, content manifest, and integrity tag. Passwords
are never stored. Import must defend against path traversal, decompression bombs,
unexpected file types, schema downgrade, and overwrite collisions.

The cryptographic primitives and format are deliberately not selected in this
planning document; they require a security decision record and independent
review.

## 13. Compatibility and migration

- Semantic versions apply to public schemas.
- Readers accept only explicitly supported older versions.
- Migration occurs on a copy/transaction with verified backup.
- Original completed-run manifests remain available after migration.
- Unknown enum values must not be reinterpreted silently.
- A newer unsupported schema opens read-only if safe, otherwise fails with a
  precise compatibility message.

