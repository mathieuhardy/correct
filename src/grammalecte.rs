use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;

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
    options: HashMap<String, bool>,
}

#[derive(Deserialize)]
struct GcResponse {
    data: Vec<GcParagraph>,
}

#[derive(Deserialize)]
struct GcParagraph {
    #[serde(rename = "iParagraph")]
    i_paragraph: usize,
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
    // Pas de aSuggestions dans la réponse réelle de l'API Grammalecte.
}

fn pid_file() -> std::path::PathBuf {
    std::path::PathBuf::from("/tmp/correct-gc.pid")
}

/// Tue le serveur GC tracké (PID file) s'il tourne encore. Noop si le fichier est absent.
/// Tue le process group du serveur précédent tracké par le fichier PID.
fn kill_tracked_server() {
    let path = pid_file();
    if let Ok(s) = std::fs::read_to_string(&path) {
        if let Ok(pgid) = s.trim().parse::<i32>() {
            #[cfg(unix)]
            unsafe {
                libc::killpg(pgid, libc::SIGKILL);
            }
        }
        let _ = std::fs::remove_file(&path);
    }
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
    /// Le script wrapper NixOS utilise `exec`, donc `child.kill()` atteint bien le processus Python.
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

        // Tuer le serveur orphelin du lancement précédent avant d'en spawner un nouveau.
        kill_tracked_server();

        #[cfg(unix)]
        let spawn_result = std::process::Command::new(cmd)
            .args(&args)
            .process_group(0) // Nouveau process group → PGID = PID du serveur.
            .spawn();
        #[cfg(not(unix))]
        let spawn_result = std::process::Command::new(cmd).args(&args).spawn();

        match spawn_result {
            Ok(child) => {
                let pid = child.id();
                let client = client(Some(child));
                for _ in 0..10 {
                    std::thread::sleep(Duration::from_millis(500));
                    if Self::ping(&client.http, &client.url) {
                        let _ = std::fs::write(pid_file(), pid.to_string());
                        return (client, GcStatus::Connected, None);
                    }
                }
                let _ = std::fs::remove_file(pid_file());
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

    pub fn checker(&self, options: &HashMap<String, bool>) -> GcChecker {
        GcChecker {
            url: self.url.clone(),
            http: self.http.clone(),
            options: options.clone(),
        }
    }
}

impl GcChecker {
    /// Envoie le texte à Grammalecte et retourne toutes les correspondances.
    /// Les offsets GC sont en caractères Unicode (pas de conversion UTF-16 nécessaire).
    pub fn check(&self, text: &str) -> Result<Vec<LtMatch>, String> {
        let options_json = serde_json::to_string(&self.options)
            .unwrap_or_else(|_| "{}".to_string());
        let resp = self
            .http
            .post(format!("{}/gc_text/fr", self.url))
            .form(&[("text", text), ("options", &options_json)])
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

    // Offsets de début de chaque paragraphe dans `text` (en caractères).
    // Paragraphe i (1-based dans GC) → para_starts[i - 1].
    let para_starts: Vec<usize> = {
        let mut starts = vec![0usize];
        let mut acc = 0usize;
        for c in text.chars() {
            acc += 1;
            if c == '\n' {
                starts.push(acc);
            }
        }
        starts
    };

    let mut matches = Vec::new();

    for para in gc_resp.data {
        let char_base = para_starts
            .get(para.i_paragraph.saturating_sub(1))
            .copied()
            .unwrap_or(0);

        // Sentence = texte du paragraphe reconstituée depuis l'entrée (GC ne la fournit pas).
        let para_text: String = text
            .chars()
            .skip(char_base)
            .take_while(|&c| c != '\n')
            .collect();
        let sentence = normalize_spaces(&para_text);

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
                replacements: vec![],
                misspelling: true,
                expanded: false,
                ignored: false,
                in_dictionary: false,
            });
        }
    }

    Ok(matches)
}

const GC_OPTIONS_FILE: &str = ".correct/gc_options.toml";

/// Charge les options depuis `<project>/.correct/gc_options.toml`.
/// Retourne les defaults si le fichier est absent.
pub fn load_gc_options(project: &std::path::Path) -> (HashMap<String, bool>, Option<String>) {
    let path = project.join(GC_OPTIONS_FILE);
    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<HashMap<String, bool>>(&content) {
            Ok(opts) => (opts, None),
            Err(e) => (
                default_gc_options(),
                Some(format!("Options GC invalides ({}) : {e}", path.display())),
            ),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (default_gc_options(), None),
        Err(e) => (
            default_gc_options(),
            Some(format!("Impossible de lire les options GC : {e}")),
        ),
    }
}

