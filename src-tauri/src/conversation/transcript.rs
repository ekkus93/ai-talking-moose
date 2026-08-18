use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub role: String, // "user" or "moose"
    pub text: String,
    pub timestamp: String,
}
