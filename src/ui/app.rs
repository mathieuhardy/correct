use crate::config::{Config, Theme};
use crate::dictionary::Dictionary;
use crate::languagetool::{LtClient, LtMatch, LtStatus};
use std::path::PathBuf;

pub struct App {
    config: Config,
    dictionary: Dictionary,
    lt_client: LtClient,
    lt_status: LtStatus,
    current_project: Option<PathBuf>,
    open_file: Option<FileBuffer>,
    lt_results: Vec<LtMatch>,
    lt_running: bool,
    selected_match: Option<usize>,
}

pub struct FileBuffer {
    pub path: PathBuf,
    pub content: String,
    pub modified: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let config = Config::default();
        apply_theme(&cc.egui_ctx, &config.app.theme);
        let lt_client = LtClient::new(
            config.languagetool.url.clone(),
            config.languagetool.language.clone(),
        );
        Self {
            config,
            dictionary: Dictionary::default(),
            lt_client,
            lt_status: LtStatus::Connecting,
            current_project: None,
            open_file: None,
            lt_results: Vec::new(),
            lt_running: false,
            selected_match: None,
        }
    }
}

fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    match theme {
        Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
        Theme::Light => ctx.set_visuals(egui::Visuals::light()),
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("file_tree")
            .min_width(180.0)
            .show(ctx, |ui| {
                ui.heading("Explorer");
            });

        egui::SidePanel::right("lt_panel")
            .min_width(260.0)
            .show(ctx, |ui| {
                ui.heading("LanguageTool");
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Ouvrir un fichier…");
        });
    }
}
