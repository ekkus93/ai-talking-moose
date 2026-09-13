use super::{
    LocalTtsInferenceOutput, LocalTtsInferenceRequest, LocalTtsRuntimeCancellation,
    LocalTtsRuntimeEngine, LocalTtsRuntimeEngineFactory, LocalTtsRuntimeError,
    LocalTtsRuntimeIdentity,
};
use crate::ai::local_tts::manifest::{
    local_tts_model_manifest, LocalTtsModelManifest, LocalTtsPlatform,
};
use flate2::read::GzDecoder;
use ort::{
    ep,
    session::{RunOptions, Session},
    value::Tensor,
};
use parking_lot::Mutex;
use piper_plus_g2p::{english::EnglishPhonemizer, Phonemizer};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tempfile::{Builder as TempFileBuilder, NamedTempFile};

mod normalize;
use super::npz::load_npz;
use super::tokenize::ipa_to_ids;
use normalize::normalize_text;

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
    #[cfg(test)]
    inference_threads_override: Option<usize>,
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

        let (_shape, samples) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|_| LocalTtsRuntimeError::inference())?;
        let samples = samples.to_vec();
        if samples.is_empty() || samples.iter().any(|sample| !sample.is_finite()) {
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
            output
                .flush()
                .map_err(|_| LocalTtsRuntimeError::model_load())?;
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
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let value =
        std::str::from_utf8(&bytes[..end]).map_err(|_| LocalTtsRuntimeError::model_load())?;
    Ok(value.trim().to_string())
}

