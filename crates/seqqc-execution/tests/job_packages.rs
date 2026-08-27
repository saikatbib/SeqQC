use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use seqqc_execution::{
    JobSpec, NormalizedState, Scheduler, normalize_state, render_script, write_job_package,
};

#[test]
fn every_scheduler_renders_a_safe_offline_package() {
    let spec = fixture_spec();
    for scheduler in [
        Scheduler::Generic,
        Scheduler::Slurm,
        Scheduler::PbsPro,
        Scheduler::Lsf,
        Scheduler::Sge,
    ] {
        let script = render_script(&spec, scheduler).expect("render scheduler script");
        assert!(script.starts_with("#!/bin/sh\nset -eu\n"));
        assert!(script.contains("'C:/Program Files/SeqQC/tool'"));
        assert!(script.contains("'sample; touch SHOULD_NOT_EXIST'"));
        assert!(script.contains("'quote'\"'\"'value'"));
        assert!(!script.contains("curl "));
        assert!(!script.contains("wget "));

        let directory = temporary_directory(scheduler);
        let package = write_job_package(&spec, scheduler, &directory).expect("write job package");
        let decoded: JobSpec =
            serde_json::from_slice(&fs::read(&package.contract).expect("read generated contract"))
                .expect("decode generated contract");
        assert_eq!(decoded, spec);
        fs::remove_dir_all(directory).expect("remove job package");
    }
}

#[test]
fn state_mapping_is_conservative() {
    assert_eq!(
        normalize_state(Scheduler::Slurm, "COMPLETED"),
        NormalizedState::Succeeded
    );
    assert_eq!(
        normalize_state(Scheduler::Lsf, "EXIT"),
        NormalizedState::Failed
    );
    assert_eq!(
        normalize_state(Scheduler::PbsPro, "surprise"),
        NormalizedState::Unknown
    );
    assert_eq!(
        normalize_state(Scheduler::Generic, "lost"),
        NormalizedState::Lost
    );
}

fn fixture_spec() -> JobSpec {
    JobSpec {
        schema_version: "1.0.0".to_owned(),
        task_id: "task-0001".to_owned(),
        executable: PathBuf::from("C:/Program Files/SeqQC/tool"),
        arguments: vec![
            "--sample".to_owned(),
            "sample; touch SHOULD_NOT_EXIST".to_owned(),
            "quote'value".to_owned(),
            "病例.fastq.gz".to_owned(),
        ],
        cpu_cores: 4,
        memory_mib: 8_192,
        wall_minutes: 90,
    }
}

fn temporary_directory(scheduler: Scheduler) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock must be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "seqqc-execution-{scheduler:?}-{}-{nonce}",
        std::process::id()
    ))
}
