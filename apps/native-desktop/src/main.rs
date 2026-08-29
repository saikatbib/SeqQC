#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(linker_messages)]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, Stroke};
use seqqc_discovery::{DiscoveryResult, GroupStatus, InputGroup, InputKind, discover};
use seqqc_engine::{Capability, Platform, check_task_platform};
use seqqc_projects::{
    Assay, CreateProjectRequest, LocalProfile, PassphraseProjectManager, ProjectContext,
    ProjectWorkspace, RecentProject, detected_local_principal_name,
};
use zeroize::Zeroize;

const APP_TITLE: &str = "SeqQC Studio";
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
                visuals.selection.bg_fill = Color32::from_rgb(18, 147, 160);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(18, 37, 50);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(26, 58, 72);
                visuals.widgets.active.bg_fill = Color32::from_rgb(22, 113, 126);
            }
            Self::ClinicalLight => {
                visuals.panel_fill = Color32::from_rgb(242, 247, 249);
                visuals.extreme_bg_color = Color32::WHITE;
                visuals.faint_bg_color = Color32::from_rgb(225, 236, 239);
                visuals.override_text_color = Some(Color32::from_rgb(20, 39, 50));
                visuals.selection.bg_fill = Color32::from_rgb(25, 139, 151);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(229, 239, 241);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(205, 229, 232);
                visuals.widgets.active.bg_fill = Color32::from_rgb(133, 200, 207);
            }
            Self::HighContrast => {
                visuals.panel_fill = Color32::BLACK;
                visuals.extreme_bg_color = Color32::BLACK;
                visuals.faint_bg_color = Color32::from_rgb(20, 20, 20);
                visuals.override_text_color = Some(Color32::WHITE);
                visuals.selection.bg_fill = Color32::from_rgb(0, 185, 205);
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 35, 35);
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(0, 90, 105);
                visuals.widgets.active.bg_fill = Color32::from_rgb(0, 135, 150);
            }
        }

        visuals.warn_fg_color = Color32::from_rgb(245, 184, 77);
        visuals.error_fg_color = Color32::from_rgb(245, 103, 103);
        context.set_visuals(visuals);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Screen {
    #[default]
    Overview,
    Projects,
    InputDiscovery,
}

struct ScanJob {
    root: PathBuf,
    receiver: Receiver<Result<DiscoveryResult, String>>,
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

struct SeqQcApp {
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
}

impl SeqQcApp {
    fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::ClinicalNight;
        theme.apply(&creation_context.egui_ctx);
        let (manager, startup_error) = match PassphraseProjectManager::for_current_user() {
            Ok(manager) => (Some(manager), None),
            Err(error) => (None, Some(error.to_string())),
        };

        Self {
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
            Err(error) => self.set_notice(NoticeKind::Error, error.to_string()),
        }
    }

