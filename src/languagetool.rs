use serde::Deserialize;
use std::time::Duration;

use crate::config::LanguageToolConfig;

#[derive(Debug, Clone)]
pub struct LtMatch {
    pub offset: usize,
    pub length: usize,
    pub message: String,
    pub rule_id: String,
    pub replacements: Vec<String>,
    pub expanded: bool,
    pub ignored: bool,
    /// Faute d'orthographe (`issueType = misspelling`) : seule éligible au dictionnaire.
    pub misspelling: bool,
    /// Mot présent dans le dictionnaire du projet (recalculé à chaque changement du dictionnaire).
    pub in_dictionary: bool,
}

impl LtMatch {
    /// Texte couvert par l'erreur (offsets en caractères).
    pub fn word(&self, text: &str) -> String {
        text.chars().skip(self.offset).take(self.length).collect()
    }

    /// Masquée dans l'éditeur et le panneau (ignorée ou dans le dictionnaire).
    pub fn hidden(&self) -> bool {
        self.ignored || self.in_dictionary
    }
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
    #[serde(rename = "issueType", default)]
    issue_type: String,
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

    /// Envoie le texte à LanguageTool et retourne toutes les correspondances
    /// (le filtrage par dictionnaire est fait par l'appelant).
    /// LT renvoie des offsets en unités UTF-16 (Java) ; ils sont convertis en index de caractères.
    pub fn check(&self, text: &str) -> Result<Vec<LtMatch>, String> {
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

        let starts = utf16_starts(text);

        let matches = lt_resp
            .matches
            .into_iter()
            .map(|m| {
                let (offset, length) = utf16_to_char_range(&starts, m.offset, m.length);
                LtMatch {
                    offset,
                    length,
                    message: m.message,
                    misspelling: m.rule.issue_type == "misspelling",
                    rule_id: m.rule.id,
                    replacements: m.replacements.into_iter().map(|r| r.value).collect(),
                    expanded: false,
                    ignored: false,
                    in_dictionary: false,
                }
            })
            .collect();

        Ok(matches)
    }
}

/// Offset UTF-16 du début de chaque caractère, plus la longueur totale en dernier élément.
fn utf16_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::with_capacity(text.len() + 1);
    let mut acc = 0;
    for c in text.chars() {
        starts.push(acc);
        acc += c.len_utf16();
    }
    starts.push(acc);
    starts
}

/// Convertit une plage (offset, longueur) en unités UTF-16 en plage en caractères.
fn utf16_to_char_range(starts: &[usize], offset: usize, length: usize) -> (usize, usize) {
    let start = starts.partition_point(|&u| u < offset);
    let end   = starts.partition_point(|&u| u < offset + length);
    (start, end.saturating_sub(start))
}

impl Drop for LtClient {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_offsets_ascii_and_accents() {
        // Les accents français restent dans le plan de base : 1 unité UTF-16 = 1 caractère.
        let starts = utf16_starts("C'est très tres bien");
        assert_eq!(utf16_to_char_range(&starts, 11, 4), (11, 4));
    }

    #[test]
    fn utf16_offsets_after_emoji() {
        // 😀 = 2 unités UTF-16 mais 1 caractère : LT place « tres » à 9, c'est le caractère 8.
        let text = "😀 C'est tres bien.";
        let starts = utf16_starts(text);
        let (off, len) = utf16_to_char_range(&starts, 9, 4);
        assert_eq!((off, len), (8, 4));
        let m = LtMatch {
            offset: off, length: len, message: String::new(), rule_id: String::new(),
            replacements: vec![], expanded: false, ignored: false, misspelling: true, in_dictionary: false,
        };
        assert_eq!(m.word(text), "tres");
    }

    #[test]
    fn utf16_range_covering_emoji() {
        let starts = utf16_starts("a😀b");
        assert_eq!(utf16_to_char_range(&starts, 1, 2), (1, 1));
    }
}
