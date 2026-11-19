use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub last_project: Option<PathBuf>,
    #[serde(default)]
    pub theme: Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub languagetool: LanguageToolConfig,
    #[serde(default)]
    pub app: AppConfig,
}
