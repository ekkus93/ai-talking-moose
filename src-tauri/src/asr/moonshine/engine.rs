use super::installer::{
    MoonshineModelInstallError, MoonshineModelInstallErrorKind, MoonshineModelInstaller,
};
use super::runtime::{
    MoonshineModelArchitecture, MoonshineStream, MoonshineTranscriber, MoonshineTranscript,
};
use crate::asr::{AsrError, AsrErrorKind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Moonshine's V1 local-ASR input contract for Tiny and Small streaming models.
///
/// V1R-ASR-009 owns microphone format conversion and must feed this engine mono
/// `f32` PCM at exactly 16 kHz. This engine never sends microphone audio to a
/// cloud provider and never falls back to Gemini Live audio recognition.
pub const MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ: u32 = 16_000;
/// Moonshine Small uses the same 16 kHz mono PCM input contract as Tiny.
pub const MOONSHINE_SMALL_INPUT_SAMPLE_RATE_HZ: u32 = MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ;

/// One meaningful transcript change emitted by the Tiny streaming engine.
///
/// Native line IDs are preserved so V1R-ASR-010 can build the higher-level
/// utterance accumulator without inferring identity from text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoonshineTinyTranscriptUpdate {
    Partial {
        line_id: u64,
        text: String,
        latency_ms: u32,
    },
    Final {
        line_id: u64,
        text: String,
        latency_ms: u32,
    },
}

/// Small emits the same native-line update shape as Tiny.
pub type MoonshineSmallTranscriptUpdate = MoonshineTinyTranscriptUpdate;

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmittedLineState {
    text: String,
    is_final: bool,
}

trait ModelResolver {
    fn resolve_verified_model(
        &self,
        architecture: MoonshineModelArchitecture,
    ) -> Result<PathBuf, AsrError>;
}

impl ModelResolver for MoonshineModelInstaller {
    fn resolve_verified_model(
        &self,
        architecture: MoonshineModelArchitecture,
    ) -> Result<PathBuf, AsrError> {
        match self.verify_installed(architecture) {
            Ok(Some(installed)) => Ok(installed.model_path),
            Ok(None) => Err(AsrError {
                kind: AsrErrorKind::ModelNotInstalled,
                message: format!(
                    "{} is not installed. Download it in Settings before starting local speech recognition. No microphone audio was sent to Google.",
                    model_display_name(architecture)
                ),
                retryable: true,
            }),
            Err(error) => Err(map_installer_error(error, architecture)),
        }
    }
}

const fn model_display_name(architecture: MoonshineModelArchitecture) -> &'static str {
    match architecture {
        MoonshineModelArchitecture::TinyStreaming => "Moonshine Tiny",
        MoonshineModelArchitecture::SmallStreaming => "Moonshine Small",
    }
}

fn map_installer_error(
    error: MoonshineModelInstallError,
    architecture: MoonshineModelArchitecture,
) -> AsrError {
    match error.kind {
        MoonshineModelInstallErrorKind::CorruptInstall
        | MoonshineModelInstallErrorKind::SizeMismatch
        | MoonshineModelInstallErrorKind::Sha256Mismatch
        | MoonshineModelInstallErrorKind::Crc32cMismatch => AsrError {
            kind: AsrErrorKind::ModelCorrupt,
            message: format!(
                "{} is incomplete or corrupt. Reinstall it in Settings before starting local speech recognition. No microphone audio was sent to Google.",
                model_display_name(architecture)
            ),
            retryable: true,
        },
        MoonshineModelInstallErrorKind::Cancelled => AsrError {
            kind: AsrErrorKind::Cancelled,
            message: format!("{} model verification was cancelled.", model_display_name(architecture)),
            retryable: true,
        },
        MoonshineModelInstallErrorKind::InvalidManifest
        | MoonshineModelInstallErrorKind::UnsupportedArtifact => AsrError {
            kind: AsrErrorKind::Internal,
            message: format!(
                "The bundled {} model metadata is invalid. Update or reinstall the application.",
                model_display_name(architecture)
            ),
            retryable: false,
        },
        MoonshineModelInstallErrorKind::InsufficientDiskSpace
        | MoonshineModelInstallErrorKind::Network
        | MoonshineModelInstallErrorKind::Http
        | MoonshineModelInstallErrorKind::Io
        | MoonshineModelInstallErrorKind::Promotion => AsrError {
            kind: AsrErrorKind::ModelLoadFailed,
            message: format!(
                "{} could not be verified before local speech recognition started.",
                model_display_name(architecture)
            ),
            retryable: true,
        },
    }
}

