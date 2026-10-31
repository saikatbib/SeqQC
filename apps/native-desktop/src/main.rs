#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(linker_messages)]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, Stroke};
use seqqc_discovery::{DiscoveryResult, GroupStatus, InputGroup, InputKind, discover_with_cancel};
use seqqc_engine::{Capability, Platform, check_task_platform};
use seqqc_projects::{
    Assay, CreateProjectRequest, LocalProfile, PassphraseProjectManager, ProjectContext,
    ProjectWorkspace, RecentProject, detected_local_principal_name,
};
use zeroize::Zeroize;

mod foundation;
#[cfg(feature = "qualification")]
mod qualification;
use foundation::{Foundation, Simulation};

#[cfg(not(feature = "qualification"))]
const APP_TITLE: &str = "SeqQC Studio";
#[cfg(feature = "qualification")]
const APP_TITLE: &str = "SeqQC Studio [QA]";
const MAX_VISIBLE_GROUPS: usize = 250;
const SESSION_TIMEOUT: Duration = Duration::from_mins(15);

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("org.seqqc.studio")
            .with_title(APP_TITLE)
            .with_inner_size([1_280.0, 820.0])
            .with_min_inner_size([920.0, 640.0]),
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|creation_context| Ok(Box::new(SeqQcApp::new(creation_context)))),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Theme {
    ClinicalNight,
    ClinicalLight,
    HighContrast,
}

impl Theme {
    const ALL: [Self; 3] = [Self::ClinicalNight, Self::ClinicalLight, Self::HighContrast];

    const fn label(self) -> &'static str {
        match self {
            Self::ClinicalNight => "Night",
            Self::ClinicalLight => "Light",
            Self::HighContrast => "High contrast",
        }
    }

    fn apply(self, context: &egui::Context) {
        let mut visuals = match self {
            Self::ClinicalLight => egui::Visuals::light(),
            Self::ClinicalNight | Self::HighContrast => egui::Visuals::dark(),
        };

        match self {
            Self::ClinicalNight => {
                visuals.panel_fill = Color32::from_rgb(9, 19, 29);
                visuals.extreme_bg_color = Color32::from_rgb(5, 13, 21);
                visuals.faint_bg_color = Color32::from_rgb(16, 32, 45);
                visuals.override_text_color = Some(Color32::from_rgb(225, 237, 244));
                visuals.selection.bg_fill = Color32::from_rgb(0, 77, 91);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(18, 37, 50);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(26, 58, 72);
                visuals.widgets.active.bg_fill = Color32::from_rgb(18, 85, 101);
            }
            Self::ClinicalLight => {
                visuals.panel_fill = Color32::from_rgb(242, 247, 249);
                visuals.extreme_bg_color = Color32::WHITE;
                visuals.faint_bg_color = Color32::from_rgb(225, 236, 239);
                visuals.override_text_color = Some(Color32::from_rgb(20, 39, 50));
                visuals.selection.bg_fill = Color32::from_rgb(190, 226, 232);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(229, 239, 241);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(205, 229, 232);
                visuals.widgets.active.bg_fill = Color32::from_rgb(133, 200, 207);
            }
            Self::HighContrast => {
                visuals.panel_fill = Color32::BLACK;
                visuals.extreme_bg_color = Color32::BLACK;
                visuals.faint_bg_color = Color32::from_rgb(20, 20, 20);
                visuals.override_text_color = Some(Color32::WHITE);
                visuals.selection.bg_fill = Color32::from_rgb(0, 92, 107);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 35, 35);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(0, 90, 105);
                visuals.widgets.active.bg_fill = Color32::from_rgb(0, 85, 100);
            }
        }

        visuals.warn_fg_color = notice_color(NoticeKind::Warning, visuals.dark_mode);
        visuals.error_fg_color = notice_color(NoticeKind::Error, visuals.dark_mode);
        visuals.selection.stroke =
            Stroke::new(2.0, visuals.override_text_color.unwrap_or(Color32::WHITE));
        visuals.widgets.active.bg_stroke = Stroke::new(2.0, accent_color(visuals.dark_mode));
        context.set_visuals(visuals);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Screen {
    #[default]
    Overview,
    Projects,
    InputDiscovery,
    Readiness,
}

struct ScanJob {
    cancelled: Arc<AtomicBool>,
    root: PathBuf,
    receiver: Receiver<Result<DiscoveryResult, String>>,
}

impl Drop for ScanJob {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
enum ScanStatus {
    #[default]
    Idle,
    Running,
    Ready,
    Failed(String),
}

#[derive(Default)]
struct NewProjectForm {
    visible: bool,
    display_name: String,
    assay: Assay,
    context: ProjectContext,
    parent_directory: Option<PathBuf>,
    input_root: Option<PathBuf>,
}

struct ProjectEditor {
    display_name: String,
    assay: Assay,
    context: ProjectContext,
    input_root: Option<PathBuf>,
}

impl ProjectEditor {
    fn from_workspace(workspace: &ProjectWorkspace) -> Self {
        let summary = workspace.summary();
        Self {
            display_name: summary.display_name.clone(),
            assay: summary.assay,
            context: summary.context,
            input_root: summary.input_root.clone(),
        }
    }
}

#[derive(Clone, Copy)]
enum NoticeKind {
    Success,
    Warning,
    Error,
}

struct Notice {
    kind: NoticeKind,
    message: String,
}

struct RecoveryRequest {
    directory: PathBuf,
    process_id: u32,
    confirmed: bool,
    profile: bool,
}

struct SeqQcApp {
    #[cfg(feature = "qualification")]
    qa_capture: qualification::Capture,
    recovery: Option<RecoveryRequest>,
    foundation: Foundation,
    theme: Theme,
    screen: Screen,
    manager: Option<PassphraseProjectManager>,
    startup_error: Option<String>,
    profile: Option<LocalProfile>,
    active_project: Option<ProjectWorkspace>,
    project_editor: Option<ProjectEditor>,
    new_project: NewProjectForm,
    principal_name_input: String,
    local_passphrase: String,
    confirm_passphrase: String,
    last_activity: Instant,
    notice: Option<Notice>,
    scan_status: ScanStatus,
    scan_job: Option<ScanJob>,
    discovery: Option<DiscoveryResult>,
    selected_root: Option<PathBuf>,
    sample_filter: String,
    sample_page: usize,
}

impl SeqQcApp {
    fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let app = Self::with_context(&creation_context.egui_ctx);
        #[cfg(feature = "qualification")]
        let app = qualification::configure(app, &creation_context.egui_ctx)
            .expect("synthetic qualification setup failed");
        app
    }

    fn with_context(context: &egui::Context) -> Self {
        let theme = Theme::ClinicalNight;
        theme.apply(context);
        let (manager, startup_error) = match PassphraseProjectManager::for_current_user() {
            Ok(manager) => (Some(manager), None),
            Err(error) => (None, Some(error.to_string())),
        };

        Self {
            #[cfg(feature = "qualification")]
            qa_capture: qualification::Capture::default(),
            recovery: None,
            foundation: Foundation::default(),
            theme,
            screen: Screen::Overview,
            manager,
            startup_error,
            profile: None,
            active_project: None,
            project_editor: None,
            new_project: NewProjectForm::default(),
            principal_name_input: detected_local_principal_name(),
            local_passphrase: String::new(),
            confirm_passphrase: String::new(),
            last_activity: Instant::now(),
            notice: None,
            scan_status: ScanStatus::Idle,
            scan_job: None,
            discovery: None,
            selected_root: None,
            sample_filter: String::new(),
            sample_page: 0,
        }
    }