fn tar_octal(bytes: &[u8]) -> Result<u64, LocalTtsRuntimeError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let value =
        std::str::from_utf8(&bytes[..end]).map_err(|_| LocalTtsRuntimeError::model_load())?;
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
    use super::super::LocalTtsRuntimeErrorKind;
    use super::*;
    use crate::ai::local_tts::{DEFAULT_LOCAL_TTS_MODEL_ID, DEFAULT_LOCAL_TTS_VOICE};
    use serde::Serialize;
    use std::time::{Duration, Instant};

    #[test]
    fn request_validation_rejects_unsupported_values() {
        let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
        assert_eq!(
            resolve_voice_key(manifest, DEFAULT_LOCAL_TTS_VOICE).unwrap(),
            "expr-voice-2-f"
        );
        assert_eq!(
            resolve_voice_key(manifest, "not-a-voice").unwrap_err().kind,
            LocalTtsRuntimeErrorKind::InvalidVoice
        );
        assert_eq!(
            validate_speed(f32::NAN).unwrap_err().kind,
            LocalTtsRuntimeErrorKind::UnsupportedConfig
        );
        assert_eq!(
            validate_speed(0.0).unwrap_err().kind,
            LocalTtsRuntimeErrorKind::UnsupportedConfig
        );
        assert_eq!(
            validate_pitch(Some(1.0)).unwrap_err().kind,
            LocalTtsRuntimeErrorKind::UnsupportedConfig
        );
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

    fn thread_sweep_platform_label(platform: LocalTtsPlatform) -> &'static str {
        match platform {
            LocalTtsPlatform::LinuxX86_64 => "linux-x86_64",
            LocalTtsPlatform::MacosArm64 => "macos-arm64",
            LocalTtsPlatform::MacosX86_64 => "macos-x86_64",
        }
    }

    fn median(values: &[f64]) -> f64 {
        let mut sorted = values.to_vec();
        sorted.sort_by(|left, right| left.total_cmp(right));
        let middle = sorted.len() / 2;
        if sorted.len().is_multiple_of(2) {
            (sorted[middle - 1] + sorted[middle]) / 2.0
        } else {
            sorted[middle]
        }
    }

    fn p95(values: &[f64]) -> f64 {
        let mut sorted = values.to_vec();
        sorted.sort_by(|left, right| left.total_cmp(right));
        let index = ((sorted.len() as f64 * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1);
        sorted[index]
    }

    fn max_rss_bytes() -> Option<u64> {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
            return None;
        }
        let usage = unsafe { usage.assume_init() };
        let raw = u64::try_from(usage.ru_maxrss).ok()?;
        #[cfg(target_os = "linux")]
        {
            return raw.checked_mul(1024);
        }
        #[cfg(target_os = "macos")]
        {
            return Some(raw);
        }
        #[allow(unreachable_code)]
        Some(raw)
    }

    #[derive(Debug, Serialize)]
    struct ThreadSweepConfigurationEvidence {
        inference_threads: usize,
        model_load_duration_ms: u64,
        first_synthesis_duration_ms: u64,
        first_audio_duration_ms: f64,
        first_real_time_factor: f64,
        warm_synthesis_duration_ms: Vec<u64>,
        warm_audio_duration_ms: Vec<f64>,
        warm_real_time_factors: Vec<f64>,
        median_warm_rtf: f64,
        p95_warm_rtf: f64,
        repeated_sample_counts: Vec<usize>,
        repeated_utterance_stable: bool,
        max_rss_bytes: Option<u64>,
    }

    #[derive(Debug, Serialize)]
    struct ThreadSweepEvidence {
        platform: &'static str,
        model_id: &'static str,
        production_default_threads: usize,
        bounded_n_threads: usize,
        network_denied: bool,
        configurations: Vec<ThreadSweepConfigurationEvidence>,
    }

    #[test]
    #[ignore = "requires exact pinned Kitten/CMUdict/ONNX Runtime acceptance artifacts"]
    fn real_kittentts_cpu_thread_sweep() {
        const THREAD_CONFIGURATIONS: [usize; 3] = [1, 2, 4];
        const WARM_REPETITIONS: usize = 5;

        let paths = [
            std::env::var("KTT301_MODEL_PATH").unwrap(),
            std::env::var("KTT301_VOICES_PATH").unwrap(),
            std::env::var("KTT301_G2P_PATH").unwrap(),
            std::env::var("KTT301_ORT_ARCHIVE_PATH").unwrap(),
        ]
        .map(PathBuf::from);
        let manifest = local_tts_model_manifest(DEFAULT_LOCAL_TTS_MODEL_ID).unwrap();
        let platform = super::super::current_platform().unwrap();
        let identity = super::super::runtime_identity(manifest, platform);
        let production_default_threads = usize::from(manifest.runtime.inference_threads);
        assert_eq!(production_default_threads, 2);
        let _network_guard = crate::test_support::deny_network_for_scope();
        assert!(crate::test_support::network_denied());

        let request = LocalTtsInferenceRequest {
            text: "The Talking Moose measures local speech performance.".to_string(),
            voice_id: "Jasper".to_string(),
            speaking_rate: 1.0,
            pitch: None,
        };
        let mut configurations = Vec::with_capacity(THREAD_CONFIGURATIONS.len());

        for inference_threads in THREAD_CONFIGURATIONS {
            let mut engine = KittenTtsRuntimeEngine::new_with_inference_threads(inference_threads);
            let load_started = Instant::now();
            engine.load(&identity, &paths).unwrap();
            let model_load_duration_ms =
                load_started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);

            let first_started = Instant::now();
            let first = engine.synthesize(&request).unwrap();
            let first_elapsed = first_started.elapsed();
            assert_eq!(first.sample_rate_hz, manifest.sample_rate_hz);
            assert!(!first.samples.is_empty());
            assert!(first.samples.iter().all(|sample| sample.is_finite()));
            let first_audio_duration_ms =
                first.samples.len() as f64 * 1_000.0 / f64::from(first.sample_rate_hz);
            let first_real_time_factor =
                first_elapsed.as_secs_f64() * 1_000.0 / first_audio_duration_ms;
            assert!(first_real_time_factor.is_finite() && first_real_time_factor > 0.0);

            let mut warm_synthesis_duration_ms = Vec::with_capacity(WARM_REPETITIONS);
            let mut warm_audio_duration_ms = Vec::with_capacity(WARM_REPETITIONS);
            let mut warm_real_time_factors = Vec::with_capacity(WARM_REPETITIONS);
            let mut repeated_sample_counts = vec![first.samples.len()];

            for _ in 0..WARM_REPETITIONS {
                let started = Instant::now();
                let output = engine.synthesize(&request).unwrap();
                let elapsed = started.elapsed();
                assert_eq!(output.sample_rate_hz, manifest.sample_rate_hz);
                assert!(!output.samples.is_empty());
                assert!(output.samples.iter().all(|sample| sample.is_finite()));
                let audio_duration_ms =
                    output.samples.len() as f64 * 1_000.0 / f64::from(output.sample_rate_hz);
                let rtf = elapsed.as_secs_f64() * 1_000.0 / audio_duration_ms;
                assert!(rtf.is_finite() && rtf > 0.0);
                if inference_threads == production_default_threads {
                    assert!(
                        rtf < 1.0,
                        "production 2-thread Local TTS policy must remain faster than real time"
                    );
                }
                warm_synthesis_duration_ms
                    .push(elapsed.as_millis().try_into().unwrap_or(u64::MAX));
                warm_audio_duration_ms.push(audio_duration_ms);
                warm_real_time_factors.push(rtf);
                repeated_sample_counts.push(output.samples.len());
            }

            let repeated_utterance_stable = repeated_sample_counts
                .windows(2)
                .all(|window| window[0] == window[1]);
            assert!(
                repeated_utterance_stable,
                "repeated identical utterances changed generated sample count"
            );
            let median_warm_rtf = median(&warm_real_time_factors);
            let p95_warm_rtf = p95(&warm_real_time_factors);
            let first_synthesis_duration_ms =
                first_elapsed.as_millis().try_into().unwrap_or(u64::MAX);
            configurations.push(ThreadSweepConfigurationEvidence {
                inference_threads,
                model_load_duration_ms,
                first_synthesis_duration_ms,
                first_audio_duration_ms,
                first_real_time_factor,
                warm_synthesis_duration_ms,
                warm_audio_duration_ms,
                warm_real_time_factors,
                median_warm_rtf,
                p95_warm_rtf,
                repeated_sample_counts,
                repeated_utterance_stable,
                max_rss_bytes: max_rss_bytes(),
            });
            engine.unload();
        }

        let evidence = ThreadSweepEvidence {
            platform: thread_sweep_platform_label(platform),
            model_id: DEFAULT_LOCAL_TTS_MODEL_ID,
            production_default_threads,
            bounded_n_threads: 4,
            network_denied: crate::test_support::network_denied(),
            configurations,
        };
        println!(
            "KITTENTTS_THREAD_SWEEP_JSON={}",
            serde_json::to_string(&evidence).unwrap()
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
        assert_eq!(
            invalid_pitch.kind,
            LocalTtsRuntimeErrorKind::UnsupportedConfig
        );

        let cancellation = LocalTtsRuntimeCancellation::default();
        let canceller = cancellation.clone();
        let cancel_thread = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !canceller.has_run_options() {
                assert!(
                    Instant::now() < deadline,
                    "real Kitten inference never registered cancellable ORT run options"
                );
                std::thread::yield_now();
            }
            canceller.cancel();
        });
        let cancelled = engine
            .synthesize_cancellable(
                &LocalTtsInferenceRequest {
                    text: "Cancel this real Kitten Mini CPU inference while ONNX Runtime is executing."
                        .to_string(),
                    voice_id: "Jasper".to_string(),
                    speaking_rate: 1.0,
                    pitch: None,
                },
                &cancellation,
            )
            .unwrap_err();
        cancel_thread.join().unwrap();
        assert_eq!(cancelled.kind, LocalTtsRuntimeErrorKind::Cancelled);
    }
}