pub fn save_gc_options(
    project: &std::path::Path,
    options: &HashMap<String, bool>,
) -> Result<(), String> {
    let path = project.join(GC_OPTIONS_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Impossible de créer .correct/ : {e}"))?;
    }
    let content = toml::to_string_pretty(options)
        .map_err(|e| format!("Erreur de sérialisation des options GC : {e}"))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("Impossible d'écrire les options GC ({}) : {e}", path.display()))
}

pub fn default_gc_options() -> HashMap<String, bool> {
    let mut m = HashMap::new();
    m.insert("redon2".into(), true);
    m.insert("md".into(), true);
    m
}

impl Drop for GcClient {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let pgid = child.id() as i32;
            // Tuer le process group entier (serveur + tous ses workers).
            #[cfg(unix)]
            unsafe {
                libc::killpg(pgid, libc::SIGKILL);
            }
            #[cfg(not(unix))]
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_file(pid_file());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Réponse /gc_text/fr réelle pour "😀 C'est tres importent pour nous."
    // Les offsets sont en caractères Unicode : 😀 = 1 caractère.
    // Grammar (importent) : nStart=13, nEnd=22
    // Spelling (tres)     : nStart=8,  nEnd=12  (pas de aSuggestions dans l'API)
    const RESPONSE: &str = r#"{
        "program": "grammalecte-fr", "version": "2.1.1", "lang": "fr", "error": "",
        "data": [
            {
                "iParagraph": 1,
                "lGrammarErrors": [
                    {
                        "nStart": 13, "nEnd": 22,
                        "sMessage": "Accord erroné.",
                        "sRuleId": "GC_RULE_1",
                        "aSuggestions": ["important"]
                    }
                ],
                "lSpellingErrors": [
                    {"i": 2, "sType": "WORD", "sValue": "tres", "nStart": 8, "nEnd": 12}
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
        assert!(spelling.replacements.is_empty()); // pas de suggestions dans l'API GC
        assert!(spelling.misspelling);
        assert_eq!(spelling.rule_id, "SPELLING_ERROR");
        assert!(spelling.message.is_empty());
    }

    #[test]
    fn emoji_offsets_are_char_based() {
        // 😀 = 1 caractère en Unicode, 2 unités UTF-16.
        // GC donne des offsets en caractères : "tres" après "😀 " est à nStart=2.
        let text = "😀 tres";
        let body = r#"{
            "program": "grammalecte-fr", "version": "2.1.1", "lang": "fr", "error": "",
            "data": [{
                "iParagraph": 1,
                "lGrammarErrors": [],
                "lSpellingErrors": [{"i": 1, "sType": "WORD", "sValue": "tres", "nStart": 2, "nEnd": 6}]
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
        // "erreur" dans le deuxième para à nStart=25 → offset absolu = 20 + 25 = 45.
        let text = "Premier paragraphe.\nDeuxième paragraphe avec erreur.";
        let body = r#"{
            "program": "grammalecte-fr", "version": "2.1.1", "lang": "fr", "error": "",
            "data": [
                {"iParagraph": 1, "lGrammarErrors": [], "lSpellingErrors": []},
                {
                    "iParagraph": 2,
                    "lGrammarErrors": [],
                    "lSpellingErrors": [{"i": 5, "sType": "WORD", "sValue": "erreur", "nStart": 25, "nEnd": 31}]
                }
            ]
        }"#;
        let m = parse_matches(text, body).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].offset, 45);
        assert_eq!(m[0].word(text), "erreur");
    }

    #[test]
    fn sentence_field_uses_paragraph_text() {
        let text = "Une phrase simple.";
        let body = r#"{
            "program": "grammalecte-fr", "version": "2.1.1", "lang": "fr", "error": "",
            "data": [{
                "iParagraph": 1,
                "lGrammarErrors": [],
                "lSpellingErrors": [{"i": 1, "sType": "WORD", "sValue": "phrase", "nStart": 4, "nEnd": 10}]
            }]
        }"#;
        let m = parse_matches(text, body).unwrap();
        assert_eq!(m[0].sentence, "Une phrase simple.");
    }

    #[test]
    fn parse_empty_and_invalid() {
        let empty = r#"{"program":"grammalecte-fr","version":"2.1.1","lang":"fr","error":"","data":[]}"#;
        assert!(parse_matches("ok", empty).unwrap().is_empty());
        assert!(parse_matches("ok", "<html>502</html>")
            .unwrap_err()
            .contains("invalide"));
    }
}
