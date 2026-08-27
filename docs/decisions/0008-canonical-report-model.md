# ADR 0008: Canonical report model

- Status: Accepted
- Date: 2026-08-26

## Context

SeqQC must export dashboard, offline HTML, PDF, JSON, CSV, and TSV without
contradictory values or independent interpretation logic.

## Decision

- Build one immutable canonical report model from a completed run.
- Dashboard, HTML, PDF, and JSON read the same model.
- CSV and TSV are documented flattened projections with stable join keys.
- Exporters do not recalculate metrics or findings.
- The report model includes source run/manifest digest, ruleset and threshold
  provenance, methods, limitations, de-identification transform, and report
  content digest.
- HTML contains only packaged/embedded assets and makes no network request.
- PDF generation is offline and validated against the same semantic report
  model plus rendered-page regression tests.

## Consequences

- Exporter differences are presentation differences only.
- A report can be reconciled mechanically with its source JSON and completion
  manifest.
- The PDF renderer remains an implementation investigation with licensing and
  reproducibility gates.

