use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use eframe::egui;

#[derive(Default)]
pub struct Foundation {
    pub reference: Option<PathBuf>,
    pub annotation: Option<PathBuf>,
    pub targets: Option<PathBuf>,
    pub simulation: Simulation,
    checks: Option<[bool; 4]>,
    check_job: Option<Receiver<[bool; 4]>>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Simulation {
    #[default]
    Idle,
    Queued,
    Running {
        step: usize,
        started: Instant,
    },
    Completed,
    Cancelled,
    Interrupted,
}

impl Simulation {
    pub const fn stored(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Queued => "queued",
            Self::Running { .. } | Self::Interrupted => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn advance(&mut self) {
        *self = match *self {
            Self::Queued => Self::Running {
                step: 0,
                started: Instant::now(),
            },
            Self::Running { step, started } if started.elapsed() >= Duration::from_millis(500) => {
                if step == 2 {
                    Self::Completed
                } else {
                    Self::Running {
                        step: step + 1,
                        started: Instant::now(),
                    }
                }
            }
            state => state,
        };
    }

    pub const fn active(self) -> bool {
        matches!(self, Self::Queued | Self::Running { .. })
    }
}

impl Foundation {
    pub fn invalidate_checks(&mut self) {
        self.checks = None;
        self.check_job = None;
    }
    pub fn with_resources(
        reference: Option<PathBuf>,
        annotation: Option<PathBuf>,
        targets: Option<PathBuf>,
        simulation: Simulation,
    ) -> Self {
        Self {
            reference,
            annotation,
            targets,
            simulation,
            ..Self::default()
        }
    }
    fn refresh(&mut self, input: Option<&std::path::Path>) {
        let input = input.map(std::path::Path::to_path_buf);
        let paths = [
            self.reference.clone(),
            self.annotation.clone(),
            self.targets.clone(),
        ];
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let mut result = [false; 4];
            result[0] = input.is_some_and(|path| path.is_dir());
            for (index, path) in paths.into_iter().enumerate() {
                result[index + 1] = path.is_some_and(|path| {
                    std::fs::File::open(path)
                        .and_then(|file| file.metadata())
                        .is_ok_and(|metadata| metadata.is_file())
                });
            }
            let _ = sender.send(result);
        });
        self.checks = None;
        self.check_job = Some(receiver);
    }

