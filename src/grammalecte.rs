use serde::Deserialize;
use std::time::Duration;

use crate::config::GrammalecteConfig;
use crate::languagetool::{normalize_spaces, LtMatch};

pub struct GcClient {
    pub url: String,
    child: Option<std::process::Child>,
    http: reqwest::blocking::Client,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GcStatus {
    Connected,
    Unreachable,
}

#[derive(Clone)]
pub struct GcChecker {
    url: String,
    http: reqwest::blocking::Client,
}

#[derive(Deserialize)]
struct GcResponse {
    data: Vec<GcParagraph>,
}

#[derive(Deserialize)]
struct GcParagraph {
    #[serde(rename = "sParagraph")]
    paragraph: String,
    #[serde(rename = "lGrammarErrors", default)]
    grammar_errors: Vec<GcGrammarError>,
    #[serde(rename = "lSpellingErrors", default)]
    spelling_errors: Vec<GcSpellingError>,
}

#[derive(Deserialize)]
struct GcGrammarError {
    #[serde(rename = "nStart")]
    n_start: usize,
    #[serde(rename = "nEnd")]
    n_end: usize,
    #[serde(rename = "sMessage", default)]
    message: String,
    #[serde(rename = "sRuleId", default)]
    rule_id: String,
    #[serde(rename = "aSuggestions", default)]
    suggestions: Vec<String>,
}

#[derive(Deserialize)]
struct GcSpellingError {
    #[serde(rename = "nStart")]
    n_start: usize,
    #[serde(rename = "nEnd")]
    n_end: usize,
    #[serde(rename = "aSuggestions", default)]
    suggestions: Vec<String>,
}

impl GcClient {
    fn ping(http: &reqwest::blocking::Client, url: &str) -> bool {
        http.get(format!("{url}/get_options/fr"))
            .timeout(Duration::from_secs(2))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Tente de se connecter au serveur GC. Si inaccessible et qu'un binaire est configuré,
    /// le lance puis retente jusqu'à ~5 s.
    pub fn init(config: &GrammalecteConfig) -> (Self, GcStatus, Option<String>) {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        let reachable = Self::ping(&http, &config.url);
        let client = |child| Self {
            url: config.url.clone(),
            child,
            http,
        };

        if reachable {
            return (client(None), GcStatus::Connected, None);
        }

        let unreachable = format!("Impossible de joindre Grammalecte ({})", config.url);
        let Some(ref binary) = config.binary else {
            return (client(None), GcStatus::Unreachable, Some(unreachable));
        };

        let mut parts = binary.split_whitespace();
        let Some(cmd) = parts.next() else {
            return (client(None), GcStatus::Unreachable, Some(unreachable));
        };
        let args: Vec<&str> = parts.collect();

        match std::process::Command::new(cmd).args(&args).spawn() {
            Ok(child) => {
                let client = client(Some(child));
                for _ in 0..10 {
                    std::thread::sleep(Duration::from_millis(500));
                    if Self::ping(&client.http, &client.url) {
                        return (client, GcStatus::Connected, None);
                    }
                }
                let msg = format!(
                    "{unreachable} : '{binary}' lancé mais ne répond pas après 5 s"
                );
                (client, GcStatus::Unreachable, Some(msg))
            }
            Err(e) => {
                let msg = format!("{unreachable} : impossible de lancer '{binary}' : {e}");
                (client(None), GcStatus::Unreachable, Some(msg))
            }
        }
    }

    pub fn checker(&self) -> GcChecker {
        GcChecker {
            url: self.url.clone(),
            http: self.http.clone(),
        }
    }
}

impl GcChecker {
    /// Envoie le texte à Grammalecte et retourne toutes les correspondances.
    /// Les offsets GC sont en caractères Unicode (pas de conversion UTF-16 nécessaire).
    pub fn check(&self, text: &str) -> Result<Vec<LtMatch>, String> {
        let resp = self
            .http
            .post(format!("{}/gc_text/fr", self.url))
            .form(&[("text", text), ("options", "{}")])
            .send()
            .map_err(|e| format!("Erreur réseau Grammalecte : {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Grammalecte a répondu avec le code {}",
                resp.status()
            ));
        }