trait StreamBackend: Send {
    fn add_audio(&mut self, pcm: &[f32], sample_rate_hz: u32) -> Result<(), AsrError>;
    fn transcribe(&mut self, force_update: bool) -> Result<MoonshineTranscript, AsrError>;
    fn stop(&mut self) -> Result<(), AsrError>;
}

struct NativeStreamBackend {
    stream: MoonshineStream,
}

impl StreamBackend for NativeStreamBackend {
    fn add_audio(&mut self, pcm: &[f32], sample_rate_hz: u32) -> Result<(), AsrError> {
        self.stream.add_audio(pcm, sample_rate_hz)
    }

    fn transcribe(&mut self, force_update: bool) -> Result<MoonshineTranscript, AsrError> {
        self.stream.transcribe(force_update)
    }

    fn stop(&mut self) -> Result<(), AsrError> {
        self.stream.stop()
    }
}

trait StreamFactory {
    fn open(
        &self,
        model_path: &Path,
        architecture: MoonshineModelArchitecture,
    ) -> Result<Box<dyn StreamBackend>, AsrError>;
}

struct NativeStreamFactory;

impl StreamFactory for NativeStreamFactory {
    fn open(
        &self,
        model_path: &Path,
        architecture: MoonshineModelArchitecture,
    ) -> Result<Box<dyn StreamBackend>, AsrError> {
        let transcriber = MoonshineTranscriber::load(model_path, architecture)?;
        let mut stream = transcriber.create_stream()?;
        stream.start()?;
        Ok(Box::new(NativeStreamBackend { stream }))
    }
}

/// Persistent local Moonshine Tiny/Small streaming recognizer.
///
/// The engine is intentionally synchronous. V1R-ASR-009 owns the dedicated
/// inference worker so native inference cannot block Tokio or the UI thread.
/// Construction verifies the ASR-006 installation before the native runtime is
/// touched. No local failure is converted into a cloud recognition request.
pub struct MoonshineStreamingEngine {
    architecture: MoonshineModelArchitecture,
    model_path: PathBuf,
    stream: Option<Box<dyn StreamBackend>>,
    emitted_lines: HashMap<u64, EmittedLineState>,
    cancelled: bool,
}

/// Backward-compatible Tiny name for the architecture-neutral streaming engine.
pub type MoonshineTinyEngine = MoonshineStreamingEngine;
/// Small streaming engine using the same lifecycle and transcript semantics as Tiny.
pub type MoonshineSmallEngine = MoonshineStreamingEngine;

impl MoonshineStreamingEngine {
    pub fn open(installer: &MoonshineModelInstaller) -> Result<Self, AsrError> {
        Self::open_architecture(installer, MoonshineModelArchitecture::TinyStreaming)
    }

    pub fn open_small(installer: &MoonshineModelInstaller) -> Result<Self, AsrError> {
        Self::open_architecture(installer, MoonshineModelArchitecture::SmallStreaming)
    }

    pub fn open_architecture(
        installer: &MoonshineModelInstaller,
        architecture: MoonshineModelArchitecture,
    ) -> Result<Self, AsrError> {
        Self::open_with_architecture_components(architecture, installer, &NativeStreamFactory)
    }

    #[cfg(test)]
    fn open_with_components(
        resolver: &dyn ModelResolver,
        factory: &dyn StreamFactory,
    ) -> Result<Self, AsrError> {
        Self::open_with_architecture_components(
            MoonshineModelArchitecture::TinyStreaming,
            resolver,
            factory,
        )
    }