    fn set_notice(&mut self, kind: NoticeKind, message: impl Into<String>) {
        self.notice = Some(Notice {
            kind,
            message: message.into(),
        });
    }

    fn start_local_session(&mut self) {
        let Some(manager) = &self.manager else {
            return;
        };
        let creating = !manager.profile_exists();
        if creating && self.local_passphrase != self.confirm_passphrase {
            self.local_passphrase.zeroize();
            self.confirm_passphrase.zeroize();
            self.set_notice(NoticeKind::Error, "The local passphrases do not match.");
            return;
        }
        let result = if creating {
            manager.create_profile(&self.principal_name_input, &self.local_passphrase)
        } else {
            manager.open_profile(&self.local_passphrase)
        };
        self.local_passphrase.zeroize();
        self.confirm_passphrase.zeroize();
        match result {
            Ok(profile) => {
                let display_name = profile.principal().display_name.clone();
                self.profile = Some(profile);
                self.last_activity = Instant::now();
                self.screen = Screen::Overview;
                self.set_notice(
                    NoticeKind::Success,
                    format!("Secure local session started for {display_name}."),
                );
            }
            Err(error) => {
                if let seqqc_projects::ProjectError::Lock(_) = &error
                    && let Some(directory) = self
                        .manager
                        .as_ref()
                        .and_then(|manager| manager.profile_path().parent())
                        .map(Path::to_path_buf)
                {
                    self.offer_recovery(&error, directory, true);
                }
                self.set_notice(NoticeKind::Error, error.to_string());
            }
        }
    }

    fn lock_session(&mut self, inactivity: bool) {
        if let Some(project) = self.active_project.as_mut() {
            let _ = project.record_event("project.locked", "success");
        }
        if let Some(profile) = self.profile.as_mut() {
            let _ = profile.record_event("local_session.locked", "success");
        }
        self.clear_discovery();
        self.active_project = None;
        self.project_editor = None;
        self.profile = None;
        self.foundation = Foundation::default();
        self.recovery = None;
        self.discovery = None;
        self.selected_root = None;
        self.scan_job = None;
        self.scan_status = ScanStatus::Idle;
        self.screen = Screen::Overview;
        self.new_project = NewProjectForm::default();
        self.sample_filter.clear();
        self.sample_page = 0;
        self.set_notice(
            NoticeKind::Warning,
            if inactivity {
                "The local session locked after 15 minutes of inactivity."
            } else {
                "The local session is locked. Encrypted stores and writer leases are closed."
            },
        );
    }

    fn create_project(&mut self) {
        let request = CreateProjectRequest {
            parent_directory: self
                .new_project
                .parent_directory
                .clone()
                .unwrap_or_default(),
            display_name: self.new_project.display_name.clone(),
            assay: self.new_project.assay,
            context: self.new_project.context,
            input_root: self.new_project.input_root.clone(),
        };
        let result = match (&self.manager, self.profile.as_mut()) {
            (Some(manager), Some(profile)) => manager.create_project(profile, &request),
            _ => return,
        };
        match result {
            Ok(workspace) => {
                self.clear_discovery();
                let display_name = workspace.summary().display_name.clone();
                self.project_editor = Some(ProjectEditor::from_workspace(&workspace));
                self.selected_root
                    .clone_from(&workspace.summary().input_root);
                self.active_project = Some(workspace);
                self.load_resources();
                self.new_project = NewProjectForm::default();
                self.set_notice(
                    NoticeKind::Success,
                    format!("Encrypted project “{display_name}” was created and opened."),
                );
            }
            Err(error) => self.set_notice(NoticeKind::Error, error.to_string()),
        }
    }

    fn open_project(&mut self, directory: &Path) {
        if self
            .active_project
            .as_ref()
            .is_some_and(|project| project.summary().directory == directory)
        {
            self.screen = Screen::Overview;
            return;
        }
        let result = match (&self.manager, self.profile.as_mut()) {
            (Some(manager), Some(profile)) => manager.open_project(profile, directory),
            _ => return,
        };
        match result {
            Ok(workspace) => {
                self.clear_discovery();
                let display_name = workspace.summary().display_name.clone();
                self.project_editor = Some(ProjectEditor::from_workspace(&workspace));
                self.selected_root
                    .clone_from(&workspace.summary().input_root);
                self.active_project = Some(workspace);
                self.load_resources();
                self.screen = Screen::Overview;
                self.set_notice(
                    NoticeKind::Success,
                    format!("Encrypted project “{display_name}” is open with a writer lock."),
                );
            }
            Err(error) => {
                self.offer_recovery(&error, directory.to_path_buf(), false);
                self.set_notice(NoticeKind::Error, error.to_string());
            }
        }
    }

    fn offer_recovery(
        &mut self,
        error: &seqqc_projects::ProjectError,
        directory: PathBuf,
        profile: bool,
    ) {
        // Match the typed lock variant without exposing the security crate to the shell.
        self.recovery = None;
        if let seqqc_projects::ProjectError::Lock(lock) = error
            && let Some(process_id) = seqqc_projects::locked_process_id(lock)
        {
            self.recovery = Some(RecoveryRequest {
                directory,
                process_id,
                confirmed: false,
                profile,
            });
        }
    }

    fn recovery_ui(&mut self, ui: &mut egui::Ui) {
        let Some(request) = self.recovery.as_mut() else {
            return;
        };
        let mut recover = false;
        let mut cancel = false;
        ui.group(|ui| {
            ui.heading("Recover interrupted session");
            ui.label(format!("A writer lease from process {} blocks {}.", request.process_id, request.directory.display()));
            ui.label("Close SeqQC on every computer using this folder before recovery. An old lease alone does not prove its writer has stopped. Recovery preserves the lease as evidence and requires it to be at least 15 minutes old.");
            ui.checkbox(&mut request.confirmed, "I have verified that no SeqQC instance is using this profile or project");
            recover = ui.add_enabled(request.confirmed, egui::Button::new("Preserve stale lease and retry")).clicked();
            cancel = ui.button("Cancel recovery").clicked();
        });
        if cancel {
            self.recovery = None;
            return;
        }
        if recover {
            let request = self.recovery.take().expect("recovery request exists");
            let result = self
                .manager
                .as_ref()
                .expect("manager exists for recovery")
                .recover_stale_project_lock(&request.directory, SESSION_TIMEOUT);
            match result {
                Ok(_) if request.profile => self.set_notice(NoticeKind::Success, "Profile lease preserved. Enter the passphrase again to unlock and verify integrity."),
                Ok(_) => self.open_project(&request.directory),
                Err(error) => self.set_notice(NoticeKind::Error, error.to_string()),
            }
        }
    }

