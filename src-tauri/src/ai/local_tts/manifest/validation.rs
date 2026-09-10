use super::catalog::{
    KITTEN_TTS_G2P_CRATE_VERSION, KITTEN_TTS_G2P_SOURCE_REVISION, KITTEN_TTS_ONNX_RUNTIME_RELEASE,
    KITTEN_TTS_ONNX_RUNTIME_VERSION, KITTEN_TTS_ORT_CRATE_VERSION,
    KITTEN_TTS_RUNTIME_COMPATIBILITY_VERSION,
};
use super::{
    local_tts_model_manifest, LocalTtsArtifact, LocalTtsArtifactKind, LocalTtsModelManifest,
    LocalTtsPlatform, LocalTtsPlatformArtifact, LocalTtsVoiceManifest, LOCAL_TTS_MODEL_CATALOG,
};
use crate::ai::local_tts::{
    DEFAULT_LOCAL_TTS_MODEL_ID, DEFAULT_LOCAL_TTS_VOICE, LOCAL_TTS_VOICE_IDS,
};
use std::collections::HashSet;
use std::path::{Component, Path};

pub(super) fn safe_single_component(value: &str) -> bool {
    if value.is_empty()
        || value == "."
        || value == ".."
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return false;
    }
    let mut components = Path::new(value).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn pinned_git_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn source_url_is_explicitly_pinned(url: &str, source_revision: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let revision = source_revision.to_ascii_lowercase();
    let moving_revision = matches!(revision.as_str(), "latest" | "main" | "master");
    url.starts_with("https://")
        && !moving_revision
        && !lower.contains("/latest/")
        && !lower.contains("/resolve/main/")
        && !lower.contains("/resolve/master/")
        && !lower.contains("/raw/main/")
        && !lower.contains("/raw/master/")
        && !source_revision.is_empty()
        && url.contains(source_revision)
}

pub(super) fn validate_artifact(artifact: &LocalTtsArtifact) -> Result<(), String> {
    if !safe_single_component(artifact.filename) {
        return Err(format!(
            "unsafe Local TTS artifact filename: {}",
            artifact.filename
        ));
    }
    if !source_url_is_explicitly_pinned(artifact.source_url, artifact.source_revision) {
        return Err(format!(
            "Local TTS artifact source is not pinned: {}",
            artifact.filename
        ));
    }
    if artifact.expected_bytes == 0 {
        return Err(format!(
            "Local TTS artifact byte count is zero: {}",
            artifact.filename
        ));
    }
    if !valid_sha256(artifact.sha256) {
        return Err(format!(
            "invalid Local TTS artifact SHA-256: {}",
            artifact.filename
        ));
    }
    if artifact.license.trim().is_empty()
        || artifact.source_provenance.trim().is_empty()
        || artifact.purpose.trim().is_empty()
    {
        return Err(format!(
            "incomplete Local TTS artifact provenance: {}",
            artifact.filename
        ));
    }
    Ok(())
}

pub(super) fn validate_voice_catalog(
    manifest_id: &str,
    voices: &[LocalTtsVoiceManifest],
) -> Result<(), String> {
    if voices.len() != LOCAL_TTS_VOICE_IDS.len() {
        return Err(format!(
            "Local TTS voice catalog is incomplete: {manifest_id}"
        ));
    }

    let mut voice_ids = HashSet::new();
    let mut embedding_keys = HashSet::new();
    for voice in voices {
        if !safe_single_component(voice.id) || !safe_single_component(voice.embedding_key) {
            return Err(format!("unsafe Local TTS voice identity: {}", voice.id));
        }
        if !voice_ids.insert(voice.id) {
            return Err(format!("duplicate Local TTS voice ID: {}", voice.id));
        }
        if !embedding_keys.insert(voice.embedding_key) {
            return Err(format!(
                "duplicate Local TTS voice embedding key: {}",
                voice.embedding_key
            ));
        }
    }

    if !LOCAL_TTS_VOICE_IDS
        .iter()
        .all(|voice_id| voices.iter().any(|voice| voice.id == *voice_id))
        || !voices
            .iter()
            .any(|voice| voice.id == DEFAULT_LOCAL_TTS_VOICE)
    {
        return Err(format!("Local TTS voice catalog drift: {manifest_id}"));
    }

    Ok(())
}

