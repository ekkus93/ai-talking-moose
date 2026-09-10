use super::{
    LocalTtsInferenceOutput, LocalTtsInferenceRequest, LocalTtsRuntimeEngine,
    LocalTtsRuntimeEngineFactory, LocalTtsRuntimeError, LocalTtsRuntimeIdentity,
};
use crate::ai::local_tts::manifest::{
    local_tts_model_manifest, LocalTtsModelManifest, LocalTtsPlatform,
};
use flate2::read::GzDecoder;
use ort::{ep, session::Session, value::Tensor};
use parking_lot::Mutex;
use piper_plus_g2p::{english::EnglishPhonemizer, Phonemizer};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tempfile::{Builder as TempFileBuilder, NamedTempFile};

mod normalize;
use normalize::normalize_text;
use super::npz::load_npz;
use super::tokenize::ipa_to_ids;

const VERIFIED_ARTIFACT_COUNT: usize = 4;
const MODEL_ARTIFACT_INDEX: usize = 0;
const VOICES_ARTIFACT_INDEX: usize = 1;
const G2P_ARTIFACT_INDEX: usize = 2;
const RUNTIME_ARTIFACT_INDEX: usize = 3;
const MAX_MODEL_TOKENS: usize = 512;
const MIN_SPEED: f32 = 0.25;
const MAX_SPEED: f32 = 4.0;
const MAX_RUNTIME_LIBRARY_BYTES: u64 = 256 * 1024 * 1024;
const TAR_BLOCK_BYTES: u64 = 512;

struct Voice {
    rows: usize,
    columns: usize,
    data: Vec<f32>,
}

impl Voice {
    fn style_row(&self, index: usize) -> Result<&[f32], LocalTtsRuntimeError> {
        if self.rows == 0 || self.columns == 0 || self.data.len() != self.rows * self.columns {
            return Err(LocalTtsRuntimeError::model_load());
        }
        let row = index.min(self.rows - 1);
        Ok(&self.data[row * self.columns..(row + 1) * self.columns])
    }
}

struct LoadedOrtRuntime {
    platform: LocalTtsPlatform,
    version: String,
    _library_file: NamedTempFile,
}

static ORT_RUNTIME: OnceLock<Mutex<Option<LoadedOrtRuntime>>> = OnceLock::new();

pub(super) struct KittenTtsRuntimeEngineFactory;

impl LocalTtsRuntimeEngineFactory for KittenTtsRuntimeEngineFactory {
    fn create(&self) -> Result<Box<dyn LocalTtsRuntimeEngine>, LocalTtsRuntimeError> {
        Ok(Box::new(KittenTtsRuntimeEngine::new()))
    }
}

struct KittenTtsRuntimeEngine {
    session: Option<Session>,
    voices: HashMap<String, Voice>,
    phonemizer: Option<EnglishPhonemizer>,
    model_id: Option<String>,
}

impl KittenTtsRuntimeEngine {
    fn new() -> Self {
        Self {
            session: None,
            voices: HashMap::new(),
            phonemizer: None,
            model_id: None,
        }
    }

