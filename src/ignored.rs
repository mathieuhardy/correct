use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::Analyzer;
use crate::languagetool::LtMatch;

/// Empreinte d'un élément ignoré, indépendante de sa position dans le fichier :
/// un mot déplacé reste ignoré, une phrase modifiée est à relire.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Ignored {
    /// Chemin relatif à la racine du projet, séparateurs `/`.
    pub file: String,
    #[serde(default)]
    pub analyzer: Analyzer,
    pub rule: String,
    pub text: String,
    /// Phrase contenant l'élément, espaces normalisés.
    pub sentence: String,
}

impl Ignored {
    pub fn of(file: &str, analyzer: Analyzer, m: &LtMatch, content: &str) -> Self {
        Self {
            file: file.to_string(),
            analyzer,
            rule: m.rule_id.clone(),
            text: m.word(content),
            sentence: m.sentence.clone(),
        }
    }
}

/// Éléments ignorés du projet : `<projet>/.correct/ignored.toml`.
/// Liste plate (une entrée porte son fichier) : un fichier sans ignoré n'a simplement
/// aucune entrée, il n'y a rien à nettoyer.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct IgnoredList {
    #[serde(default)]
    ignored: Vec<Ignored>,
    /// Fichier présent mais invalide : écriture refusée pour ne pas l'écraser.
    #[serde(skip)]
    locked: bool,
}

impl IgnoredList {
    /// Vide si le fichier n'existe pas ; erreur s'il existe mais est illisible ou invalide.
    pub fn load(project: &Path) -> Result<Self, String> {
        let path = Self::path(project);
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content)
                .map_err(|e| format!("Liste des ignorés invalide ({}) : {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!(
                "Impossible de lire la liste des ignorés ({}) : {e}",
                path.display()
            )),
        }
    }

    /// Liste vide en lecture seule, utilisée quand le fichier existant est invalide.
    pub fn locked() -> Self {
        Self {
            ignored: Vec::new(),
            locked: true,
        }
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Écriture triée (diffs git stables) et atomique (tmp + rename).
    /// Liste vide : le fichier est supprimé plutôt que laissé vide.
    pub fn save(&mut self, project: &Path) -> Result<(), String> {
        if self.locked {
            return Err(
                "Liste des ignorés invalide : corrigez .correct/ignored.toml avant de la modifier"
                    .to_string(),
            );
        }
        let path = Self::path(project);
        if self.ignored.is_empty() {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    Err(format!("Impossible de supprimer {} : {e}", path.display()))
                }
                _ => Ok(()),
            };
        }
        self.ignored.sort();
        self.ignored.dedup();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Impossible de créer .correct/ : {e}"))?;
        }
        let content = toml::to_string_pretty(&*self)
            .map_err(|e| format!("Erreur de sérialisation des ignorés : {e}"))?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, content)
            .and_then(|_| std::fs::rename(&tmp, &path))
            .map_err(|e| {
                format!(
                    "Impossible d'écrire la liste des ignorés ({}) : {e}",
                    path.display()
                )
            })
    }

    pub fn contains(&self, entry: &Ignored) -> bool {
        self.ignored.contains(entry)
    }

    /// Ajoute une empreinte ; false si elle était déjà présente.
    pub fn add(&mut self, entry: Ignored) -> bool {
        if self.contains(&entry) {
            return false;
        }
        self.ignored.push(entry);
        true
    }

    /// Empreintes d'un fichier pour un analyseur donné, triées par mot puis phrase.
    pub fn for_file(&self, file: &str, analyzer: Analyzer) -> Vec<Ignored> {
        let mut v: Vec<Ignored> = self
            .ignored
            .iter()
            .filter(|e| e.file == file && e.analyzer == analyzer)
            .cloned()
            .collect();
        v.sort_by(|a, b| (&a.text, &a.sentence).cmp(&(&b.text, &b.sentence)));
        v
    }

    /// Retire une empreinte (rétablissement, ou annulation d'un ajout non sauvegardé).
    pub fn remove(&mut self, entry: &Ignored) {
        self.ignored.retain(|e| e != entry);
    }

    /// Réconciliation après une analyse réussie de `file` pour `analyzer` : retire les
    /// empreintes de cet analyseur qui ne correspondent plus à aucune erreur.
    /// Retourne true si la liste a changé.
    pub fn prune(&mut self, file: &str, analyzer: Analyzer, current: &[Ignored]) -> bool {
        let before = self.ignored.len();
        self.ignored
            .retain(|e| e.file != file || e.analyzer != analyzer || current.contains(e));
        self.ignored.len() != before
    }

    /// Retire les entrées des fichiers qui n'existent plus. Retourne true si la liste a changé.
    pub fn prune_missing(&mut self, project: &Path) -> bool {
        let before = self.ignored.len();
        self.ignored.retain(|e| project.join(&e.file).is_file());
        self.ignored.len() != before
    }

    pub fn path(project: &Path) -> PathBuf {
        project.join(".correct").join("ignored.toml")
    }
}

