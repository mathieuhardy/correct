use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// `#[serde(default)]` : toute clé absente prend la valeur de `Default`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrammalecteConfig {
    pub binary: Option<String>,
    pub url: String,
}

impl Default for GrammalecteConfig {
    fn default() -> Self {
        Self {
            binary: None,
            url: "http://localhost:8080".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Analyzer {
    #[default]
    LanguageTool,
    Grammalecte,
}

/// `#[serde(default)]` : toute clé absente prend la valeur de `Default` (impl manuelle,
/// car un `Default` dérivé donnerait `font_size = 0` et ferait paniquer epaint).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub last_project: Option<PathBuf>,
    /// Taille de la police en points (défaut : 14.0).
    pub font_size: f32,
    /// Chemin vers un fichier .ttf/.otf à utiliser comme police principale (optionnel).
    pub font_family: Option<String>,
    /// Multiplicateur d'interligne dans l'éditeur (1.0 = défaut, 1.5 = 150 %).
    pub line_height: f32,
    /// Hinting des glyphes (calage sur la grille de pixels). Défaut : true.
    pub font_hinting: bool,
    /// Gamma de l'anticrénelage du texte (ex. 1.0 = traits plus fins et plus lisses).
    /// Absent : réglage egui par défaut du thème sombre, plus gras.
    pub font_gamma: Option<f32>,
    /// Largeur max du texte en pixels pour le mode centré (optionnel).
    pub editor_max_width: Option<f32>,
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
    /// Survol des suggestions dans le popup (optionnel, défaut vert).
    pub accept_color_bg: Option<String>,
    pub accept_color_fg: Option<String>,
    /// Survol du bouton « Ignorer » dans le popup (optionnel).
    pub ignore_color_bg: Option<String>,
    pub ignore_color_fg: Option<String>,
    /// Survol du bouton « Ajouter au dictionnaire » dans le popup (optionnel).
    pub dictionary_color_bg: Option<String>,
    pub dictionary_color_fg: Option<String>,
    /// Analyseur utilisé par défaut au démarrage.
    pub default_analyzer: Analyzer,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_project: None,
            font_size: 14.0,
            font_family: None,
            line_height: 1.0,
            font_hinting: true,
            font_gamma: None,
            editor_max_width: None,
            accent_color_bg: None,
            accent_color_fg: None,
            app_color_bg: None,
            app_color_fg: None,
            selection_color_bg: None,
            selection_color_fg: None,
            accept_color_bg: None,
            accept_color_fg: None,
            ignore_color_bg: None,
            ignore_color_fg: None,
            dictionary_color_bg: None,
            dictionary_color_fg: None,
            default_analyzer: Analyzer::LanguageTool,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub languagetool: LanguageToolConfig,
    #[serde(default)]
    pub grammalecte: GrammalecteConfig,
    #[serde(default)]
    pub app: AppConfig,
    /// Fichier présent mais illisible / invalide : on tourne avec les valeurs par défaut
    /// mais toute écriture est refusée pour ne pas écraser les réglages de l'utilisateur.
    #[serde(skip)]
    locked: bool,
}

impl Config {
    /// Charge la config ; crée le fichier avec les valeurs par défaut s'il est absent.
    /// Retourne aussi l'erreur éventuelle, à afficher dans la barre de statut.
    pub fn load() -> (Self, Option<String>) {
        Self::load_from(&Self::path())
    }

    fn load_from(path: &Path) -> (Self, Option<String>) {
        let locked = |msg: String| {
            (
                Config {
                    locked: true,
                    ..Config::default()
                },
                Some(msg),
            )
        };
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(mut config) => {
                    let err = config.sanitize();
                    (config, err)
                }
                Err(e) => locked(format!("Config invalide ({}) : {e}", path.display())),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let config = Config::default();
                let err = config.save_to(path).err();
                (config, err)
            }
            Err(e) => locked(format!(
                "Impossible de lire la config ({}) : {e}",
                path.display()
            )),
        }
    }

