# Execution and scheduler specification

## 1. Scope

SeqQC supports a common workflow contract across local execution, generic script
export, Slurm, PBS Pro, LSF, and SGE. Scheduler support is offline and local to
the host: version 1 invokes installed scheduler command-line clients and does not
open SSH sessions or call remote APIs.

QC execution targets Windows, macOS, and Linux. Tasks that invoke a sequence
aligner are eligible only on validated Linux providers in the initial release.
The planner rejects an alignment graph on a Windows/macOS provider before
submission; it does not attempt a compatibility fallback.

## 2. Workflow model

A workflow is a versioned directed acyclic graph of typed tasks. Each task
declares:

- task/module ID and version;
- immutable input and reference artifact identities;
- expected outputs and schemas;
- argument vector and restricted environment requirements;
- CPU, memory, wall-time, scratch, GPU (normally none), and I/O estimates;
- dependencies and retry policy;
- cancellation/checkpoint behavior;
- validation and commit procedure; and
- platform/tool capability requirements.

Scientific parameters are part of the task digest. Scheduler options such as
partition/queue are execution metadata and do not change scientific identity.

## 3. Preflight and estimation

Preflight detects:

- logical/physical CPU count and relevant instruction sets;
- available/total RAM;
- free bytes and filesystem type for project, output, and scratch paths;
- path writability and filename/path-length constraints;
- input/reference sizes and estimated read/record counts when possible;
- validated tool availability and executable fingerprints;
- scheduler client/version/profile availability; and
- configured quotas/limits supplied by the user or institution.

The estimator produces expected and conservative peak values plus assumptions.
It recommends concurrency separately for CPU, memory, and I/O pressure. The user
may lower concurrency freely; unsafe increases require a clear warning and may
be prohibited by institutional policy.

## 4. Local executor

- Use a bounded worker pool and a central resource token manager.
- Launch processes without a shell wherever possible.
- Use explicit working/output/scratch directories per task attempt.
- Capture structured events plus bounded, redacted stdout/stderr.
- Stream large tool output directly to files/parsers rather than UI memory.
- Respond to cancellation through a graceful signal, timeout, then process-tree
  termination as supported.
- Do not commit output until validation and checksumming succeed.
- Persist state transitions transactionally so restart recovery is possible.

## 5. HPC path mapping

A scheduler profile maps desktop-visible roots to compute-visible roots:

```text
Desktop: X:\sequencing\runs     Compute: /shared/sequencing/runs
Desktop: D:\SeqQC\projects     Compute: /shared/seqqc/projects
```

Mappings are administrator/user configuration and are tested with a harmless
read/write probe before scientific submission. Path mapping must use structural
prefix rules, not string replacement in generated shell commands.

If the application is running directly on a Linux login node, mappings may be
identity mappings. SeqQC does not transfer large files to a cluster in version
1.

## 6. Scheduler-neutral job package

Each submitted unit contains:

- immutable `job.json` task contract and digest;
- generated scheduler script;
- environment/tool manifest;
- input/reference manifest;
- attempt directory;
- status heartbeat/events;
- stdout/stderr files; and
- authenticated/checksummed result envelope.

The script reads fixed, quoted values from the package. User-supplied filenames,
sample names, and metadata must never become executable shell syntax. Sample
names are not used as scheduler job names without sanitization; an opaque task
ID is preferred.

## 7. Adapter contract

Each adapter implements:

1. detect client and scheduler version;
2. validate profile;
3. render script/directives;
4. submit and return normalized job ID;
5. query normalized state;
6. cancel;
7. resolve terminal reason/exit status;
8. locate accounting/resource data when available; and
9. recover status after application restart.

Normalized states are `queued`, `held`, `running`, `completing`, `succeeded`,
`failed`, `cancelled`, `preempted`, `unknown`, and `lost`. Native states and raw
messages remain in expert/audit detail.

## 8. Adapter mapping

| Capability | Slurm | PBS Pro | LSF | SGE |
| --- | --- | --- | --- | --- |
| Submit | `sbatch` | `qsub` | `bsub` | `qsub` |
| Query | `squeue`/accounting | `qstat` | `bjobs` | `qstat`/accounting |
| Cancel | `scancel` | `qdel` | `bkill` | `qdel` |
| Array jobs | Adapter capability | Adapter capability | Adapter capability | Adapter capability |
| Dependencies | Native where profile permits | Native | Native | Native where available |

Exact flags and parsing must be based on detected scheduler/version and tested
against supported versions. This table is not an implementation command recipe.

## 9. Generic script export

Generic export creates a self-contained offline job package and human-readable
instructions. It does not mark a run complete automatically. Results are
imported through a verification action that checks task, workflow, input,
reference, tool, output, and schema digests.

Generic export is the fallback for unsupported scheduler versions and controlled
environments that prohibit direct submission.

## 10. Arrays and scaling

- Independent sample tasks may become scheduler arrays when the adapter/profile
  supports them.
- Aggregation tasks wait for all required verified sample results.
- Array chunk size and maximum concurrent tasks follow profile limits.
- Retries target failed elements rather than rerunning successful samples.
- Large cohorts use paged state updates and bounded event ingestion.
- The UI must never poll every job at an unbounded frequency.

## 11. Containers and environments

SeqQC does not require internet access to resolve an environment. Optional
institutional execution modes may use preinstalled Apptainer/Singularity or
other locally approved runtimes and local image paths. Images are fingerprinted
and included in provenance.

Container use does not expand the declared alignment platform automatically.
Version 1 must clearly label which execution providers and tools are validated
per platform. A Windows/macOS container or WSL alignment path remains
unsupported until separately specified and validated. No image is pulled
automatically.

## 12. Failure and recovery

- Submission ambiguity triggers status reconciliation before retry.
- A disappeared job becomes `unknown`/`lost`, not automatically failed.
- Preemption is distinct from tool failure and may be retryable.
- Out-of-memory and wall-time termination receive actionable resource guidance.
- Staged but complete outputs can be verified after restart.
- Changed inputs/references invalidate the attempt and require a new run plan.
- Scheduler output parsing errors preserve raw evidence and never fabricate a
  successful state.

## 13. Resource safety

The scheduler profile defines maximum jobs, cores, memory, wall time, scratch,
allowed queues/partitions, account/project code, and environment initialization.
Secrets and credentials must not be embedded in scripts. Scheduler profiles are
versioned and changes are audited.

## 14. Execution acceptance tests

- Contract-test every adapter against recorded outputs plus live test fixtures
  for supported scheduler versions.
- Verify paths containing spaces, Unicode, leading dashes, and shell metacharacters.
- Verify cancellation, preemption, timeout, OOM, lost job, delayed accounting,
  array partial failure, and application restart.
- Demonstrate equivalent normalized metrics for local and HPC execution on the
  same validation inputs within declared tolerances.
- Demonstrate that Windows/macOS providers reject alignment tasks during
  planning while continuing to run supported QC tasks.
- Demonstrate zero internet connection attempts.
