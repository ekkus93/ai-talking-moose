use super::catalog::{
    KITTEN_TTS_COMMON_ARTIFACTS, KITTEN_TTS_PLATFORM_ARTIFACTS, KITTEN_TTS_VOICES,
};
use super::validation::{
    safe_single_component, source_url_is_explicitly_pinned, validate_artifact,
    validate_artifact_sets, validate_local_tts_model_catalog_entries, validate_manifest,
    validate_voice_catalog,
};
use super::*;
use crate::ai::local_tts::DEFAULT_LOCAL_TTS_MODEL_ID;

#[test]
fn bundled_catalog_is_valid_and_default_is_present() {
    validate_local_tts_model_catalog().unwrap();
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    assert_eq!(manifest.id, "kitten-tts-mini-0-8");
    assert_eq!(manifest.model_source_revision, KITTEN_TTS_MODEL_REVISION);
    assert_eq!(manifest.family, "KittenTTS");
    assert_eq!(manifest.version, "0.8");
    assert_eq!(manifest.language, "en");
    assert_eq!(manifest.sample_rate_hz, 24_000);
    assert_eq!(manifest.parameter_scale, "80M");
    assert_eq!(manifest.license, "Apache-2.0");
    assert_eq!(manifest.runtime.compatibility_version, 1);
    assert_eq!(manifest.runtime.inference_threads, 2);
}

#[test]
fn production_voice_mapping_is_exact_and_does_not_require_upstream_config() {
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    let actual: Vec<_> = manifest
        .voices
        .iter()
        .map(|voice| (voice.id, voice.embedding_key))
        .collect();
    assert_eq!(
        actual,
        vec![
            ("Bella", "expr-voice-2-f"),
            ("Jasper", "expr-voice-2-m"),
            ("Luna", "expr-voice-3-f"),
            ("Bruno", "expr-voice-3-m"),
            ("Rosie", "expr-voice-4-f"),
            ("Hugo", "expr-voice-4-m"),
            ("Kiki", "expr-voice-5-f"),
            ("Leo", "expr-voice-5-m"),
        ]
    );
}

#[test]
fn production_common_artifact_identity_is_exact_and_minimal() {
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    assert_eq!(manifest.common_artifacts.len(), 3);

    let model = &manifest.common_artifacts[0];
    assert_eq!(model.filename, "kitten_tts_mini_v0_8.onnx");
    assert_eq!(model.expected_bytes, 78_268_016);
    assert_eq!(
        model.sha256,
        "0f5bbae4fc4800c98dbc544a87ecfa79510de2fb8222db30d12e5bfe9177df91"
    );

    let voices = &manifest.common_artifacts[1];
    assert_eq!(voices.filename, "voices.npz");
    assert_eq!(voices.expected_bytes, 3_278_902);
    assert_eq!(
        voices.sha256,
        "40ad2638952b77b7b2f30127e2608e169fc69dd256b53bd8aaa3409a33193c42"
    );

    let g2p = &manifest.common_artifacts[2];
    assert_eq!(g2p.filename, "cmudict_data.json");
    assert_eq!(g2p.expected_bytes, 3_748_042);
    assert_eq!(
        g2p.sha256,
        "f78ed3bfa40146b70b9de78a7c67ef3be6b6c11671bb2a050442f232252a3f01"
    );

    assert!(manifest
        .common_artifacts
        .iter()
        .all(|artifact| artifact.filename != "config.json"));
}

#[test]
fn production_runtime_archives_are_platform_complete_and_exact() {
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();

    let linux = local_tts_platform_artifact(manifest, LocalTtsPlatform::LinuxX86_64).unwrap();
    assert_eq!(linux.expected_bytes, 8_309_231);
    assert_eq!(
        linux.sha256,
        "1fa4dcaef22f6f7d5cd81b28c2800414350c10116f5fdd46a2160082551c5f9b"
    );

    let arm64 = local_tts_platform_artifact(manifest, LocalTtsPlatform::MacosArm64).unwrap();
    assert_eq!(arm64.expected_bytes, 9_999_931);
    assert_eq!(
        arm64.sha256,
        "b4d513ab2b26f088c66891dbbc1408166708773d7cc4163de7bdca0e9bbb7856"
    );

    let x86_64 = local_tts_platform_artifact(manifest, LocalTtsPlatform::MacosX86_64).unwrap();
    assert_eq!(x86_64.expected_bytes, 11_676_322);
    assert_eq!(
        x86_64.sha256,
        "d10359e16347b57d9959f7e80a225a5b4a66ed7d7e007274a15cae86836485a6"
    );
}

