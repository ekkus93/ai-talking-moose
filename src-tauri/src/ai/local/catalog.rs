use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path};

pub const DEFAULT_LOCAL_TEXT_MODEL_ID: &str = "smollm2-360m-instruct-q4-k-m";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalModelTemplateHint {
    SmolLm2,
    Qwen3NonThinking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LocalModelCatalogEntry {
    pub id: &'static str,
    pub display_name: &'static str,
    pub family: &'static str,
    pub parameter_scale: &'static str,
    pub quantization: &'static str,
    pub artifact_filename: &'static str,
    pub source_url: &'static str,
    pub revision: &'static str,
    pub expected_bytes: u64,
    pub sha256: &'static str,
    pub license: &'static str,
    pub context_limit: u32,
    pub recommended_max_output: u32,
    pub template_hint: LocalModelTemplateHint,
}

pub const LOCAL_MODEL_CATALOG: &[LocalModelCatalogEntry] = &[
    LocalModelCatalogEntry {
        id: DEFAULT_LOCAL_TEXT_MODEL_ID,
        display_name: "SmolLM2 360M Instruct (Q4_K_M)",
        family: "SmolLM2",
        parameter_scale: "360M",
        quantization: "Q4_K_M",
        artifact_filename: "SmolLM2-360M-Instruct-Q4_K_M.gguf",
        source_url: "https://huggingface.co/bartowski/SmolLM2-360M-Instruct-GGUF/resolve/ab928a97ee49f3a015f35194879f68211291d6ca/SmolLM2-360M-Instruct-Q4_K_M.gguf?download=true",
        revision: "ab928a97ee49f3a015f35194879f68211291d6ca",
        expected_bytes: 270_590_880,
        sha256: "2fa3f013dcdd7b99f9b237717fa0b12d75bbb89984cc1274be1471a465bac9c2",
        license: "Apache-2.0",
        context_limit: 8_192,
        recommended_max_output: 192,
        template_hint: LocalModelTemplateHint::SmolLm2,
    },
    LocalModelCatalogEntry {
        id: "qwen3-0-6b-instruct-q4-k-m",
        display_name: "Qwen3 0.6B (Q4_K_M, non-thinking)",
        family: "Qwen3",
        parameter_scale: "0.6B",
        quantization: "Q4_K_M",
        artifact_filename: "Qwen_Qwen3-0.6B-Q4_K_M.gguf",
        source_url: "https://huggingface.co/bartowski/Qwen_Qwen3-0.6B-GGUF/resolve/7bcae0bc7b0606f1e948f8cdb31b98a2c10635db/Qwen_Qwen3-0.6B-Q4_K_M.gguf?download=true",
        revision: "7bcae0bc7b0606f1e948f8cdb31b98a2c10635db",
        expected_bytes: 484_220_320,
        sha256: "9acfc1e001311f34b4252001b626f2e466d592a42065f66571bff3790d4e1b14",
        license: "Apache-2.0",
        context_limit: 32_768,
        recommended_max_output: 192,
        template_hint: LocalModelTemplateHint::Qwen3NonThinking,
    },
];

pub fn local_model_entry(model_id: &str) -> Option<&'static LocalModelCatalogEntry> {
    LOCAL_MODEL_CATALOG
        .iter()
        .find(|entry| entry.id == model_id)
}

fn safe_single_component(value: &str) -> bool {
    if value.is_empty() || value == "." || value == ".." {
        return false;
    }
    let mut components = Path::new(value).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn pinned_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn validate_local_model_catalog() -> Result<(), String> {
    let mut ids = HashSet::new();
    for entry in LOCAL_MODEL_CATALOG {
        if !ids.insert(entry.id) {
            return Err(format!("duplicate local model ID: {}", entry.id));
        }
        if !safe_single_component(entry.id) {
            return Err(format!("unsafe local model ID: {}", entry.id));
        }
        if !safe_single_component(entry.artifact_filename)
            || !entry.artifact_filename.ends_with(".gguf")
        {
            return Err(format!(
                "unsafe local model artifact filename: {}",
                entry.artifact_filename
            ));
        }
        if !entry.source_url.starts_with("https://") {
            return Err(format!("local model source must use HTTPS: {}", entry.id));
        }
        if !entry.source_url.contains(entry.revision) || !pinned_revision(entry.revision) {
            return Err(format!(
                "local model revision is not immutably pinned: {}",
                entry.id
            ));
        }
        if entry.expected_bytes == 0 {
            return Err(format!(
                "local model expected byte count is zero: {}",
                entry.id
            ));
        }
        if !valid_sha256(entry.sha256) {
            return Err(format!("invalid local model SHA-256: {}", entry.id));
        }
        if entry.license.trim().is_empty() || entry.family.trim().is_empty() {
            return Err(format!("local model metadata is incomplete: {}", entry.id));
        }
        if entry.context_limit == 0 || entry.recommended_max_output == 0 {
            return Err(format!(
                "local model runtime bounds are invalid: {}",
                entry.id
            ));
        }
    }

    if local_model_entry(DEFAULT_LOCAL_TEXT_MODEL_ID).is_none() {
        return Err("default local text model is missing from catalog".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_is_valid_and_default_is_present() {
        validate_local_model_catalog().unwrap();
        let default = local_model_entry(DEFAULT_LOCAL_TEXT_MODEL_ID).unwrap();
        assert_eq!(default.quantization, "Q4_K_M");
        assert!(default.source_url.contains(default.revision));
    }

    #[test]
    fn path_component_validation_rejects_traversal_and_absolute_paths() {
        for unsafe_value in [
            "",
            ".",
            "..",
            "../model.gguf",
            "dir/model.gguf",
            "/tmp/model.gguf",
        ] {
            assert!(!safe_single_component(unsafe_value), "{unsafe_value}");
        }
        assert!(safe_single_component("model.gguf"));
        assert!(safe_single_component("smollm2-360m"));
    }

    #[test]
    fn bundled_artifact_identity_is_exact_not_rounded_metadata() {
        let smol = local_model_entry(DEFAULT_LOCAL_TEXT_MODEL_ID).unwrap();
        assert_eq!(smol.expected_bytes, 270_590_880);
        assert_eq!(
            smol.sha256,
            "2fa3f013dcdd7b99f9b237717fa0b12d75bbb89984cc1274be1471a465bac9c2"
        );
        let qwen = local_model_entry("qwen3-0-6b-instruct-q4-k-m").unwrap();
        assert_eq!(qwen.expected_bytes, 484_220_320);
        assert_eq!(
            qwen.sha256,
            "9acfc1e001311f34b4252001b626f2e466d592a42065f66571bff3790d4e1b14"
        );
        assert_eq!(qwen.template_hint, LocalModelTemplateHint::Qwen3NonThinking);
    }
}