    fn choose_project_folder(&mut self) {
        if let Some(directory) = rfd::FileDialog::new()
            .set_title("Open SeqQC project folder")
            .pick_folder()
        {
            self.open_project(&directory);
        }
    }

    fn save_project_metadata(&mut self) {
        let Some(editor) = &self.project_editor else {
            return;
        };
        let display_name = editor.display_name.clone();
        let assay = editor.assay;
        let context = editor.context;
        let input_root = editor.input_root.clone();
        let update = self.active_project.as_mut().map_or_else(
            || Err("No active project is open.".to_owned()),
            |project| {
                project
                    .update_metadata(&display_name, assay, context, input_root.as_deref())
                    .map_err(|error| error.to_string())
            },
        );
        if let Err(error) = update {
            self.set_notice(NoticeKind::Error, error);
            return;
        }
        let refresh = match (
            &self.manager,
            self.profile.as_mut(),
            self.active_project.as_ref(),
        ) {
            (Some(manager), Some(profile), Some(project)) => manager
                .refresh_recent(profile, project)
                .map_err(|error| error.to_string()),
            _ => Ok(()),
        };
        match refresh {
            Ok(()) => {
                if self.selected_root != input_root {
                    self.clear_discovery();
                    self.load_resources();
                }
                self.selected_root = input_root;
                self.set_notice(NoticeKind::Success, "Encrypted project metadata was saved.");
            }
            Err(error) => self.set_notice(NoticeKind::Error, error),
        }
    }

    fn choose_folder(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Choose sequencing input folder");
        if let Some(root) = &self.selected_root {
            dialog = dialog.set_directory(root);
        }
        if let Some(root) = dialog.pick_folder()
            && self.persist_input_root(&root)
        {
            self.start_scan(root);
        }
    }

    fn persist_input_root(&mut self, root: &Path) -> bool {
        let Some(project) = self.active_project.as_mut() else {
            return false;
        };
        let summary = project.summary().clone();
        if let Err(error) = project.update_metadata(
            &summary.display_name,
            summary.assay,
            summary.context,
            Some(root),
        ) {
            self.set_notice(NoticeKind::Error, error.to_string());
            return false;
        }
        if let Some(editor) = self.project_editor.as_mut() {
            editor.input_root = Some(root.to_path_buf());
        }
        if let (Some(manager), Some(profile), Some(project)) = (
            &self.manager,
            self.profile.as_mut(),
            self.active_project.as_ref(),
        ) && let Err(error) = manager.refresh_recent(profile, project)
        {
            self.set_notice(NoticeKind::Error, error.to_string());
        }
        true
    }

    fn clear_discovery(&mut self) {
        if self.foundation.simulation.active()
            && let Some(project) = self.active_project.as_mut()
        {
            let _ = project.set_simulation_state("cancelled");
        }
        self.foundation = Foundation::default();
        self.recovery = None;
        self.scan_job = None;
        self.discovery = None;
        self.scan_status = ScanStatus::Idle;
        self.sample_filter.clear();
        self.sample_page = 0;
    }

    fn load_resources(&mut self) {
        if let Some(project) = &self.active_project {
            let result = (|| -> Result<Foundation, seqqc_projects::ProjectError> {
                let simulation = match project.simulation_state()?.as_str() {
                    "queued" | "running" => Simulation::Interrupted,
                    "completed" => Simulation::Completed,
                    "cancelled" => Simulation::Cancelled,
                    _ => Simulation::Idle,
                };
                Ok(Foundation::with_resources(
                    project.resource_path("reference_fasta")?,
                    project.resource_path("annotation_gtf_gff")?,
                    project.resource_path("targets_bed")?,
                    simulation,
                ))
            })();
            match result {
                Ok(foundation) => self.foundation = foundation,
                Err(error) => self.set_notice(NoticeKind::Error, error.to_string()),
            }
        }
    }

    fn start_scan(&mut self, root: PathBuf) {
        self.foundation.invalidate_checks();
        let (sender, receiver) = sync_channel(1);
        let scan_root = root.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        std::thread::spawn(move || {
            let outcome =
                discover_with_cancel(&scan_root, || worker_cancelled.load(Ordering::Relaxed))
                    .map_err(|error| error.to_string());
            let _result = sender.send(outcome);
        });

        self.selected_root = Some(root.clone());
        self.discovery = None;
        self.scan_status = ScanStatus::Running;
        self.scan_job = Some(ScanJob {
            cancelled,
            root,
            receiver,
        });
        self.sample_filter.clear();
        self.sample_page = 0;
    }

    fn poll_scan(&mut self, context: &egui::Context) {
        let Some(job) = &self.scan_job else {
            return;
        };

        let message = match job.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => {
                context.request_repaint_after(Duration::from_millis(75));
                None
            }
            Err(TryRecvError::Disconnected) => Some(Err(format!(
                "The input scan stopped unexpectedly for {}.",
                job.root.display()
            ))),
        };