    fn open_with_architecture_components(
        architecture: MoonshineModelArchitecture,
        resolver: &dyn ModelResolver,
        factory: &dyn StreamFactory,
    ) -> Result<Self, AsrError> {
        let model_path = resolver.resolve_verified_model(architecture)?;
        let stream = factory.open(&model_path, architecture)?;
        Ok(Self {
            architecture,
            model_path,
            stream: Some(stream),
            emitted_lines: HashMap::new(),
            cancelled: false,
        })
    }

    pub const fn architecture(&self) -> MoonshineModelArchitecture {
        self.architecture
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub const fn input_sample_rate_hz(&self) -> u32 {
        MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ
    }

    pub fn is_active(&self) -> bool {
        self.stream.is_some() && !self.cancelled
    }

    /// Append one increment of 16 kHz mono `f32` PCM and return only
    /// transcript lines whose text/finality changed since the previous call.
    pub fn push_pcm(
        &mut self,
        pcm: &[f32],
    ) -> Result<Vec<MoonshineTinyTranscriptUpdate>, AsrError> {
        self.ensure_active()?;
        if pcm.is_empty() {
            return Ok(Vec::new());
        }

        let transcript = {
            let stream = self.active_stream()?;
            stream.add_audio(pcm, MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ)?;
            stream.transcribe(false)?
        };
        Ok(self.collect_updates(transcript))
    }

    /// Force the native streaming decoder to expose its latest transcript
    /// state without appending synthetic audio.
    pub fn flush(&mut self) -> Result<Vec<MoonshineTinyTranscriptUpdate>, AsrError> {
        self.ensure_active()?;
        let transcript = self.active_stream()?.transcribe(true)?;
        Ok(self.collect_updates(transcript))
    }

    /// Stop the native stream. Safe to call repeatedly.
    pub fn stop(&mut self) -> Result<(), AsrError> {
        let Some(mut stream) = self.stream.take() else {
            return Ok(());
        };
        stream.stop()
    }

    /// Cancel this engine and tear down its native stream. A cancelled engine
    /// cannot be reused; callers must construct a new engine for a new session.
    pub fn cancel(&mut self) -> Result<(), AsrError> {
        self.cancelled = true;
        self.stop()
    }

    fn ensure_active(&self) -> Result<(), AsrError> {
        if self.cancelled {
            return Err(AsrError {
                kind: AsrErrorKind::Cancelled,
                message: format!(
                    "{} recognition was cancelled.",
                    model_display_name(self.architecture)
                ),
                retryable: true,
            });
        }
        if self.stream.is_none() {
            return Err(AsrError {
                kind: AsrErrorKind::InvalidState,
                message: format!(
                    "{} recognition is not active.",
                    model_display_name(self.architecture)
                ),
                retryable: true,
            });
        }
        Ok(())
    }

    fn active_stream(&mut self) -> Result<&mut (dyn StreamBackend + '_), AsrError> {
        match self.stream.as_mut() {
            Some(stream) => Ok(stream.as_mut()),
            None => Err(AsrError {
                kind: AsrErrorKind::InvalidState,
                message: format!(
                    "{} recognition is not active.",
                    model_display_name(self.architecture)
                ),
                retryable: true,
            }),
        }
    }

    fn collect_updates(
        &mut self,
        transcript: MoonshineTranscript,
    ) -> Vec<MoonshineTinyTranscriptUpdate> {
        let mut updates = Vec::new();
        for line in transcript.lines {
            if line.text.is_empty() {
                continue;
            }

            let changed = self.emitted_lines.get(&line.id).is_none_or(|previous| {
                previous.text != line.text || previous.is_final != line.is_complete
            });
            if !changed {
                continue;
            }

            let update = if line.is_complete {
                MoonshineTinyTranscriptUpdate::Final {
                    line_id: line.id,
                    text: line.text.clone(),
                    latency_ms: line.last_transcription_latency_ms,
                }
            } else {
                MoonshineTinyTranscriptUpdate::Partial {
                    line_id: line.id,
                    text: line.text.clone(),
                    latency_ms: line.last_transcription_latency_ms,
                }
            };
            self.emitted_lines.insert(
                line.id,
                EmittedLineState {
                    text: line.text,
                    is_final: line.is_complete,
                },
            );
            updates.push(update);
        }
        updates
    }
}

impl Drop for MoonshineStreamingEngine {
    fn drop(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.stop();
        }
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
