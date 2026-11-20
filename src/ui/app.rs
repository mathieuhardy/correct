use crate::config::Config;
use crate::dictionary::Dictionary;
use crate::languagetool::{LtClient, LtMatch, LtStatus};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use super::file_tree::FileTree;
use crate::watcher::{DictWatcher, FileWatcher};

pub struct App {
    pub config: Config,
    pub dictionary: Dictionary,
    pub dict_watcher: Option<DictWatcher>,
    /// Watch du fichier ouvert ; remplacé (et l'ancien libéré par drop) à chaque ouverture.
    pub file_watcher: Option<FileWatcher>,
    ctx: egui::Context,
    pub lt_client: LtClient,
    pub lt_status: LtStatus,
    pub current_project: Option<PathBuf>,
    pub open_file: Option<FileBuffer>,
    pub file_tree: FileTree,
    pub lt_results: Vec<LtMatch>,
    /// Analyse LT en cours sur un thread (None = aucune).
    analysis: Option<PendingAnalysis>,
    pub selected_match: Option<usize>,
    pub popup_open: bool,
    pub popup_opened_at: f64,
    pub popup_armed: bool,
    pub popup_colors: super::popup::PopupColors,
    pub accent_bg: Option<egui::Color32>,
    pub accent_fg: Option<egui::Color32>,
    pub selection_bg: Option<egui::Color32>,
    pub selection_fg: Option<egui::Color32>,
    pub app_error: Option<String>,
}

/// Analyse lancée sur un thread. Le résultat n'est appliqué que si le fichier ouvert
/// et son contenu sont toujours ceux analysés ; l'abandonner (drop) l'annule.
struct PendingAnalysis {
    path: PathBuf,
    text: String,
    rx: Receiver<Result<Vec<LtMatch>, String>>,
}

pub struct FileBuffer {
    pub path: PathBuf,
    pub content: String,
    pub modified: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let (config, config_error) = Config::load();
        let mut errors: Vec<String> = config_error.into_iter().collect();
        errors.extend(invalid_colors(&config.app));

        // Thème sombre par défaut — l'utilisateur contrôle les couleurs via la config.
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        apply_colors(&cc.egui_ctx, &config.app);
        let font_error = apply_font(&cc.egui_ctx, &config.app);

        errors.extend(font_error);

        let (lt_client, lt_status, lt_error) = LtClient::init(&config.languagetool);
        errors.extend(lt_error);

        let accent_bg = config
            .app
            .accent_color_bg
            .as_deref()
            .and_then(parse_hex_color);
        let accent_fg = config
            .app
            .accent_color_fg
            .as_deref()
            .and_then(parse_hex_color);
        let selection_bg = config
            .app
            .selection_color_bg
            .as_deref()
            .and_then(parse_hex_color);
        let selection_fg = config
            .app
            .selection_color_fg
            .as_deref()
            .and_then(parse_hex_color);

        let color_or = |c: &Option<String>, default: egui::Color32| {
            c.as_deref().and_then(parse_hex_color).unwrap_or(default)
        };
        let dark = egui::Color32::from_rgb(30, 30, 30);
        let popup_colors = super::popup::PopupColors {
            accept: (
                color_or(
                    &config.app.accept_color_bg,
                    egui::Color32::from_rgb(102, 187, 106),
                ),
                color_or(&config.app.accept_color_fg, dark),
            ),
            ignore: (
                color_or(
                    &config.app.ignore_color_bg,
                    egui::Color32::from_rgb(229, 115, 115),
                ),
                color_or(&config.app.ignore_color_fg, dark),
            ),
            dictionary: (
                color_or(
                    &config.app.dictionary_color_bg,
                    egui::Color32::from_rgb(100, 181, 246),
                ),
                color_or(&config.app.dictionary_color_fg, dark),
            ),
        };

        // `last_project = ""` signifie « aucun projet » ; un chemin disparu est signalé.
        let last_project = config
            .app
            .last_project
            .clone()
            .filter(|p| !p.as_os_str().is_empty());
        if let Some(p) = last_project.as_ref().filter(|p| !p.is_dir()) {
            errors.push(format!("Dernier projet introuvable : {}", p.display()));
        }
        let last_project = last_project.filter(|p| p.is_dir());

