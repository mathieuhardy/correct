use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageToolConfig {
    pub binary: Option<PathBuf>,
    pub url: String,
    pub language: String,
}

impl Default for LanguageToolConfig {
    fn default() -> Self {
        Self {
            binary: None,
            url: "http://localhost:8081".to_string(),
            language: "auto".to_string(),
        }
    }
}

fn default_font_size() -> f32 {
    14.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub last_project: Option<PathBuf>,
    /// Taille de la police en points (défaut : 14.0).
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    /// Chemin vers un fichier .ttf/.otf à utiliser comme police principale (optionnel).
    pub font_family: Option<String>,
    /// Couleur de fond des en-têtes en hex HTML, ex. "#5277C3" (optionnel).
    pub accent_color_bg: Option<String>,
    /// Couleur du texte des en-têtes en hex HTML, ex. "#ffffff" (optionnel).
    pub accent_color_fg: Option<String>,
    /// Couleur de fond de l'application en hex HTML (optionnel).
    pub app_color_bg: Option<String>,
    /// Couleur du texte de l'application en hex HTML (optionnel).
    pub app_color_fg: Option<String>,
    /// Fond du fichier actif dans l'arbre (optionnel).
    pub selection_color_bg: Option<String>,
    /// Texte du fichier actif dans l'arbre (optionnel).
    pub selection_color_fg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub languagetool: LanguageToolConfig,
    #[serde(default)]
    pub app: AppConfig,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::path();
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let config = Config::default();
                let _ = config.save();
                config
            }
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Impossible de créer le dossier de config : {e}"))?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Erreur de sérialisation config : {e}"))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Impossible d'écrire la config ({}) : {e}", path.display()))
    }

    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("correct")
            .join("config.toml")
    }
}
