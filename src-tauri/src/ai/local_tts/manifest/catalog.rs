use super::{
    LocalTtsArtifact, LocalTtsArtifactKind, LocalTtsModelManifest, LocalTtsPlatform,
    LocalTtsPlatformArtifact, LocalTtsRuntimeCompatibility, LocalTtsVoiceManifest,
};
use crate::ai::local_tts::DEFAULT_LOCAL_TTS_MODEL_ID;

pub const KITTEN_TTS_MODEL_REVISION: &str = "c02725660cea441db4c383af69f1f26f5cd00947";
pub const KITTEN_TTS_G2P_SOURCE_REVISION: &str = "244ffeb44108347a514ebfc0c2f773d938c9613b";
pub const KITTEN_TTS_ORT_CRATE_VERSION: &str = "2.0.0-rc.13";
pub const KITTEN_TTS_ONNX_RUNTIME_VERSION: &str = "1.23.2";
pub const KITTEN_TTS_ONNX_RUNTIME_RELEASE: &str = "v1.23.2";
pub const KITTEN_TTS_G2P_CRATE_VERSION: &str = "0.4.0";
pub const KITTEN_TTS_INFERENCE_THREADS: u8 = 2;
pub const KITTEN_TTS_RUNTIME_COMPATIBILITY_VERSION: u32 = 1;
pub const KITTEN_TTS_SAMPLE_RATE_HZ: u32 = 24_000;
pub const KITTEN_TTS_PARAMETER_SCALE: &str = "80M";

pub const LOCAL_TTS_MODEL_CATALOG: &[LocalTtsModelManifest] = &[KITTEN_TTS_MINI_0_8_MANIFEST];

pub(super) const KITTEN_TTS_VOICES: [LocalTtsVoiceManifest; 8] = [
    LocalTtsVoiceManifest {
        id: "Bella",
        embedding_key: "expr-voice-2-f",
    },
    LocalTtsVoiceManifest {
        id: "Jasper",
        embedding_key: "expr-voice-2-m",
    },
    LocalTtsVoiceManifest {
        id: "Luna",
        embedding_key: "expr-voice-3-f",
    },
    LocalTtsVoiceManifest {
        id: "Bruno",
        embedding_key: "expr-voice-3-m",
    },
    LocalTtsVoiceManifest {
        id: "Rosie",
        embedding_key: "expr-voice-4-f",
    },
    LocalTtsVoiceManifest {
        id: "Hugo",
        embedding_key: "expr-voice-4-m",
    },
    LocalTtsVoiceManifest {
        id: "Kiki",
        embedding_key: "expr-voice-5-f",
    },
    LocalTtsVoiceManifest {
        id: "Leo",
        embedding_key: "expr-voice-5-m",
    },
];

pub(super) const KITTEN_TTS_COMMON_ARTIFACTS: [LocalTtsArtifact; 3] = [
    LocalTtsArtifact {
        kind: LocalTtsArtifactKind::Model,
        filename: "kitten_tts_mini_v0_8.onnx",
        source_url: "https://huggingface.co/KittenML/kitten-tts-mini-0.8/resolve/c02725660cea441db4c383af69f1f26f5cd00947/kitten_tts_mini_v0_8.onnx?download=true",
        source_revision: KITTEN_TTS_MODEL_REVISION,
        expected_bytes: 78_268_016,
        sha256: "0f5bbae4fc4800c98dbc544a87ecfa79510de2fb8222db30d12e5bfe9177df91",
        license: "Apache-2.0",
        source_provenance: "KittenML/kitten-tts-mini-0.8",
        purpose: "KittenTTS Mini 0.8 ONNX acoustic/vocoder model",
    },
    LocalTtsArtifact {
        kind: LocalTtsArtifactKind::Voices,
        filename: "voices.npz",
        source_url: "https://huggingface.co/KittenML/kitten-tts-mini-0.8/resolve/c02725660cea441db4c383af69f1f26f5cd00947/voices.npz?download=true",
        source_revision: KITTEN_TTS_MODEL_REVISION,
        expected_bytes: 3_278_902,
        sha256: "40ad2638952b77b7b2f30127e2608e169fc69dd256b53bd8aaa3409a33193c42",
        license: "Apache-2.0",
        source_provenance: "KittenML/kitten-tts-mini-0.8",
        purpose: "Voice style embeddings for the eight accepted KittenTTS voices",
    },
    LocalTtsArtifact {
        kind: LocalTtsArtifactKind::G2pData,
        filename: "cmudict_data.json",
        source_url: "https://raw.githubusercontent.com/ayutaz/piper-plus/244ffeb44108347a514ebfc0c2f773d938c9613b/src/rust/piper-plus-g2p/data/cmudict_data.json",
        source_revision: KITTEN_TTS_G2P_SOURCE_REVISION,
        expected_bytes: 3_748_042,
        sha256: "f78ed3bfa40146b70b9de78a7c67ef3be6b6c11671bb2a050442f232252a3f01",
        license: "BSD-style (CMU)",
        source_provenance: "CMU Pronouncing Dictionary v0.7b via ayutaz/piper-plus",
        purpose: "English word-to-ARPABET dictionary consumed by piper-plus-g2p",
    },
];