        if let Some(result) = message {
            self.scan_job = None;
            match result {
                Ok(discovery) => {
                    self.discovery = Some(discovery);
                    self.scan_status = ScanStatus::Ready;
                    if let Some(project) = self.active_project.as_mut() {
                        let _ = project.record_event("inputs.discovered", "success");
                    }
                }
                Err(message) => {
                    self.discovery = None;
                    self.scan_status = ScanStatus::Failed(message);
                    if let Some(project) = self.active_project.as_mut() {
                        let _ = project.record_event("inputs.discovered", "failed");
                    }
                }
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        let mut lock_requested = false;
        egui::Panel::top("application_header")
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .inner_margin(egui::Margin::symmetric(22, 14))
                    .stroke(Stroke::new(
                        1.0,
                        ui.visuals().widgets.inactive.bg_stroke.color,
                    )),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(APP_TITLE)
                                .size(24.0)
                                .strong()
                                .color(accent_color(ui.visuals().dark_mode)),
                        );
                        ui.label(RichText::new("LOCAL SEQUENCING QUALITY CONTROL").size(10.0));
                    });

                    ui.label(
                        RichText::new("OFFLINE · NATIVE")
                            .size(11.0)
                            .strong()
                            .color(notice_color(NoticeKind::Success, ui.visuals().dark_mode)),
                    );
                    if self.profile.is_some() && ui.button("Lock session").clicked() {
                        lock_requested = true;
                    }
                    if let Some(profile) = &self.profile {
                        ui.label(format!("Local: {}", profile.principal().display_name));
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Theme");
                    for theme in Theme::ALL {
                        if ui
                            .selectable_value(&mut self.theme, theme, theme.label())
                            .changed()
                        {
                            self.theme.apply(ui.ctx());
                        }
                    }
                    ui.separator();
                    ui.label("Text size");
                    for percent in [100_u16, 125, 150, 200] {
                        let zoom = f32::from(percent) / 100.0;
                        if ui
                            .selectable_label(
                                (ui.ctx().zoom_factor() - zoom).abs() < 0.01,
                                format!("{percent}%"),
                            )
                            .clicked()
                        {
                            ui.ctx().set_zoom_factor(zoom);
                        }
                    }
                });
            });
        if lock_requested {
            self.lock_session(false);
        }
    }

    fn navigation(&mut self, ui: &mut egui::Ui) {
        if ui.available_width() < 900.0 {
            egui::Panel::top("compact_navigation").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (screen, label) in [
                        (Screen::Overview, "Overview"),
                        (Screen::Projects, "Projects"),
                        (Screen::InputDiscovery, "Input discovery"),
                        (Screen::Readiness, "System readiness"),
                    ] {
                        ui.selectable_value(&mut self.screen, screen, label);
                    }
                });
            });
            return;
        }
        egui::Panel::left("primary_navigation")
            .exact_size(210.0)
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ui, |ui| {
                ui.label(RichText::new("WORKSPACE").size(10.0).strong());
                ui.add_space(10.0);
                if ui
                    .add(egui::Button::new("Overview").selected(self.screen == Screen::Overview))
                    .clicked()
                {
                    self.screen = Screen::Overview;
                }
                if ui
                    .add(egui::Button::new("Projects").selected(self.screen == Screen::Projects))
                    .clicked()
                {
                    self.screen = Screen::Projects;
                }
                if ui
                    .add(
                        egui::Button::new("Input discovery")
                            .selected(self.screen == Screen::InputDiscovery),
                    )
                    .clicked()
                {
                    self.screen = Screen::InputDiscovery;
                }
                ui.add_enabled(false, egui::Button::new("QC findings"));
                if ui
                    .selectable_label(self.screen == Screen::Readiness, "System readiness")
                    .clicked()
                {
                    self.screen = Screen::Readiness;
                }
                ui.add_enabled(false, egui::Button::new("Reports"));
                ui.add_space(28.0);
                ui.label(RichText::new("SECURE FOUNDATION").size(10.0).strong());
                ui.label("SQLCipher metadata");
                ui.label("Argon2id local unlock");
                ui.label("Single writer");
                ui.label("15-minute auto-lock");
                ui.add_space(18.0);

                let decision = check_task_platform(Capability::Alignment, current_platform());
                if decision.allowed {
                    ui.colored_label(
                        notice_color(NoticeKind::Success, ui.visuals().dark_mode),
                        "Alignment platform: eligible",
                    );
                } else {
                    ui.colored_label(
                        notice_color(NoticeKind::Warning, ui.visuals().dark_mode),
                        "Alignment: Linux only",
                    );
                }
            });
    }

    fn content(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.notice_card(ui);
                self.recovery_ui(ui);
                match self.screen {
                    Screen::Overview => self.overview_content(ui),
                    Screen::Projects => self.projects_content(ui),
                    Screen::InputDiscovery => self.discovery_content(ui),
                    Screen::Readiness => self.readiness_content(ui),
                }
            });
        });
    }

    fn locked_content(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading("Secure local session");
                ui.label(
                    "SeqQC uses the signed-in operating-system account plus a local application passphrase. No online account is created and no internet connection is used.",
                );
                ui.add_space(18.0);
                self.notice_card(ui);
                self.recovery_ui(ui);
                if let Some(error) = &self.startup_error {
                    status_frame(ui, "Application-data setup needs attention", error, NoticeKind::Error);
                    return;
                }
                let profile_exists = self
                    .manager
                    .as_ref()
                    .is_some_and(PassphraseProjectManager::profile_exists);
                if profile_exists {
                    ui.label(format!("OS account: {}", detected_local_principal_name()));
                    ui.label("Your encrypted profile and recent-project catalog are locked.");
                    let passphrase_label = ui.label("Local passphrase");
                    ui.add_sized(
                        [340.0, 34.0],
                        egui::TextEdit::singleline(&mut self.local_passphrase)
                            .password(true)
                            .hint_text("Local passphrase"),
                    ).labelled_by(passphrase_label.id);
                    ui.add_space(12.0);
                    if ui
                        .add_sized(
                            [240.0, 42.0],
                            egui::Button::new(RichText::new("Unlock local session").strong()),
                        )
                        .clicked()
                    {
                        self.start_local_session();
                    }
                } else {
                    ui.label("Create the encrypted profile for this OS account.");
                    let name_label = ui.label("Local display name");
                    ui.add_sized(
                        [340.0, 34.0],
                        egui::TextEdit::singleline(&mut self.principal_name_input)
                            .hint_text("Local display name"),
                    ).labelled_by(name_label.id);
                    let passphrase_label = ui.label("Local passphrase");
                    ui.add_sized(
                        [340.0, 34.0],
                        egui::TextEdit::singleline(&mut self.local_passphrase)
                            .password(true)
                            .hint_text("Local passphrase (12+ characters)"),
                    ).labelled_by(passphrase_label.id);
                    let confirm_label = ui.label("Confirm local passphrase");
                    ui.add_sized(
                        [340.0, 34.0],
                        egui::TextEdit::singleline(&mut self.confirm_passphrase)
                            .password(true)
                            .hint_text("Confirm local passphrase"),
                    ).labelled_by(confirm_label.id);
                    ui.label("The display name and recent paths are encrypted. The passphrase cannot be recovered online.");
                    ui.add_space(12.0);
                    if ui
                        .add_sized(
                            [240.0, 42.0],
                            egui::Button::new(RichText::new("Create secure profile").strong()),
                        )
                        .clicked()
                    {
                        self.start_local_session();
                    }
                }
                ui.add_space(22.0);
                ui.small("If this passphrase is lost, this preview cannot recover encrypted project keys. Portable encrypted archives are a later gated milestone.");
            });
            });
        });
    }

    fn notice_card(&mut self, ui: &mut egui::Ui) {
        let Some(notice) = &self.notice else {
            return;
        };
        let color = notice_color(notice.kind, ui.visuals().dark_mode);
        let message = notice.message.clone();
        let mut dismiss = false;
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .corner_radius(8)
            .stroke(Stroke::new(1.0, color))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(color, message);
                    if ui.small_button("Dismiss").clicked() {
                        dismiss = true;
                    }
                });
            });
        if dismiss {
            self.notice = None;
        }
        ui.add_space(14.0);
    }

    fn overview_content(&mut self, ui: &mut egui::Ui) {
        ui.heading("Project readiness");
        ui.label("A guided local workspace for encrypted sequencing quality-control projects.");
        ui.add_space(14.0);
        let recent_count = self
            .profile
            .as_ref()
            .map_or(0, |profile| profile.recent_projects().len());
        ui.columns(3, |columns| {
            text_metric_card(
                &mut columns[0],
                "ACTIVE PROJECT",
                self.active_project
                    .as_ref()
                    .map_or("None", |project| project.summary().display_name.as_str()),
                "One writer at a time",
            );
            metric_card(
                &mut columns[1],
                "RECENT PROJECTS",
                recent_count,
                "Encrypted local catalog",
            );
            text_metric_card(
                &mut columns[2],
                "DATA BOUNDARY",
                "Offline",
                "No runtime network client",
            );
        });
        ui.add_space(18.0);

        if let Some(project) = &self.active_project {
            let summary = project.summary();
            ui.heading(&summary.display_name);
            egui::Grid::new("active_project_summary")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label(RichText::new("Assay").strong());
                    ui.label(summary.assay.label());
                    ui.end_row();
                    ui.label(RichText::new("Context").strong());
                    ui.label(summary.context.label());
                    ui.end_row();
                    ui.label(RichText::new("Project folder").strong());
                    ui.label(RichText::new(summary.directory.display().to_string()).monospace());
                    ui.end_row();
                    ui.label(RichText::new("Input folder").strong());
                    ui.label(summary.input_root.as_ref().map_or_else(
                        || "Not selected".to_owned(),
                        |path| path.display().to_string(),
                    ));
                    ui.end_row();
                    ui.label(RichText::new("Encryption").strong());
                    ui.colored_label(
                        notice_color(NoticeKind::Success, ui.visuals().dark_mode),
                        "SQLCipher active",
                    );
                    ui.end_row();
                });
            ui.add_space(14.0);
            if ui.button("Discover inputs for this project").clicked() {
                self.screen = Screen::InputDiscovery;
            }
        } else {
            status_frame(
                ui,
                "No project is open",
                "Create a new encrypted project or open one from the encrypted recent-project catalog.",
                NoticeKind::Warning,
            );
            if ui.button("Go to Projects").clicked() {
                self.screen = Screen::Projects;
            }
        }
    }

    fn readiness_content(&mut self, ui: &mut egui::Ui) {
        let input = self
            .active_project
            .as_ref()
            .and_then(|project| project.summary().input_root.as_deref());
        let before = self.foundation.simulation;
        let changed = self.foundation.ui(ui, self.active_project.is_some(), input);
        if ui.button("Save readiness report…").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .set_file_name("seqqc-readiness.txt")
                .add_filter("Text", &["txt"])
                .save_file()
        {
            match std::fs::write(path, self.foundation.report(self.active_project.is_some())) {
                Ok(()) => self.set_notice(
                    NoticeKind::Success,
                    "Readiness report saved without project names or resource paths.",
                ),
                Err(error) => self.set_notice(
                    NoticeKind::Error,
                    format!("[SEQ-READINESS-001] Cannot save readiness report: {error}"),
                ),
            }
        }
        if let Some(project) = self.active_project.as_mut() {
            if changed {
                let resources = [
                    ("reference_fasta", self.foundation.reference.as_deref()),
                    ("annotation_gtf_gff", self.foundation.annotation.as_deref()),
                    ("targets_bed", self.foundation.targets.as_deref()),
                ];
                if let Err(error) = project.set_resources(&resources) {
                    self.load_resources();
                    self.set_notice(NoticeKind::Error, error.to_string());
                }
            } else if self.foundation.simulation != before
                && let Err(error) =
                    project.set_simulation_state(self.foundation.simulation.stored())
            {
                self.foundation.simulation = before;
                self.set_notice(NoticeKind::Error, error.to_string());
            }
        }
    }

    fn projects_content(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Projects");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Open project folder…").clicked() {
                    self.choose_project_folder();
                }
                if ui.button("New encrypted project").clicked() {
                    self.new_project.visible = true;
                }
            });
        });
        ui.label("Project names, contexts, paths, and recents are encrypted at rest.");
        ui.add_space(14.0);

        if self.new_project.visible {
            self.new_project_form(ui);
            ui.add_space(18.0);
        }
        if self.active_project.is_some() {
            self.project_editor(ui);
            ui.add_space(18.0);
        }

        ui.heading("Recent projects");
        let recents = self
            .profile
            .as_ref()
            .map_or_else(Vec::new, |profile| profile.recent_projects().to_vec());
        if recents.is_empty() {
            ui.label("No recent projects are stored in this encrypted profile yet.");
            return;
        }
        let mut open_requested = None;
        let mut forget_requested = None;
        for recent in &recents {
            recent_project_card(ui, recent, &mut open_requested, &mut forget_requested);
            ui.add_space(8.0);
        }
        if let Some(directory) = open_requested {
            self.open_project(&directory);
        }
        if let Some(project_id) = forget_requested
            && let Some(profile) = self.profile.as_mut()
        {
            match profile.forget_recent(&project_id) {
                Ok(()) => self.set_notice(
                    NoticeKind::Success,
                    "The recent-project entry was removed. Project files were not deleted.",
                ),
                Err(error) => self.set_notice(NoticeKind::Error, error.to_string()),
            }
        }
    }

    fn new_project_form(&mut self, ui: &mut egui::Ui) {
        let mut create_requested = false;
        let mut cancel_requested = false;
        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .corner_radius(10)
            .stroke(Stroke::new(1.0, accent_color(ui.visuals().dark_mode)))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.heading("Create encrypted project");
                ui.label("The on-disk folder name is opaque so the display name is not exposed.");
                ui.add_space(8.0);
                let field_width = (ui.available_width() - 160.0).clamp(150.0, 360.0);
                egui::Grid::new("new_project_grid")
                    .num_columns(2)
                    .spacing([14.0, 10.0])
                    .show(ui, |ui| {
                        let name_label = ui.label("Project name");
                        ui.add_sized(
                            [field_width, 30.0],
                            egui::TextEdit::singleline(&mut self.new_project.display_name)
                                .hint_text("Example: Pilot sequencing cohort"),
                        )
                        .labelled_by(name_label.id);
                        ui.end_row();
                        ui.label("Assay");
                        egui::ComboBox::from_id_salt("new_project_assay")
                            .selected_text(self.new_project.assay.label())
                            .show_ui(ui, |ui| {
                                for assay in Assay::ALL {
                                    ui.selectable_value(
                                        &mut self.new_project.assay,
                                        assay,
                                        assay.label(),
                                    );
                                }
                            });
                        ui.end_row();
                        ui.label("Context");
                        egui::ComboBox::from_id_salt("new_project_context")
                            .selected_text(self.new_project.context.label())
                            .show_ui(ui, |ui| {
                                for context in ProjectContext::ALL {
                                    ui.selectable_value(
                                        &mut self.new_project.context,
                                        context,
                                        context.label(),
                                    );
                                }
                            });
                        ui.end_row();
                        ui.label("Project location");
                        ui.horizontal(|ui| {
                            if ui.button("Choose parent folder…").clicked() {
                                self.new_project.parent_directory = rfd::FileDialog::new()
                                    .set_title("Choose where SeqQC will create the project")
                                    .pick_folder();
                            }
                            ui.label(path_or_not_selected(
                                self.new_project.parent_directory.as_deref(),
                            ));
                        });
                        ui.end_row();
                        ui.label("Input folder (optional)");
                        ui.horizontal(|ui| {
                            if ui.button("Choose input folder…").clicked() {
                                self.new_project.input_root = rfd::FileDialog::new()
                                    .set_title("Choose sequencing input folder")
                                    .pick_folder();
                            }
                            ui.label(path_or_not_selected(self.new_project.input_root.as_deref()));
                        });
                        ui.end_row();
                    });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let ready = !self.new_project.display_name.trim().is_empty()
                        && self.new_project.parent_directory.is_some();
                    if ui
                        .add_enabled(ready, egui::Button::new("Create and open project"))
                        .clicked()
                    {
                        create_requested = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel_requested = true;
                    }
                });
            });
        if create_requested {
            self.create_project();
        }
        if cancel_requested {
            self.new_project = NewProjectForm::default();
        }
    }

    fn project_editor(&mut self, ui: &mut egui::Ui) {
        let Some(editor) = self.project_editor.as_mut() else {
            return;
        };
        let mut save_requested = false;
        let mut close_requested = false;
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .corner_radius(10)
            .stroke(Stroke::new(
                1.0,
                ui.visuals().widgets.inactive.bg_stroke.color,
            ))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.heading("Open project metadata");
                let field_width = (ui.available_width() - 160.0).clamp(150.0, 360.0);
                egui::Grid::new("project_editor_grid")
                    .num_columns(2)
                    .spacing([14.0, 10.0])
                    .show(ui, |ui| {
                        let name_label = ui.label("Project name");
                        ui.add_sized(
                            [field_width, 30.0],
                            egui::TextEdit::singleline(&mut editor.display_name),
                        )
                        .labelled_by(name_label.id);
                        ui.end_row();
                        ui.label("Assay");
                        egui::ComboBox::from_id_salt("edit_project_assay")
                            .selected_text(editor.assay.label())
                            .show_ui(ui, |ui| {
                                for assay in Assay::ALL {
                                    ui.selectable_value(&mut editor.assay, assay, assay.label());
                                }
                            });
                        ui.end_row();
                        ui.label("Context");
                        egui::ComboBox::from_id_salt("edit_project_context")
                            .selected_text(editor.context.label())
                            .show_ui(ui, |ui| {
                                for context in ProjectContext::ALL {
                                    ui.selectable_value(
                                        &mut editor.context,
                                        context,
                                        context.label(),
                                    );
                                }
                            });
                        ui.end_row();
                        ui.label("Input folder");
                        ui.horizontal(|ui| {
                            if ui.button("Choose…").clicked() {
                                editor.input_root = rfd::FileDialog::new()
                                    .set_title("Choose sequencing input folder")
                                    .pick_folder();
                            }
                            if editor.input_root.is_some() && ui.small_button("Clear").clicked() {
                                editor.input_root = None;
                            }
                            ui.label(path_or_not_selected(editor.input_root.as_deref()));
                        });
                        ui.end_row();
                    });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Save encrypted metadata").clicked() {
                        save_requested = true;
                    }
                    if ui.button("Close project").clicked() {
                        close_requested = true;
                    }
                });
            });
        if save_requested {
            self.save_project_metadata();
        }
        if close_requested {
            if let Some(project) = self.active_project.as_mut() {
                let _ = project.record_event("project.closed", "success");
            }
            self.clear_discovery();
            self.active_project = None;
            self.project_editor = None;
            self.selected_root = None;
            self.discovery = None;
            self.scan_status = ScanStatus::Idle;
            self.set_notice(
                NoticeKind::Success,
                "Project closed and writer lock released.",
            );
        }
    }

    fn discovery_content(&mut self, ui: &mut egui::Ui) {
        ui.heading("Discover sequencing inputs");
        ui.label(
            "Choose a local or mounted folder. SeqQC scans nested folders without copying, uploading, or modifying sequencing files.",
        );
        ui.add_space(14.0);

        if self.active_project.is_none() {
            status_frame(
                ui,
                "Open a project first",
                "Input locations and discovery evidence are project metadata, so SeqQC requires an encrypted project before scanning.",
                NoticeKind::Warning,
            );
            if ui.button("Go to Projects").clicked() {
                self.screen = Screen::Projects;
            }
            return;
        }

        ui.horizontal_wrapped(|ui| {
            if ui
                .add_sized(
                    [190.0, 38.0],
                    egui::Button::new(RichText::new("Choose input folder…").strong()),
                )
                .clicked()
            {
                self.choose_folder();
            }

            if let Some(root) = &self.selected_root {
                ui.add(
                    egui::Label::new(RichText::new(root.display().to_string()).monospace()).wrap(),
                );
            } else {
                ui.label("No folder selected");
            }
        });

        ui.add_space(14.0);
        self.status_card(ui);

        if let Some(discovery) = &self.discovery {
            ui.add_space(18.0);
            summary_cards(ui, discovery);
            ui.add_space(18.0);
            sample_map(
                ui,
                discovery,
                &mut self.sample_filter,
                &mut self.sample_page,
            );
        }
    }

    fn status_card(&self, ui: &mut egui::Ui) {
        let (title, detail, color) = match &self.scan_status {
            ScanStatus::Idle => (
                "Ready for local input",
                "Supported: FASTQ, FASTQ.GZ, FQ, FQ.GZ, BAM, and CRAM.",
                accent_color(ui.visuals().dark_mode),
            ),
            ScanStatus::Running => (
                "Scanning folder…",
                "Discovery runs away from the interface thread so the native window stays responsive.",
                accent_color(ui.visuals().dark_mode),
            ),
            ScanStatus::Ready => (
                "Input map ready for review",
                "Pairing and sample names are proposals. Confirm ambiguous or missing mates before analysis.",
                notice_color(NoticeKind::Success, ui.visuals().dark_mode),
            ),
            ScanStatus::Failed(message) => (
                "Input scan needs attention",
                message.as_str(),
                notice_color(NoticeKind::Error, ui.visuals().dark_mode),
            ),
        };

        egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .corner_radius(10)
            .stroke(Stroke::new(1.0, color))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.label(RichText::new(title).strong().color(color));
                ui.label(detail);
                if matches!(self.scan_status, ScanStatus::Running) {
                    ui.spinner();
                }
            });
    }
}

