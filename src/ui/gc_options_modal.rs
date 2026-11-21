use std::collections::HashMap;

struct OptionMeta {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    /// Valeur par défaut dans Grammalecte (sans aucune option passée).
    gc_default: bool,
    recommendation: &'static str,
}

static RECOMMENDED: &[OptionMeta] = &[
    OptionMeta {
        key: "redon2",
        label: "Répétitions (phrase)",
        description: "Détecte les répétitions de mots dans la même phrase.",
        gc_default: false,
        recommendation: "Activer — les répétitions dans la même phrase sont rarement intentionnelles ; peu de faux positifs.",
    },
    OptionMeta {
        key: "md",
        label: "Markdown",
        description: "Ignore la syntaxe Markdown (`**`, `#`, `[]()`…) pour éviter du bruit.",
        gc_default: false,
        recommendation: "Activer — correct édite du Markdown.",
    },
];

static SITUATIONAL: &[OptionMeta] = &[
    OptionMeta {
        key: "eepi",
        label: "Écriture épicène",
        description: "Vérifie l'usage du langage inclusif et de l'écriture non genrée.",
        gc_default: false,
        recommendation: "Activer pour documents officiels ou communication RH — irritant pour notes ou fiction.",
    },
    OptionMeta {
        key: "redon1",
        label: "Répétitions (paragraphe)",
        description: "Détecte les répétitions de mots entre les phrases d'un même paragraphe.",
        gc_default: false,
        recommendation: "Activer pour textes formels — les répétitions entre phrases peuvent être stylistiques.",
    },
    OptionMeta {
        key: "poncfin",
        label: "Ponctuation finale",
        description: "Vérifie que chaque phrase se termine par une ponctuation.",
        gc_default: false,
        recommendation: "Activer pour rapports et emails — bruyant pour prise de notes fragmentée.",
    },
    OptionMeta {
        key: "html",
        label: "HTML",
        description: "Ignore les balises HTML dans le texte.",
        gc_default: false,
        recommendation: "Activer si le projet contient des fichiers HTML.",
    },
    OptionMeta {
        key: "latex",
        label: "LaTeX",
        description: "Ignore les commandes LaTeX dans le texte.",
        gc_default: false,
        recommendation: "Activer si le projet contient des fichiers LaTeX.",
    },
    OptionMeta {
        key: "liga",
        label: "Ligatures",
        description: "Vérifie les ligatures typographiques (fi, fl…).",
        gc_default: false,
        recommendation: "Activer pour documents typographiques soignés.",
    },
];

pub enum Action {
    None,
    Toggle(String),
    Close,
}

pub fn show(ctx: &egui::Context, options: &HashMap<String, bool>) -> Action {
    let mut action = Action::None;

    let res = egui::Modal::new(egui::Id::unique("gc_options_modal")).show(ctx, |ui| {
        ui.set_width(500.0);
        ui.heading("Options Grammalecte");
        ui.add_space(8.0);

        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
            section(ui, "Recommandées", RECOMMENDED, options, &mut action);
            ui.add_space(8.0);
            section(ui, "Situationnelles", SITUATIONAL, options, &mut action);
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        if ui.button("Fermer").clicked() {
            action = Action::Close;
        }
    });

    let escape = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    if matches!(action, Action::None) && (escape || res.backdrop_response.clicked()) {
        Action::Close
    } else {
        action
    }
}

fn section(
    ui: &mut egui::Ui,
    title: &str,
    opts: &[OptionMeta],
    options: &HashMap<String, bool>,
    action: &mut Action,
) {
    ui.label(egui::RichText::new(title).strong());
    ui.add_space(4.0);

    for opt in opts {
        let current = options.get(opt.key).copied().unwrap_or(opt.gc_default);
        let mut checked = current;

        ui.horizontal(|ui| {
            if ui.checkbox(&mut checked, opt.label).changed() {
                *action = Action::Toggle(opt.key.to_string());
            }
            let default_tag = if opt.gc_default { "défaut GC : ✓" } else { "défaut GC : ✗" };
            ui.label(egui::RichText::new(default_tag).weak().small());
        });

        ui.indent(opt.key, |ui| {
            ui.label(egui::RichText::new(opt.description).weak());
            ui.label(egui::RichText::new(format!("→ {}", opt.recommendation)).italics().small());
        });

        ui.add_space(4.0);
    }
}
