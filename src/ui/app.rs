use crate::config::Config;
use crate::dictionary::Dictionary;
use crate::languagetool::{LtClient, LtMatch, LtStatus};
use std::path::PathBuf;

use super::file_tree::FileTree;

pub struct App {
    pub config: Config,
    pub dictionary: Dictionary,
    pub lt_client: LtClient,
    pub lt_status: LtStatus,
    pub current_project: Option<PathBuf>,
    pub open_file: Option<FileBuffer>,
    pub file_tree: FileTree,
    pub lt_results: Vec<LtMatch>,
    pub lt_running: bool,
    pub selected_match: Option<usize>,
    pub accent_bg: Option<egui::Color32>,
    pub accent_fg: Option<egui::Color32>,
    pub selection_bg: Option<egui::Color32>,
    pub selection_fg: Option<egui::Color32>,
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

        // Thème sombre par défaut — l'utilisateur contrôle les couleurs via la config.
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        apply_colors(&cc.egui_ctx, &config.app);
        let font_error = apply_font(&cc.egui_ctx, &config.app);

        let (lt_client, lt_status) = LtClient::init(&config.languagetool);

        let accent_bg    = config.app.accent_color_bg.as_deref().and_then(parse_hex_color);
        let accent_fg    = config.app.accent_color_fg.as_deref().and_then(parse_hex_color);
        let selection_bg = config.app.selection_color_bg.as_deref().and_then(parse_hex_color);
        let selection_fg = config.app.selection_color_fg.as_deref().and_then(parse_hex_color);

        let (current_project, dictionary, file_tree) = match &config.app.last_project {
            Some(path) if path.is_dir() => {
                let dict = Dictionary::load(path);
                let tree = FileTree::load(path);
                (Some(path.clone()), dict, tree)
            }
            _ => (None, Dictionary::default(), FileTree::empty()),
        };

        Self {
            config,
            dictionary,
            lt_client,
            lt_status,
            current_project,
            open_file: None,
            file_tree,
            lt_results: Vec::new(),
            lt_running: false,
            selected_match: None,
            accent_bg,
            accent_fg,
            selection_bg,
            selection_fg,
            app_error: font_error,
        }
    }

    pub fn open_project(&mut self, path: PathBuf) {
        self.open_file = None;
        self.lt_results.clear();
        self.selected_match = None;

        self.dictionary = Dictionary::load(&path);
        self.file_tree = FileTree::load(&path);
        self.current_project = Some(path.clone());
        self.config.app.last_project = Some(path);

        if let Err(e) = self.config.save() {
            self.app_error = Some(e);
        }
    }

    pub fn load_file(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.open_file = Some(FileBuffer { path, content, modified: false });
                self.lt_results.clear();
                self.selected_match = None;
                self.app_error = None;
            }
            Err(e) => {
                self.app_error = Some(format!("Impossible de lire '{}' : {e}", path.display()));
            }
        }
    }

    fn run_analysis(&mut self) {
        let Some(ref buf) = self.open_file else { return };
        self.lt_running = true;
        match self.lt_client.check(&buf.content, &self.dictionary) {
            Ok(matches) => {
                self.lt_results = matches;
                self.app_error = None;
            }
            Err(e) => {
                self.app_error = Some(e);
            }
        }
        self.lt_running = false;
    }
}

pub fn apply_colors(ctx: &egui::Context, app_config: &crate::config::AppConfig) {
    let bg = app_config.app_color_bg.as_deref().and_then(parse_hex_color);
    let fg = app_config.app_color_fg.as_deref().and_then(parse_hex_color);

    if bg.is_none() && fg.is_none() {
        return;
    }

    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.style_mut_of(theme, |s| {
            if let Some(c) = bg {
                s.visuals.panel_fill = c;
                s.visuals.window_fill = c;
            }
            if let Some(c) = fg {
                s.visuals.override_text_color = Some(c);
            }
        });
    }
}

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

/// En-tête de panneau pleine largeur avec fond coloré.
/// `trailing_btn` : label du bouton affiché à droite ; retourne true si cliqué.
fn panel_header(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    accent_bg: Option<egui::Color32>,
    accent_fg: Option<egui::Color32>,
    trailing_btn: Option<&str>,
) -> bool {
    let bg = accent_bg.unwrap_or_else(|| {
        if ui.visuals().dark_mode {
            egui::Color32::from_rgb(35, 38, 46)
        } else {
            egui::Color32::from_rgb(215, 220, 232)
        }
    });
    let text_color = accent_fg.unwrap_or_else(|| contrasting_text_color(bg));

    let clip = ui.clip_rect();
    let bg_slot = ui.painter().add(egui::Shape::Noop);

    let mut btn_clicked = false;

    egui::Frame::new()
        .inner_margin(egui::Margin { left: 10, right: 10, top: 7, bottom: 7 })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{icon}  {title}")).strong().color(text_color));
                if let Some(lbl) = trailing_btn {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(
                            egui::RichText::new(lbl).color(text_color)
                        ).frame(false)).clicked() {
                            btn_clicked = true;
                        }
                    });
                }
            });
        });

    let full_rect = egui::Rect::from_min_max(
        egui::pos2(clip.min.x, clip.min.y),
        egui::pos2(clip.max.x, ui.cursor().min.y),
    );
    ui.painter().set(bg_slot, egui::Shape::rect_filled(full_rect, 0.0, bg));

    ui.add_space(2.0);
    btn_clicked
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

fn open_folder_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Ouvrir un projet")
        .pick_folder()
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Ctrl+O
        if ui.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.ctrl) {
            if let Some(path) = open_folder_dialog() {
                self.open_project(path);
            }
        }

        // Barre de statut
        if let Some(ref err) = self.app_error.clone() {
            egui::Panel::bottom("status_bar").show(ui, |ui| {
                ui.colored_label(egui::Color32::RED, format!("⬤  {err}"));
            });
        }

        let (abg, afg) = (self.accent_bg, self.accent_fg);

        // Panneau gauche — explorateur
        egui::Panel::left("file_tree")
            .default_size(200.0)
            .show(ui, |ui| {
                let open_clicked = panel_header(ui, "󰙅", "Explorer", abg, afg, Some("󰉗"));
                if open_clicked {
                    if let Some(path) = open_folder_dialog() {
                        self.open_project(path);
                    }
                }

                if self.current_project.is_some() {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let open_path = self.open_file.as_ref().map(|f| f.path.as_path());
                        if let Some(path) = self.file_tree.show(ui, open_path, self.selection_bg, self.selection_fg) {
                            self.load_file(path);
                        }
                    });
                }
            });

        // Panneau droit — LanguageTool
        egui::Panel::right("lt_panel")
            .default_size(260.0)
            .show(ui, |ui| {
                panel_header(ui, "󰓆", "LanguageTool", abg, afg, None);
                let action = super::lt_panel::show(
                    ui,
                    &self.lt_status,
                    &self.lt_results,
                    self.lt_running,
                );
                if matches!(action, super::lt_panel::Action::Analyse) {
                    self.run_analysis();
                }
            });

        // Panneau central — éditeur
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(i) = super::editor::show(
                ui,
                self.open_file.as_mut(),
                &self.lt_results,
                self.config.app.line_height,
                self.selected_match,
                self.config.app.editor_max_width,
            ) {
                self.selected_match = Some(i);
            }
        });
    }
}
