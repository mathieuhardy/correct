use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Dictionary {
    #[serde(default)]
    pub words: Vec<String>,
}

impl Dictionary {
    /// Charge le dictionnaire du projet. Retourne vide si le fichier n'existe pas encore.
    pub fn load(project: &Path) -> Self {
        let path = Self::path(project);
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Dictionary::default(),
        }
    }

    pub fn save(&self, project: &Path) -> Result<(), String> {
        let path = Self::path(project);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Impossible de créer .correct/ : {e}"))?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Erreur de sérialisation dictionnaire : {e}"))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Impossible d'écrire le dictionnaire ({}) : {e}", path.display()))
    }

    /// Ajoute un mot (insensible à la casse) et persiste immédiatement.
    pub fn add_word(&mut self, word: &str, project: &Path) -> Result<(), String> {
        if !self.is_exception(word) {
            self.words.push(word.to_string());
            self.save(project)?;
        }
        Ok(())
    }

    pub fn is_exception(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        self.words.iter().any(|w| w.to_lowercase() == lower)
    }

    pub fn path(project: &Path) -> PathBuf {
        project.join(".correct").join("dictionary.toml")
    }
}
