#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocalTtsPlatform {
    LinuxX86_64,
    MacosArm64,
    MacosX86_64,
}

impl LocalTtsPlatform {
    pub const ALL: [Self; 3] = [Self::LinuxX86_64, Self::MacosArm64, Self::MacosX86_64];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocalTtsArtifactKind {
    Model,
    Voices,
    G2pData,
    OnnxRuntimeArchive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTtsArtifact {
    pub kind: LocalTtsArtifactKind,
    pub filename: &'static str,
    pub source_url: &'static str,
    pub source_revision: &'static str,
    pub expected_bytes: u64,
    pub sha256: &'static str,
    pub license: &'static str,
    pub source_provenance: &'static str,
    pub purpose: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTtsPlatformArtifact {
    pub platform: LocalTtsPlatform,
    pub artifact: LocalTtsArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTtsVoiceManifest {
    pub id: &'static str,
    pub embedding_key: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTtsRuntimeCompatibility {
    pub compatibility_version: u32,
    pub adapter_contract: &'static str,
    pub ort_crate_version: &'static str,
    pub onnx_runtime_version: &'static str,
    pub g2p_crate_version: &'static str,
    pub g2p_source_revision: &'static str,
    pub inference_threads: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTtsModelManifest {
    /// Safe application-managed storage/catalog identifier. This is deliberately
    /// distinct from the persisted provider model ID, which contains `/`.
    pub id: &'static str,
    pub provider_model_id: &'static str,
    pub display_name: &'static str,
    pub family: &'static str,
    pub version: &'static str,
    pub language: &'static str,
    pub sample_rate_hz: u32,
    pub parameter_scale: &'static str,
    pub license: &'static str,
    pub model_source_revision: &'static str,
    pub voices: &'static [LocalTtsVoiceManifest],
    pub runtime: LocalTtsRuntimeCompatibility,
    pub common_artifacts: &'static [LocalTtsArtifact],
    pub platform_artifacts: &'static [LocalTtsPlatformArtifact],
}

mod catalog;
mod validation;

#[cfg(test)]
mod tests;

pub use catalog::{
    KITTEN_TTS_G2P_CRATE_VERSION, KITTEN_TTS_G2P_SOURCE_REVISION, KITTEN_TTS_INFERENCE_THREADS,
    KITTEN_TTS_MINI_0_8_MANIFEST, KITTEN_TTS_MODEL_REVISION, KITTEN_TTS_ONNX_RUNTIME_RELEASE,
    KITTEN_TTS_ONNX_RUNTIME_VERSION, KITTEN_TTS_ORT_CRATE_VERSION, KITTEN_TTS_PARAMETER_SCALE,
    KITTEN_TTS_RUNTIME_COMPATIBILITY_VERSION, KITTEN_TTS_SAMPLE_RATE_HZ, LOCAL_TTS_MODEL_CATALOG,
};
pub use validation::validate_local_tts_model_catalog;

pub fn local_tts_model_manifest(provider_model_id: &str) -> Option<&'static LocalTtsModelManifest> {
    LOCAL_TTS_MODEL_CATALOG
        .iter()
        .find(|manifest| manifest.provider_model_id == provider_model_id)
}

pub fn local_tts_platform_artifact(
    manifest: &LocalTtsModelManifest,
    platform: LocalTtsPlatform,
) -> Option<&LocalTtsArtifact> {
    manifest
        .platform_artifacts
        .iter()
        .find(|entry| entry.platform == platform)
        .map(|entry| &entry.artifact)
}