pub(super) const KITTEN_TTS_PLATFORM_ARTIFACTS: [LocalTtsPlatformArtifact; 3] = [
    LocalTtsPlatformArtifact {
        platform: LocalTtsPlatform::LinuxX86_64,
        artifact: LocalTtsArtifact {
            kind: LocalTtsArtifactKind::OnnxRuntimeArchive,
            filename: "onnxruntime-linux-x64-1.23.2.tgz",
            source_url: "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-linux-x64-1.23.2.tgz",
            source_revision: "v1.23.2",
            expected_bytes: 8_309_231,
            sha256: "1fa4dcaef22f6f7d5cd81b28c2800414350c10116f5fdd46a2160082551c5f9b",
            license: "MIT",
            source_provenance: "microsoft/onnxruntime release v1.23.2",
            purpose: "CPU ONNX Runtime dynamic library payload for Linux x86_64",
        },
    },
    LocalTtsPlatformArtifact {
        platform: LocalTtsPlatform::MacosArm64,
        artifact: LocalTtsArtifact {
            kind: LocalTtsArtifactKind::OnnxRuntimeArchive,
            filename: "onnxruntime-osx-arm64-1.23.2.tgz",
            source_url: "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-osx-arm64-1.23.2.tgz",
            source_revision: "v1.23.2",
            expected_bytes: 9_999_931,
            sha256: "b4d513ab2b26f088c66891dbbc1408166708773d7cc4163de7bdca0e9bbb7856",
            license: "MIT",
            source_provenance: "microsoft/onnxruntime release v1.23.2",
            purpose: "CPU ONNX Runtime dynamic library payload for macOS arm64",
        },
    },
    LocalTtsPlatformArtifact {
        platform: LocalTtsPlatform::MacosX86_64,
        artifact: LocalTtsArtifact {
            kind: LocalTtsArtifactKind::OnnxRuntimeArchive,
            filename: "onnxruntime-osx-x86_64-1.23.2.tgz",
            source_url: "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-osx-x86_64-1.23.2.tgz",
            source_revision: "v1.23.2",
            expected_bytes: 11_676_322,
            sha256: "d10359e16347b57d9959f7e80a225a5b4a66ed7d7e007274a15cae86836485a6",
            license: "MIT",
            source_provenance: "microsoft/onnxruntime release v1.23.2",
            purpose: "CPU ONNX Runtime dynamic library payload for macOS x86_64",
        },
    },
];

pub const KITTEN_TTS_MINI_0_8_MANIFEST: LocalTtsModelManifest = LocalTtsModelManifest {
    id: "kitten-tts-mini-0-8",
    provider_model_id: DEFAULT_LOCAL_TTS_MODEL_ID,
    display_name: "KittenTTS Mini 0.8",
    family: "KittenTTS",
    version: "0.8",
    language: "en",
    sample_rate_hz: KITTEN_TTS_SAMPLE_RATE_HZ,
    parameter_scale: KITTEN_TTS_PARAMETER_SCALE,
    license: "Apache-2.0",
    model_source_revision: KITTEN_TTS_MODEL_REVISION,
    voices: &KITTEN_TTS_VOICES,
    runtime: LocalTtsRuntimeCompatibility {
        compatibility_version: KITTEN_TTS_RUNTIME_COMPATIBILITY_VERSION,
        adapter_contract: "talking-moose-kittentts-v1",
        ort_crate_version: KITTEN_TTS_ORT_CRATE_VERSION,
        onnx_runtime_version: KITTEN_TTS_ONNX_RUNTIME_VERSION,
        g2p_crate_version: KITTEN_TTS_G2P_CRATE_VERSION,
        g2p_source_revision: KITTEN_TTS_G2P_SOURCE_REVISION,
        inference_threads: KITTEN_TTS_INFERENCE_THREADS,
    },
    common_artifacts: &KITTEN_TTS_COMMON_ARTIFACTS,
    platform_artifacts: &KITTEN_TTS_PLATFORM_ARTIFACTS,
};
