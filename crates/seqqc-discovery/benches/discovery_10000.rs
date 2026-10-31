use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use seqqc_discovery::{GroupStatus, discover};

const BATCH_COUNT: usize = 50;
const SAMPLES_PER_BATCH: usize = 200;
const EXPECTED_FILES: usize = BATCH_COUNT * SAMPLES_PER_BATCH * 2;

fn main() -> io::Result<()> {
    let fixture = BenchmarkDirectory::new()?;
    fixture.populate()?;

    let started = Instant::now();
    let result = discover(&fixture.path)?;
    let elapsed = started.elapsed();
    let paired_groups = result
        .groups
        .iter()
        .filter(|group| group.status == GroupStatus::PairedCandidate)
        .count();

    assert_eq!(result.summary.supported_files, EXPECTED_FILES);
    assert_eq!(result.summary.fastq_files, EXPECTED_FILES);
    assert_eq!(paired_groups, EXPECTED_FILES / 2);
    assert!(result.ignored.is_empty());

    println!(
        "{{\"supported_files\":{},\"paired_groups\":{},\"elapsed_ms\":{:.3}}}",
        result.summary.supported_files,
        paired_groups,
        elapsed.as_secs_f64() * 1000.0
    );
    Ok(())
}

struct BenchmarkDirectory {
    path: PathBuf,
}

impl BenchmarkDirectory {
    fn new() -> io::Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "seqqc-discovery-benchmark-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn populate(&self) -> io::Result<()> {
        for batch in 0..BATCH_COUNT {
            let directory = self.path.join(format!("project/batch-{batch:02}"));
            fs::create_dir_all(&directory)?;
            for sample in 0..SAMPLES_PER_BATCH {
                let prefix = format!("sample-{batch:02}-{sample:03}_S1_L001");
                File::create(directory.join(format!("{prefix}_R1_001.fastq.gz")))?;
                File::create(directory.join(format!("{prefix}_R2_001.fastq.gz")))?;
            }
        }
        Ok(())
    }
}

impl Drop for BenchmarkDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