impl eframe::App for SeqQcApp {
    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "qualification")]
        self.qa_capture.tick(context);
        let previous = self.foundation.simulation;
        let was_active = previous.active();
        self.foundation.simulation.advance();
        if was_active {
            context.request_repaint_after(Duration::from_millis(75));
            if self.foundation.simulation.stored() != previous.stored()
                && let Some(project) = self.active_project.as_mut()
                && let Err(error) =
                    project.set_simulation_state(self.foundation.simulation.stored())
            {
                self.foundation.simulation = Simulation::Interrupted;
                self.set_notice(NoticeKind::Error, error.to_string());
            }
        }
        self.poll_scan(context);
        if self.profile.is_some() {
            if context.input(|input| !input.events.is_empty()) {
                self.last_activity = Instant::now();
            } else if self.last_activity.elapsed() >= SESSION_TIMEOUT {
                self.lock_session(true);
            } else {
                context.request_repaint_after(Duration::from_secs(1));
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.theme.apply(ui.ctx());
        self.header(ui);
        if self.profile.is_some() {
            self.navigation(ui);
            self.content(ui);
        } else {
            self.locked_content(ui);
        }
    }
}

impl Drop for SeqQcApp {
    fn drop(&mut self) {
        self.local_passphrase.zeroize();
        self.confirm_passphrase.zeroize();
    }
}

