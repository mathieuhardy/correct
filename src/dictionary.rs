use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Dictionary {
    #[serde(default)]
    pub words: Vec<String>,
}

impl Dictionary {
    pub fn is_exception(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        self.words.iter().any(|w| w.to_lowercase() == lower)
    }
}
