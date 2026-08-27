//! Scheduler-neutral, offline job-package rendering for `SeqQC`.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Supported scheduler script dialects.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scheduler {
    Generic,
    Slurm,
    PbsPro,
    Lsf,
    Sge,
}

/// A non-shell command and bounded resource request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JobSpec {
    pub schema_version: String,
    pub task_id: String,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub cpu_cores: u16,
    pub memory_mib: u64,
    pub wall_minutes: u32,
}

/// Normalized scheduler states retained alongside native state text.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedState {
    Queued,
    Held,
    Running,
    Completing,
    Succeeded,
    Failed,
    Cancelled,
    Preempted,
    Unknown,
    Lost,
}

/// One generated scheduler-neutral package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobPackage {
    pub directory: PathBuf,
    pub contract: PathBuf,
    pub script: PathBuf,
}

#[derive(Debug)]
pub enum ExecutionError {
    InvalidTaskId,
    InvalidResources,
    EmptyExecutable,
    Io(io::Error),
    Serialization(serde_json::Error),
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTaskId => formatter.write_str("task ID must be an opaque safe identifier"),
            Self::InvalidResources => {
                formatter.write_str("job resources must be greater than zero")
            }
            Self::EmptyExecutable => formatter.write_str("job executable cannot be empty"),
            Self::Io(error) => write!(formatter, "job package I/O error: {error}"),
            Self::Serialization(error) => write!(formatter, "job contract error: {error}"),
        }
    }
}

impl std::error::Error for ExecutionError {}

impl From<io::Error> for ExecutionError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ExecutionError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

/// Renders a scheduler script using quoted fixed argument values.
///
/// # Errors
///
/// Returns an error when the task identifier, executable, or resources are
/// invalid.
pub fn render_script(spec: &JobSpec, scheduler: Scheduler) -> Result<String, ExecutionError> {
    validate(spec)?;
    let mut lines = vec!["#!/bin/sh".to_owned(), "set -eu".to_owned()];
    lines.extend(directives(spec, scheduler));
    let command = std::iter::once(spec.executable.to_string_lossy().into_owned())
        .chain(spec.arguments.iter().cloned())
        .map(|value| shell_quote(&value))
        .collect::<Vec<_>>()
        .join(" ");
    lines.push(format!("exec {command}"));
    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// Writes `job.json` and the rendered script into an offline job package.
///
/// # Errors
///
/// Returns a validation, serialization, or I/O error when the package cannot
/// be created.
pub fn write_job_package(
    spec: &JobSpec,
    scheduler: Scheduler,
    directory: impl AsRef<Path>,
) -> Result<JobPackage, ExecutionError> {
    let script_text = render_script(spec, scheduler)?;
    let directory = directory.as_ref();
    fs::create_dir_all(directory)?;
    let contract = directory.join("job.json");
    let script = directory.join("run.sh");
    fs::write(&contract, serde_json::to_vec_pretty(spec)?)?;
    fs::write(&script, script_text)?;
    Ok(JobPackage {
        directory: directory.to_path_buf(),
        contract,
        script,
    })
}

/// Maps common scheduler states without fabricating success for unknown text.
#[must_use]
pub fn normalize_state(scheduler: Scheduler, native: &str) -> NormalizedState {
    let state = native.trim().to_ascii_uppercase();
    match scheduler {
        Scheduler::Slurm => match state.as_str() {
            "PENDING" | "PD" => NormalizedState::Queued,
            "RUNNING" | "R" => NormalizedState::Running,
            "COMPLETING" | "CG" => NormalizedState::Completing,
            "COMPLETED" | "CD" => NormalizedState::Succeeded,
            "FAILED" | "F" | "TIMEOUT" | "OUT_OF_MEMORY" => NormalizedState::Failed,
            "CANCELLED" | "CA" => NormalizedState::Cancelled,
            "PREEMPTED" | "PR" => NormalizedState::Preempted,
            _ => NormalizedState::Unknown,
        },
        Scheduler::PbsPro | Scheduler::Sge => match state.as_str() {
            "Q" | "W" => NormalizedState::Queued,
            "H" => NormalizedState::Held,
            "R" => NormalizedState::Running,
            "C" | "F" => NormalizedState::Completing,
            "E" => NormalizedState::Failed,
            _ => NormalizedState::Unknown,
        },
        Scheduler::Lsf => match state.as_str() {
            "PEND" | "WAIT" => NormalizedState::Queued,
            "RUN" => NormalizedState::Running,
            "DONE" => NormalizedState::Succeeded,
            "EXIT" | "ZOMBI" => NormalizedState::Failed,
            "PSUSP" | "USUSP" | "SSUSP" => NormalizedState::Held,
            _ => NormalizedState::Unknown,
        },
        Scheduler::Generic => match state.as_str() {
            "QUEUED" => NormalizedState::Queued,
            "RUNNING" => NormalizedState::Running,
            "SUCCEEDED" => NormalizedState::Succeeded,
            "FAILED" => NormalizedState::Failed,
            "CANCELLED" => NormalizedState::Cancelled,
            "LOST" => NormalizedState::Lost,
            _ => NormalizedState::Unknown,
        },
    }
}

fn validate(spec: &JobSpec) -> Result<(), ExecutionError> {
    if spec.task_id.is_empty()
        || !spec
            .task_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ExecutionError::InvalidTaskId);
    }
    if spec.executable.as_os_str().is_empty() {
        return Err(ExecutionError::EmptyExecutable);
    }
    if spec.cpu_cores == 0 || spec.memory_mib == 0 || spec.wall_minutes == 0 {
        return Err(ExecutionError::InvalidResources);
    }
    Ok(())
}

fn directives(spec: &JobSpec, scheduler: Scheduler) -> Vec<String> {
    match scheduler {
        Scheduler::Generic => vec![format!(
            "# resources: cpus={} memory_mib={} wall_minutes={}",
            spec.cpu_cores, spec.memory_mib, spec.wall_minutes
        )],
        Scheduler::Slurm => vec![
            format!("#SBATCH --job-name={}", spec.task_id),
            format!("#SBATCH --cpus-per-task={}", spec.cpu_cores),
            format!("#SBATCH --mem={}M", spec.memory_mib),
            format!("#SBATCH --time={}:00", spec.wall_minutes),
        ],
        Scheduler::PbsPro => vec![
            format!("#PBS -N {}", spec.task_id),
            format!(
                "#PBS -l select=1:ncpus={}:mem={}mb",
                spec.cpu_cores, spec.memory_mib
            ),
            format!("#PBS -l walltime={}:00", spec.wall_minutes),
        ],
        Scheduler::Lsf => vec![
            format!("#BSUB -J {}", spec.task_id),
            format!("#BSUB -n {}", spec.cpu_cores),
            format!("#BSUB -M {}", spec.memory_mib),
            format!("#BSUB -W {}", spec.wall_minutes),
        ],
        Scheduler::Sge => vec![
            format!("#$ -N {}", spec.task_id),
            format!("#$ -pe smp {}", spec.cpu_cores),
            format!("#$ -l h_vmem={}M", spec.memory_mib),
            format!("#$ -l h_rt={}:00", spec.wall_minutes),
        ],
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
