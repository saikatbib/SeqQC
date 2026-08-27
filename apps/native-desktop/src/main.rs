#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
use std::time::Duration;

use eframe::egui::{self, Color32, RichText, Stroke};
use seqqc_discovery::{DiscoveryResult, GroupStatus, InputGroup, InputKind, discover};
use seqqc_engine::{Capability, Platform, check_task_platform};

const APP_TITLE: &str = "SeqQC Studio";
const MAX_VISIBLE_GROUPS: usize = 250;

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

struct SeqQcApp {
    theme: Theme,
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

        Self {
            theme,
            scan_status: ScanStatus::Idle,
            scan_job: None,
            discovery: None,
            selected_root: None,
            sample_filter: String::new(),
        }
    }

    fn choose_folder(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Choose sequencing input folder");
        if let Some(root) = &self.selected_root {
            dialog = dialog.set_directory(root);
        }
        if let Some(root) = dialog.pick_folder() {
            self.start_scan(root);
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
                }
                Err(message) => {
                    self.discovery = None;
                    self.scan_status = ScanStatus::Failed(message);
                }
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
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
                            RichText::new("OFFLINE · NATIVE SHELL")
                                .size(11.0)
                                .strong()
                                .color(Color32::from_rgb(92, 213, 152)),
                        );
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
    }

    fn navigation(ui: &mut egui::Ui) {
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
                ui.add_enabled(false, egui::Button::new("Overview"));
                ui.add_enabled(false, egui::Button::new("Projects"));
                ui.add(egui::Button::new(RichText::new("Input discovery").strong()).selected(true));
                ui.add_enabled(false, egui::Button::new("QC findings"));
                ui.add_enabled(false, egui::Button::new("Reports"));
                ui.add_space(28.0);
                ui.label(RichText::new("MILESTONE 1").size(10.0).strong());
                ui.label("Native shell spike");
                ui.label("No clinical interpretation");
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
                ui.heading("Discover sequencing inputs");
                ui.label(
                    "Choose a local or mounted folder. SeqQC scans nested folders without copying, uploading, or modifying sequencing files.",
                );
                ui.add_space(14.0);

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
            });
        });
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
                "Discovery is running away from the interface thread. Large folders remain cancellable in the next slice.",
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
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.theme.apply(ui.ctx());
        self.header(ui);
        Self::navigation(ui);
        self.content(ui);
    }
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
            ui.label(RichText::new(value.to_string()).size(28.0).strong());
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

    use seqqc_discovery::{GroupStatus, InputGroup, InputKind, PairConfidence};

    use super::{group_file_count, group_matches, layout_label};

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
}