#[test]
fn catalog_rejects_duplicate_model_identities() {
    let duplicate = [KITTEN_TTS_MINI_0_8_MANIFEST, KITTEN_TTS_MINI_0_8_MANIFEST];
    let error = validate_local_tts_model_catalog_entries(&duplicate).unwrap_err();
    assert!(error.contains("duplicate Local TTS model ID"));
}

#[test]
fn manifest_validation_rejects_unsafe_model_id() {
    let mut manifest = KITTEN_TTS_MINI_0_8_MANIFEST;
    manifest.id = "../kitten";
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn artifact_set_validation_rejects_duplicate_filenames() {
    let mut duplicate_artifacts = KITTEN_TTS_COMMON_ARTIFACTS;
    duplicate_artifacts[1].filename = duplicate_artifacts[0].filename;
    assert!(validate_artifact_sets(
        KITTEN_TTS_MINI_0_8_MANIFEST.id,
        KITTEN_TTS_MODEL_REVISION,
        &duplicate_artifacts,
        &KITTEN_TTS_PLATFORM_ARTIFACTS,
    )
    .is_err());
}

#[test]
fn voice_catalog_validation_rejects_incomplete_catalog() {
    assert!(
        validate_voice_catalog(KITTEN_TTS_MINI_0_8_MANIFEST.id, &KITTEN_TTS_VOICES[..7],).is_err()
    );
}

#[test]
fn artifact_validation_rejects_unsafe_or_moving_sources() {
    let mut artifact = KITTEN_TTS_COMMON_ARTIFACTS[0];
    artifact.filename = "../model.onnx";
    assert!(validate_artifact(&artifact).is_err());

    artifact = KITTEN_TTS_COMMON_ARTIFACTS[0];
    artifact.source_url = "https://huggingface.co/example/resolve/main/model.onnx";
    artifact.source_revision = "main";
    assert!(validate_artifact(&artifact).is_err());

    artifact = KITTEN_TTS_COMMON_ARTIFACTS[0];
    artifact.source_url = "http://example.invalid/model.onnx";
    assert!(validate_artifact(&artifact).is_err());
}

#[test]
fn catalog_rejects_path_traversal_and_unpinned_sources() {
    for unsafe_value in [
        "",
        ".",
        "..",
        "../model.onnx",
        "dir/model.onnx",
        "/tmp/model.onnx",
        "C:model.onnx",
        "model name.onnx",
    ] {
        assert!(!safe_single_component(unsafe_value), "{unsafe_value}");
    }
    assert!(safe_single_component("model.onnx"));

    assert!(!source_url_is_explicitly_pinned(
        "http://example.invalid/model.onnx",
        "deadbeef"
    ));
    assert!(!source_url_is_explicitly_pinned(
        "https://huggingface.co/example/resolve/main/model.onnx",
        "main"
    ));
    assert!(!source_url_is_explicitly_pinned(
        "https://example.invalid/latest/model.onnx",
        "latest"
    ));
    assert!(!source_url_is_explicitly_pinned(
        "https://example.invalid/releases/main/model.onnx",
        "main"
    ));
    assert!(!source_url_is_explicitly_pinned(
        "https://example.invalid/releases/master/model.onnx",
        "master"
    ));
    assert!(!safe_single_component("dir\\model.onnx"));
}

#[test]
fn catalog_declares_runtime_and_license_provenance_for_every_download() {
    let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
    for artifact in manifest.common_artifacts.iter().chain(
        manifest
            .platform_artifacts
            .iter()
            .map(|entry| &entry.artifact),
    ) {
        assert!(artifact.source_url.starts_with("https://"));
        assert!(artifact.source_url.contains(artifact.source_revision));
        assert!(!artifact.license.is_empty());
        assert!(!artifact.source_provenance.is_empty());
        assert!(!artifact.purpose.is_empty());
    }
}
