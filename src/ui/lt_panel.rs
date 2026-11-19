use crate::languagetool::{LtMatch, LtStatus};

pub enum Action {
    None,
    Analyse,
}

pub fn show(ui: &mut egui::Ui, status: &LtStatus, matches: &[LtMatch], running: bool) -> Action {
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
        ui.add_enabled(running, egui::Button::new("⏹"))
            .on_hover_text("Arrêter");
    });

    ui.add_space(4.0);
    ui.separator();

    if !matches.is_empty() {
        let n = matches.len();
        ui.label(format!("⚠ {} problème{}", n, if n > 1 { "s" } else { "" }));
    }

    action
}
