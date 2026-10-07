use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GeneratedCorpusIndex {
    pub corpus_id: String,
    pub generator: String,
    pub generator_version: String,
    pub acceptance_criteria: AcceptanceCriteria,
    pub fixtures: Vec<GeneratedFixture>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptanceCriteria {
    pub criteria_version: u32,
    pub positive_recall_minimum: f64,
    pub negative_false_accepts_maximum: u64,
}

#[derive(Debug, Deserialize)]
pub struct GeneratedFixture {
    pub id: String,
    pub label: String,
    pub path: String,
    pub expected_detection: bool,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct FixtureAcceptanceResult {
    pub id: String,
    pub label: String,
    pub expected_detection: bool,
    pub detected: bool,
    pub detected_score: Option<f32>,
    pub audio_ms: u64,
    pub inference_wall_time_ms: u64,
    pub real_time_factor: f64,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct WakeWordAcceptanceReport {
    pub schema_version: u32,
    pub corpus_id: String,
    pub generator: String,
    pub generator_version: String,
    pub platform: String,
    pub architecture: String,
    pub sample_rate_hz: u32,
    pub inference_threads: u16,
    pub score: f32,
    pub threshold: f32,
    pub positive_total: u64,
    pub positive_detected: u64,
    pub positive_recall: f64,
    pub negative_total: u64,
    pub negative_false_accepts: u64,
    pub process_cpu_time_ms: u64,
    pub idle_cpu_percent: f64,
    pub idle_observation_ms: u64,
    pub peak_resident_memory_bytes: Option<u64>,
    pub criteria_version: u32,
    pub positive_recall_minimum: f64,
    pub negative_false_accepts_maximum: u64,
    pub passed: bool,
    pub fixtures: Vec<FixtureAcceptanceResult>,
}
