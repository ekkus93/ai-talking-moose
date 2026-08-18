#[derive(Debug, Clone)]
pub struct GoogleAuth {
    pub api_key: String,
}

impl GoogleAuth {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn is_valid(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    pub fn redact(&self, input: &str) -> String {
        crate::secrets::redact_secret(input, &self.api_key)
    }

    pub fn masked_key(&self) -> String {
        if self.api_key.len() <= 8 {
            "********".to_string()
        } else {
            let prefix = &self.api_key[..4];
            let suffix = &self.api_key[self.api_key.len() - 4..];
            format!("{}...{}", prefix, suffix)
        }
    }
}