        let body = resp
            .text()
            .map_err(|e| format!("Réponse Grammalecte illisible : {e}"))?;
        parse_matches(text, &body)
    }
}

fn parse_matches(text: &str, body: &str) -> Result<Vec<LtMatch>, String> {
    let gc_resp: GcResponse = serde_json::from_str(body)
        .map_err(|e| format!("Réponse Grammalecte invalide : {e}"))?;

    let mut matches = Vec::new();
    let mut search_from = 0usize;

    for para in gc_resp.data {
        let (char_base, next_search) = paragraph_char_base(text, search_from, &para.paragraph);
        search_from = next_search;

        let sentence = normalize_spaces(&para.paragraph);

        for e in para.grammar_errors {
            let length = e.n_end.saturating_sub(e.n_start);
            matches.push(LtMatch {
                offset: char_base + e.n_start,
                length,
                message: e.message,
                rule_id: e.rule_id,
                sentence: sentence.clone(),
                replacements: e.suggestions,
                misspelling: false,
                expanded: false,
                ignored: false,
                in_dictionary: false,
            });
        }

        for e in para.spelling_errors {
            let length = e.n_end.saturating_sub(e.n_start);
            matches.push(LtMatch {
                offset: char_base + e.n_start,
                length,
                message: String::new(),
                rule_id: "SPELLING_ERROR".to_string(),
                sentence: sentence.clone(),
                replacements: e.suggestions,
                misspelling: true,
                expanded: false,
                ignored: false,
                in_dictionary: false,
            });
        }
    }

    Ok(matches)
}

/// Localise `para` dans `text` à partir de `from_byte`.
/// Retourne `(char_offset_du_paragraphe, next_from_byte)`.
fn paragraph_char_base(text: &str, from_byte: usize, para: &str) -> (usize, usize) {
    let found_byte = text[from_byte..]
        .find(para)
        .map(|i| from_byte + i)
        .unwrap_or(from_byte);
    let char_offset = text[..found_byte].chars().count();
    (char_offset, found_byte + para.len())
}

impl Drop for GcClient {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Réponse /gc_text/fr pour "😀 C'est tres importent pour nous."
    // Les offsets sont en caractères Unicode : 😀 = 1 caractère.
    // tres   : chars 8–11 → nStart=8, nEnd=12
    // importent : chars 13–21 → nStart=13, nEnd=22
    const RESPONSE: &str = r#"{
        "data": [
            {
                "iParagraph": 1,
                "sParagraph": "😀 C'est tres importent pour nous.",
                "lGrammarErrors": [
                    {
                        "nStart": 13,
                        "nEnd": 22,
                        "sMessage": "Accord erroné.",
                        "sRuleId": "GC_RULE_1",
                        "aSuggestions": ["important"]
                    }
                ],
                "lSpellingErrors": [
                    {
                        "nStart": 8,
                        "nEnd": 12,
                        "aSuggestions": ["très", "trees"]
                    }
                ]
            }
        ]
    }"#;

    #[test]
    fn parse_response_fields() {
        let text = "😀 C'est tres importent pour nous.";
        let m = parse_matches(text, RESPONSE).unwrap();
        assert_eq!(m.len(), 2);

        let grammar = m.iter().find(|e| !e.misspelling).unwrap();
        assert_eq!(grammar.word(text), "importent");
        assert_eq!(grammar.rule_id, "GC_RULE_1");
        assert_eq!(grammar.message, "Accord erroné.");
        assert_eq!(grammar.replacements, vec!["important"]);
        assert!(!grammar.misspelling);

        let spelling = m.iter().find(|e| e.misspelling).unwrap();
        assert_eq!(spelling.word(text), "tres");
        assert_eq!(spelling.replacements, vec!["très", "trees"]);
        assert!(spelling.misspelling);
        assert_eq!(spelling.rule_id, "SPELLING_ERROR");
        assert!(spelling.message.is_empty());
    }

    #[test]
    fn emoji_offsets_are_char_based() {
        // 😀 = 1 caractère en Unicode, 2 unités UTF-16.
        // GC donne des offsets en caractères, donc "tres" après "😀 " est à nStart=2.
        let text = "😀 tres";
        let body = r#"{
            "data": [{
                "iParagraph": 1,
                "sParagraph": "😀 tres",
                "lGrammarErrors": [],
                "lSpellingErrors": [{"nStart": 2, "nEnd": 6, "aSuggestions": ["très"]}]
            }]
        }"#;
        let m = parse_matches(text, body).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].offset, 2);
        assert_eq!(m[0].length, 4);
        assert_eq!(m[0].word(text), "tres");
    }

    #[test]
    fn multiline_paragraph_offsets() {
        // "Premier paragraphe." = 19 chars, '\n' = char 19, deuxième para commence à char 20.
        // "erreur" dans le deuxième para commence à nStart=25 → offset absolu = 20 + 25 = 45.
        let text = "Premier paragraphe.\nDeuxième paragraphe avec erreur.";
        let body = r#"{
            "data": [
                {
                    "iParagraph": 1,
                    "sParagraph": "Premier paragraphe.",
                    "lGrammarErrors": [],
                    "lSpellingErrors": []
                },
                {
                    "iParagraph": 2,
                    "sParagraph": "Deuxième paragraphe avec erreur.",
                    "lGrammarErrors": [],
                    "lSpellingErrors": [{"nStart": 25, "nEnd": 31, "aSuggestions": []}]
                }
            ]
        }"#;
        let m = parse_matches(text, body).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].offset, 45);
        assert_eq!(m[0].word(text), "erreur");
    }

    #[test]
    fn sentence_from_paragraph() {
        let text = "Bonjour.";
        let body = r#"{"data": [{"iParagraph": 1, "sParagraph": "Bonjour.", "lGrammarErrors": [], "lSpellingErrors": []}]}"#;
        let m = parse_matches(text, body).unwrap();
        assert!(m.is_empty());
    }

    #[test]
    fn sentence_field_uses_paragraph() {
        let text = "Une phrase simple.";
        let body = r#"{
            "data": [{
                "iParagraph": 1,
                "sParagraph": "Une phrase simple.",
                "lGrammarErrors": [],
                "lSpellingErrors": [{"nStart": 4, "nEnd": 10, "aSuggestions": []}]
            }]
        }"#;
        let m = parse_matches(text, body).unwrap();
        assert_eq!(m[0].sentence, "Une phrase simple.");
    }

    #[test]
    fn parse_empty_and_invalid() {
        assert!(parse_matches("ok", r#"{"data": []}"#).unwrap().is_empty());
        assert!(parse_matches("ok", "<html>502</html>")
            .unwrap_err()
            .contains("invalide"));
    }
}
