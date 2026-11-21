use crate::config::Analyzer;
use crate::grammalecte::GcStatus;
use crate::ignored::Ignored;
use crate::languagetool::{LtMatch, LtStatus};

pub enum Action {
    None,
    Analyse,
    Select(usize),
    RemoveWord(String),
    Unignore(Ignored),
    Stop,
    SwitchAnalyzer(Analyzer),
}

#[allow(clippy::too_many_arguments)] // état d'affichage passé à plat, un seul appelant
pub fn show(
    ui: &mut egui::Ui,
    lt_status: &LtStatus,
    gc_status: &GcStatus,
    active_analyzer: &Analyzer,
    text: &str,
    matches: &mut [LtMatch],
    dictionary: &[String],
    ignored: Option<&[Ignored]>,
    selected: Option<usize>,
    selection_bg: Option<egui::Color32>,
    selection_fg: Option<egui::Color32>,
    running: bool,
) -> Action {
    ui.add_space(6.0);

    let mut action = Action::None;

    let status_dot = |connected: bool| -> egui::Color32 {
        if connected {
            egui::Color32::from_rgb(80, 200, 120)
        } else {
            egui::Color32::from_rgb(220, 80, 80)
        }
    };
    let status_tip = |connected: bool| if connected { "Connecté" } else { "Inaccessible" };

    // Toggle LT / Gc avec statut ● indépendant pour chaque analyseur.
    ui.horizontal(|ui| {
        let lt_conn = matches!(lt_status, LtStatus::Connected);
        ui.colored_label(status_dot(lt_conn), "●")
            .on_hover_text(format!("LanguageTool : {}", status_tip(lt_conn)));
        if ui
            .selectable_label(matches!(active_analyzer, Analyzer::LanguageTool), "LT")
            .on_hover_text("LanguageTool")
            .clicked()
            && !matches!(active_analyzer, Analyzer::LanguageTool)
        {
            action = Action::SwitchAnalyzer(Analyzer::LanguageTool);
        }

        ui.separator();

        let gc_conn = matches!(gc_status, GcStatus::Connected);
        ui.colored_label(status_dot(gc_conn), "●")
            .on_hover_text(format!("Grammalecte : {}", status_tip(gc_conn)));
        if ui
            .selectable_label(matches!(active_analyzer, Analyzer::Grammalecte), "Gc")
            .on_hover_text("Grammalecte")
            .clicked()
            && !matches!(active_analyzer, Analyzer::Grammalecte)
        {
            action = Action::SwitchAnalyzer(Analyzer::Grammalecte);
        }
    });

    let can_analyse = match active_analyzer {
        Analyzer::LanguageTool => matches!(lt_status, LtStatus::Connected),
        Analyzer::Grammalecte => matches!(gc_status, GcStatus::Connected),
    } && !running;

    ui.horizontal(|ui| {
        if ui
            .add_enabled(can_analyse, egui::Button::new("▶"))
            .on_hover_text("Analyser")
            .clicked()
        {
            action = Action::Analyse;
        }

        // Bouton stop — actif uniquement si une analyse tourne.
        if ui
            .add_enabled(running, egui::Button::new("■"))
            .on_hover_text("Arrêter")
            .clicked()
        {
            action = Action::Stop;
        }

        if running {
            ui.add(egui::Spinner::new());
            ui.label(egui::RichText::new("Analyse…").weak());
        }
    });

    ui.add_space(4.0);
    ui.separator();

    // Mots du dictionnaire du projet, retirables un par un.
    egui::CollapsingHeader::new(format!("Dictionnaire ({})", dictionary.len()))
        .id_salt("lt_dictionary")
        .show(ui, |ui| {
            if dictionary.is_empty() {
                ui.label(egui::RichText::new("Aucun mot").weak());
            }
            for w in dictionary {
                ui.horizontal(|ui| {
                    if ui
                        .small_button("✗")
                        .on_hover_text("Retirer du dictionnaire")
                        .clicked()
                    {
                        action = Action::RemoveWord(w.clone());
                    }
                    ui.label(w);
                });
            }
        });

    // Éléments ignorés du fichier ouvert, rétablissables un par un.
    if let Some(ignored) = ignored {
        egui::CollapsingHeader::new(format!("Ignorés ({})", ignored.len()))
            .id_salt("lt_ignored")
            .show(ui, |ui| {
                if ignored.is_empty() {
                    ui.label(egui::RichText::new("Aucun élément ignoré").weak());
                }
                for e in ignored {
                    ui.horizontal(|ui| {
                        if ui
                            .small_button("✗")
                            .on_hover_text("Ne plus ignorer")
                            .clicked()
                        {
                            action = Action::Unignore(e.clone());
                        }
                        ui.label(format!("\"{}\"", e.text))
                            .on_hover_text(&e.sentence);
                    });
                }
            });
    }
    ui.separator();

    let n = matches.iter().filter(|m| !m.hidden()).count();
    if n > 0 {
        ui.label(format!("⚠ {} problème{}", n, if n > 1 { "s" } else { "" }));
        ui.add_space(4.0);
    }

    // Liste collapsée : clic sur un item → le sélectionne et bascule son détail.
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, m) in matches.iter_mut().enumerate().filter(|(_, m)| !m.hidden()) {
                let word = m.word(text);
                let arrow = if m.expanded { "▾" } else { "▸" };
                let head = match m.replacements.first() {
                    Some(r) => format!("{arrow} \"{word}\" → {r}"),
                    None => format!("{arrow} \"{word}\""),
                };

                let is_selected = selected == Some(i);
                let clicked = ui
                    .scope(|ui| {
                        if is_selected {
                            if let Some(bg) = selection_bg {
                                ui.visuals_mut().selection.bg_fill = bg;
                            }
                            if let Some(fg) = selection_fg {
                                ui.visuals_mut().override_text_color = Some(fg);
                            }
                        }
                        ui.selectable_label(is_selected, head).clicked()
                    })
                    .inner;

                if clicked {
                    m.expanded = !m.expanded;
                    action = Action::Select(i);
                }

                if m.expanded {
                    ui.indent(i, |ui| {
                        ui.label(egui::RichText::new(&m.message).weak());
                        if m.replacements.len() > 1 {
                            ui.label(format!("Autres : {}", m.replacements[1..].join(", ")));
                        }
                    });
                }
            }
        });

    action
}
