//! Synthetic fixtures compiled only into the opt-in qualification build.
//! No user profile, credentials, or real sequencing data are used.

use std::sync::Arc;
use std::{env, fs, io};

use super::{
    Assay, CreateProjectRequest, Duration, GroupStatus, Instant, NoticeKind,
    PassphraseProjectManager, PathBuf, ProjectContext, ProjectEditor, RecoveryRequest, ScanStatus,
    Screen, SeqQcApp, Theme, egui,
};

const MARKER: &str = "SeqQC synthetic qualification fixture v1\n";
const PASSPHRASE: &str = "synthetic qualification only passphrase";

pub struct Capture {
    started: Instant,
    requested: bool,
}

impl Default for Capture {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            requested: false,
        }
    }
}

impl Capture {
    pub fn tick(&mut self, context: &egui::Context) {
        let Some(path) = env::var_os("SEQQC_QA_SCREENSHOT") else {
            return;
        };
        let path = PathBuf::from(path);
        assert!(
            path.is_absolute() && path.extension().is_some_and(|extension| extension == "png"),
            "qualification screenshot must be an absolute PNG path"
        );
        let frames = context.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(Arc::clone(image))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        });
        for image in frames {
            let first = image.pixels.first().expect("frame has pixels");
            assert!(
                image.pixels.iter().filter(|pixel| *pixel != first).count() > 100,
                "refusing a blank native framebuffer"
            );
            let pixels: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(egui::Color32::to_array)
                .collect();
            image::save_buffer(
                &path,
                &pixels,
                u32::try_from(image.size[0]).expect("frame width"),
                u32::try_from(image.size[1]).expect("frame height"),
                image::ColorType::Rgba8,
            )
            .expect("save qualification frame");
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !self.requested && self.started.elapsed() >= Duration::from_millis(500) {
            context.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.requested = true;
        }
        context.request_repaint_after(Duration::from_millis(50));
    }
}

#[allow(clippy::too_many_lines)] // Single fixture setup; excluded from production builds.
pub fn configure(mut app: SeqQcApp, context: &egui::Context) -> io::Result<SeqQcApp> {
    let root =
        PathBuf::from(env::var_os("SEQQC_QA_ROOT").ok_or_else(|| {
            io::Error::other("SEQQC_QA_ROOT must name a new disposable directory")
        })?);
    if !root.is_absolute() {
        return Err(io::Error::other("qualification root must be absolute"));
    }
    if root.exists() {
        if fs::read_to_string(root.join("SYNTHETIC-QUALIFICATION.txt"))? != MARKER {
            return Err(io::Error::other("refusing a non-qualification directory"));
        }
    } else {
        fs::create_dir(&root)?;
        fs::write(root.join("SYNTHETIC-QUALIFICATION.txt"), MARKER)?;
    }
    app.manager = Some(PassphraseProjectManager::new(
        root.join("profile/profile.db"),
    ));
    "Synthetic reviewer".clone_into(&mut app.principal_name_input);
    app.theme = match env::var("SEQQC_QA_THEME").as_deref() {
        Ok("light") => Theme::ClinicalLight,
        Ok("contrast") => Theme::HighContrast,
        _ => Theme::ClinicalNight,
    };
    app.theme.apply(context);
    let zoom = env::var("SEQQC_QA_ZOOM")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(1.0);
    if !(1.0..=2.0).contains(&zoom) {
        return Err(io::Error::other(
            "qualification zoom must be between 1 and 2",
        ));
    }
    context.set_zoom_factor(zoom);
    let scene = env::var("SEQQC_QA_SCENE").unwrap_or_else(|_| "inputs".to_owned());
    if scene == "locked" {
        return Ok(app);
    }
    let manager = app.manager.as_ref().expect("synthetic manager set");
    let mut profile = if manager.profile_exists() {
        manager.open_profile(PASSPHRASE)
    } else {
        manager.create_profile("Synthetic reviewer", PASSPHRASE)
    }
    .map_err(io::Error::other)?;
    let inputs = root.join("inputs");
    fs::create_dir_all(&inputs)?;
    for name in [
        "paired_S1_L001_R1_001.fastq",
        "paired_S1_L001_R2_001.fastq",
        "orphan_R1.fastq",
        "duplicate_R1.fastq",
        "duplicate_R1.fq",
        "duplicate_R2.fastq",
        "unknown.fastq",
    ] {
        fs::write(inputs.join(name), "@synthetic\nACGT\n+\nIIII\n")?;
    }
    let project = if let Some(recent) = profile.recent_projects().first() {
        let directory = recent.directory.clone();
        manager.open_project(&mut profile, directory)
    } else {
        manager.create_project(
            &mut profile,
            &CreateProjectRequest {
                parent_directory: root.clone(),
                display_name: "Synthetic qualification cohort".to_owned(),
                assay: Assay::Wgs,
                context: ProjectContext::HumanResearch,
                input_root: Some(inputs.clone()),
            },
        )
    }
    .map_err(io::Error::other)?;
    app.profile = Some(profile);
    app.project_editor = Some(ProjectEditor::from_workspace(&project));
    app.active_project = Some(project);
    app.selected_root = Some(inputs.clone());
    app.discovery = Some(seqqc_discovery::discover(&inputs)?);
    app.scan_status = ScanStatus::Ready;
    app.screen = match scene.as_str() {
        "overview" => Screen::Overview,
        "projects" => {
            app.new_project.visible = true;
            Screen::Projects
        }
        "readiness" => Screen::Readiness,
        "recovery" => {
            app.recovery = Some(RecoveryRequest {
                directory: root.join("synthetic-interrupted-project"),
                process_id: 0,
                confirmed: false,
                profile: false,
            });
            Screen::Projects
        }
        "inputs" | "large" => Screen::InputDiscovery,
        _ => return Err(io::Error::other("unknown qualification scene")),
    };
    if scene == "large" {
        let discovery = app.discovery.as_mut().expect("fixture discovered");
        let template = discovery
            .groups
            .iter()
            .find(|group| group.status == GroupStatus::PairedCandidate)
            .expect("paired fixture")
            .clone();
        discovery.groups = (0..10_000)
            .map(|index| {
                let mut group = template.clone();
                group.proposed_sample = Some(format!("synthetic-sample-{index:05}"));
                group
            })
            .collect();
        discovery.summary.supported_files = 20_000;
        discovery.summary.fastq_files = 20_000;
        discovery.summary.ignored_files = 0;
        app.sample_page = 39;
    }
    app.set_notice(
        NoticeKind::Warning,
        "Synthetic qualification fixture — no real samples or biological QC findings",
    );
    context.request_repaint();
    Ok(app)
}
