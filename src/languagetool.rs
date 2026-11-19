use serde::Deserialize;
use std::time::Duration;

use crate::config::LanguageToolConfig;
use crate::dictionary::Dictionary;

#[derive(Debug, Clone)]
pub struct LtMatch {
    pub offset: usize,
    pub length: usize,
    pub message: String,
    pub rule_id: String,
    pub replacements: Vec<String>,
    pub expanded: bool,
    pub ignored: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LtStatus {
    Connected,
    Connecting,
    Unreachable,
}

pub struct LtClient {
    pub url: String,
    pub language: String,
    child: Option<std::process::Child>,
    http: reqwest::blocking::Client,
}

#[derive(Deserialize)]
struct LtResponse {
    matches: Vec<LtResponseMatch>,
}

#[derive(Deserialize)]
struct LtResponseMatch {
    offset: usize,
    length: usize,
    message: String,
    rule: LtRule,
    #[serde(default)]
    replacements: Vec<LtReplacement>,
}

#[derive(Deserialize)]
struct LtRule {
    id: String,
}

#[derive(Deserialize)]
struct LtReplacement {
    value: String,
}

impl LtClient {
    pub fn new(url: String, language: String) -> Self {
        Self {
            url,
            language,
            child: None,
            http: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    fn ping(http: &reqwest::blocking::Client, url: &str) -> bool {
        http.get(format!("{url}/v2/languages"))
            .timeout(Duration::from_secs(2))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Tente de se connecter au serveur LT. Si inaccessible et qu'un binaire est configuré,
    /// le lance puis retente jusqu'à ~5 s.
    pub fn init(config: &LanguageToolConfig) -> (Self, LtStatus) {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        if Self::ping(&http, &config.url) {
            return (
                Self { url: config.url.clone(), language: config.language.clone(), child: None, http },
                LtStatus::Connected,
            );
        }

        if let Some(ref binary) = config.binary {
            if let Ok(child) = std::process::Command::new(binary).spawn() {
                let client = Self {
                    url: config.url.clone(),
                    language: config.language.clone(),
                    child: Some(child),
                    http,
                };
                for _ in 0..10 {
                    std::thread::sleep(Duration::from_millis(500));
                    if Self::ping(&client.http, &client.url) {
                        return (client, LtStatus::Connected);
                    }
                }
                return (client, LtStatus::Unreachable);
            }
        }

        (
            Self { url: config.url.clone(), language: config.language.clone(), child: None, http },
            LtStatus::Unreachable,
        )
    }

    /// Envoie le texte à LanguageTool et retourne les correspondances filtrées par le dictionnaire.
    /// Les offsets retournés sont en caractères Unicode (tels que renvoyés par LT).
    pub fn check(&self, text: &str, dict: &Dictionary) -> Result<Vec<LtMatch>, String> {
        let resp = self
            .http
            .post(format!("{}/v2/check", self.url))
            .form(&[("text", text), ("language", &self.language)])
            .send()
            .map_err(|e| format!("Erreur réseau LanguageTool : {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("LanguageTool a répondu avec le code {}", resp.status()));
        }

        let lt_resp: LtResponse = resp
            .json()
            .map_err(|e| format!("Réponse LanguageTool invalide : {e}"))?;

        let chars: Vec<char> = text.chars().collect();

        let matches = lt_resp
            .matches
            .into_iter()
            .filter(|m| {
                // LT offsets are char-based; extract the matched word to check the dictionary.
                let word: String = chars
                    .get(m.offset..m.offset + m.length)
                    .unwrap_or(&[])
                    .iter()
                    .collect();
                !dict.is_exception(&word)
            })
            .map(|m| LtMatch {
                offset: m.offset,
                length: m.length,
                message: m.message,
                rule_id: m.rule.id,
                replacements: m.replacements.into_iter().map(|r| r.value).collect(),
                expanded: false,
                ignored: false,
            })
            .collect();

        Ok(matches)
    }
}

impl Drop for LtClient {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
    }
}