    /// Remplace par leur défaut les valeurs numériques inutilisables (une taille de police
    /// nulle fait paniquer epaint) et retourne le message d'erreur correspondant.
    fn sanitize(&mut self) -> Option<String> {
        let default = AppConfig::default();
        let mut errors = Vec::new();
        if self.app.font_size <= 0.0 || !self.app.font_size.is_finite() {
            errors.push(format!(
                "font_size invalide ({}), {} utilisé",
                self.app.font_size, default.font_size
            ));
            self.app.font_size = default.font_size;
        }
        if self.app.line_height <= 0.0 || !self.app.line_height.is_finite() {
            errors.push(format!(
                "line_height invalide ({}), {} utilisé",
                self.app.line_height, default.line_height
            ));
            self.app.line_height = default.line_height;
        }
        if let Some(g) = self.app.font_gamma
            && (g <= 0.0 || !g.is_finite())
        {
            errors.push(format!(
                "font_gamma invalide ({g}), réglage par défaut utilisé"
            ));
            self.app.font_gamma = None;
        }
        (!errors.is_empty()).then(|| errors.join("  •  "))
    }

    pub fn save(&self) -> Result<(), String> {
        self.save_to(&Self::path())
    }

    fn save_to(&self, path: &Path) -> Result<(), String> {
        if self.locked {
            return Err(format!(
                "Config invalide : corrigez {} (modifications non sauvegardées)",
                path.display()
            ));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Impossible de créer le dossier de config : {e}"))?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Erreur de sérialisation config : {e}"))?;
        std::fs::write(path, content)
            .map_err(|e| format!("Impossible d'écrire la config ({}) : {e}", path.display()))
    }

    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("correct")
            .join("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path() -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("correct-cfg-{}-{n}", std::process::id()))
            .join("config.toml")
    }

    /// Bug : sans fichier, `font_size` valait 0 (Default dérivé) → panic epaint au démarrage.
    #[test]
    fn missing_file_defaults_are_usable() {
        let path = tmp_path();
        let (cfg, err) = Config::load_from(&path);
        assert!(err.is_none());
        assert_eq!(cfg.app.font_size, 14.0);
        assert_eq!(cfg.app.line_height, 1.0);
        // Et le fichier créé se relit avec les mêmes valeurs.
        let (back, _) = Config::load_from(&path);
        assert_eq!(back.app.font_size, 14.0);
        assert_eq!(back.app.line_height, 1.0);
    }

    #[test]
    fn zero_or_negative_sizes_are_reported_and_replaced() {
        let path = tmp_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "[app]\nfont_size = 0\nline_height = -1\nfont_gamma = 0\n",
        )
        .unwrap();

        let (cfg, err) = Config::load_from(&path);
        let err = err.unwrap();
        assert!(
            err.contains("font_size") && err.contains("line_height") && err.contains("font_gamma"),
            "{err}"
        );
        assert_eq!(cfg.app.font_size, 14.0);
        assert_eq!(cfg.app.line_height, 1.0);
        assert_eq!(cfg.app.font_gamma, None);
    }

    /// Clés ou sections manquantes : valeurs par défaut, pas d'erreur.
    #[test]
    fn missing_keys_and_sections_use_defaults() {
        let path = tmp_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        for content in ["", "[languagetool]\nurl = \"http://x:1\"\n", "[app]\n"] {
            std::fs::write(&path, content).unwrap();
            let (cfg, err) = Config::load_from(&path);
            assert!(err.is_none(), "{content:?} : {err:?}");
            assert_eq!(cfg.app.font_size, 14.0, "{content:?}");
            assert_eq!(cfg.app.line_height, 1.0, "{content:?}");
            assert_eq!(cfg.languagetool.language, "auto", "{content:?}");
        }
    }

    #[test]
    fn missing_file_is_created_with_defaults() {
        let path = tmp_path();
        let (cfg, err) = Config::load_from(&path);
        assert!(err.is_none());
        assert!(path.is_file());
        assert_eq!(cfg.languagetool.url, "http://localhost:8081");
    }

    #[test]
    fn partial_file_uses_defaults_for_missing_keys() {
        let path = tmp_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[app]\naccent_color_bg = \"#5277C3\"\n").unwrap();

        let (cfg, err) = Config::load_from(&path);
        assert!(err.is_none());
        assert_eq!(cfg.app.accent_color_bg.as_deref(), Some("#5277C3"));
        assert_eq!(cfg.app.font_size, 14.0);
        assert_eq!(cfg.app.line_height, 1.0);
        assert_eq!(cfg.languagetool.language, "auto");
    }

    #[test]
    fn save_then_load_roundtrip() {
        let path = tmp_path();
        let (mut cfg, _) = Config::load_from(&path);
        cfg.app.last_project = Some(PathBuf::from("/tmp/projet"));
        cfg.app.font_size = 16.0;
        cfg.languagetool.binary = Some(PathBuf::from("languagetool-http-server"));
        cfg.save_to(&path).unwrap();

        let (back, err) = Config::load_from(&path);
        assert!(err.is_none());
        assert_eq!(back.app.last_project, cfg.app.last_project);
        assert_eq!(back.app.font_size, 16.0);
        assert_eq!(back.languagetool.binary, cfg.languagetool.binary);
    }

    #[test]
    fn invalid_file_is_reported_and_never_overwritten() {
        let path = tmp_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[app\nfont_size = 18").unwrap();

        let (cfg, err) = Config::load_from(&path);
        assert!(err.unwrap().contains("Config invalide"));
        assert!(cfg.save_to(&path).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[app\nfont_size = 18"
        );
    }

    // --- Phase 7 : config multi-analyseur ---

    #[test]
    fn default_analyzer_is_languagetool() {
        let path = tmp_path();
        let (cfg, err) = Config::load_from(&path);
        assert!(err.is_none());
        assert_eq!(cfg.app.default_analyzer, Analyzer::LanguageTool);
    }

    #[test]
    fn analyzer_round_trip_toml() {
        let path = tmp_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[app]\ndefault_analyzer = \"grammalecte\"\n").unwrap();

        let (cfg, err) = Config::load_from(&path);
        assert!(err.is_none());
        assert_eq!(cfg.app.default_analyzer, Analyzer::Grammalecte);

        cfg.save_to(&path).unwrap();
        let (back, err) = Config::load_from(&path);
        assert!(err.is_none());
        assert_eq!(back.app.default_analyzer, Analyzer::Grammalecte);
    }

    #[test]
    fn grammalecte_config_round_trip() {
        let path = tmp_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "[grammalecte]\nbinary = \"python3 gc-server.py\"\nurl = \"http://localhost:9090\"\n",
        )
        .unwrap();

        let (cfg, err) = Config::load_from(&path);
        assert!(err.is_none());
        assert_eq!(cfg.grammalecte.binary.as_deref(), Some("python3 gc-server.py"));
        assert_eq!(cfg.grammalecte.url, "http://localhost:9090");
    }

    #[test]
    fn grammalecte_defaults_when_section_absent() {
        let path = tmp_path();
        let (cfg, err) = Config::load_from(&path);
        assert!(err.is_none());
        assert_eq!(cfg.grammalecte.binary, None);
        assert_eq!(cfg.grammalecte.url, "http://localhost:8080");
    }

    #[test]
    fn unknown_analyzer_gives_readable_error() {
        let path = tmp_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[app]\ndefault_analyzer = \"unknown_tool\"\n").unwrap();

        let (_, err) = Config::load_from(&path);
        let msg = err.unwrap();
        assert!(
            msg.contains("unknown_tool") || msg.contains("Config invalide"),
            "message d'erreur peu lisible : {msg}"
        );
    }
}
