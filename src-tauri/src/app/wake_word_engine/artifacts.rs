use super::types::{WakeWordError, WakeWordErrorKind};
use super::V1_KWS_SAMPLE_RATE_HZ;
use crate::asr::wake_word_sherpa_manifest::{
    SHERPA_KWS_KEYWORD_BYTES, SHERPA_KWS_KEYWORD_FILE, SHERPA_KWS_KEYWORD_SHA256,
    V1_SHERPA_KWS_MODEL_FILES,
};
use ring::digest::{Context, SHA256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VerifiedArtifact {
    pub(super) relative_path: &'static str,
    pub(super) bytes: u64,
    pub(super) sha256: &'static str,
    pub(super) architecture: Option<NativeArchitecture>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NativeArchitecture {
    ElfX86_64,
    MachOArm64,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const V1_RUNTIME_PLATFORM: &str = "linux-x86_64";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const V1_RUNTIME_FILES: [VerifiedArtifact; 2] = [
    VerifiedArtifact {
        relative_path: "sherpa-onnx-v1.13.8-linux-x64-shared/lib/libonnxruntime.so",
        bytes: 27_026_609,
        sha256: "4b3607aebd1784b26b6f9b20e4bd974c7ab8287043e4d095cb7d2cb40b5e566e",
        architecture: Some(NativeArchitecture::ElfX86_64),
    },
    VerifiedArtifact {
        relative_path: "sherpa-onnx-v1.13.8-linux-x64-shared/lib/libsherpa-onnx-c-api.so",
        bytes: 5_124_192,
        sha256: "b8351ca1632571ac108adbb317bcc4bf7cfe84b72690e3017316b0da3e1e344f",
        architecture: Some(NativeArchitecture::ElfX86_64),
    },
];

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const V1_RUNTIME_PLATFORM: &str = "macos-arm64";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const V1_RUNTIME_FILES: [VerifiedArtifact; 2] = [
    VerifiedArtifact {
        relative_path: "sherpa-onnx-v1.13.8-osx-arm64-shared/lib/libonnxruntime.dylib",
        bytes: 28_775_120,
        sha256: "3567d114f7299d559993e536d605a6f46d7bc9d2542004accc80ee9bf5457f0b",
        architecture: Some(NativeArchitecture::MachOArm64),
    },
    VerifiedArtifact {
        relative_path: "sherpa-onnx-v1.13.8-osx-arm64-shared/lib/libsherpa-onnx-c-api.dylib",
        bytes: 4_172_832,
        sha256: "ee098d8b419d49b92101cde3c970a333b361066eb2d79a11ab480a116552b908",
        architecture: Some(NativeArchitecture::MachOArm64),
    },
];

pub fn validate_pcm_frame(sample_rate_hz: u32, samples: &[i16]) -> Result<(), WakeWordError> {
    if sample_rate_hz != V1_KWS_SAMPLE_RATE_HZ {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidConfiguration,
            "wake KWS frame sample rate must be 16000 Hz",
            false,
        ));
    }
    if samples.is_empty() {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidConfiguration,
            "wake KWS frame must contain at least one sample",
            false,
        ));
    }
    Ok(())
}

pub(super) fn verify_model_artifacts(model_dir: &Path) -> Result<(), WakeWordError> {
    for file in V1_SHERPA_KWS_MODEL_FILES {
        verify_file_identity(
            &model_dir.join(file.name),
            file.bytes,
            file.sha256,
            WakeWordErrorKind::MissingArtifact,
            None,
        )?;
    }
    verify_file_identity(
        &model_dir.join(SHERPA_KWS_KEYWORD_FILE),
        SHERPA_KWS_KEYWORD_BYTES,
        SHERPA_KWS_KEYWORD_SHA256,
        WakeWordErrorKind::MissingArtifact,
        None,
    )
}

pub(super) fn verify_runtime_artifacts(runtime_dir: &Path) -> Result<(), WakeWordError> {
    let runtime_files = runtime_files()?;
    for file in runtime_files {
        verify_file_identity(
            &runtime_dir.join(file.relative_path),
            file.bytes,
            file.sha256,
            WakeWordErrorKind::RuntimeUnavailable,
            file.architecture,
        )?;
    }
    Ok(())
}

pub(super) fn runtime_files() -> Result<&'static [VerifiedArtifact], WakeWordError> {
    #[cfg(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    ))]
    {
        Ok(&V1_RUNTIME_FILES)
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    {
        Err(WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "Wake Word native runtime is unsupported on this platform",
            false,
        ))
    }
}

pub(super) fn runtime_platform_name() -> &'static str {
    #[cfg(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    ))]
    {
        V1_RUNTIME_PLATFORM
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64")
    )))]
    {
        "unsupported"
    }
}

pub(super) fn verify_file_identity(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
    missing_kind: WakeWordErrorKind,
    expected_architecture: Option<NativeArchitecture>,
) -> Result<(), WakeWordError> {
    let mut file = File::open(path).map_err(|_| {
        WakeWordError::sanitized(
            missing_kind.clone(),
            "missing required wake artifact",
            false,
        )
    })?;
    let metadata = file.metadata().map_err(|_| {
        WakeWordError::sanitized(
            WakeWordErrorKind::InvalidArtifact,
            "invalid wake artifact",
            false,
        )
    })?;
    if metadata.len() != expected_bytes {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidArtifact,
            "wake artifact identity mismatch",
            false,
        ));
    }

    let mut context = Context::new(&SHA256);
    let mut header = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(|_| {
            WakeWordError::sanitized(
                WakeWordErrorKind::InvalidArtifact,
                "failed to read wake artifact",
                false,
            )
        })?;
        if count == 0 {
            break;
        }
        if header.len() < 64 {
            let needed = 64 - header.len();
            header.extend_from_slice(&buffer[..count.min(needed)]);
        }
        context.update(&buffer[..count]);
    }
    let digest = hex_digest(context.finish().as_ref());
    if digest != expected_sha256 {
        return Err(WakeWordError::sanitized(
            WakeWordErrorKind::InvalidArtifact,
            "wake artifact identity mismatch",
            false,
        ));
    }
    if let Some(expected) = expected_architecture {
        verify_native_architecture(&header, expected)?;
    }
    Ok(())
}

pub(super) fn verify_native_architecture(
    header: &[u8],
    expected: NativeArchitecture,
) -> Result<(), WakeWordError> {
    let actual = if header.len() >= 20
        && &header[..4] == b"\x7fELF"
        && header.get(4..6) == Some(&[2, 1])
        && u16::from_le_bytes([header[18], header[19]]) == 62
    {
        Some(NativeArchitecture::ElfX86_64)
    } else if header.len() >= 8
        && &header[..4] == b"\xcf\xfa\xed\xfe"
        && u32::from_le_bytes([header[4], header[5], header[6], header[7]]) == 0x0100000C
    {
        Some(NativeArchitecture::MachOArm64)
    } else {
        None
    };
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(WakeWordError::sanitized(
            WakeWordErrorKind::RuntimeUnavailable,
            "native runtime architecture mismatch",
            false,
        ))
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
