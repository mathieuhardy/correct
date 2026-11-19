use crate::config::{Config, Theme};
use crate::dictionary::Dictionary;
use crate::languagetool::{LtClient, LtMatch, LtStatus};
use std::path::PathBuf;

pub struct App {
    pub config: Config,
    pub dictionary: Dictionary,
    pub lt_client: LtClient,
    pub lt_status: LtStatus,
    pub current_project: Option<PathBuf>,
    pub open_file: Option<FileBuffer>,
    pub lt_results: Vec<LtMatch>,
    pub lt_running: bool,
    pub selected_match: Option<usize>,
    /// Message d'erreur affiché dans la barre de statut en bas.
    pub app_error: Option<String>,
}

pub struct FileBuffer {
    pub path: PathBuf,
    pub content: String,
    pub modified: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let config = Config::load();
        apply_theme(&cc.egui_ctx, &config.app.theme);
        let font_error = apply_font(&cc.egui_ctx, &config.app);

        let (lt_client, lt_status) = LtClient::init(&config.languagetool);

        // Charge le dictionnaire du dernier projet s'il existe encore sur le disque.
        let (current_project, dictionary) = match &config.app.last_project {
            Some(path) if path.is_dir() => {
                let dict = Dictionary::load(path);
                (Some(path.clone()), dict)
            }
            _ => (None, Dictionary::default()),
        };

        Self {
            config,
            dictionary,
            lt_client,
            lt_status,
            current_project,
            open_file: None,
            lt_results: Vec::new(),
            lt_running: false,
            selected_match: None,
            app_error: font_error,
        }
    }

    /// Ouvre un projet : charge son dictionnaire et persiste `last_project`.
    pub fn open_project(&mut self, path: PathBuf) {
        // Fermer le fichier ouvert (drop du watcher futur inclus)
        self.open_file = None;
        self.lt_results.clear();
        self.selected_match = None;

        self.dictionary = Dictionary::load(&path);
        self.current_project = Some(path.clone());
        self.config.app.last_project = Some(path);

        if let Err(e) = self.config.save() {
            self.app_error = Some(e);
        }
    }
}

pub fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    match theme {
        Theme::Dark => ctx.set_visuals(egui::Visuals::dark()),
        Theme::Light => ctx.set_visuals(egui::Visuals::light()),
    }
}

/// Applique la taille et la famille de police issues de la config.
/// Retourne un message d'erreur si le fichier de police est introuvable.
pub fn apply_font(ctx: &egui::Context, app_config: &crate::config::AppConfig) -> Option<String> {
    const DEFAULT_SIZE: f32 = 14.0;

    let mut fonts = egui::FontDefinitions::default();
    let mut error: Option<String> = None;

    if let Some(ref path) = app_config.font_family {
        if !path.is_empty() {
            match std::fs::read(path) {
                Ok(bytes) => {
                    fonts.font_data.insert("custom".to_owned(), egui::FontData::from_owned(bytes).into());
                    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                        fonts.families.entry(family).or_default().insert(0, "custom".to_owned());
                    }
                }
                Err(e) => {
                    error = Some(format!("Police '{path}' introuvable : {e}"));
                }
            }
        }
    }

    ctx.set_fonts(fonts);

    if (app_config.font_size - DEFAULT_SIZE).abs() > f32::EPSILON {
        let scale = app_config.font_size / DEFAULT_SIZE;
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            ctx.style_mut_of(theme, |s| {
                for style in s.text_styles.values_mut() {
                    style.size *= scale;
                }
            });
        }
    }

    error
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Barre de statut en bas (rendue avant les panneaux pour qu'elle ne soit pas couverte)
        if let Some(ref err) = self.app_error.clone() {
            egui::Panel::bottom("status_bar").show(ui, |ui| {
                ui.colored_label(egui::Color32::RED, format!("⬤ {err}"));
            });
        }

        egui::Panel::left("file_tree")
            .default_size(180.0)
            .show(ui, |ui| {
                ui.heading("Explorer");
            });

        egui::Panel::right("lt_panel")
            .default_size(260.0)
            .show(ui, |ui| {
                ui.heading("LanguageTool");
            });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Ouvrir un fichier…");
        });
    }
}
