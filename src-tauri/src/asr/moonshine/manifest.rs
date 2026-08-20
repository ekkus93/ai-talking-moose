use super::ffi::MOONSHINE_HEADER_VERSION;
use super::runtime::MoonshineModelArchitecture;
use std::collections::HashSet;

pub(crate) const MOONSHINE_RUNTIME_RELEASE: &str = "v0.1.3";
pub(crate) const MOONSHINE_RUNTIME_COMMIT: &str = "db88bffd14574212b6094a2e230d4f328029c31b";
pub(crate) const MOONSHINE_ASSET_ARCHIVE_REPOSITORY: &str = "moonshine-ai/moonshine-voice-assets";
pub(crate) const MOONSHINE_ASSET_ARCHIVE_COMMIT: &str = "35d84fc0eb2d7451da9973c990e8a77066abb105";
pub(crate) const MOONSHINE_MODEL_REVISION: &str = "quantized_26_07_30";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MoonshineModelFile {
    pub name: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
    pub upstream_crc32c_base64: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MoonshineModelProvenance {
    pub source_repository: &'static str,
    pub source_commit: &'static str,
    pub asset_archive_repository: &'static str,
    pub asset_archive_commit: &'static str,
    pub license: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MoonshineRuntimeCompatibility {
    pub release: &'static str,
    pub source_commit: &'static str,
    pub c_header_version: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MoonshineModelManifest {
    pub id: &'static str,
    pub display_name: &'static str,
    pub architecture: MoonshineModelArchitecture,
    pub revision: &'static str,
    pub base_url: &'static str,
    pub expected_bytes: u64,
    pub runtime: MoonshineRuntimeCompatibility,
    pub provenance: MoonshineModelProvenance,
    pub files: &'static [MoonshineModelFile],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManifestValidationError {
    EmptyModelId,
    EmptyDisplayName,
    ArchitectureMismatch,
    InsecureBaseUrl,
    RevisionNotPinnedInUrl,
    NoFiles,
    InvalidFileName(&'static str),
    DuplicateFileName(&'static str),
    InvalidFileSize(&'static str),
    InvalidSha256(&'static str),
    MissingCrc32c(&'static str),
    ExpectedBytesMismatch { declared: u64, computed: u64 },
    RuntimeHeaderMismatch { manifest: i32, ffi: i32 },
    MissingProvenance,
}

impl MoonshineModelManifest {
    pub(crate) fn validate(&self) -> Result<(), ManifestValidationError> {
        if self.id.is_empty() {
            return Err(ManifestValidationError::EmptyModelId);
        }
        if self.display_name.is_empty() {
            return Err(ManifestValidationError::EmptyDisplayName);
        }
        let architecture_matches = match self.architecture {
            MoonshineModelArchitecture::TinyStreaming => {
                self.id == "moonshine-tiny-streaming-en"
                    && self.base_url.contains("/tiny-streaming-en/")
            }
            MoonshineModelArchitecture::SmallStreaming => {
                self.id == "moonshine-small-streaming-en"
                    && self.base_url.contains("/small-streaming-en/")
            }
        };
        if !architecture_matches {
            return Err(ManifestValidationError::ArchitectureMismatch);
        }
        if !self.base_url.starts_with("https://") {
            return Err(ManifestValidationError::InsecureBaseUrl);
        }
        if self.revision.is_empty() || !self.base_url.contains(self.revision) {
            return Err(ManifestValidationError::RevisionNotPinnedInUrl);
        }
        if self.files.is_empty() {
            return Err(ManifestValidationError::NoFiles);
        }
        if self.runtime.c_header_version != MOONSHINE_HEADER_VERSION {
            return Err(ManifestValidationError::RuntimeHeaderMismatch {
                manifest: self.runtime.c_header_version,
                ffi: MOONSHINE_HEADER_VERSION,
            });
        }
        if self.runtime.release.is_empty()
            || self.runtime.source_commit.len() != 40
            || self.provenance.source_repository.is_empty()
            || self.provenance.source_commit.len() != 40
            || self.provenance.asset_archive_repository.is_empty()
            || self.provenance.asset_archive_commit.len() != 40
            || self.provenance.license.is_empty()
        {
            return Err(ManifestValidationError::MissingProvenance);
        }

        let mut names = HashSet::with_capacity(self.files.len());
        let mut computed_bytes = 0_u64;
        for file in self.files {
            if file.name.is_empty()
                || file.name.contains('/')
                || file.name.contains('\\')
                || file.name == "."
                || file.name == ".."
            {
                return Err(ManifestValidationError::InvalidFileName(file.name));
            }
            if !names.insert(file.name) {
                return Err(ManifestValidationError::DuplicateFileName(file.name));
            }
            if file.bytes == 0 {
                return Err(ManifestValidationError::InvalidFileSize(file.name));
            }
            if file.sha256.len() != 64 || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(ManifestValidationError::InvalidSha256(file.name));
            }
            if file.upstream_crc32c_base64.is_empty() {
                return Err(ManifestValidationError::MissingCrc32c(file.name));
            }
            computed_bytes = computed_bytes.saturating_add(file.bytes);
        }

        if computed_bytes != self.expected_bytes {
            return Err(ManifestValidationError::ExpectedBytesMismatch {
                declared: self.expected_bytes,
                computed: computed_bytes,
            });
        }
        Ok(())
    }
}

const RUNTIME_COMPATIBILITY: MoonshineRuntimeCompatibility = MoonshineRuntimeCompatibility {
    release: MOONSHINE_RUNTIME_RELEASE,
    source_commit: MOONSHINE_RUNTIME_COMMIT,
    c_header_version: MOONSHINE_HEADER_VERSION,
};

const TINY_PROVENANCE: MoonshineModelProvenance = MoonshineModelProvenance {
    source_repository: "UsefulSensors/moonshine-streaming-tiny",
    source_commit: "f8e9dfd8c562c257c151a907b7b7f2fe8ff8511a",
    asset_archive_repository: MOONSHINE_ASSET_ARCHIVE_REPOSITORY,
    asset_archive_commit: MOONSHINE_ASSET_ARCHIVE_COMMIT,
    license: "MIT",
};

const SMALL_PROVENANCE: MoonshineModelProvenance = MoonshineModelProvenance {
    source_repository: "UsefulSensors/moonshine-streaming-small",
    source_commit: "2c036506f23a09c18df5a50057599ba6d9280999",
    asset_archive_repository: MOONSHINE_ASSET_ARCHIVE_REPOSITORY,
    asset_archive_commit: MOONSHINE_ASSET_ARCHIVE_COMMIT,
    license: "MIT",
};

const TINY_FILES: [MoonshineModelFile; 7] = [
    MoonshineModelFile {
        name: "adapter.ort",
        bytes: 1_319_664,
        sha256: "22ecc949e146c49667fda28d102d4e30749a107dc88a396292aa8f277ef1347c",
        upstream_crc32c_base64: "kwQ+Bw==",
    },
    MoonshineModelFile {
        name: "cross_kv.ort",
        bytes: 1_287_544,
        sha256: "143a36667b8d05fd9d04e8c337b7ee121f37ef299aea6b3d82bdb3d3401950b4",
        upstream_crc32c_base64: "76wzFQ==",
    },
    MoonshineModelFile {
        name: "decoder_kv.ort",
        bytes: 32_583_720,
        sha256: "8852553f312adb6c9aa4d17418015049b30f412209ee569d336548c0044627de",
        upstream_crc32c_base64: "KJjeNw==",
    },
    MoonshineModelFile {
        name: "encoder.ort",
        bytes: 7_675_440,
        sha256: "a8414e1a5dedf9f2093d7680601dd8a9b0433e7020260eafe0e370ead91134ca",
        upstream_crc32c_base64: "UjAIpQ==",
    },
    MoonshineModelFile {
        name: "frontend.ort",
        bytes: 8_324_920,
        sha256: "271a563251f11e6311949530f8025ed4d345c5d69d4ac1efa74093779927d636",
        upstream_crc32c_base64: "AW9qsg==",
    },
    MoonshineModelFile {
        name: "streaming_config.json",
        bytes: 509,
        sha256: "74fe5ddebd63b17caf59e8a3b18c17547ff7bce1642050edbb1c3962674f8950",
        upstream_crc32c_base64: "HGL0Ug==",
    },
    MoonshineModelFile {
        name: "tokenizer.bin",
        bytes: 249_974,
        sha256: "6884b35fd6377d4c4d32336a0bc152f36b64d1e45b6503683cdc238250a8472d",
        upstream_crc32c_base64: "B7s10Q==",
    },
];

const SMALL_FILES: [MoonshineModelFile; 7] = [
    MoonshineModelFile {
        name: "adapter.ort",
        bytes: 2_870_368,
        sha256: "c665f742364febad597cc9ac1e0b341ffbee0e24a1466e2f3bde95e6e4771762",
        upstream_crc32c_base64: "XlWjTg==",
    },
    MoonshineModelFile {
        name: "cross_kv.ort",
        bytes: 5_356_536,
        sha256: "e2d3417144e9514055ebfefe8dcc4c0a55a55adcb8530435844c75c53e352bf6",
        upstream_crc32c_base64: "5ySK8g==",
    },
    MoonshineModelFile {
        name: "decoder_kv.ort",
        bytes: 81_878_600,
        sha256: "1a05465b1dd955858dfcbee039c0020fb5dd982b0f5094c34e61735d518d771b",
        upstream_crc32c_base64: "Rn/VUA==",
    },
    MoonshineModelFile {
        name: "encoder.ort",
        bytes: 44_148_576,
        sha256: "2d4d973e91e8aca08c51e7e7efa28a46ab265b63d809d5294d18b86bcd85b993",
        upstream_crc32c_base64: "41D2jQ==",
    },
    MoonshineModelFile {
        name: "frontend.ort",
        bytes: 30_984_520,
        sha256: "d1be8145bc9ef3e8625bb7bcb7a5b2930eeb828c91d6c1897bc215d7de6f2b93",
        upstream_crc32c_base64: "m6NCLA==",
    },
    MoonshineModelFile {
        name: "streaming_config.json",
        bytes: 512,
        sha256: "26f02b6afb22d60871a5efd85c3d38e569cc0ddb6c5eb6e93d3260152ae8a47a",
        upstream_crc32c_base64: "dPbFiw==",
    },
    MoonshineModelFile {
        name: "tokenizer.bin",
        bytes: 249_974,
        sha256: "6884b35fd6377d4c4d32336a0bc152f36b64d1e45b6503683cdc238250a8472d",
        upstream_crc32c_base64: "B7s10Q==",
    },
];

pub(crate) const TINY_STREAMING_MANIFEST: MoonshineModelManifest = MoonshineModelManifest {
    id: "moonshine-tiny-streaming-en",
    display_name: "Moonshine Tiny Streaming (English)",
    architecture: MoonshineModelArchitecture::TinyStreaming,
    revision: MOONSHINE_MODEL_REVISION,
    base_url: "https://download.moonshine.ai/model/tiny-streaming-en/quantized_26_07_30",
    expected_bytes: 51_441_771,
    runtime: RUNTIME_COMPATIBILITY,
    provenance: TINY_PROVENANCE,
    files: &TINY_FILES,
};

pub(crate) const SMALL_STREAMING_MANIFEST: MoonshineModelManifest = MoonshineModelManifest {
    id: "moonshine-small-streaming-en",
    display_name: "Moonshine Small Streaming (English)",
    architecture: MoonshineModelArchitecture::SmallStreaming,
    revision: MOONSHINE_MODEL_REVISION,
    base_url: "https://download.moonshine.ai/model/small-streaming-en/quantized_26_07_30",
    expected_bytes: 165_489_086,
    runtime: RUNTIME_COMPATIBILITY,
    provenance: SMALL_PROVENANCE,
    files: &SMALL_FILES,
};

pub(crate) fn manifest_for_architecture(
    architecture: MoonshineModelArchitecture,
) -> &'static MoonshineModelManifest {
    match architecture {
        MoonshineModelArchitecture::TinyStreaming => &TINY_STREAMING_MANIFEST,
        MoonshineModelArchitecture::SmallStreaming => &SMALL_STREAMING_MANIFEST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_manifests_are_structurally_valid() {
        TINY_STREAMING_MANIFEST.validate().unwrap();
        SMALL_STREAMING_MANIFEST.validate().unwrap();
    }

    #[test]
    fn manifests_pin_expected_payload_totals_and_file_sets() {
        assert_eq!(TINY_STREAMING_MANIFEST.expected_bytes, 51_441_771);
        assert_eq!(SMALL_STREAMING_MANIFEST.expected_bytes, 165_489_086);
        let expected = [
            "adapter.ort",
            "cross_kv.ort",
            "decoder_kv.ort",
            "encoder.ort",
            "frontend.ort",
            "streaming_config.json",
            "tokenizer.bin",
        ];
        assert_eq!(
            TINY_STREAMING_MANIFEST
                .files
                .iter()
                .map(|file| file.name)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            SMALL_STREAMING_MANIFEST
                .files
                .iter()
                .map(|file| file.name)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn manifests_pin_runtime_and_archive_provenance() {
        for manifest in [&TINY_STREAMING_MANIFEST, &SMALL_STREAMING_MANIFEST] {
            assert_eq!(manifest.runtime.release, "v0.1.3");
            assert_eq!(
                manifest.runtime.source_commit,
                "db88bffd14574212b6094a2e230d4f328029c31b"
            );
            assert_eq!(manifest.runtime.c_header_version, MOONSHINE_HEADER_VERSION);
            assert_eq!(
                manifest.provenance.asset_archive_repository,
                "moonshine-ai/moonshine-voice-assets"
            );
            assert_eq!(
                manifest.provenance.asset_archive_commit,
                "35d84fc0eb2d7451da9973c990e8a77066abb105"
            );
            assert_eq!(manifest.provenance.license, "MIT");
        }
    }

    #[test]
    fn file_urls_cannot_escape_the_pinned_model_directory() {
        for manifest in [&TINY_STREAMING_MANIFEST, &SMALL_STREAMING_MANIFEST] {
            for file in manifest.files {
                let url = format!("{}/{}", manifest.base_url, file.name);
                assert!(url.starts_with(manifest.base_url));
                assert!(url.starts_with("https://"));
                assert!(url.contains(MOONSHINE_MODEL_REVISION));
            }
        }
    }

    #[test]
    fn invalid_sha_is_rejected() {
        const INVALID_FILE: [MoonshineModelFile; 1] = [MoonshineModelFile {
            name: "adapter.ort",
            bytes: 1,
            sha256: "not-a-sha256",
            upstream_crc32c_base64: "kwQ+Bw==",
        }];
        let manifest = MoonshineModelManifest {
            files: &INVALID_FILE,
            expected_bytes: 1,
            ..TINY_STREAMING_MANIFEST
        };
        assert_eq!(
            manifest.validate(),
            Err(ManifestValidationError::InvalidSha256("adapter.ort"))
        );
    }

    #[test]
    fn duplicate_filename_is_rejected() {
        const DUPLICATE_FILES: [MoonshineModelFile; 2] = [
            MoonshineModelFile {
                name: "adapter.ort",
                bytes: 1,
                sha256: "22ecc949e146c49667fda28d102d4e30749a107dc88a396292aa8f277ef1347c",
                upstream_crc32c_base64: "kwQ+Bw==",
            },
            MoonshineModelFile {
                name: "adapter.ort",
                bytes: 1,
                sha256: "22ecc949e146c49667fda28d102d4e30749a107dc88a396292aa8f277ef1347c",
                upstream_crc32c_base64: "kwQ+Bw==",
            },
        ];
        let manifest = MoonshineModelManifest {
            files: &DUPLICATE_FILES,
            expected_bytes: 2,
            ..TINY_STREAMING_MANIFEST
        };
        assert_eq!(
            manifest.validate(),
            Err(ManifestValidationError::DuplicateFileName("adapter.ort"))
        );
    }

    #[test]
    fn total_size_mismatch_is_rejected() {
        let manifest = MoonshineModelManifest {
            expected_bytes: TINY_STREAMING_MANIFEST.expected_bytes + 1,
            ..TINY_STREAMING_MANIFEST
        };
        assert_eq!(
            manifest.validate(),
            Err(ManifestValidationError::ExpectedBytesMismatch {
                declared: 51_441_772,
                computed: 51_441_771,
            })
        );
    }
}
