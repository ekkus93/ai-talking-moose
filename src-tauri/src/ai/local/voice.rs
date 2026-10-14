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

const MAX_LOCAL_TTS_RESPONSE_CHARS: usize = 350;

fn prepare_local_tts_response(text: &str) -> String {
    let compact = text
        .chars()
        .filter(|character| !character.is_control() || character.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let compact = strip_outer_quotes(strip_leading_meta_cue(&compact));
    let characters = compact.chars().collect::<Vec<_>>();
    if characters.len() <= MAX_LOCAL_TTS_RESPONSE_CHARS {
        return compact.to_string();
    }

    let prefix = characters
        .iter()
        .take(MAX_LOCAL_TTS_RESPONSE_CHARS)
        .copied()
        .collect::<String>();
    let sentence_end = prefix
        .char_indices()
        .filter(|(_, character)| matches!(character, '.' | '!' | '?'))
        .map(|(index, character)| index + character.len_utf8())
        .rfind(|end| *end >= MAX_LOCAL_TTS_RESPONSE_CHARS / 2);
    let end = sentence_end.unwrap_or_else(|| prefix.rfind(' ').unwrap_or(prefix.len()));
    let bounded = prefix[..end].trim_end();
    if bounded.is_empty() {
        String::new()
    } else if bounded.ends_with('.') || bounded.ends_with('!') || bounded.ends_with('?') {
        bounded.to_string()
    } else {
        format!("{bounded}.")
    }
}

fn strip_leading_meta_cue(text: &str) -> &str {
    let text = if let Some(after_open) = text.strip_prefix('[') {
        if let Some(close_index) = after_open.find(']') {
            let cue = after_open[..close_index].trim().to_ascii_lowercase();
            let is_delivery_cue = cue.len() <= 80
                && [
                    "voice", "speaking", "speak ", "whisper", "tone", "delivery", "deadpan",
                    "cheerful", "laugh", "sigh", "chuckle",
                ]
                .iter()
                .any(|marker| cue.contains(marker));
            if is_delivery_cue {
                after_open[close_index + 1..].trim_start()
            } else {
                text
            }
        } else {
            text
        }
    } else {
        text
    };

    let lowercase = text.to_ascii_lowercase();
    for label in [
        "the moose says:",
        "moose says:",
        "moose:",
        "joke:",
        "a joke:",
        "answer:",
        "response:",
        "reply:",
    ] {
        if lowercase.starts_with(label) {
            return text[label.len()..].trim_start();
        }
    }

    text
}

fn strip_outer_quotes(text: &str) -> &str {
    for quote in ['"', '\''] {
        if text.starts_with(quote) && text.ends_with(quote) && text.len() >= 2 {
            return text[quote.len_utf8()..text.len() - quote.len_utf8()].trim();
        }
    }
    text
}

/// One-turn local conversation adapter for the existing conversation lifecycle. Local ASR
/// supplies finalized transcripts; this session generates a local text response and local audio.
pub struct LocalConversationProvider {
    text_model: Arc<dyn TextModel>,
    speech_synthesizer: Arc<dyn SpeechSynthesizer>,
    voice_name: String,
    speaking_rate: f32,
}

impl LocalConversationProvider {
    pub fn new(
        text_model: Arc<dyn TextModel>,
        speech_synthesizer: Arc<dyn SpeechSynthesizer>,
        voice_name: String,
        speaking_rate: f32,
    ) -> Self {
        Self {
            text_model,
            speech_synthesizer,
            voice_name,
            speaking_rate,
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
        let prompt = text.to_string();

        tauri::async_runtime::spawn(async move {
            let generated = text_model
                .generate(TextRequest {
                    prompt,
                    system_instruction: Some(format!(
                        "{}\n\nKeep replies brief (about 60 words), in plain spoken English, without markdown. Output only the words Moose should say; do not include voice labels, stage directions, sound cues, or bracketed delivery notes.",
                        system_instruction.unwrap_or_default()
                    )),
                    temperature: Some(0.85),
                    max_tokens: Some(100),
                })
                .await;
            let response = match generated {
                Ok(response) => prepare_local_tts_response(&response.text),
                Err(error) => {
                    // The previous turn may have been interrupted while its
                    // model request was in flight. Never publish its failure
                    // into a newer conversation generation.
                    if current_generation.load(Ordering::SeqCst) == turn {
                        let _ = event_sender.send(LiveServerEvent::Error(error)).await;
                    }
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
                        // KittenTTS has no pitch control. The saved pitch belongs to Google TTS
                        // and must not make Local conversation synthesis fail validation.
                        pitch: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::types::{AudioStreamData, TextResponse};
    use crate::tools::policy::ToolDeclaration;
    use std::sync::Mutex as StdMutex;

    struct ReplyModel;

    #[async_trait]
    impl TextModel for ReplyModel {
        async fn generate(&self, _request: TextRequest) -> Result<TextResponse, ProviderError> {
            Ok(TextResponse {
                text: "Hello there.".to_string(),
                finish_reason: None,
            })
        }
    }

    struct DelayedFailureModel(StdMutex<Option<tokio::sync::oneshot::Receiver<()>>>);

    #[async_trait]
    impl TextModel for DelayedFailureModel {
        async fn generate(&self, _request: TextRequest) -> Result<TextResponse, ProviderError> {
            let ready = self.0.lock().unwrap().take().unwrap();
            let _ = ready.await;
            Err(ProviderError::from_kind(ProviderErrorKind::Network))
        }
    }

    struct RequestCapture(Arc<StdMutex<Option<TtsRequest>>>);

    #[async_trait]
    impl SpeechSynthesizer for RequestCapture {
        async fn synthesize(&self, request: TtsRequest) -> Result<AudioStreamData, ProviderError> {
            *self.0.lock().unwrap() = Some(request);
            Ok(AudioStreamData {
                pcm_bytes: vec![1, 2],
                sample_rate: 24_000,
            })
        }
    }

    #[tokio::test]
    async fn local_conversation_does_not_apply_google_pitch_to_kittentts() {
        let request_capture = Arc::new(StdMutex::new(None));
        let provider = LocalConversationProvider::new(
            Arc::new(ReplyModel),
            Arc::new(RequestCapture(request_capture.clone())),
            "Leo".to_string(),
            0.95,
        );
        let (event_sender, mut events) = mpsc::channel(8);
        let mut session = provider
            .connect(
                LiveSessionConfig {
                    model: "local".to_string(),
                    voice_name: None,
                    system_instruction: None,
                    sample_rate_in: 16_000,
                    sample_rate_out: 24_000,
                    tools: Vec::<ToolDeclaration>::new(),
                },
                event_sender,
            )
            .await
            .unwrap();

        assert!(matches!(
            events.recv().await,
            Some(LiveServerEvent::Connected)
        ));
        session.send_text_turn("Hi Moose.").await.unwrap();

        loop {
            match events.recv().await {
                Some(LiveServerEvent::TurnComplete) => break,
                Some(_) => {}
                None => panic!("local conversation closed before synthesis completed"),
            }
        }

        let request = request_capture.lock().unwrap().clone().unwrap();
        assert_eq!(request.voice_name.as_deref(), Some("Leo"));
        assert_eq!(request.speaking_rate, Some(0.95));
        assert_eq!(request.pitch, None);
    }

    #[tokio::test]
    async fn an_interrupted_text_request_never_emits_a_stale_provider_error() {
        let (release, pending) = tokio::sync::oneshot::channel();
        let provider = LocalConversationProvider::new(
            Arc::new(DelayedFailureModel(StdMutex::new(Some(pending)))),
            Arc::new(RequestCapture(Arc::new(StdMutex::new(None)))),
            "Leo".to_string(),
            1.0,
        );
        let (sender, mut events) = mpsc::channel(8);
        let mut session = provider
            .connect(
                LiveSessionConfig {
                    model: "local".to_string(),
                    voice_name: None,
                    system_instruction: None,
                    sample_rate_in: 16_000,
                    sample_rate_out: 24_000,
                    tools: Vec::<ToolDeclaration>::new(),
                },
                sender,
            )
            .await
            .unwrap();
        assert!(matches!(events.recv().await, Some(LiveServerEvent::Connected)));
        session.send_text_turn("Hello").await.unwrap();
        session.interrupt().await.unwrap();
        assert!(matches!(events.recv().await, Some(LiveServerEvent::Interrupted)));
        release.send(()).unwrap();
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(100),
            events.recv(),
        )
        .await
        .is_err());
    }

    #[test]
    fn local_tts_response_is_bounded_and_plain_text() {
        let response = format!("{}\n{}", "Hello there.".repeat(50), "extra");

        let prepared = prepare_local_tts_response(&response);

        assert!(prepared.chars().count() <= MAX_LOCAL_TTS_RESPONSE_CHARS);
        assert!(!prepared.contains('\n'));
        assert!(prepared.ends_with('.'));
    }

    #[test]
    fn local_tts_response_removes_leading_voice_cue() {
        assert_eq!(
            prepare_local_tts_response("[Moose-esque voice] Ah, how can I help?"),
            "Ah, how can I help?"
        );
    }

    #[test]
    fn local_tts_response_removes_speaker_prefix_and_outer_quotes() {
        assert_eq!(
            prepare_local_tts_response(
                "Moose says: \"Ah, nice! Love watching you riff on those flicks.\""
            ),
            "Ah, nice! Love watching you riff on those flicks."
        );
    }

    #[test]
    fn local_tts_response_removes_joke_label() {
        assert_eq!(
            prepare_local_tts_response(
                "Joke: What do you call a moose with no legs? He's no longer a moose"
            ),
            "What do you call a moose with no legs? He's no longer a moose"
        );
    }

    #[test]
    fn local_tts_response_preserves_bracketed_spoken_content() {
        assert_eq!(
            prepare_local_tts_response("[sic], that's exactly what I meant."),
            "[sic], that's exactly what I meant."
        );
    }
}