    fn resource_status(&self, configured: bool, index: usize) -> &'static str {
        if configured {
            match self.checks {
                Some(checks) if checks[index] => "readable; compatibility not evaluated",
                Some(_) => "unavailable or unreadable",
                None => "checking local accessibility",
            }
        } else {
            "not configured"
        }
    }

    pub fn report(&self, project_open: bool) -> String {
        format!(
            "SeqQC Studio system readiness\nVersion: {}\nPlatform: {}/{}\nEncrypted project open: {project_open}\nInput directory available: {}\n\nTools\nDiscovery: built in\nWorkflow simulator: built in; no biological QC\nBiological QC providers: not implemented\nAlignment providers: none registered\n\nReferences\nFASTA: {}\nGTF/GFF: {}\nBED: {}\n\nLive offline qualification: requires platform evidence; this report does not perform network observation.\nScientific reference compatibility: not evaluated.\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.checks
                .map_or("not checked", |checks| if checks[0] { "yes" } else { "no" }),
            self.resource_status(self.reference.is_some(), 1),
            self.resource_status(self.annotation.is_some(), 2),
            self.resource_status(self.targets.is_some(), 3)
        )
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        project_open: bool,
        input: Option<&std::path::Path>,
    ) -> bool {
        if let Some(job) = &self.check_job
            && let Ok(checks) = job.try_recv()
        {
            self.checks = Some(checks);
            self.check_job = None;
        }
        if self.checks.is_none() && self.check_job.is_none() {
            self.refresh(input);
        }
        if self.check_job.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        ui.heading("System readiness");
        ui.label("Local resource registry. Resources are never downloaded or executed.");
        ui.label(format!(
            "Platform: {} / {} · SeqQC {}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            env!("CARGO_PKG_VERSION")
        ));
        ui.label(if project_open {
            "Encrypted project: open with exclusive writer lease"
        } else {
            "Encrypted project: open a project to configure resources"
        });
        ui.label(match (input, self.checks) {
            (None, _) => "Input folder: not selected",
            (Some(_), Some(checks)) if checks[0] => "Input folder: available",
            (Some(_), Some(_)) => "Input folder: unavailable — choose an accessible mounted folder",
            _ => "Input folder: checking local accessibility",
        });
        if ui.button("Refresh local accessibility checks").clicked() {
            self.refresh(input);
        }
        ui.separator();
        ui.heading("Tools");
        ui.label("SeqQC discovery · built in · FASTQ/BAM/CRAM filename mapping");
        ui.label("SeqQC workflow simulator · built in · no biological calculations");
        ui.label("Biological QC providers: not implemented in this preview");
        ui.label("Alignment providers: none registered; Linux eligibility alone does not enable alignment");
        ui.separator();
        let mut changed = false;
        let checks = self.checks;
        ui.add_enabled_ui(project_open, |ui| {
            ui.heading("Project references");
            for (index, (label, path)) in [
                ("Reference FASTA", &mut self.reference),
                ("Annotation GTF/GFF", &mut self.annotation),
                ("Target intervals BED", &mut self.targets),
            ]
            .into_iter()
            .enumerate()
            {
                ui.horizontal_wrapped(|ui| {
                    ui.label(label);
                    if ui.button(format!("Choose {label}…")).clicked()
                        && let Some(selected) = rfd::FileDialog::new().set_title(label).pick_file()
                    {
                        *path = Some(selected);
                        changed = true;
                    }
                    if path.is_some() && ui.small_button(format!("Clear {label}")).clicked() {
                        *path = None;
                        changed = true;
                    }
                    if let Some(selected) = path {
                        ui.label(selected.display().to_string());
                        ui.label(match checks {
                            Some(checks) if checks[index + 1] => {
                                "Readable; scientific compatibility not evaluated"
                            }
                            Some(_) => "Unavailable or unreadable",
                            None => "Checking local accessibility",
                        });
                    } else {
                        ui.label("Not configured (optional for foundation simulation)");
                    }
                });
            }
        });
        if changed {
            self.refresh(input);
        }
        self.simulation_ui(ui, project_open);
        changed
    }

    fn simulation_ui(&mut self, ui: &mut egui::Ui, project_open: bool) {
        ui.separator();
        ui.heading("Simulated workflow");
        ui.label("Exercises queue, run, completion, and cancellation. It does not read sequencing content or produce QC findings.");
        if ui
            .add_enabled(
                project_open && !self.simulation.active(),
                egui::Button::new("Queue simulated workflow"),
            )
            .clicked()
        {
            self.simulation = Simulation::Queued;
        }
        if ui
            .add_enabled(
                self.simulation.active(),
                egui::Button::new("Cancel simulation"),
            )
            .clicked()
        {
            self.simulation = Simulation::Cancelled;
        }
        ui.label(match self.simulation {
            Simulation::Idle => "Idle",
            Simulation::Queued => "Queued",
            Simulation::Running { step: 0, .. } => "Running: simulated input review",
            Simulation::Running { step: 1, .. } => "Running: simulated module execution",
            Simulation::Running { .. } => "Running: simulated result commit",
            Simulation::Completed => "Completed — simulation only; no QC results",
            Simulation::Cancelled => "Cancelled — no QC results",
            Simulation::Interrupted => "Interrupted by an earlier shutdown — queue again to safely restart the simulation; no results were committed",
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_export_omits_private_paths_and_distinguishes_missing_resources() {
        let foundation = Foundation {
            reference: Some(PathBuf::from("private-subject/reference.fa")),
            checks: Some([false; 4]),
            ..Foundation::default()
        };
        let report = foundation.report(true);
        assert!(report.contains("unavailable or unreadable"));
        assert!(report.contains("GTF/GFF: not configured"));
        assert!(!report.contains("private-subject"));
        assert!(!report.contains("reference.fa"));
        assert!(report.contains("does not perform network observation"));
    }

    #[test]
    fn simulator_completes_in_order_and_terminal_states_do_not_restart() {
        let mut state = Simulation::Queued;
        state.advance();
        assert!(matches!(state, Simulation::Running { step: 0, .. }));
        for step in 0..3 {
            state = Simulation::Running {
                step,
                started: Instant::now()
                    .checked_sub(Duration::from_secs(1))
                    .expect("monotonic clock supports test interval"),
            };
            state.advance();
            if step < 2 {
                assert!(
                    matches!(state, Simulation::Running { step: next, .. } if next == step + 1)
                );
            }
        }
        assert_eq!(state, Simulation::Completed);
        state.advance();
        assert_eq!(state, Simulation::Completed);
        state = Simulation::Cancelled;
        state.advance();
        assert_eq!(state, Simulation::Cancelled);
    }
}
