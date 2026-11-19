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
    pub accent_color: Option<egui::Color32>,
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

        let accent_color = config.app.accent_color.as_deref().and_then(parse_hex_color);

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
            accent_color,
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
/// Retourne un message d'erreur si la police est introuvable.
pub fn apply_font(ctx: &egui::Context, app_config: &crate::config::AppConfig) -> Option<String> {
    const DEFAULT_SIZE: f32 = 14.0;

    let mut fonts = egui::FontDefinitions::default();
    let mut error: Option<String> = None;

    if let Some(ref name_or_path) = app_config.font_family {
        if !name_or_path.is_empty() {
            let bytes = if std::path::Path::new(name_or_path).is_file() {
                std::fs::read(name_or_path)
                    .map_err(|e| format!("Police '{name_or_path}' illisible : {e}"))
            } else {
                font_bytes_by_name(name_or_path)
                    .ok_or_else(|| format!("Police '{name_or_path}' introuvable sur le système"))
            };
            match bytes {
                Ok(b) => {
                    fonts.font_data.insert("custom".to_owned(), egui::FontData::from_owned(b).into());
                    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                        fonts.families.entry(family).or_default().insert(0, "custom".to_owned());
                    }
                }
                Err(e) => error = Some(e),
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

fn panel_header(ui: &mut egui::Ui, icon: &str, title: &str, accent: Option<egui::Color32>) {
    let bg = accent.unwrap_or_else(|| {
        if ui.visuals().dark_mode {
            egui::Color32::from_rgb(35, 38, 46)
        } else {
            egui::Color32::from_rgb(215, 220, 232)
        }
    });
    let text_color = contrasting_text_color(bg);

    let clip = ui.clip_rect();
    let top = ui.cursor().min.y;

    // Réserver un slot AVANT le contenu : le fond sera inséré derrière le texte dans la paint list.
    let bg_slot = ui.painter().add(egui::Shape::Noop);

    // Frame transparent pour le layout (avance le curseur, marges, texte).
    egui::Frame::new()
        .inner_margin(egui::Margin { left: 10, right: 10, top: 7, bottom: 7 })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(egui::RichText::new(format!("{icon}  {title}")).strong().color(text_color));
        });

    // Remplir le slot avec le rectangle pleine largeur (clip_rect.x dépasse les marges du panel).
    let full_rect = egui::Rect::from_min_max(
        egui::pos2(clip.min.x, top),
        egui::pos2(clip.max.x, ui.cursor().min.y),
    );
    ui.painter().set(bg_slot, egui::Shape::rect_filled(full_rect, 0.0, bg));

    ui.add_space(2.0);
}

fn contrasting_text_color(bg: egui::Color32) -> egui::Color32 {
    let r = bg.r() as f32 / 255.0;
    let g = bg.g() as f32 / 255.0;
    let b = bg.b() as f32 / 255.0;
    if 0.299 * r + 0.587 * g + 0.114 * b > 0.5 {
        egui::Color32::from_gray(20)
    } else {
        egui::Color32::WHITE
    }
}

fn parse_hex_color(hex: &str) -> Option<egui::Color32> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(egui::Color32::from_rgb(r, g, b))
    } else {
        None
    }
}

fn font_bytes_by_name(name: &str) -> Option<Vec<u8>> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let query = fontdb::Query {
        families: &[fontdb::Family::Name(name)],
        weight: fontdb::Weight::NORMAL,
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    };
    let id = db.query(&query)?;
    db.with_face_data(id, |data, _| data.to_vec())
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Barre de statut en bas (rendue avant les panneaux pour qu'elle ne soit pas couverte)
        if let Some(ref err) = self.app_error.clone() {
            egui::Panel::bottom("status_bar").show(ui, |ui| {
                ui.colored_label(egui::Color32::RED, format!("⬤ {err}"));
            });
        }

        let accent = self.accent_color;

        egui::Panel::left("file_tree")
            .default_size(180.0)
            .show(ui, |ui| {
                panel_header(ui, "󰙅", "Explorer", accent);
            });

        egui::Panel::right("lt_panel")
            .default_size(260.0)
            .show(ui, |ui| {
                panel_header(ui, "󰓆", "LanguageTool", accent);
            });

        egui::CentralPanel::default().show(ui, |_ui| {});
    }
}