fn recent_project_card(
    ui: &mut egui::Ui,
    recent: &RecentProject,
    open_requested: &mut Option<PathBuf>,
    forget_requested: &mut Option<String>,
) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(8)
        .stroke(Stroke::new(
            1.0,
            ui.visuals().widgets.inactive.bg_stroke.color,
        ))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&recent.display_name).strong().size(16.0));
                    ui.label(format!(
                        "{} · {} · last opened {}",
                        recent.assay.label(),
                        recent.context.label(),
                        recent.last_opened_at
                    ));
                    ui.label(RichText::new(recent.directory.display().to_string()).monospace());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Forget").clicked() {
                        *forget_requested = Some(recent.project_id.clone());
                    }
                    if ui.button("Open").clicked() {
                        *open_requested = Some(recent.directory.clone());
                    }
                });
            });
        });
}

fn status_frame(ui: &mut egui::Ui, title: &str, detail: &str, kind: NoticeKind) {
    let color = notice_color(kind, ui.visuals().dark_mode);
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(10)
        .stroke(Stroke::new(1.0, color))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong().color(color));
            ui.label(detail);
        });
}

const fn accent_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(73, 210, 203)
    } else {
        Color32::from_rgb(0, 94, 110)
    }
}

const fn notice_color(kind: NoticeKind, dark: bool) -> Color32 {
    match (kind, dark) {
        (NoticeKind::Success, true) => Color32::from_rgb(92, 213, 152),
        (NoticeKind::Warning, true) => Color32::from_rgb(245, 184, 77),
        (NoticeKind::Error, true) => Color32::from_rgb(245, 103, 103),
        (NoticeKind::Success, false) => Color32::from_rgb(20, 105, 61),
        (NoticeKind::Warning, false) => Color32::from_rgb(133, 76, 0),
        (NoticeKind::Error, false) => Color32::from_rgb(175, 34, 34),
    }
}

