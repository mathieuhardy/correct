use crate::languagetool::{LtMatch, LtStatus};

pub enum Action {
    None,
    Analyse,
    Select(usize),
}

pub fn show(
    ui: &mut egui::Ui,
    status: &LtStatus,
    text: &str,
    matches: &mut [LtMatch],
    selected: Option<usize>,
    selection_bg: Option<egui::Color32>,
    selection_fg: Option<egui::Color32>,
    running: bool,
) -> Action {
    let (dot_color, dot_tip) = match status {
        LtStatus::Connected   => (egui::Color32::from_rgb(80, 200, 120), "Connecté"),
        LtStatus::Connecting  => (egui::Color32::YELLOW,                 "Connexion…"),
        LtStatus::Unreachable => (egui::Color32::from_rgb(220, 80, 80),  "Inaccessible"),
    };

    ui.add_space(6.0);

    let mut action = Action::None;
    let can_analyse = matches!(status, LtStatus::Connected) && !running;

    ui.horizontal(|ui| {
        ui.colored_label(dot_color, "●").on_hover_text(dot_tip);
        ui.separator();

        if ui.add_enabled(can_analyse, egui::Button::new("▶"))
            .on_hover_text("Analyser")
            .clicked()
        {
            action = Action::Analyse;
        }

        // Bouton stop — actif uniquement si une analyse tourne (async, Phase 5).
        ui.add_enabled(running, egui::Button::new("■"))
            .on_hover_text("Arrêter");
    });

    ui.add_space(4.0);
    ui.separator();

    if !matches.is_empty() {
        let n = matches.len();
        ui.label(format!("⚠ {} problème{}", n, if n > 1 { "s" } else { "" }));
        ui.add_space(4.0);
    }

    // Liste collapsée : clic sur un item → le sélectionne et bascule son détail.
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for (i, m) in matches.iter_mut().enumerate().filter(|(_, m)| !m.ignored) {
            let word: String = text.chars().skip(m.offset).take(m.length).collect();
            let arrow = if m.expanded { "▾" } else { "▸" };
            let head = match m.replacements.first() {
                Some(r) => format!("{arrow} \"{word}\" → {r}"),
                None    => format!("{arrow} \"{word}\""),
            };

            let is_selected = selected == Some(i);
            let clicked = ui.scope(|ui| {
                if is_selected {
                    if let Some(bg) = selection_bg {
                        ui.visuals_mut().selection.bg_fill = bg;
                    }
                    if let Some(fg) = selection_fg {
                        ui.visuals_mut().override_text_color = Some(fg);
                    }
                }
                ui.selectable_label(is_selected, head).clicked()
            }).inner;

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