pub(super) fn validate_artifact_sets(
    manifest_id: &str,
    model_source_revision: &str,
    common_artifacts: &[LocalTtsArtifact],
    platform_artifacts: &[LocalTtsPlatformArtifact],
) -> Result<(), String> {
    if common_artifacts.len() != 3 || platform_artifacts.len() != 3 {
        return Err(format!(
            "Local TTS manifest artifact set is incomplete: {manifest_id}"
        ));
    }

    let mut filenames = HashSet::new();
    let mut common_kinds = HashSet::new();
    for artifact in common_artifacts {
        validate_artifact(artifact)?;
        match artifact.kind {
            LocalTtsArtifactKind::Model | LocalTtsArtifactKind::Voices
                if artifact.source_revision != model_source_revision =>
            {
                return Err(format!(
                    "Local TTS model artifact revision mismatch: {}",
                    artifact.filename
                ));
            }
            LocalTtsArtifactKind::G2pData
                if artifact.source_revision != KITTEN_TTS_G2P_SOURCE_REVISION =>
            {
                return Err(format!(
                    "Local TTS G2P artifact revision mismatch: {}",
                    artifact.filename
                ));
            }
            LocalTtsArtifactKind::OnnxRuntimeArchive => {
                return Err(format!(
                    "runtime archive cannot be a common Local TTS artifact: {}",
                    artifact.filename
                ));
            }
            _ => {}
        }
        if !filenames.insert(artifact.filename) {
            return Err(format!(
                "duplicate Local TTS artifact filename: {}",
                artifact.filename
            ));
        }
        if !common_kinds.insert(artifact.kind) {
            return Err(format!(
                "duplicate Local TTS common artifact kind: {:?}",
                artifact.kind
            ));
        }
    }

    for required_kind in [
        LocalTtsArtifactKind::Model,
        LocalTtsArtifactKind::Voices,
        LocalTtsArtifactKind::G2pData,
    ] {
        if !common_kinds.contains(&required_kind) {
            return Err(format!(
                "missing Local TTS common artifact kind: {required_kind:?}"
            ));
        }
    }

    let mut platforms = HashSet::new();
    for entry in platform_artifacts {
        validate_artifact(&entry.artifact)?;
        if entry.artifact.kind != LocalTtsArtifactKind::OnnxRuntimeArchive {
            return Err(format!(
                "non-runtime artifact used as platform runtime: {}",
                entry.artifact.filename
            ));
        }
        if entry.artifact.source_revision != KITTEN_TTS_ONNX_RUNTIME_RELEASE {
            return Err(format!(
                "Local TTS ONNX Runtime release mismatch: {}",
                entry.artifact.filename
            ));
        }
        if !filenames.insert(entry.artifact.filename) {
            return Err(format!(
                "duplicate Local TTS artifact filename: {}",
                entry.artifact.filename
            ));
        }
        if !platforms.insert(entry.platform) {
            return Err(format!(
                "duplicate Local TTS runtime platform: {:?}",
                entry.platform
            ));
        }
    }

    for platform in LocalTtsPlatform::ALL {
        if !platforms.contains(&platform) {
            return Err(format!("missing Local TTS runtime platform: {platform:?}"));
        }
    }

    Ok(())
}

pub(super) fn validate_manifest(manifest: &LocalTtsModelManifest) -> Result<(), String> {
    if !safe_single_component(manifest.id) {
        return Err(format!("unsafe Local TTS model ID: {}", manifest.id));
    }
    if manifest.provider_model_id.trim().is_empty()
        || manifest.display_name.trim().is_empty()
        || manifest.family.trim().is_empty()
        || manifest.version.trim().is_empty()
        || manifest.language.trim().is_empty()
        || manifest.parameter_scale.trim().is_empty()
        || manifest.license.trim().is_empty()
        || manifest.sample_rate_hz == 0
    {
        return Err(format!(
            "incomplete Local TTS model identity: {}",
            manifest.id
        ));
    }
    if !pinned_git_revision(manifest.model_source_revision) {
        return Err(format!(
            "Kitten model revision is not immutably pinned: {}",
            manifest.id
        ));
    }
    if manifest.runtime.compatibility_version != KITTEN_TTS_RUNTIME_COMPATIBILITY_VERSION
        || manifest.runtime.adapter_contract.trim().is_empty()
        || manifest.runtime.ort_crate_version != KITTEN_TTS_ORT_CRATE_VERSION
        || manifest.runtime.onnx_runtime_version != KITTEN_TTS_ONNX_RUNTIME_VERSION
        || manifest.runtime.g2p_crate_version != KITTEN_TTS_G2P_CRATE_VERSION
        || manifest.runtime.g2p_source_revision != KITTEN_TTS_G2P_SOURCE_REVISION
        || manifest.runtime.inference_threads == 0
    {
        return Err(format!(
            "invalid Local TTS runtime compatibility: {}",
            manifest.id
        ));
    }

    validate_voice_catalog(manifest.id, manifest.voices)?;
    validate_artifact_sets(
        manifest.id,
        manifest.model_source_revision,
        manifest.common_artifacts,
        manifest.platform_artifacts,
    )?;

    Ok(())
}

pub(super) fn validate_local_tts_model_catalog_entries(
    catalog: &[LocalTtsModelManifest],
) -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut provider_model_ids = HashSet::new();
    for manifest in catalog {
        if !ids.insert(manifest.id) {
            return Err(format!("duplicate Local TTS model ID: {}", manifest.id));
        }
        if !provider_model_ids.insert(manifest.provider_model_id) {
            return Err(format!(
                "duplicate Local TTS provider model ID: {}",
                manifest.provider_model_id
            ));
        }
        validate_manifest(manifest)?;
    }
    Ok(())
}

pub fn validate_local_tts_model_catalog() -> Result<(), String> {
    validate_local_tts_model_catalog_entries(LOCAL_TTS_MODEL_CATALOG)?;
    if local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).is_none() {
        return Err("default Local TTS model is missing from catalog".to_string());
    }
    Ok(())
}