    fn manifest(&self) -> Result<&'static LocalTtsModelManifest, LocalTtsRuntimeError> {
        self.model_id
            .as_deref()
            .and_then(local_tts_model_manifest)
            .ok_or_else(LocalTtsRuntimeError::model_load)
    }

    fn synthesize_impl(
        &mut self,
        request: &LocalTtsInferenceRequest,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        let manifest = self.manifest()?;
        let normalized = normalize_text(&request.text)?;
        let voice_key = resolve_voice_key(manifest, &request.voice_id)?;
        validate_speed(request.speaking_rate)?;
        validate_pitch(request.pitch)?;

        let phonemizer = self
            .phonemizer
            .as_ref()
            .ok_or_else(LocalTtsRuntimeError::model_load)?;
        let (tokens, _prosody) = phonemizer
            .phonemize_with_prosody(&normalized)
            .map_err(|_| LocalTtsRuntimeError::invalid_input())?;
        let ipa = tokens.concat();
        let ids = ipa_to_ids(&ipa);
        if ipa.trim().is_empty() || ids.len() <= 3 || ids.len() > MAX_MODEL_TOKENS {
            return Err(LocalTtsRuntimeError::invalid_input());
        }

        let voice = self
            .voices
            .get(voice_key)
            .ok_or_else(LocalTtsRuntimeError::invalid_voice)?;
        // P0 matched the official v0.8 style-row contract: byte length of the normalized
        // text, clamped to the available voice-style rows.
        let style = voice.style_row(normalized.len())?;
        let input_ids = Tensor::<i64>::from_array(([1usize, ids.len()], ids))
            .map_err(|_| LocalTtsRuntimeError::inference())?;
        let style_tensor = Tensor::<f32>::from_array(([1usize, style.len()], style.to_vec()))
            .map_err(|_| LocalTtsRuntimeError::inference())?;
        let speed_tensor = Tensor::<f32>::from_array(([1usize], vec![request.speaking_rate]))
            .map_err(|_| LocalTtsRuntimeError::inference())?;

        let session = self
            .session
            .as_mut()
            .ok_or_else(LocalTtsRuntimeError::model_load)?;
        let outputs = session
            .run(ort::inputs![input_ids, style_tensor, speed_tensor])
            .map_err(|_| LocalTtsRuntimeError::inference())?;
        let (_shape, samples) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|_| LocalTtsRuntimeError::inference())?;
        let samples = samples.to_vec();
        if samples.is_empty() || samples.iter().any(|sample| !sample.is_finite()) {
            return Err(LocalTtsRuntimeError::inference());
        }

        Ok(LocalTtsInferenceOutput {
            samples,
            sample_rate_hz: manifest.sample_rate_hz,
        })
    }
}