fn path_or_not_selected(path: Option<&Path>) -> String {
    path.map_or_else(
        || "Not selected".to_owned(),
        |path| path.display().to_string(),
    )
}

fn summary_cards(ui: &mut egui::Ui, discovery: &DiscoveryResult) {
    ui.columns(4, |columns| {
        metric_card(
            &mut columns[0],
            "SUPPORTED FILES",
            discovery.summary.supported_files,
            "FASTQ, BAM, and CRAM",
        );
        metric_card(
            &mut columns[1],
            "FASTQ",
            discovery.summary.fastq_files,
            "Plain or gzip-compressed",
        );
        metric_card(
            &mut columns[2],
            "BAM / CRAM",
            discovery.summary.bam_files + discovery.summary.cram_files,
            "Existing aligned inputs",
        );
        metric_card(
            &mut columns[3],
            "PROPOSED GROUPS",
            discovery.groups.len(),
            "Review before analysis",
        );
    });
}

fn metric_card(ui: &mut egui::Ui, label: &str, value: usize, detail: &str) {
    text_metric_card(ui, label, &value.to_string(), detail);
}

fn text_metric_card(ui: &mut egui::Ui, label: &str, value: &str, detail: &str) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(10)
        .stroke(Stroke::new(
            1.0,
            ui.visuals().widgets.inactive.bg_stroke.color,
        ))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_height(82.0);
            ui.label(RichText::new(label).size(10.0).strong());
            ui.label(RichText::new(value).size(24.0).strong());
            ui.label(RichText::new(detail).size(11.0));
        });
}

fn sample_map(
    ui: &mut egui::Ui,
    discovery: &DiscoveryResult,
    filter: &mut String,
    page: &mut usize,
) {
    ui.horizontal_wrapped(|ui| {
        ui.heading("Proposed sample map");
        ui.horizontal(|ui| {
            let filter_label = ui.label("Filter sample or file");
            if ui
                .add_sized(
                    [260.0, 30.0],
                    egui::TextEdit::singleline(filter).hint_text("Filter sample or file…"),
                )
                .labelled_by(filter_label.id)
                .changed()
            {
                *page = 0;
            }
        });
    });
    ui.label(
        "SeqQC does not silently resolve duplicate mates, missing mates, or uncertain layouts.",
    );
    ui.add_space(8.0);

    let normalized_filter = filter.trim().to_lowercase();
    let matching = discovery
        .groups
        .iter()
        .filter(|group| group_matches(group, &normalized_filter))
        .collect::<Vec<_>>();
    let page_count = matching.len().div_ceil(MAX_VISIBLE_GROUPS).max(1);
    *page = (*page).min(page_count - 1);
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(*page > 0, egui::Button::new("Previous page"))
            .clicked()
        {
            *page -= 1;
        }
        ui.label(format!(
            "Page {} of {page_count} · {} matching groups",
            *page + 1,
            matching.len()
        ));
        if ui
            .add_enabled(*page + 1 < page_count, egui::Button::new("Next page"))
            .clicked()
        {
            *page += 1;
        }
    });
    let visible = matching
        .into_iter()
        .skip(*page * MAX_VISIBLE_GROUPS)
        .take(MAX_VISIBLE_GROUPS);

    sample_map_rows(ui, visible, *page);
}

fn sample_map_rows<'a>(
    ui: &mut egui::Ui,
    visible: impl Iterator<Item = &'a InputGroup>,
    page: usize,
) {
    egui::ScrollArea::horizontal()
        .id_salt("sample_map_horizontal")
        .show(ui, |ui| {
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(10)
                .stroke(Stroke::new(
                    1.0,
                    ui.visuals().widgets.inactive.bg_stroke.color,
                ))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    egui::Grid::new("sample_map_grid")
                        .num_columns(6)
                        .striped(true)
                        .min_col_width(110.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("SAMPLE").strong());
                            ui.label(RichText::new("TYPE").strong());
                            ui.label(RichText::new("LAYOUT").strong());
                            ui.label(RichText::new("FILES").strong());
                            ui.label(RichText::new("REVIEW").strong());
                            ui.label(RichText::new("EVIDENCE").strong());
                            ui.end_row();

                            for (index, group) in visible.enumerate() {
                                ui.label(group.proposed_sample.as_deref().unwrap_or("Unresolved"));
                                ui.label(kind_label(group.kind));
                                ui.label(layout_label(group));
                                ui.label(group_file_count(group).to_string());
                                let (status, color) =
                                    group_status(group.status, ui.visuals().dark_mode);
                                ui.colored_label(color, status);
                                ui.push_id((page, index), |ui| {
                                    ui.menu_button(
                                        format!(
                                            "Files: {}",
                                            group
                                                .proposed_sample
                                                .as_deref()
                                                .unwrap_or("unresolved")
                                        ),
                                        |ui| {
                                            ui.label(format!(
                                                "Lane: {} · pairing confidence: {:?}",
                                                group.lane.as_deref().unwrap_or("unspecified"),
                                                group.confidence
                                            ));
                                            for path in
                                                group.r1.iter().chain(&group.r2).chain(&group.files)
                                            {
                                                ui.label(path.display().to_string());
                                            }
                                        },
                                    );
                                });
                                ui.end_row();
                            }
                        });
                });
        });
}

fn group_matches(group: &InputGroup, normalized_filter: &str) -> bool {
    if normalized_filter.is_empty() {
        return true;
    }

    group
        .proposed_sample
        .as_deref()
        .is_some_and(|sample| sample.to_lowercase().contains(normalized_filter))
        || group
            .r1
            .iter()
            .chain(&group.r2)
            .chain(&group.files)
            .any(|path| {
                path.to_string_lossy()
                    .to_lowercase()
                    .contains(normalized_filter)
            })
}

