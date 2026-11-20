use crate::languagetool::LtMatch;

/// Couleurs (fond, texte) appliquées au survol des boutons du popup.
pub struct PopupColors {
    pub accept: (egui::Color32, egui::Color32),
    pub ignore: (egui::Color32, egui::Color32),
    pub dictionary: (egui::Color32, egui::Color32),
}

pub enum Action {
    None,
    Close,
    Accept(String),
    Ignore,
    AddToDictionary,
}

/// Popup modal sur une erreur : accepter une suggestion, ignorer, ajouter au dictionnaire.
/// `backdrop_armed` : un appui a eu lieu après l'ouverture ; sinon un clic sur le fond
/// est ignoré (c'est le clic qui a ouvert le popup, revu lors d'une passe rejouée par egui).
pub fn show(
    ctx: &egui::Context,
    word: &str,
    m: &LtMatch,
    colors: &PopupColors,
    backdrop_armed: bool,
) -> Action {
    let res = egui::Modal::new(egui::Id::unique("lt_popup")).show(ctx, |ui| {
        let mut action = Action::None;
        ui.set_width(320.0);

        ui.label(egui::RichText::new(format!("\"{word}\"")).strong());
        ui.label(egui::RichText::new(&m.message).weak());

        if !m.replacements.is_empty() {
            ui.add_space(ui.text_style_height(&egui::TextStyle::Body));
            ui.label("Suggestions :");
            ui.horizontal_wrapped(|ui| {
                for r in m.replacements.iter().take(5) {
                    if hover_button(ui, format!("✓ {r}"), colors.accept).clicked() {
                        action = Action::Accept(r.clone());
                    }
                }
            });
        }

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if hover_button(ui, "✗ Ignorer", colors.ignore).clicked() {
                action = Action::Ignore;
            }
            // Seules les fautes d'orthographe vont au dictionnaire (pas les erreurs de grammaire).
            if m.misspelling
                && hover_button(ui, "+ Ajouter au dictionnaire", colors.dictionary).clicked()
            {
                action = Action::AddToDictionary;
            }
        });
        action
    });

    let backdrop_click = backdrop_armed && res.backdrop_response.clicked();
    let escape = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

    match res.inner {
        Action::None if backdrop_click || escape => Action::Close,
        a => a,
    }
}

/// Bouton dont le fond et le texte prennent `(bg, fg)` au survol / clic.
fn hover_button(
    ui: &mut egui::Ui,
    label: impl Into<egui::WidgetText>,
    (bg, fg): (egui::Color32, egui::Color32),
) -> egui::Response {
    ui.scope(|ui| {
        let normal = ui.visuals().text_color();
        let v = ui.visuals_mut();
        // override_text_color (app_color_fg) primerait sur fg_stroke : on le reporte sur l'état inactif.
        v.override_text_color = None;
        v.widgets.inactive.fg_stroke.color = normal;
        for w in [&mut v.widgets.hovered, &mut v.widgets.active] {
            w.weak_bg_fill = bg;
            w.bg_fill = bg;
            w.fg_stroke.color = fg;
        }
        // Pas de wrap interne : un bouton trop long passe à la ligne au lieu d'être écrasé.
        ui.add(egui::Button::new(label).wrap_mode(egui::TextWrapMode::Extend))
    })
    .inner
}
