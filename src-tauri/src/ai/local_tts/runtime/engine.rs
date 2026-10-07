use super::{
    LocalTtsInferenceOutput, LocalTtsInferenceRequest, LocalTtsRuntimeCancellation,
    LocalTtsRuntimeEngine, LocalTtsRuntimeEngineFactory, LocalTtsRuntimeError,
    LocalTtsRuntimeIdentity,
};
use crate::ai::local_tts::manifest::{
    local_tts_model_manifest, LocalTtsModelManifest, LocalTtsPlatform,
};
use ort::{
    ep,
    session::{RunOptions, Session},
    value::Tensor,
};
use parking_lot::Mutex;
use piper_plus_g2p::{english::EnglishPhonemizer, Phonemizer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tempfile::NamedTempFile;

mod archive;
mod normalize;
use super::npz::load_npz;
use super::tokenize::ipa_to_ids;
use archive::extract_runtime_library;
use normalize::normalize_text;

const VERIFIED_ARTIFACT_COUNT: usize = 4;
const MODEL_ARTIFACT_INDEX: usize = 0;
const VOICES_ARTIFACT_INDEX: usize = 1;
const G2P_ARTIFACT_INDEX: usize = 2;
const RUNTIME_ARTIFACT_INDEX: usize = 3;
const MAX_MODEL_TOKENS: usize = 512;
const MIN_SPEED: f32 = 0.25;
const MAX_SPEED: f32 = 4.0;
const EXPECTED_MODEL_OUTPUT_COUNT: usize = 2;
const WAVEFORM_OUTPUT_NAME: &str = "waveform";
// Kitten Mini also returns `duration`. The adapter currently consumes only `waveform`, but
// requiring `duration` to be present catches ONNX contract drift without inventing a downstream
// semantic dependency on that output.
const DURATION_OUTPUT_NAME: &str = "duration";

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
    #[cfg(test)]
    inference_threads_override: Option<usize>,
    #[cfg(test)]
    last_waveform_shape: Option<Vec<i64>>,
}

impl KittenTtsRuntimeEngine {
    fn new() -> Self {
        Self {
            session: None,
            voices: HashMap::new(),
            phonemizer: None,
            model_id: None,
            #[cfg(test)]
            inference_threads_override: None,
            #[cfg(test)]
            last_waveform_shape: None,
        }
    }

    #[cfg(test)]
    fn new_with_inference_threads(inference_threads: usize) -> Self {
        assert!((1..=4).contains(&inference_threads));
        Self {
            session: None,
            voices: HashMap::new(),
            phonemizer: None,
            model_id: None,
            inference_threads_override: Some(inference_threads),
            last_waveform_shape: None,
        }
    }

    #[cfg(test)]
    fn last_waveform_shape(&self) -> Option<&[i64]> {
        self.last_waveform_shape.as_deref()
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
        cancellation: &LocalTtsRuntimeCancellation,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        cancellation.check_cancelled()?;
        let manifest = self.manifest()?;
        let normalized = normalize_text(&request.text)?;
        let voice_key = resolve_voice_key(manifest, &request.voice_id)?;
        validate_speed(request.speaking_rate)?;
        validate_pitch(request.pitch)?;
        cancellation.check_cancelled()?;

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
        cancellation.check_cancelled()?;

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
        cancellation.check_cancelled()?;

        let run_options =
            Arc::new(RunOptions::new().map_err(|_| LocalTtsRuntimeError::inference())?);
        cancellation.install_run_options(run_options.clone());
        let session = self
            .session
            .as_mut()
            .ok_or_else(LocalTtsRuntimeError::model_load)?;
        let run_result = session.run_with_options(
            ort::inputs![input_ids, style_tensor, speed_tensor],
            run_options.as_ref(),
        );
        cancellation.clear_run_options();
        let outputs = match run_result {
            Ok(outputs) => outputs,
            Err(_) if cancellation.is_cancelled() => {
                return Err(LocalTtsRuntimeError::cancelled());
            }
            Err(_) => return Err(LocalTtsRuntimeError::inference()),
        };
        cancellation.check_cancelled()?;

        validate_model_output_contract(&outputs)?;
        let waveform = outputs
            .get(WAVEFORM_OUTPUT_NAME)
            .ok_or_else(LocalTtsRuntimeError::inference)?;
        let (shape, samples) = waveform
            .try_extract_tensor::<f32>()
            .map_err(|_| LocalTtsRuntimeError::inference())?;
        let shape_values: &[i64] = shape;
        if !waveform_shape_is_valid(shape_values, samples.len()) {
            return Err(LocalTtsRuntimeError::inference());
        }
        #[cfg(test)]
        {
            self.last_waveform_shape = Some(shape_values.to_vec());
        }
        let samples = samples.to_vec();
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(LocalTtsRuntimeError::inference());
        }
        cancellation.check_cancelled()?;

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

        let phonemizer =
            EnglishPhonemizer::new_with_dict(&verified_artifact_paths[G2P_ARTIFACT_INDEX])
                .map_err(|_| LocalTtsRuntimeError::model_load())?;

        let session = Session::builder()
            .map_err(|_| LocalTtsRuntimeError::model_load())?
            .with_execution_providers([ep::CPU::default().build()])
            .map_err(|_| LocalTtsRuntimeError::model_load())?;
        #[cfg(not(test))]
        let session = session
            .with_intra_threads(manifest.runtime.inference_threads as usize)
            .map_err(|_| LocalTtsRuntimeError::model_load())?;
        #[cfg(test)]
        let session = session
            .with_intra_threads(
                self.inference_threads_override
                    .unwrap_or(manifest.runtime.inference_threads as usize),
            )
            .map_err(|_| LocalTtsRuntimeError::model_load())?;
        let session = session
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
        self.synthesize_impl(request, &LocalTtsRuntimeCancellation::default())
    }

    fn synthesize_cancellable(
        &mut self,
        request: &LocalTtsInferenceRequest,
        cancellation: &LocalTtsRuntimeCancellation,
    ) -> Result<LocalTtsInferenceOutput, LocalTtsRuntimeError> {
        self.synthesize_impl(request, cancellation)
    }

    fn unload(&mut self) {
        self.session = None;
        self.voices.clear();
        self.phonemizer = None;
        self.model_id = None;
    }
}

fn waveform_shape_is_valid(shape: &[i64], sample_count: usize) -> bool {
    if sample_count == 0 {
        return false;
    }

    let samples_match = |dimension: i64| usize::try_from(dimension).ok() == Some(sample_count);

    match shape {
        [samples] => samples_match(*samples),
        [batch, samples] => *batch == 1 && samples_match(*samples),
        _ => false,
    }
}

fn model_output_contract_is_valid(
    output_count: usize,
    has_waveform: bool,
    has_duration: bool,
) -> bool {
    output_count == EXPECTED_MODEL_OUTPUT_COUNT && has_waveform && has_duration
}

fn validate_model_output_contract(
    outputs: &ort::session::SessionOutputs<'_>,
) -> Result<(), LocalTtsRuntimeError> {
    // Validate the exact named output contract before reading generated samples by name. `duration`
    // is intentionally presence-only drift detection here; `waveform` is the only output consumed
    // by this adapter.
    if model_output_contract_is_valid(
        outputs.len(),
        outputs.contains_key(WAVEFORM_OUTPUT_NAME),
        outputs.contains_key(DURATION_OUTPUT_NAME),
    ) {
        Ok(())
    } else {
        Err(LocalTtsRuntimeError::inference())
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

#[cfg(test)]
#[path = "engine/tests.rs"]
mod tests;