/// Chemin de `file` relatif au projet, avec des `/` (stable entre plateformes et pour git).
/// Fichier hors du projet : chemin absolu.
pub fn relative_path(project: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(project).unwrap_or(file);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("correct-ign-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Empreinte de l'erreur couvrant `word` dans `text` (phrase = texte entier).
    fn entry(text: &str, word: &str) -> Ignored {
        let offset = text[..text.find(word).unwrap()].chars().count();
        let m = LtMatch::for_test(offset, word.chars().count(), "FR_SPELLING_RULE", text);
        Ignored::of("docs/notes.md", Analyzer::default(), &m, text)
    }

    #[test]
    fn fingerprint_survives_move_and_rewrap() {
        let before = entry("Le dossier est tres important.", "tres");
        // Paragraphe déplacé plus bas et re-wrappé : autre offset, même empreinte.
        let text = "Intro ajoutée.\n\nLe dossier est tres\nimportant.";
        let sentence = "Le dossier est tres\nimportant.";
        let offset = text[..text.find("tres").unwrap()].chars().count();
        let m = LtMatch::for_test(offset, 4, "FR_SPELLING_RULE", sentence);
        assert_eq!(Ignored::of("docs/notes.md", Analyzer::default(), &m, text), before);
    }

    #[test]
    fn prune_removes_vanished_entries_of_that_file_only() {
        let mut list = IgnoredList::default();
        let kept = entry("Le dossier est tres important.", "tres");
        let gone = entry("Il est tres bien.", "tres");
        let other = Ignored {
            file: "autre.md".into(),
            ..gone.clone()
        };
        list.add(kept.clone());
        list.add(gone.clone());
        list.add(other.clone());

        // Nouvelle analyse de notes.md : seule `kept` est encore signalée.
        assert!(list.prune("docs/notes.md", Analyzer::default(), std::slice::from_ref(&kept)));
        assert!(list.contains(&kept));
        assert!(!list.contains(&gone));
        assert!(
            list.contains(&other),
            "les autres fichiers ne sont pas touchés"
        );

        // Plus rien de signalé pour notes.md : plus aucune entrée pour ce fichier.
        assert!(list.prune("docs/notes.md", Analyzer::default(), &[]));
        assert_eq!(list.ignored, vec![other]);
        assert!(
            !list.prune("docs/notes.md", Analyzer::default(), &[]),
            "rien à purger : pas de changement"
        );
    }

    #[test]
    fn save_load_roundtrip_and_empty_list_removes_file() {
        let p = project();
        let mut list = IgnoredList::load(&p).unwrap();
        let e = entry("Le dossier est tres important.", "tres");
        list.add(e.clone());
        list.add(e.clone()); // doublon ignoré
        list.save(&p).unwrap();

        let back = IgnoredList::load(&p).unwrap();
        assert_eq!(back.ignored, vec![e.clone()]);

        let mut back = back;
        back.remove(&e);
        back.save(&p).unwrap();
        assert!(!IgnoredList::path(&p).exists());
    }

    #[test]
    fn invalid_file_is_error_and_never_overwritten() {
        let p = project();
        let path = IgnoredList::path(&p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[[ignored]\nfile = ").unwrap();

        assert!(IgnoredList::load(&p).is_err());
        let mut list = IgnoredList::locked();
        list.add(entry("Il est tres bien.", "tres"));
        assert!(list.save(&p).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[[ignored]\nfile = "
        );
    }

    #[test]
    fn prune_missing_drops_deleted_files() {
        let p = project();
        std::fs::create_dir_all(p.join("docs")).unwrap();
        std::fs::write(p.join("docs/notes.md"), "x").unwrap();
        let mut list = IgnoredList::default();
        let present = entry("Il est tres bien.", "tres");
        let missing = Ignored {
            file: "supprime.md".into(),
            ..present.clone()
        };
        list.add(present.clone());
        list.add(missing);
        assert!(list.prune_missing(&p));
        assert_eq!(list.ignored, vec![present]);
    }

    #[test]
    fn relative_paths_use_slashes() {
        let p = Path::new("/home/u/projet");
        assert_eq!(
            relative_path(p, Path::new("/home/u/projet/docs/notes.md")),
            "docs/notes.md"
        );
    }
}
