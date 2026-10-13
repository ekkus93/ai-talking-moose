use crate::ai::traits::{LiveSession, RealtimeConversationProvider, SpeechSynthesizer, TextModel};
use crate::ai::types::{
    LiveServerEvent, LiveSessionConfig, ProviderError, ProviderErrorKind, TextRequest,
    ToolCallResponse, TranscriptUpdate, TtsRequest,
};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// One-turn local conversation adapter for the existing conversation lifecycle. Local ASR
/// supplies finalized transcripts; this session generates a local text response and local audio.
pub struct LocalConversationProvider {
    text_model: Arc<dyn TextModel>,
    speech_synthesizer: Arc<dyn SpeechSynthesizer>,
    voice_name: String,
    speaking_rate: f32,
    pitch: f32,
}

impl LocalConversationProvider {
    pub fn new(
        text_model: Arc<dyn TextModel>,
        speech_synthesizer: Arc<dyn SpeechSynthesizer>,
        voice_name: String,
        speaking_rate: f32,
        pitch: f32,
    ) -> Self {
        Self {
            text_model,
            speech_synthesizer,
            voice_name,
            speaking_rate,
            pitch,
        }
    }
}

#[async_trait]
impl RealtimeConversationProvider for LocalConversationProvider {
    async fn connect(
        &self,
        config: LiveSessionConfig,
        event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        let _ = event_sender.send(LiveServerEvent::Connected).await;
        Ok(Box::new(LocalConversationSession {
            config,
            event_sender,
            text_model: self.text_model.clone(),
            speech_synthesizer: self.speech_synthesizer.clone(),
            voice_name: self.voice_name.clone(),
            speaking_rate: self.speaking_rate,
            pitch: self.pitch,
            generation: Arc::new(AtomicU64::new(0)),
            active_synthesis: Arc::new(Mutex::new(None)),
            closed: false,
        }))
    }
}

struct LocalConversationSession {
    config: LiveSessionConfig,
    event_sender: mpsc::Sender<LiveServerEvent>,
    text_model: Arc<dyn TextModel>,
    speech_synthesizer: Arc<dyn SpeechSynthesizer>,
    voice_name: String,
    speaking_rate: f32,
    pitch: f32,
    generation: Arc<AtomicU64>,
    active_synthesis: Arc<Mutex<Option<CancellationToken>>>,
    closed: bool,
}

#[async_trait]
impl LiveSession for LocalConversationSession {
    async fn send_audio_chunk(&mut self, _pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        Err(ProviderError::from_kind(ProviderErrorKind::Setup))
    }

    async fn send_text_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        if self.closed {
            return Err(ProviderError::from_kind(ProviderErrorKind::Closed));
        }
        let turn = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let current_generation = self.generation.clone();
        let active_synthesis = self.active_synthesis.clone();
        let event_sender = self.event_sender.clone();
        let text_model = self.text_model.clone();
        let speech_synthesizer = self.speech_synthesizer.clone();
        let system_instruction = self.config.system_instruction.clone();
        let voice_name = self.voice_name.clone();
        let speaking_rate = self.speaking_rate;
        let pitch = self.pitch;
        let prompt = text.to_string();

        tauri::async_runtime::spawn(async move {
            let generated = text_model
                .generate(TextRequest {
                    prompt,
                    system_instruction,
                    temperature: Some(0.85),
                    max_tokens: Some(192),
                })
                .await;
            let response = match generated {
                Ok(response) => response.text.trim().to_string(),
                Err(error) => {
                    let _ = event_sender.send(LiveServerEvent::Error(error)).await;
                    return;
                }
            };
            if response.is_empty() || current_generation.load(Ordering::SeqCst) != turn {
                return;
            }
            let _ = event_sender
                .send(LiveServerEvent::ModelTranscript(TranscriptUpdate {
                    text: response.clone(),
                    is_final: true,
                }))
                .await;

            let cancellation = CancellationToken::new();
            *active_synthesis.lock() = Some(cancellation.clone());
            let audio = speech_synthesizer
                .synthesize_cancellable(
                    TtsRequest {
                        text: response,
                        voice_name: Some(voice_name),
                        speaking_rate: Some(speaking_rate),
                        pitch: Some(pitch),
                    },
                    &cancellation,
                )
                .await;
            active_synthesis.lock().take();
            if current_generation.load(Ordering::SeqCst) != turn {
                return;
            }
            match audio {
                Ok(audio) => {
                    let _ = event_sender
                        .send(LiveServerEvent::AudioData(audio.pcm_bytes))
                        .await;
                    let _ = event_sender.send(LiveServerEvent::TurnComplete).await;
                }
                Err(error) => {
                    let _ = event_sender.send(LiveServerEvent::Error(error)).await;
                }
            }
        });
        Ok(())
    }

    async fn send_tool_response(
        &mut self,
        _response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::from_kind(ProviderErrorKind::Setup))
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(cancellation) = self.active_synthesis.lock().take() {
            cancellation.cancel();
        }
        self.event_sender
            .send(LiveServerEvent::Interrupted)
            .await
            .map_err(|_| ProviderError::from_kind(ProviderErrorKind::Closed))
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        self.closed = true;
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(cancellation) = self.active_synthesis.lock().take() {
            cancellation.cancel();
        }
        Ok(())
    }
}
