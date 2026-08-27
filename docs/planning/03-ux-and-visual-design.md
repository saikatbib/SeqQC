# UX and visual design specification

## 1. Experience principles

1. **Answer before inventory.** Lead with what needs attention and why, then
   expose plots and raw evidence.
2. **Progressive disclosure.** Guided, Standard, and Expert modes use identical
   calculations but different density.
3. **Never guess silently.** Ambiguous pairing, assay, reference, strandedness,
   or sample relationships require visible resolution.
4. **Separate failure from biology.** A corrupt file is an operational error; an
   unusual GC profile is a scientific finding.
5. **Make provenance adjacent.** Users can see the effective threshold and
   method without leaving the result context.
6. **Protect people from color-only meaning.** Status always combines text,
   icon/shape, and color.
7. **Stay calm.** The interface is scientific and precise, not alarmist or
   cyberpunk.

## 2. Information architecture

```text
Home
├── New project
├── Recent projects
└── Import encrypted project

Project
├── Overview
├── Findings
├── Samples
│   └── Sample detail
├── Cohort
├── Runs
│   └── Run monitor / task detail
├── Reports
├── References & presets
├── Audit & provenance
└── Project settings

Application
├── Tool registry
├── Scheduler profiles
├── Security & storage
├── Appearance & accessibility
└── Offline system readiness
```

## 3. New-project wizard

### Step 1: Project context

Collect project name, human clinical/human research/preclinical/other context,
local storage location, and security policy. Explain why context affects privacy
and audit behavior but not silent scientific assumptions.

### Step 2: Input selection

Select folders or a sample sheet. Show recursion and include/exclude settings.
Discovery immediately reports supported files, ignored files, duplicates,
unreadable items, and changing files.

### Step 3: Sample mapping

Display proposed sample → library → lane → mate relationships. Every proposal
has confidence and evidence. Ambiguity is resolved inline; bulk acceptance is
allowed only for high-confidence, non-conflicting mappings.

### Step 4: Assay and library

Select WGS, WES, or bulk RNA-seq and relevant metadata. Show only applicable
fields. “Unknown” is available where scientifically permissible and results in
explicitly unevaluated metrics.

### Step 5: References

Select a registered local reference bundle or register one. Show fingerprints,
contig compatibility, missing indexes, and storage/time estimates for local
index generation.

### Step 6: QC or alignment

Existing aligned inputs go directly to QC. FASTQ projects choose QC only or the
curated assay alignment workflow followed by QC. The user sees output/scratch
requirements before choosing alignment.

The initial release offers the alignment choice only when a validated Linux
execution provider is active. Windows and macOS show the Linux requirement in
the same step and keep FASTQ QC available; the disabled action must not look like
a transient error or suggest that SeqQC will download/install a runtime.

### Step 7: Thresholds and execution

Choose threshold preset and Local/HPC profile. Scientific and performance
settings are visually separated. Expert overrides display comparability impact.

### Step 8: Preflight and confirmation

Summarize samples, inputs, references, workflow, tools, estimates, warnings, and
blocking errors. No analysis begins until the plan is confirmed.

## 4. Dashboard hierarchy

The Overview screen answers four questions in order:

1. Is this run complete and trustworthy enough to review?
2. How many samples are within range or need review?
3. What are the most important findings and actions?
4. Where is the underlying evidence?

### Desktop layout

```text
┌────────────────────────────────────────────────────────────────────┐
│ SeqQC Studio | project | search | mode | theme | local/offline     │
├──────────────┬─────────────────────────────────────────────────────┤
│ Overview     │ Run identity, assay, preset, completion/integrity   │
│ Findings     ├─────────────────────────────────────────────────────┤
│ Samples      │ Status counts + prioritized findings                │
│ Cohort       ├────────────────────────────┬────────────────────────┤
│ Runs         │ Cohort distribution/trend  │ Recommended actions    │
│ Reports      ├────────────────────────────┴────────────────────────┤
│ Audit        │ Sample table with status, key metrics, drill-down   │
└──────────────┴─────────────────────────────────────────────────────┘
```

At narrow widths the sidebar becomes a labeled menu and panels stack. Essential
status and actions remain visible without hover.

## 5. Finding presentation

