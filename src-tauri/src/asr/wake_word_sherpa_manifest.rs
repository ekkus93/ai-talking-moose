use std::collections::HashSet;

pub const SHERPA_KWS_MODEL_ID: &str = "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01";
pub const SHERPA_KWS_MODEL_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2";
pub const SHERPA_KWS_MODEL_LICENSE: &str = "Apache-2.0";
pub const SHERPA_KWS_ENCODER_FILE: &str = "encoder-epoch-12-avg-2-chunk-16-left-64.onnx";
pub const SHERPA_KWS_DECODER_FILE: &str = "decoder-epoch-12-avg-2-chunk-16-left-64.onnx";
pub const SHERPA_KWS_JOINER_FILE: &str = "joiner-epoch-12-avg-2-chunk-16-left-64.onnx";
pub const SHERPA_KWS_TOKENS_FILE: &str = "tokens.txt";
pub const SHERPA_KWS_BPE_FILE: &str = "bpe.model";
pub const SHERPA_KWS_KEYWORD_SOURCE: &str = "HEY MOOSE";
pub const SHERPA_KWS_KEYWORD_REPRESENTATION: &str = "▁HE Y ▁MO O SE";
pub const SHERPA_KWS_KEYWORD_FILE: &str = "hey-moose.tokens.txt";
pub const SHERPA_KWS_KEYWORD_BYTES: u64 = 19;
pub const SHERPA_KWS_KEYWORD_SHA256: &str =
    "3b1ad407b63b5e89edd8e253b9c104ac85d74a2a862b0d07ac4a0d4ee27770a6";