    fn lock_session(&mut self, inactivity: bool) {
        if let Some(project) = self.active_project.as_mut() {
            let _ = project.record_event("project.locked", "success");
        }
        if let Some(profile) = self.profile.as_mut() {
            let _ = profile.record_event("local_session.locked", "success");
        }
        self.active_project = None;
        self.project_editor = None;
        self.profile = None;
        self.discovery = None;
        self.selected_root = None;
        self.scan_job = None;
        self.scan_status = ScanStatus::Idle;
        self.screen = Screen::Overview;
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
                let display_name = workspace.summary().display_name.clone();
                self.project_editor = Some(ProjectEditor::from_workspace(&workspace));
                self.selected_root
                    .clone_from(&workspace.summary().input_root);
                self.active_project = Some(workspace);
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
        let result = match (&self.manager, self.profile.as_mut()) {
            (Some(manager), Some(profile)) => manager.open_project(profile, directory),
            _ => return,
        };
        match result {
            Ok(workspace) => {
                let display_name = workspace.summary().display_name.clone();
                self.project_editor = Some(ProjectEditor::from_workspace(&workspace));
                self.selected_root
                    .clone_from(&workspace.summary().input_root);
                self.active_project = Some(workspace);
                self.screen = Screen::Overview;
                self.set_notice(
                    NoticeKind::Success,
                    format!("Encrypted project “{display_name}” is open with a writer lock."),
                );
            }
            Err(error) => self.set_notice(NoticeKind::Error, error.to_string()),
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
        if let Some(root) = dialog.pick_folder() {
            self.persist_input_root(&root);
            self.start_scan(root);
        }
    }

    fn persist_input_root(&mut self, root: &Path) {
        let Some(project) = self.active_project.as_mut() else {
            return;
        };
        let summary = project.summary().clone();
        if let Err(error) = project.update_metadata(
            &summary.display_name,
            summary.assay,
            summary.context,
            Some(root),
        ) {
            self.set_notice(NoticeKind::Error, error.to_string());
            return;
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
    }

    fn start_scan(&mut self, root: PathBuf) {
        let (sender, receiver) = sync_channel(1);
        let scan_root = root.clone();
        std::thread::spawn(move || {
            let outcome = discover(&scan_root).map_err(|error| error.to_string());
            let _result = sender.send(outcome);
        });

        self.selected_root = Some(root.clone());
        self.discovery = None;
        self.scan_status = ScanStatus::Running;
        self.scan_job = Some(ScanJob { root, receiver });
        self.sample_filter.clear();
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
            .exact_size(72.0)
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
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(APP_TITLE)
                                .size(24.0)
                                .strong()
                                .color(Color32::from_rgb(73, 210, 203)),
                        );
                        ui.label(RichText::new("LOCAL SEQUENCING QUALITY CONTROL").size(10.0));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("OFFLINE · NATIVE")
                                .size(11.0)
                                .strong()
                                .color(Color32::from_rgb(92, 213, 152)),
                        );
                        if self.profile.is_some() && ui.button("Lock session").clicked() {
                            lock_requested = true;
                        }
                        if let Some(profile) = &self.profile {
                            ui.label(format!("Local: {}", profile.principal().display_name));
                        }
                        ui.separator();
                        for theme in Theme::ALL.into_iter().rev() {
                            if ui
                                .selectable_value(&mut self.theme, theme, theme.label())
                                .changed()
                            {
                                self.theme.apply(ui.ctx());
                            }
                        }
                    });
                });
            });
        if lock_requested {
            self.lock_session(false);
        }
    }

    fn navigation(&mut self, ui: &mut egui::Ui) {
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
                        Color32::from_rgb(92, 213, 152),
                        "Alignment platform: eligible",
                    );
                } else {
                    ui.colored_label(Color32::from_rgb(245, 184, 77), "Alignment: Linux only");
                }
            });
    }

    fn content(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.notice_card(ui);
                match self.screen {
                    Screen::Overview => self.overview_content(ui),
                    Screen::Projects => self.projects_content(ui),
                    Screen::InputDiscovery => self.discovery_content(ui),
                }
            });
        });
    }

    fn locked_content(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading("Secure local session");
                ui.label(
                    "SeqQC uses the signed-in operating-system account plus a local application passphrase. No online account is created and no internet connection is used.",
                );
                ui.add_space(18.0);
                self.notice_card(ui);
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
                    ui.add_sized(
                        [340.0, 34.0],
                        egui::TextEdit::singleline(&mut self.local_passphrase)
                            .password(true)
                            .hint_text("Local passphrase"),
                    );
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
                    ui.add_sized(
                        [340.0, 34.0],
                        egui::TextEdit::singleline(&mut self.principal_name_input)
                            .hint_text("Local display name"),
                    );
                    ui.add_sized(
                        [340.0, 34.0],
                        egui::TextEdit::singleline(&mut self.local_passphrase)
                            .password(true)
                            .hint_text("Local passphrase (12+ characters)"),
                    );
                    ui.add_sized(
                        [340.0, 34.0],
                        egui::TextEdit::singleline(&mut self.confirm_passphrase)
                            .password(true)
                            .hint_text("Confirm local passphrase"),
                    );
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
    }

    fn notice_card(&mut self, ui: &mut egui::Ui) {
        let Some(notice) = &self.notice else {
            return;
        };
        let color = notice_color(notice.kind);
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
                    ui.colored_label(Color32::from_rgb(92, 213, 152), "SQLCipher active");
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
            .stroke(Stroke::new(1.0, Color32::from_rgb(73, 210, 203)))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.heading("Create encrypted project");
                ui.label("The on-disk folder name is opaque so the display name is not exposed.");
                ui.add_space(8.0);
                egui::Grid::new("new_project_grid")
                    .num_columns(2)
                    .spacing([14.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Project name");
                        ui.add_sized(
                            [360.0, 30.0],
                            egui::TextEdit::singleline(&mut self.new_project.display_name)
                                .hint_text("Example: Pilot sequencing cohort"),
                        );
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
                egui::Grid::new("project_editor_grid")
                    .num_columns(2)
                    .spacing([14.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Project name");
                        ui.add_sized(
                            [360.0, 30.0],
                            egui::TextEdit::singleline(&mut editor.display_name),
                        );
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

        ui.horizontal(|ui| {
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
                ui.label(RichText::new(root.display().to_string()).monospace());
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
            sample_map(ui, discovery, &mut self.sample_filter);
        }
    }

    fn status_card(&self, ui: &mut egui::Ui) {
        let (title, detail, color) = match &self.scan_status {
            ScanStatus::Idle => (
                "Ready for local input",
                "Supported: FASTQ, FASTQ.GZ, FQ, FQ.GZ, BAM, and CRAM.",
                Color32::from_rgb(104, 173, 215),
            ),
            ScanStatus::Running => (
                "Scanning folder…",
                "Discovery runs away from the interface thread so the native window stays responsive.",
                Color32::from_rgb(73, 210, 203),
            ),
            ScanStatus::Ready => (
                "Input map ready for review",
                "Pairing and sample names are proposals. Confirm ambiguous or missing mates before analysis.",
                Color32::from_rgb(92, 213, 152),
            ),
            ScanStatus::Failed(message) => (
                "Input scan needs attention",
                message.as_str(),
                Color32::from_rgb(245, 103, 103),
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
    let color = notice_color(kind);
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

const fn notice_color(kind: NoticeKind) -> Color32 {
    match kind {
        NoticeKind::Success => Color32::from_rgb(92, 213, 152),
        NoticeKind::Warning => Color32::from_rgb(245, 184, 77),
        NoticeKind::Error => Color32::from_rgb(245, 103, 103),
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

fn sample_map(ui: &mut egui::Ui, discovery: &DiscoveryResult, filter: &mut String) {
    ui.horizontal(|ui| {
        ui.heading("Proposed sample map");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_sized(
                [260.0, 30.0],
                egui::TextEdit::singleline(filter).hint_text("Filter sample or file…"),
            );
        });
    });
    ui.label(
        "SeqQC does not silently resolve duplicate mates, missing mates, or uncertain layouts.",
    );
    ui.add_space(8.0);

    let normalized_filter = filter.trim().to_lowercase();
    let visible = discovery
        .groups
        .iter()
        .filter(|group| group_matches(group, &normalized_filter))
        .take(MAX_VISIBLE_GROUPS)
        .collect::<Vec<_>>();

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
                .num_columns(5)
                .striped(true)
                .min_col_width(110.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("SAMPLE").strong());
                    ui.label(RichText::new("TYPE").strong());
                    ui.label(RichText::new("LAYOUT").strong());
                    ui.label(RichText::new("FILES").strong());
                    ui.label(RichText::new("REVIEW").strong());
                    ui.end_row();

                    for group in &visible {
                        ui.label(group.proposed_sample.as_deref().unwrap_or("Unresolved"));
                        ui.label(kind_label(group.kind));
                        ui.label(layout_label(group));
                        ui.label(group_file_count(group).to_string());
                        let (status, color) = group_status(group.status);
                        ui.colored_label(color, status);
                        ui.end_row();
                    }
                });
        });

    let matching_count = discovery
        .groups
        .iter()
        .filter(|group| group_matches(group, &normalized_filter))
        .count();
    if matching_count > visible.len() {
        ui.label(format!(
            "Showing the first {} of {matching_count} matching groups.",
            visible.len()
        ));
    } else {
        ui.label(format!("Showing {matching_count} groups."));
    }
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

const fn group_status(status: GroupStatus) -> (&'static str, Color32) {
    match status {
        GroupStatus::PairedCandidate | GroupStatus::AlignedInput => {
            ("Expected structure", Color32::from_rgb(92, 213, 152))
        }
        GroupStatus::MissingMate
        | GroupStatus::AmbiguousDuplicateMate
        | GroupStatus::UnresolvedLayout => ("Needs review", Color32::from_rgb(245, 184, 77)),
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
}
