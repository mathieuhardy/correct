use std::collections::HashMap;

struct OptionMeta {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    /// Valeur par défaut dans Grammalecte (sans aucune option passée).
    gc_default: bool,
    recommendation: &'static str,
}

/// Options actives par défaut dans GC — à exposer pour permettre leur désactivation.
static ESSENTIAL: &[OptionMeta] = &[
    OptionMeta { key: "apos",  label: "Apostrophe typographique",          description: "Remplace l'apostrophe droite par l'apostrophe courbe.",                             gc_default: true,  recommendation: "Fondamental pour le français écrit." },
    OptionMeta { key: "conf",  label: "Confusions et faux-amis",           description: "Détecte a/à, ou/où, se/ce, et autres confusions courantes.",                      gc_default: true,  recommendation: "Très fréquent, peu de faux positifs." },
    OptionMeta { key: "conj",  label: "Conjugaisons",                      description: "Vérifie les formes verbales conjuguées.",                                          gc_default: true,  recommendation: "Essentiel." },
    OptionMeta { key: "eleu",  label: "Élisions et euphonies",             description: "Détecte « le heure » → « l'heure », « de ami » → « d'ami »…",                    gc_default: true,  recommendation: "Fondamental." },
    OptionMeta { key: "esp",   label: "Espaces surnuméraires",             description: "Signale les espaces doubles ou superflus.",                                        gc_default: true,  recommendation: "Utile, peu de faux positifs." },
    OptionMeta { key: "gn",    label: "Accords genre et nombre",           description: "Vérifie l'accord en genre et en nombre des groupes nominaux.",                    gc_default: true,  recommendation: "Essentiel." },
    OptionMeta { key: "imp",   label: "Impératif",                         description: "Vérifie les formes à l'impératif.",                                               gc_default: true,  recommendation: "Utile." },
    OptionMeta { key: "infi",  label: "Infinitif",                         description: "Vérifie les formes à l'infinitif (infinitif vs participe passé…).",               gc_default: true,  recommendation: "Utile." },
    OptionMeta { key: "inte",  label: "Interrogatif",                      description: "Vérifie la syntaxe des tournures interrogatives.",                                 gc_default: true,  recommendation: "Utile." },
    OptionMeta { key: "loc",   label: "Locutions",                         description: "Vérifie l'orthographe des locutions figées.",                                     gc_default: true,  recommendation: "Utile." },
    OptionMeta { key: "maj",   label: "Majuscules",                        description: "Vérifie l'usage des majuscules (noms propres, débuts de phrase…).",               gc_default: true,  recommendation: "Utile." },
    OptionMeta { key: "nbsp",  label: "Espaces insécables",                description: "Vérifie la présence d'espaces insécables avant ? ! ; : « ».",                    gc_default: true,  recommendation: "Fondamental pour la typographie française." },
    OptionMeta { key: "nf",    label: "Normes françaises",                 description: "Applique les recommandations orthotypographiques françaises.",                    gc_default: true,  recommendation: "Utile pour l'écriture formelle." },
    OptionMeta { key: "num",   label: "Nombres",                           description: "Vérifie la graphie des nombres (accord, espacement…).",                           gc_default: true,  recommendation: "Utile." },
    OptionMeta { key: "pleo",  label: "Pléonasmes",                        description: "Détecte « monter en haut », « prévoir à l'avance »…",                            gc_default: true,  recommendation: "Intéressant, peu bruyant." },
    OptionMeta { key: "ppas",  label: "Participes passés et adjectifs",    description: "Vérifie l'accord des participes passés.",                                         gc_default: true,  recommendation: "Essentiel." },
    OptionMeta { key: "tu",    label: "Traits d'union et soudures",        description: "Vérifie la présence ou l'absence de trait d'union.",                             gc_default: true,  recommendation: "Important." },
    OptionMeta { key: "typo",  label: "Signes typographiques",             description: "Vérifie guillemets, tirets cadratins, points de suspension…",                   gc_default: true,  recommendation: "Fondamental." },
    OptionMeta { key: "unit",  label: "Espaces avant unités",              description: "Vérifie l'espace insécable entre nombre et unité de mesure.",                    gc_default: true,  recommendation: "Utile." },
    OptionMeta { key: "virg",  label: "Virgules",                          description: "Vérifie l'usage des virgules.",                                                   gc_default: true,  recommendation: "Essentiel." },
    OptionMeta { key: "vmode", label: "Modes verbaux",                     description: "Vérifie le choix du mode verbal (indicatif, subjonctif, conditionnel…).",        gc_default: true,  recommendation: "Essentiel." },
];

/// Options désactivées par défaut dans GC mais recommandées dans correct.
static RECOMMENDED: &[OptionMeta] = &[
    OptionMeta { key: "redon2", label: "Répétitions (même phrase)",       description: "Détecte les répétitions de mots dans la même phrase.",                            gc_default: false, recommendation: "Activer — rarement intentionnelles ; peu de faux positifs." },
    OptionMeta { key: "md",     label: "Markdown",                        description: "Ignore la syntaxe Markdown (**gras**, # titres, [lien]()…) pour éviter du bruit.", gc_default: false, recommendation: "Activer — correct édite du Markdown." },
];

/// Options désactivées par défaut, utiles selon le type de projet.
static SITUATIONAL: &[OptionMeta] = &[
    OptionMeta { key: "eepi",   label: "Écriture épicène",                description: "Vérifie l'usage du langage inclusif et de l'écriture non genrée.",              gc_default: false, recommendation: "Activer pour documents officiels ou communication RH — irritant pour notes ou fiction." },
    OptionMeta { key: "redon1", label: "Répétitions (entre phrases)",     description: "Détecte les répétitions de mots entre les phrases d'un même paragraphe.",        gc_default: false, recommendation: "Activer pour textes formels — les répétitions peuvent être stylistiques." },
    OptionMeta { key: "poncfin",label: "Ponctuation finale",              description: "Vérifie que chaque phrase se termine par une ponctuation.",                       gc_default: false, recommendation: "Activer pour rapports et emails — bruyant pour prise de notes fragmentée." },
    OptionMeta { key: "html",   label: "HTML",                            description: "Ignore les balises HTML dans le texte.",                                          gc_default: false, recommendation: "Activer si le projet contient des fichiers HTML." },
    OptionMeta { key: "latex",  label: "LaTeX",                           description: "Ignore les commandes LaTeX dans le texte.",                                       gc_default: false, recommendation: "Activer si le projet contient des fichiers LaTeX." },
    OptionMeta { key: "liga",   label: "Ligatures",                       description: "Vérifie les ligatures typographiques (fi, fl…).",                                gc_default: false, recommendation: "Activer pour documents typographiques soignés." },
];

pub enum Action {
    None,
    Toggle(String),
    Close,
}

pub fn show(ctx: &egui::Context, options: &HashMap<String, bool>) -> Action {
    let mut action = Action::None;

    let res = egui::Modal::new(egui::Id::unique("gc_options_modal")).show(ctx, |ui| {
        ui.set_width(520.0);
        ui.heading("Options Grammalecte");
        ui.add_space(8.0);

        egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
            section(ui, "Essentielles (actives par défaut)", ESSENTIAL, options, &mut action);
            ui.add_space(8.0);
            section(ui, "Recommandées (inactives par défaut dans GC)", RECOMMENDED, options, &mut action);
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
