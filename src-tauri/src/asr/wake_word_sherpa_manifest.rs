use std::collections::HashSet;

pub const SHERPA_KWS_MODEL_ID: &str = "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01";
pub const SHERPA_KWS_MODEL_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2";
pub const SHERPA_KWS_MODEL_LICENSE: &str = "Apache-2.0";

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
            if !names.insert(file.name) {
                return Err(SherpaKwsManifestError::DuplicateFileName(file.name));
            }
            if file.bytes == 0 || !valid_sha256(file.sha256) {
                return Err(SherpaKwsManifestError::InvalidFileIdentity(file.name));
            }
        }
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

// Production remains deliberately unavailable until the exact upstream archive and consumed
// fp32 files have been independently hashed. This fail-closed placeholder prevents model-family
// selection from silently becoming permission to consume mutable/unverified bytes.
pub const V1_SHERPA_KWS_MODEL_MANIFEST: SherpaKwsModelManifest = SherpaKwsModelManifest {
    id: SHERPA_KWS_MODEL_ID,
    archive_url: SHERPA_KWS_MODEL_ARCHIVE_URL,
    archive_bytes: 0,
    archive_sha256: "",
    license: SHERPA_KWS_MODEL_LICENSE,
    files: &[],
};

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const FILES: [SherpaKwsModelFile; 5] = [
        SherpaKwsModelFile {
            name: "encoder.onnx",
            bytes: 1,
            sha256: HASH,
        },
        SherpaKwsModelFile {
            name: "decoder.onnx",
            bytes: 2,
            sha256: HASH,
        },
        SherpaKwsModelFile {
            name: "joiner.onnx",
            bytes: 3,
            sha256: HASH,
        },
        SherpaKwsModelFile {
            name: "tokens.txt",
            bytes: 4,
            sha256: HASH,
        },
        SherpaKwsModelFile {
            name: "bpe.model",
            bytes: 5,
            sha256: HASH,
        },
    ];

    fn valid_manifest() -> SherpaKwsModelManifest {
        SherpaKwsModelManifest {
            id: SHERPA_KWS_MODEL_ID,
            archive_url: SHERPA_KWS_MODEL_ARCHIVE_URL,
            archive_bytes: 15,
            archive_sha256: HASH,
            license: SHERPA_KWS_MODEL_LICENSE,
            files: &FILES,
        }
    }

    #[test]
    fn qualified_shape_validates() {
        assert_eq!(valid_manifest().validate(), Ok(()));
    }

    #[test]
    fn production_placeholder_fails_closed_until_hashes_are_recorded() {
        assert_eq!(
            V1_SHERPA_KWS_MODEL_MANIFEST.validate(),
            Err(SherpaKwsManifestError::MissingArchiveIdentity)
        );
    }

    #[test]
    fn mutable_or_insecure_sources_are_rejected() {
        let mut manifest = valid_manifest();
        manifest.archive_url = "https://example.invalid/latest/model.tar.bz2";
        assert_eq!(
            manifest.validate(),
            Err(SherpaKwsManifestError::MutableOrInsecureSource)
        );
    }

    #[test]
    fn duplicate_and_unhashed_files_are_rejected() {
        const DUPLICATES: [SherpaKwsModelFile; 2] = [
            SherpaKwsModelFile {
                name: "tokens.txt",
                bytes: 1,
                sha256: HASH,
            },
            SherpaKwsModelFile {
                name: "tokens.txt",
                bytes: 1,
                sha256: HASH,
            },
        ];
        let mut manifest = valid_manifest();
        manifest.files = &DUPLICATES;
        assert_eq!(
            manifest.validate(),
            Err(SherpaKwsManifestError::DuplicateFileName("tokens.txt"))
        );

        const UNHASHED: [SherpaKwsModelFile; 1] = [SherpaKwsModelFile {
            name: "tokens.txt",
            bytes: 1,
            sha256: "",
        }];
        manifest.files = &UNHASHED;
        assert_eq!(
            manifest.validate(),
            Err(SherpaKwsManifestError::InvalidFileIdentity("tokens.txt"))
        );
    }
}