        let mut app = Self {
            config,
            dictionary: Dictionary::default(),
            dict_watcher: None,
            file_watcher: None,
            ctx: cc.egui_ctx.clone(),
            lt_client,
            lt_status,
            current_project: None,
            open_file: None,
            file_tree: FileTree::empty(),
            lt_results: Vec::new(),
            analysis: None,
            selected_match: None,
            popup_open: false,
            popup_opened_at: 0.0,
            popup_armed: false,
            popup_colors,
            accent_bg,
            accent_fg,
            selection_bg,
            selection_fg,
            app_error: None,
        };
        if let Some(path) = last_project {
            app.load_project(path);
        }
        // Toutes les erreurs de démarrage sont affichées ensemble dans la barre de statut.
        errors.extend(app.app_error.take());
        app.app_error = (!errors.is_empty()).then(|| errors.join("  •  "));
        app
    }

    /// Charge le dictionnaire, l'arbre et le watch du dictionnaire d'un projet.
    fn load_project(&mut self, path: PathBuf) {
        self.reload_dictionary(&path);
        self.dict_watcher = match DictWatcher::new(&path, self.ctx.clone()) {
            Ok(w) => Some(w),
            Err(e) => {
                self.app_error = Some(e);
                None
            }
        };
        self.file_tree = FileTree::load(&path);
        self.current_project = Some(path);
    }

    /// (Re)lit le dictionnaire. S'il est invalide : erreur affichée et dictionnaire verrouillé
    /// (aucune écriture possible) pour ne pas écraser le fichier existant.
    fn reload_dictionary(&mut self, project: &std::path::Path) {
        match Dictionary::load(project) {
            Ok(d) => {
                if self.dictionary.is_locked() {
                    self.app_error = None;
                }
                self.dictionary = d;
            }
            Err(e) => {
                self.app_error = Some(e);
                self.dictionary = Dictionary::locked();
            }
        }
        self.refilter();
    }

    /// Recalcule `in_dictionary` pour chaque résultat ; ferme le popup si son erreur disparaît.
    fn refilter(&mut self) {
        let Some(buf) = self.open_file.as_ref() else {
            return;
        };
        for m in &mut self.lt_results {
            m.in_dictionary = self.dictionary.is_exception(&m.word(&buf.content));
        }
        if self
            .selected_match
            .and_then(|i| self.lt_results.get(i))
            .is_some_and(|m| m.hidden())
        {
            self.close_popup();
        }
    }

    pub fn open_project(&mut self, path: PathBuf) {
        self.open_file = None;
        self.file_watcher = None;
        self.analysis = None;
        self.lt_results.clear();
        self.selected_match = None;
        self.popup_open = false;

        self.load_project(path.clone());
        self.config.app.last_project = Some(path);

        if let Err(e) = self.config.save() {
            self.app_error = Some(e);
        }
    }

    pub fn load_file(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.lt_results.clear();
                self.selected_match = None;
                self.popup_open = false;
                self.app_error = None;
                self.file_watcher = match FileWatcher::new(&path, self.ctx.clone()) {
                    Ok(w) => Some(w),
                    Err(e) => {
                        self.app_error = Some(e);
                        None
                    }
                };
                self.open_file = Some(FileBuffer {
                    path,
                    content,
                    modified: false,
                });
                // Analyse automatique à l'ouverture, en arrière-plan.
                self.start_analysis();
            }
            Err(e) => {
                self.app_error = Some(format!("Impossible de lire '{}' : {e}", path.display()));
            }
        }
    }

    /// Fichier ouvert modifié sur disque : recharge, vide le panneau et relance l'analyse.
    /// Nos propres écritures (suggestion acceptée) laissent un contenu identique : ignorées.
    fn reload_open_file(&mut self) {
        let Some(buf) = self.open_file.as_mut() else {
            return;
        };
        let content = match std::fs::read_to_string(&buf.path) {
            Ok(c) => c,
            Err(e) => {
                self.app_error = Some(format!(
                    "Impossible de relire '{}' : {e}",
                    buf.path.display()
                ));
                return;
            }
        };
        if content == buf.content {
            return;
        }
        buf.content = content;
        self.start_analysis();
    }

    /// Lance l'analyse du fichier ouvert sur un thread (remplace toute analyse en cours).
    /// Vide le panneau ; sans connexion LT, se contente d'annuler l'analyse précédente.
    fn start_analysis(&mut self) {
        self.lt_results.clear();
        self.close_popup();
        self.analysis = None;

        let Some(buf) = self.open_file.as_ref() else {
            return;
        };
        if self.lt_status != LtStatus::Connected {
            return;
        }

        let (tx, rx) = channel();
        let checker = self.lt_client.checker();
        let ctx = self.ctx.clone();
        let text = buf.content.clone();
        let thread_text = text.clone();
        std::thread::spawn(move || {
            // Réception abandonnée (annulation, autre fichier) : envoi ignoré.
            let _ = tx.send(checker.check(&thread_text));
            ctx.request_repaint();
        });

        self.analysis = Some(PendingAnalysis {
            path: buf.path.clone(),
            text,
            rx,
        });
    }

    /// Applique le résultat de l'analyse s'il est arrivé.
    fn poll_analysis(&mut self) {
        let Some(pending) = self.analysis.as_ref() else {
            return;
        };
        let result = match pending.rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.analysis = None;
                return;
            }
        };
        let Some(pending) = self.analysis.take() else {
            return;
        };

        let Some(buf) = self.open_file.as_ref() else {
            return;
        };
        if buf.path != pending.path {
            return;
        }
        if buf.content != pending.text {
            // Le texte a changé pendant l'analyse : offsets périmés, on relance.
            self.start_analysis();
            return;
        }
        match result {
            Ok(matches) => {
                self.lt_results = matches;
                self.app_error = None;
                self.refilter();
            }
            Err(e) => self.app_error = Some(e),
        }
    }

    fn open_popup(&mut self, i: usize) {
        self.selected_match = Some(i);
        self.popup_open = true;
        self.popup_armed = false;
    }

    fn close_popup(&mut self) {
        self.selected_match = None;
        self.popup_open = false;
    }

    fn show_popup(&mut self, ctx: &egui::Context) {
        if !self.popup_open {
            return;
        }
        let (Some(i), Some(buf)) = (self.selected_match, self.open_file.as_ref()) else {
            return;
        };
        let Some(m) = self.lt_results.get(i) else {
            return;
        };

        if ctx.input(|inp| inp.pointer.any_pressed() && inp.time > self.popup_opened_at) {
            self.popup_armed = true;
        }

        match super::popup::show(
            ctx,
            &m.word(&buf.content),
            m,
            &self.popup_colors,
            self.popup_armed,
        ) {
            super::popup::Action::None => {}
            super::popup::Action::Close => self.close_popup(),
            super::popup::Action::Accept(r) => self.accept_suggestion(i, &r),
            super::popup::Action::Ignore => {
                self.lt_results[i].ignored = true;
                self.close_popup();
            }
            super::popup::Action::AddToDictionary => self.add_to_dictionary(i),
        }
    }

    /// Remplace le texte de l'erreur, décale les offsets des erreurs suivantes,
    /// supprime celles qui chevauchaient la zone remplacée, puis écrit sur disque.
    fn accept_suggestion(&mut self, i: usize, replacement: &str) {
        self.close_popup();
        let Some(buf) = self.open_file.as_mut() else {
            return;
        };

        let m = self.lt_results.remove(i);
        let byte_at = |n: usize| {
            buf.content
                .char_indices()
                .nth(n)
                .map_or(buf.content.len(), |(b, _)| b)
        };
        let range = byte_at(m.offset)..byte_at(m.offset + m.length);
        buf.content.replace_range(range, replacement);

        let end = m.offset + m.length;
        let new_end = m.offset + replacement.chars().count();
        self.lt_results.retain_mut(|o| {
            if o.offset >= end {
                o.offset = o.offset - end + new_end;
                true
            } else {
                o.offset + o.length <= m.offset
            }
        });

        match std::fs::write(&buf.path, &buf.content) {
            Ok(()) => buf.modified = false,
            Err(e) => {
                self.app_error = Some(format!(
                    "Impossible d'écrire '{}' : {e}",
                    buf.path.display()
                ))
            }
        }
    }

    /// Ajoute le mot au dictionnaire du projet et retire toutes ses occurrences des résultats.
    fn add_to_dictionary(&mut self, i: usize) {
        self.close_popup();
        let (Some(project), Some(buf)) = (self.current_project.as_ref(), self.open_file.as_ref())
        else {
            return;
        };

        let word = self.lt_results[i].word(&buf.content);
        if let Err(e) = self.dictionary.add_word(&word, project) {
            self.app_error = Some(e);
            return;
        }
        self.refilter();
    }

    /// Retire un mot du dictionnaire ; ses occurrences réapparaissent dans les résultats.
    fn remove_from_dictionary(&mut self, word: &str) {
        let Some(project) = self.current_project.as_ref() else {
            return;
        };
        if let Err(e) = self.dictionary.remove_word(word, project) {
            self.app_error = Some(e);
            return;
        }
        self.refilter();
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

    if let Some(ref name_or_path) = app_config.font_family
        && !name_or_path.is_empty()
    {
        let bytes = if std::path::Path::new(name_or_path).is_file() {
            std::fs::read(name_or_path)
                .map_err(|e| format!("Police '{name_or_path}' illisible : {e}"))
        } else {
            font_bytes_by_name(name_or_path)
                .ok_or_else(|| format!("Police '{name_or_path}' introuvable sur le système"))
        };
        match bytes {
            Ok(b) => {
                fonts
                    .font_data
                    .insert("custom".to_owned(), egui::FontData::from_owned(b).into());
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts
                        .families
                        .entry(family)
                        .or_default()
                        .insert(0, "custom".to_owned());
                }
            }
            Err(e) => error = Some(e),
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
        .inner_margin(egui::Margin {
            left: 10,
            right: 10,
            top: 7,
            bottom: 7,
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{icon}  {title}"))
                        .strong()
                        .color(text_color),
                );
                if let Some(lbl) = trailing_btn {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(lbl).color(text_color))
                                    .frame(false),
                            )
                            .clicked()
                        {
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
    ui.painter()
        .set(bg_slot, egui::Shape::rect_filled(full_rect, 0.0, bg));

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

/// Couleurs renseignées dans la config mais qui ne sont pas au format `#RRGGBB`.
fn invalid_colors(app: &crate::config::AppConfig) -> Vec<String> {
    [
        ("accent_color_bg", &app.accent_color_bg),
        ("accent_color_fg", &app.accent_color_fg),
        ("app_color_bg", &app.app_color_bg),
        ("app_color_fg", &app.app_color_fg),
        ("selection_color_bg", &app.selection_color_bg),
        ("selection_color_fg", &app.selection_color_fg),
        ("accept_color_bg", &app.accept_color_bg),
        ("accept_color_fg", &app.accept_color_fg),
        ("ignore_color_bg", &app.ignore_color_bg),
        ("ignore_color_fg", &app.ignore_color_fg),
        ("dictionary_color_bg", &app.dictionary_color_bg),
        ("dictionary_color_fg", &app.dictionary_color_fg),
    ]
    .into_iter()
    .filter_map(|(key, value)| {
        let v = value.as_deref().filter(|v| !v.is_empty())?;
        parse_hex_color(v)
            .is_none()
            .then(|| format!("Couleur invalide pour {key} : \"{v}\" (attendu #RRGGBB)"))
    })
    .collect()
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
        if ui.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.ctrl)
            && let Some(path) = open_folder_dialog()
        {
            self.open_project(path);
        }

        // Résultat d'analyse arrivé depuis le thread
        self.poll_analysis();

        // Fichier ouvert modifié sur disque
        if self.file_watcher.as_mut().is_some_and(|w| w.poll()) {
            self.reload_open_file();
        }

        // Dictionnaire modifié sur disque (hors application ou par nous-mêmes)
        if self.dict_watcher.as_mut().is_some_and(|w| w.poll())
            && let Some(project) = self.current_project.clone()
        {
            self.reload_dictionary(&project);
        }

        // Barre de statut
        if let Some(ref err) = self.app_error.clone() {
            // Texte 3 pt sous la taille configurée, non gras.
            let size = (self.config.app.font_size - 3.0).max(8.0);
            egui::Panel::bottom("status_bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("⬤")
                            .size(size)
                            .color(egui::Color32::RED),
                    );
                    ui.label(
                        egui::RichText::new(err)
                            .size(size)
                            .color(egui::Color32::RED),
                    );
                });
            });
        }

        let (abg, afg) = (self.accent_bg, self.accent_fg);

        // Panneau gauche — explorateur
        egui::Panel::left("file_tree")
            .default_size(200.0)
            .show(ui, |ui| {
                let open_clicked = panel_header(ui, "󰙅", "Explorer", abg, afg, Some("󰉗"));
                if open_clicked && let Some(path) = open_folder_dialog() {
                    self.open_project(path);
                }

                if self.current_project.is_some() {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let open_path = self.open_file.as_ref().map(|f| f.path.as_path());
                        if let Some(path) =
                            self.file_tree
                                .show(ui, open_path, self.selection_bg, self.selection_fg)
                        {
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
                let text = self
                    .open_file
                    .as_ref()
                    .map(|f| f.content.as_str())
                    .unwrap_or("");
                let action = super::lt_panel::show(
                    ui,
                    &self.lt_status,
                    text,
                    &mut self.lt_results,
                    &self.dictionary.words,
                    self.selected_match,
                    self.selection_bg,
                    self.selection_fg,
                    self.analysis.is_some(),
                );
                match action {
                    super::lt_panel::Action::Analyse => self.start_analysis(),
                    super::lt_panel::Action::Stop => self.analysis = None,
                    super::lt_panel::Action::Select(i) => self.selected_match = Some(i),
                    super::lt_panel::Action::RemoveWord(w) => self.remove_from_dictionary(&w),
                    super::lt_panel::Action::None => {}
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
                self.popup_opened_at = ui.input(|inp| inp.time);
                self.open_popup(i);
            }
        });

        let ctx = ui.ctx().clone();
        self.show_popup(&ctx);
    }
}