const fn kind_label(kind: InputKind) -> &'static str {
    match kind {
        InputKind::Fastq => "FASTQ",
        InputKind::Bam => "BAM",
        InputKind::Cram => "CRAM",
    }
}

fn layout_label(group: &InputGroup) -> &'static str {
    if !group.r1.is_empty() && !group.r2.is_empty() {
        "Paired-end"
    } else if group.kind == InputKind::Fastq {
        "Single / unresolved"
    } else {
        "Aligned"
    }
}

fn group_file_count(group: &InputGroup) -> usize {
    group.r1.len() + group.r2.len() + group.files.len()
}

const fn group_status(status: GroupStatus, dark: bool) -> (&'static str, Color32) {
    match status {
        GroupStatus::PairedCandidate | GroupStatus::AlignedInput => (
            "Expected structure",
            notice_color(NoticeKind::Success, dark),
        ),
        GroupStatus::MissingMate
        | GroupStatus::AmbiguousDuplicateMate
        | GroupStatus::UnresolvedLayout => {
            ("Needs review", notice_color(NoticeKind::Warning, dark))
        }
    }
}

#[cfg(target_os = "windows")]
const fn current_platform() -> Platform {
    Platform::Windows
}

#[cfg(target_os = "macos")]
const fn current_platform() -> Platform {
    Platform::MacOs
}

#[cfg(target_os = "linux")]
const fn current_platform() -> Platform {
    Platform::Linux
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use seqqc_discovery::{GroupStatus, InputGroup, InputKind, PairConfidence};

    use super::{SESSION_TIMEOUT, group_file_count, group_matches, layout_label};

    fn paired_group() -> InputGroup {
        InputGroup {
            kind: InputKind::Fastq,
            pair_key: Some("sample-a".to_owned()),
            proposed_sample: Some("Sample-A".to_owned()),
            lane: Some("L001".to_owned()),
            r1: vec![PathBuf::from("Sample-A_R1.fastq.gz")],
            r2: vec![PathBuf::from("Sample-A_R2.fastq.gz")],
            files: Vec::new(),
            confidence: PairConfidence::High,
            status: GroupStatus::PairedCandidate,
        }
    }

    #[test]
    fn sample_filter_matches_names_and_paths_case_insensitively() {
        let group = paired_group();
        assert!(group_matches(&group, "sample-a"));
        assert!(group_matches(&group, "_r2.fastq"));
        assert!(!group_matches(&group, "control"));
    }

    #[test]
    fn paired_layout_and_file_count_are_stable() {
        let group = paired_group();
        assert_eq!(layout_label(&group), "Paired-end");
        assert_eq!(group_file_count(&group), 2);
    }

    #[test]
    fn secure_session_timeout_is_fifteen_minutes() {
        assert_eq!(SESSION_TIMEOUT, Duration::from_mins(15));
    }

    #[test]
    fn themes_meet_text_contrast_for_semantics_controls_and_selection() {
        fn luminance(color: eframe::egui::Color32) -> f64 {
            let [r, g, b, _] = color.to_array();
            let linear = |channel: u8| {
                let value = f64::from(channel) / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
        }
        let ratio = |foreground, background| {
            let a = luminance(foreground);
            let b = luminance(background);
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        };
        let context = eframe::egui::Context::default();
        for theme in super::Theme::ALL {
            theme.apply(&context);
            let visuals = context.global_style().visuals.clone();
            let text = visuals.override_text_color.expect("theme text");
            for background in [
                visuals.panel_fill,
                visuals.extreme_bg_color,
                visuals.faint_bg_color,
                visuals.widgets.inactive.bg_fill,
                visuals.widgets.hovered.bg_fill,
                visuals.widgets.active.bg_fill,
                visuals.selection.bg_fill,
            ] {
                assert!(
                    ratio(text, background) >= 4.5,
                    "{theme:?} text contrast {}",
                    ratio(text, background)
                );
            }
            for color in [
                super::accent_color(visuals.dark_mode),
                super::notice_color(super::NoticeKind::Success, visuals.dark_mode),
                visuals.warn_fg_color,
                visuals.error_fg_color,
            ] {
                for background in [
                    visuals.panel_fill,
                    visuals.extreme_bg_color,
                    visuals.faint_bg_color,
                ] {
                    assert!(
                        ratio(color, background) >= 4.5,
                        "{theme:?} status contrast {}",
                        ratio(color, background)
                    );
                }
            }
        }
    }

    #[test]
    fn clearing_project_state_cancels_worker_and_removes_sensitive_form_data_on_lock() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc::sync_channel,
        };
        let mut app = super::SeqQcApp::with_context(&eframe::egui::Context::default());
        let cancelled = Arc::new(AtomicBool::new(false));
        let (_sender, receiver) = sync_channel(1);
        app.scan_job = Some(super::ScanJob {
            cancelled: Arc::clone(&cancelled),
            root: PathBuf::from("old-project"),
            receiver,
        });
        app.scan_status = super::ScanStatus::Running;
        app.new_project.display_name = "private draft".to_owned();
        app.sample_filter = "private sample".to_owned();
        app.foundation.reference = Some(PathBuf::from("private.fa"));
        app.foundation.simulation = super::Simulation::Queued;
        app.lock_session(false);
        assert!(cancelled.load(Ordering::Relaxed));
        assert!(app.scan_job.is_none());
        assert!(app.sample_filter.is_empty());
        assert!(app.new_project.display_name.is_empty());
        assert!(app.foundation.reference.is_none());
        assert_eq!(app.foundation.simulation, super::Simulation::Idle);
    }

    #[test]
    fn sample_map_renders_last_page_of_ten_thousand_samples_in_every_theme() {
        use eframe::egui::{Context, RawInput, Rect, pos2, vec2};
        let context = Context::default();
        let mut discovery = seqqc_discovery::DiscoveryResult {
            root: PathBuf::new(),
            files: Vec::new(),
            ignored: Vec::new(),
            groups: (0..10_000)
                .map(|index| {
                    let mut group = paired_group();
                    group.proposed_sample = Some(format!("sample-{index:05}"));
                    group
                })
                .collect(),
            summary: seqqc_discovery::DiscoverySummary::default(),
        };
        for theme in super::Theme::ALL {
            theme.apply(&context);
            let mut filter = String::new();
            let mut page = 39;
            let mut output = context.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 820.0))),
                    ..Default::default()
                },
                |ui| {
                    super::sample_map(ui, &discovery, &mut filter, &mut page);
                },
            );
            output.textures_delta.clear(); // No GPU backend in this headless rendering test.
            assert_eq!(page, 39);
            assert!(!output.shapes.is_empty());
            filter = "sample-09999".to_owned();
            let mut output = context.run_ui(RawInput::default(), |ui| {
                super::sample_map(ui, &discovery, &mut filter, &mut page);
            });
            output.textures_delta.clear();
            assert_eq!(page, 0);
        }
        discovery.groups.clear();
        let mut page = usize::MAX;
        let mut output = context.run_ui(RawInput::default(), |ui| {
            super::sample_map(ui, &discovery, &mut String::new(), &mut page);
        });
        output.textures_delta.clear();
        assert_eq!(page, 0);
    }
}