A finding row shows status text, title, affected scope, observed value, expected
range, and one recommended action. Expanding it reveals:

- why the metric matters;
- plot/table evidence;
- calculation method and filters;
- threshold source and competing overrides;
- possible causes stated conditionally;
- prioritized troubleshooting steps;
- limitations and applicability;
- user comment/adjudication history; and
- direct navigation to logs/provenance for experts.

Status order is critical issue, needs review, borderline, within expected range,
not evaluated, and not applicable. Users may sort differently but the default
must not bury serious findings.

## 6. Cohort experience

- Compatibility banner states assay, reference, target/annotation, workflow,
  and preset consistency.
- Filters use declared metadata and expose sample counts after filtering.
- Distribution plots show individual samples plus cohort context; summary
  statistics never hide data points by default.
- Outliers state the fixed and/or robust cohort rule used.
- Selecting a sample highlights it consistently across plots and the table.
- PCA/correlation views show transformation, filtering, cohort size, and a clear
  descriptive—not diagnostic—notice.

## 7. Run monitor

The monitor shows workflow stages, task status, elapsed time, resource use when
available, scheduler identity, and recoverability. It avoids fabricated progress
percentages. Tasks with measurable units may show percent; otherwise show the
current stage and elapsed time.

Cancellation explains whether the current task can stop immediately, at a safe
checkpoint, or only through the scheduler. Partial artifacts are clearly marked
and never appear as completed results.

## 8. Report builder

Users choose audience, sections, samples, identifiers, and output formats. A
privacy preview lists fields that will be included or removed. A clinical-
context project defaults to de-identified exports unless policy specifies an
approved identified destination.

The same report model feeds HTML, PDF, and JSON. CSV/TSV options are described
as metric/finding tables rather than equivalent narrative reports.

## 9. Theme system

### Clinical night (default dark)

- Graphite/navy surfaces
- Off-white text with high contrast
- Restrained teal/cyan focus and quantitative series
- Amber for review, red reserved for critical/error, green used sparingly
- Subtle elevation, no neon glow, no animated background

### Clinical light

- Neutral white/soft-gray surfaces
- Ink/navy text
- The same semantic mapping at accessible contrast

### High contrast

- Maximum separation, stronger focus indicators, simplified surfaces
- Compatible with OS contrast preferences where practical

### Aurora (optional expressive theme)

- Deeper gradients and cyan/violet accents limited to chrome and selected data
  marks
- Never reduces contrast or changes semantic meaning

Theme changes appearance only. Exported reports record theme and allow a
print-optimized independent theme. Motion must respect reduced-motion settings.

## 10. Typography and visualization

- Use a locally packaged, redistributable sans-serif family with tabular
  numerals; no remote fonts.
- Maintain a clear type scale and avoid all-uppercase prose.
- Scientific symbols and units use accessible Unicode plus export-safe fallback.
- Plots include title, axis labels, units, direct labels where possible, and
  accessible summaries.
- Use consistent metric colors across samples, but never assign decorative
  rainbow colors to every metric.
- Tooltips supplement rather than contain essential information.
- Tables support keyboard navigation, column explanations, filtering, export,
  and large-cohort virtualization.

## 11. Accessibility acceptance

- Complete primary journeys using keyboard alone.
- Visible focus and logical focus order.
- Minimum AA contrast for ordinary text and meaningful graphics.
- Status understandable in monochrome.
- 200% text scaling/reflow without lost controls.
- Screen-reader names for plots plus underlying accessible tables or summaries.
- No essential timed interaction.
- Reduced motion and high-contrast options.
- Plain-language glossary available beside scientific terms.

## 12. Usability studies

Test each milestone with at least one representative from beginner/wet-lab,
core/bioinformatics, and controlled-workflow roles. Version 1 targets:

- at least 90% task completion for creating and running a valid guided project;
- no unresolved high-severity use error in input mapping or report
  interpretation;
- median beginner time to identify the highest-priority finding under five
  minutes after opening a completed report; and
- expert ability to locate raw metric, method, threshold source, and tool log in
  under two minutes.

## 13. Planning wireframe

The interactive dashboard wireframe is maintained as a conversation planning
artifact rather than application source. Its purpose is to validate hierarchy,
theme behavior, presentation-level changes, and realistic terminology before
implementation.