impl LocalTtsRuntimeEngine for KittenTtsRuntimeEngine {
    fn load(
        &mut self,
        identity: &LocalTtsRuntimeIdentity,
        verified_artifact_paths: &[PathBuf],
    ) -> Result<(), LocalTtsRuntimeError> {
        if verified_artifact_paths.len() != VERIFIED_ARTIFACT_COUNT {
            return Err(LocalTtsRuntimeError::model_load());
        }
        let manifest = local_tts_model_manifest(&identity.model_id)
            .ok_or_else(LocalTtsRuntimeError::unknown_model)?;
        validate_runtime_identity(manifest, identity)?;
        ensure_ort_runtime(
            &verified_artifact_paths[RUNTIME_ARTIFACT_INDEX],
            identity.platform,
            &identity.onnx_runtime_version,
        )?;

        let voices = load_npz(&verified_artifact_paths[VOICES_ARTIFACT_INDEX])?
            .into_iter()
            .map(|(name, array)| {
                let rows = array.nrows();
                let columns = array.ncols();
                (
                    name,
                    Voice {
                        rows,
                        columns,
                        data: array.data,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        for voice in manifest.voices {
            if !voices.contains_key(voice.embedding_key) {
                return Err(LocalTtsRuntimeError::model_load());
            }
        }

        let phonemizer = EnglishPhonemizer::new_with_dict(
            &verified_artifact_paths[G2P_ARTIFACT_INDEX],
        )
        .map_err(|_| LocalTtsRuntimeError::model_load())?;

        let session = Session::builder()
            .map_err(|_| LocalTtsRuntimeError::model_load())?
            .with_execution_providers([ep::CPU::default().build()])
            .map_err(|_| LocalTtsRuntimeError::model_load())?
            .with_intra_threads(manifest.runtime.inference_threads as usize)
            .map_err(|_| LocalTtsRuntimeError::model_load())?
            .with_inter_threads(1)
            .map_err(|_| LocalTtsRuntimeError::model_load())?
            .commit_from_file(&verified_artifact_paths[MODEL_ARTIFACT_INDEX])
            .map_err(|_| LocalTtsRuntimeError::model_load())?;

        self.session = Some(session);
        self.voices = voices;
        self.phonemizer = Some(phonemizer);
        self.model_id = Some(identity.model_id.clone());
        Ok(())
    }

    fn synthesize(
        &mut self,
        request: &LocalTtsInferenceRequest,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        self.synthesize_impl(request)
    }

    fn unload(&mut self) {
        self.session = None;
        self.voices.clear();
        self.phonemizer = None;
        self.model_id = None;
    }
}

fn resolve_voice_key(
    manifest: &LocalTtsModelManifest,
    voice_id: &str,
) -> Result<&'static str, LocalTtsRuntimeError> {
    manifest
        .voices
        .iter()
        .find(|voice| voice.id == voice_id)
        .map(|voice| voice.embedding_key)
        .ok_or_else(LocalTtsRuntimeError::invalid_voice)
}

fn validate_speed(speed: f32) -> Result<(), LocalTtsRuntimeError> {
    if speed.is_finite() && (MIN_SPEED..=MAX_SPEED).contains(&speed) {
        Ok(())
    } else {
        Err(LocalTtsRuntimeError::unsupported_config())
    }
}

fn validate_pitch(pitch: Option<f32>) -> Result<(), LocalTtsRuntimeError> {
    match pitch {
        None => Ok(()),
        Some(value) if value.is_finite() && value == 0.0 => Ok(()),
        Some(_) => Err(LocalTtsRuntimeError::unsupported_config()),
    }
}

fn validate_runtime_identity(
    manifest: &LocalTtsModelManifest,
    identity: &LocalTtsRuntimeIdentity,
) -> Result<(), LocalTtsRuntimeError> {
    if identity.model_revision != manifest.model_source_revision
        || identity.runtime_compatibility_version != manifest.runtime.compatibility_version
        || identity.adapter_contract != manifest.runtime.adapter_contract
        || identity.onnx_runtime_version != manifest.runtime.onnx_runtime_version
        || identity.g2p_source_revision != manifest.runtime.g2p_source_revision
    {
        return Err(LocalTtsRuntimeError::model_load());
    }
    Ok(())
}

fn ensure_ort_runtime(
    archive_path: &Path,
    platform: LocalTtsPlatform,
    version: &str,
) -> Result<(), LocalTtsRuntimeError> {
    let slot = ORT_RUNTIME.get_or_init(|| Mutex::new(None));
    let mut slot = slot.lock();
    if let Some(runtime) = slot.as_ref() {
        return if runtime.platform == platform && runtime.version == version {
            Ok(())
        } else {
            Err(LocalTtsRuntimeError::model_load())
        };
    }

    let library_file = extract_runtime_library(archive_path, platform)?;
    let committed = ort::init_from(library_file.path())
        .map_err(|_| LocalTtsRuntimeError::model_load())?
        .with_telemetry(false)
        .with_execution_providers([ep::CPU::default().build()])
        .commit();
    if !committed {
        return Err(LocalTtsRuntimeError::model_load());
    }
    *slot = Some(LoadedOrtRuntime {
        platform,
        version: version.to_string(),
        _library_file: library_file,
    });
    Ok(())
}

fn runtime_library_suffix(platform: LocalTtsPlatform) -> &'static str {
    match platform {
        LocalTtsPlatform::LinuxX86_64 => "/lib/libonnxruntime.so.1.23.2",
        LocalTtsPlatform::MacosArm64 | LocalTtsPlatform::MacosX86_64 => {
            "/lib/libonnxruntime.1.23.2.dylib"
        }
    }
}

fn extract_runtime_library(
    archive_path: &Path,
    platform: LocalTtsPlatform,
) -> Result<NamedTempFile, LocalTtsRuntimeError> {
    let file = File::open(archive_path).map_err(|_| LocalTtsRuntimeError::model_load())?;
    let mut archive = GzDecoder::new(file);
    let suffix = runtime_library_suffix(platform);
    let mut header = [0_u8; TAR_BLOCK_BYTES as usize];

    loop {
        read_exact(&mut archive, &mut header)?;
        if header.iter().all(|byte| *byte == 0) {
            return Err(LocalTtsRuntimeError::model_load());
        }
        let name = tar_entry_name(&header)?;
        let size = tar_octal(&header[124..136])?;
        let type_flag = header[156];
        if name.ends_with(suffix) {
            if !matches!(type_flag, 0 | b'0') || size == 0 || size > MAX_RUNTIME_LIBRARY_BYTES {
                return Err(LocalTtsRuntimeError::model_load());
            }
            let suffix = match platform {
                LocalTtsPlatform::LinuxX86_64 => ".so",
                LocalTtsPlatform::MacosArm64 | LocalTtsPlatform::MacosX86_64 => ".dylib",
            };
            let mut output = TempFileBuilder::new()
                .prefix("talking-moose-onnxruntime-")
                .suffix(suffix)
                .tempfile()
                .map_err(|_| LocalTtsRuntimeError::model_load())?;
            copy_exact(&mut archive, &mut output, size)?;
            output.flush().map_err(|_| LocalTtsRuntimeError::model_load())?;
            return Ok(output);
        }
        skip_exact(&mut archive, padded_tar_size(size))?;
    }
}

fn tar_entry_name(header: &[u8; 512]) -> Result<String, LocalTtsRuntimeError> {
    let name = tar_string(&header[..100])?;
    let prefix = tar_string(&header[345..500])?;
    if name.is_empty() {
        return Err(LocalTtsRuntimeError::model_load());
    }
    Ok(if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    })
}

fn tar_string(bytes: &[u8]) -> Result<String, LocalTtsRuntimeError> {
    let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    let value = std::str::from_utf8(&bytes[..end]).map_err(|_| LocalTtsRuntimeError::model_load())?;
    Ok(value.trim().to_string())
}

fn tar_octal(bytes: &[u8]) -> Result<u64, LocalTtsRuntimeError> {
    let end = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    let value = std::str::from_utf8(&bytes[..end]).map_err(|_| LocalTtsRuntimeError::model_load())?;
    let value = value.trim();
    if value.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(value, 8).map_err(|_| LocalTtsRuntimeError::model_load())
}

fn padded_tar_size(size: u64) -> u64 {
    size.saturating_add((TAR_BLOCK_BYTES - (size % TAR_BLOCK_BYTES)) % TAR_BLOCK_BYTES)
}

fn read_exact(reader: &mut impl Read, buffer: &mut [u8]) -> Result<(), LocalTtsRuntimeError> {
    reader
        .read_exact(buffer)
        .map_err(|_| LocalTtsRuntimeError::model_load())
}

fn skip_exact(reader: &mut impl Read, bytes: u64) -> Result<(), LocalTtsRuntimeError> {
    let copied = io::copy(&mut reader.take(bytes), &mut io::sink())
        .map_err(|_| LocalTtsRuntimeError::model_load())?;
    if copied == bytes {
        Ok(())
    } else {
        Err(LocalTtsRuntimeError::model_load())
    }
}

fn copy_exact(
    reader: &mut impl Read,
    writer: &mut impl Write,
    bytes: u64,
) -> Result<(), LocalTtsRuntimeError> {
    let copied = io::copy(&mut reader.take(bytes), writer)
        .map_err(|_| LocalTtsRuntimeError::model_load())?;
    if copied == bytes {
        Ok(())
    } else {
        Err(LocalTtsRuntimeError::model_load())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::local_tts::{DEFAULT_LOCAL_TTS_MODEL_ID, DEFAULT_LOCAL_TTS_VOICE};
    use super::super::LocalTtsRuntimeErrorKind;

    #[test]
    fn request_validation_rejects_unsupported_values() {
        let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
        assert_eq!(resolve_voice_key(manifest, DEFAULT_LOCAL_TTS_VOICE).unwrap(), "expr-voice-2-f");
        assert_eq!(
            resolve_voice_key(manifest, "not-a-voice").unwrap_err().kind,
            LocalTtsRuntimeErrorKind::InvalidVoice
        );
        assert_eq!(validate_speed(f32::NAN).unwrap_err().kind, LocalTtsRuntimeErrorKind::UnsupportedConfig);
        assert_eq!(validate_speed(0.0).unwrap_err().kind, LocalTtsRuntimeErrorKind::UnsupportedConfig);
        assert_eq!(validate_pitch(Some(1.0)).unwrap_err().kind, LocalTtsRuntimeErrorKind::UnsupportedConfig);
        assert!(validate_pitch(None).is_ok());
        assert!(validate_pitch(Some(0.0)).is_ok());
    }

    #[test]
    fn unknown_runtime_identity_fails_closed_before_artifact_use() {
        let mut engine = KittenTtsRuntimeEngine::new();
        let identity = LocalTtsRuntimeIdentity {
            model_id: "unsupported/model".to_string(),
            model_revision: "invalid".to_string(),
            runtime_compatibility_version: 0,
            adapter_contract: "invalid".to_string(),
            onnx_runtime_version: "invalid".to_string(),
            g2p_source_revision: "invalid".to_string(),
            platform: LocalTtsPlatform::LinuxX86_64,
        };
        let paths = vec![PathBuf::from("missing"); VERIFIED_ARTIFACT_COUNT];
        assert_eq!(
            engine.load(&identity, &paths).unwrap_err().kind,
            LocalTtsRuntimeErrorKind::UnknownModel
        );
    }

    #[test]
    #[ignore = "requires exact pinned Kitten/CMUdict/ONNX Runtime acceptance artifacts"]
    fn real_kitten_cpu_acceptance() {
        let paths = [
            std::env::var("KTT301_MODEL_PATH").unwrap(),
            std::env::var("KTT301_VOICES_PATH").unwrap(),
            std::env::var("KTT301_G2P_PATH").unwrap(),
            std::env::var("KTT301_ORT_ARCHIVE_PATH").unwrap(),
        ]
        .map(PathBuf::from);
        let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
        let identity = LocalTtsRuntimeIdentity {
            model_id: manifest.provider_model_id.to_string(),
            model_revision: manifest.model_source_revision.to_string(),
            runtime_compatibility_version: manifest.runtime.compatibility_version,
            adapter_contract: manifest.runtime.adapter_contract.to_string(),
            onnx_runtime_version: manifest.runtime.onnx_runtime_version.to_string(),
            g2p_source_revision: manifest.runtime.g2p_source_revision.to_string(),
            platform: LocalTtsPlatform::LinuxX86_64,
        };
        let mut engine = KittenTtsRuntimeEngine::new();
        engine.load(&identity, &paths).unwrap();
        let output = engine
            .synthesize(&LocalTtsInferenceRequest {
                text: "The Talking Moose is speaking locally.".to_string(),
                voice_id: "Jasper".to_string(),
                speaking_rate: 1.0,
                pitch: None,
            })
            .unwrap();
        assert_eq!(output.sample_rate_hz, 24_000);
        assert!(!output.samples.is_empty());
        assert!(output.samples.iter().all(|sample| sample.is_finite()));

        let faster = engine
            .synthesize(&LocalTtsInferenceRequest {
                text: "Speed is passed directly to Kitten Mini.".to_string(),
                voice_id: "Jasper".to_string(),
                speaking_rate: 1.2,
                pitch: Some(0.0),
            })
            .unwrap();
        assert_eq!(faster.sample_rate_hz, 24_000);
        assert!(!faster.samples.is_empty());
        assert!(faster.samples.iter().all(|sample| sample.is_finite()));

        let invalid_voice = engine
            .synthesize(&LocalTtsInferenceRequest {
                text: "This must fail before inference.".to_string(),
                voice_id: "invalid".to_string(),
                speaking_rate: 1.0,
                pitch: None,
            })
            .unwrap_err();
        assert_eq!(invalid_voice.kind, LocalTtsRuntimeErrorKind::InvalidVoice);

        let invalid_pitch = engine
            .synthesize(&LocalTtsInferenceRequest {
                text: "Pitch is unsupported by Kitten Mini.".to_string(),
                voice_id: "Jasper".to_string(),
                speaking_rate: 1.0,
                pitch: Some(1.0),
            })
            .unwrap_err();
        assert_eq!(invalid_pitch.kind, LocalTtsRuntimeErrorKind::UnsupportedConfig);
    }
}
