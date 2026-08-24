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
        if self.api_key.chars().count() <= 8 {
            return "********".to_string();
        }

        let prefix: String = self.api_key.chars().take(4).collect();
        let suffix_chars: Vec<char> = self.api_key.chars().rev().take(4).collect();
        let suffix: String = suffix_chars.into_iter().rev().collect();
        format!("{prefix}...{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_key_uses_character_boundaries_for_non_ascii_keys() {
        let auth = GoogleAuth::new("密钥ABCD1234終".to_string());
        assert_eq!(auth.masked_key(), "密钥AB...234終");
    }
}
