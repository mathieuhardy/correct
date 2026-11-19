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
}

impl LtClient {
    pub fn new(url: String, language: String) -> Self {
        Self { url, language, child: None }
    }
}

impl Drop for LtClient {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
    }
}