pub const SHERPA_KWS_REQUIRED_FILES: [&str; 5] = [
    SHERPA_KWS_ENCODER_FILE,
    SHERPA_KWS_DECODER_FILE,
    SHERPA_KWS_JOINER_FILE,
    SHERPA_KWS_TOKENS_FILE,
    SHERPA_KWS_BPE_FILE,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SherpaKwsModelFile {
    pub name: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SherpaKwsModelManifest {
    pub id: &'static str,
    pub archive_url: &'static str,
    pub archive_bytes: u64,
    pub archive_sha256: &'static str,
    pub license: &'static str,
    pub files: &'static [SherpaKwsModelFile],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SherpaKwsManifestError {
    WrongModelId,
    MutableOrInsecureSource,
    MissingLicense,
    MissingArchiveIdentity,
    NoFiles,
    InvalidFileName(&'static str),
    DuplicateFileName(&'static str),
    InvalidFileIdentity(&'static str),
    MissingRequiredFile(&'static str),
    UnexpectedFile(&'static str),
}

impl SherpaKwsModelManifest {
    pub fn validate(&self) -> Result<(), SherpaKwsManifestError> {
        if self.id != SHERPA_KWS_MODEL_ID {
            return Err(SherpaKwsManifestError::WrongModelId);
        }
        if !self
            .archive_url
            .starts_with("https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/")
            || !self
                .archive_url
                .ends_with(&format!("/{SHERPA_KWS_MODEL_ID}.tar.bz2"))
        {
            return Err(SherpaKwsManifestError::MutableOrInsecureSource);
        }
        if self.license.is_empty() {
            return Err(SherpaKwsManifestError::MissingLicense);
        }
        if self.archive_bytes == 0 || !valid_sha256(self.archive_sha256) {
            return Err(SherpaKwsManifestError::MissingArchiveIdentity);
        }
        if self.files.is_empty() {
            return Err(SherpaKwsManifestError::NoFiles);
        }
        let mut names = HashSet::with_capacity(self.files.len());
        for file in self.files {
            if file.name.is_empty()
                || file.name.contains('/')
                || file.name.contains('\\')
                || file.name == "."
                || file.name == ".."
            {
                return Err(SherpaKwsManifestError::InvalidFileName(file.name));
            }
            if !SHERPA_KWS_REQUIRED_FILES.contains(&file.name) {
                return Err(SherpaKwsManifestError::UnexpectedFile(file.name));
            }
            if !names.insert(file.name) {
                return Err(SherpaKwsManifestError::DuplicateFileName(file.name));
            }
            if file.bytes == 0 || !valid_sha256(file.sha256) {
                return Err(SherpaKwsManifestError::InvalidFileIdentity(file.name));
            }
        }
        for required in SHERPA_KWS_REQUIRED_FILES {
            if !names.contains(required) {
                return Err(SherpaKwsManifestError::MissingRequiredFile(required));
            }
        }
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub const V1_SHERPA_KWS_MODEL_FILES: [SherpaKwsModelFile; 5] = [
    SherpaKwsModelFile {
        name: SHERPA_KWS_ENCODER_FILE,
        bytes: 12174219,
        sha256: "063fbc1aeae8a9b574607a331a00e60371846ef9eaa3c1d9ea48176665dfc693",
    },
    SherpaKwsModelFile {
        name: SHERPA_KWS_DECODER_FILE,
        bytes: 1063189,
        sha256: "f61ebd3eed3773a44d088d53dfae92dbb6aec4839f4dcaee2d402414741663a3",
    },
    SherpaKwsModelFile {
        name: SHERPA_KWS_JOINER_FILE,
        bytes: 642462,
        sha256: "0d7a37e749d8055223029318d6ffae82db1dae2d315d0892a68ba5dad17c1d2d",
    },
    SherpaKwsModelFile {
        name: SHERPA_KWS_TOKENS_FILE,
        bytes: 5006,
        sha256: "fd2ded4050a55d2b1578870ba8697d02371980217806b7558bd0a5cc60f3ba53",
    },
    SherpaKwsModelFile {
        name: SHERPA_KWS_BPE_FILE,
        bytes: 244837,
        sha256: "c8a2a0129c4ab8e463164c142f82d25649661b122c8cd0b7aab5c9e80b90ad24",
    },
];

pub const V1_SHERPA_KWS_MODEL_MANIFEST: SherpaKwsModelManifest = SherpaKwsModelManifest {
    id: SHERPA_KWS_MODEL_ID,
    archive_url: SHERPA_KWS_MODEL_ARCHIVE_URL,
    archive_bytes: 17626723,
    archive_sha256: "f170013b4716e41b62b9bfd809687c207cef798ef9bc6534d524e17af9b6561a",
    license: SHERPA_KWS_MODEL_LICENSE,
    files: &V1_SHERPA_KWS_MODEL_FILES,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_manifest_is_fully_qualified() {
        assert_eq!(V1_SHERPA_KWS_MODEL_MANIFEST.validate(), Ok(()));
        assert!(valid_sha256(SHERPA_KWS_KEYWORD_SHA256));
        assert_eq!(SHERPA_KWS_KEYWORD_SOURCE, "HEY MOOSE");
        assert!(!SHERPA_KWS_KEYWORD_REPRESENTATION.is_empty());
        assert_eq!(SHERPA_KWS_KEYWORD_BYTES, 19);
    }

    #[test]
    fn json_and_rust_production_identities_cannot_drift_silently() {
        let json = include_str!("../../../wake-word-artifacts.json");
        assert!(json.contains(V1_SHERPA_KWS_MODEL_MANIFEST.archive_sha256));
        assert!(json.contains(&V1_SHERPA_KWS_MODEL_MANIFEST.archive_bytes.to_string()));
        assert!(json.contains(SHERPA_KWS_KEYWORD_SHA256));
        assert!(json.contains(SHERPA_KWS_KEYWORD_REPRESENTATION));
        for file in V1_SHERPA_KWS_MODEL_FILES {
            assert!(json.contains(file.name));
            assert!(json.contains(file.sha256));
            assert!(json.contains(&file.bytes.to_string()));
        }
    }

    #[test]
    fn mutable_or_insecure_sources_are_rejected() {
        let mut manifest = V1_SHERPA_KWS_MODEL_MANIFEST;
        manifest.archive_url = "https://example.invalid/latest/model.tar.bz2";
        assert_eq!(
            manifest.validate(),
            Err(SherpaKwsManifestError::MutableOrInsecureSource)
        );
    }

    #[test]
    fn exact_fp32_consumed_file_set_is_required() {
        let mut manifest = V1_SHERPA_KWS_MODEL_MANIFEST;
        manifest.files = &V1_SHERPA_KWS_MODEL_FILES[..4];
        assert_eq!(
            manifest.validate(),
            Err(SherpaKwsManifestError::MissingRequiredFile(
                SHERPA_KWS_BPE_FILE
            ))
        );
    }
}
