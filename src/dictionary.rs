use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Dictionary {
    #[serde(default)]
    pub words: Vec<String>,
    /// Fichier présent mais illisible / invalide : toute écriture est refusée
    /// pour ne pas écraser les mots qu'il contient.
    #[serde(skip)]
    locked: bool,
}

impl Dictionary {
    /// Charge le dictionnaire du projet. Vide si le fichier n'existe pas encore ;
    /// erreur s'il existe mais ne peut pas être lu ou parsé.
    pub fn load(project: &Path) -> Result<Self, String> {
        let path = Self::path(project);
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content)
                .map_err(|e| format!("Dictionnaire invalide ({}) : {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Dictionary::default()),
            Err(e) => Err(format!(
                "Impossible de lire le dictionnaire ({}) : {e}",
                path.display()
            )),
        }
    }

    /// Dictionnaire vide en lecture seule, utilisé quand le fichier existant est invalide.
    pub fn locked() -> Self {
        Self {
            words: Vec::new(),
            locked: true,
        }
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Écrit via un fichier temporaire + rename : un lecteur (watcher) ne voit jamais
    /// un fichier à moitié écrit.
    pub fn save(&self, project: &Path) -> Result<(), String> {
        if self.locked {
            return Err(
                "Dictionnaire invalide : corrigez .correct/dictionary.toml avant de le modifier"
                    .to_string(),
            );
        }
        let path = Self::path(project);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Impossible de créer .correct/ : {e}"))?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Erreur de sérialisation dictionnaire : {e}"))?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, content)
            .and_then(|_| std::fs::rename(&tmp, &path))
            .map_err(|e| {
                format!(
                    "Impossible d'écrire le dictionnaire ({}) : {e}",
                    path.display()
                )
            })
    }

    /// Ajoute un mot (insensible à la casse) et persiste immédiatement.
    pub fn add_word(&mut self, word: &str, project: &Path) -> Result<(), String> {
        if self.is_exception(word) {
            return Ok(());
        }
        self.words.push(word.to_string());
        let res = self.save(project);
        if res.is_err() {
            self.words.pop();
        }
        res
    }

    /// Retire un mot (insensible à la casse) et persiste immédiatement.
    pub fn remove_word(&mut self, word: &str, project: &Path) -> Result<(), String> {
        let before = std::mem::take(&mut self.words);
        let lower = word.to_lowercase();
        self.words = before
            .iter()
            .filter(|w| w.to_lowercase() != lower)
            .cloned()
            .collect();
        let res = self.save(project);
        if res.is_err() {
            self.words = before;
        }
        res
    }

    pub fn is_exception(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        self.words.iter().any(|w| w.to_lowercase() == lower)
    }

    pub fn path(project: &Path) -> PathBuf {
        project.join(".correct").join("dictionary.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("correct-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_file_is_empty() {
        let p = project();
        assert!(Dictionary::load(&p).unwrap().words.is_empty());
    }

    #[test]
    fn add_remove_roundtrip() {
        let p = project();
        let mut d = Dictionary::load(&p).unwrap();
        d.add_word("Qwant", &p).unwrap();
        d.add_word("qwant", &p).unwrap(); // doublon insensible à la casse
        d.add_word("nix", &p).unwrap();
        assert_eq!(Dictionary::load(&p).unwrap().words, vec!["Qwant", "nix"]);

        d.remove_word("QWANT", &p).unwrap();
        assert_eq!(Dictionary::load(&p).unwrap().words, vec!["nix"]);
    }

    #[test]
    fn invalid_file_is_error_and_never_overwritten() {
        let p = project();
        let path = Dictionary::path(&p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "words = [\"a\", oops").unwrap();

        assert!(Dictionary::load(&p).is_err());

        let mut d = Dictionary::locked();
        assert!(d.add_word("b", &p).is_err());
        assert!(d.words.is_empty());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "words = [\"a\", oops"
        );
    }
}
