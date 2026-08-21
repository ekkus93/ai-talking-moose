use crate::ai::traits::*;
use crate::ai::types::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[derive(Default)]
pub struct FakeTextModel {
    pub custom_response: Option<String>,
}

#[async_trait]
impl TextModel for FakeTextModel {
    async fn generate(&self, _request: TextRequest) -> Result<TextResponse, String> {
        let text = self.custom_response.clone().unwrap_or_else(|| {
            "Well, look who's back at the terminal again. Don't strain yourself.".to_string()
        });
        Ok(TextResponse {
            text,
            finish_reason: Some("STOP".to_string()),
        })
    }
}

#[derive(Default)]
pub struct FakeSpeechSynthesizer;

#[async_trait]
impl SpeechSynthesizer for FakeSpeechSynthesizer {
    async fn synthesize(&self, _request: TtsRequest) -> Result<AudioStreamData, String> {
        // Generate 0.5s of synthetic beep/speech tones (24000 Hz, 16-bit PCM mono)
        let sample_rate = 24000;
        let num_samples = sample_rate / 2;
        let mut samples = Vec::with_capacity(num_samples as usize);

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let freq = 220.0 + (t * 50.0).sin() * 40.0;
            let val = (t * freq * 2.0 * std::f32::consts::PI).sin() * 0.4;
            samples.push((val * 32767.0) as i16);
        }

        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for s in samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }

        Ok(AudioStreamData {
            pcm_bytes: bytes,
            sample_rate,
        })
    }
}

pub struct FakeLiveSession {
    is_active: Arc<AtomicBool>,
}

#[async_trait]
impl LiveSession for FakeLiveSession {
    async fn send_audio_chunk(&mut self, _pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        // In fake mode, we simply accept audio chunks
        Ok(())
    }

    async fn send_text_turn(&mut self, _text: &str) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn send_tool_response(
        &mut self,
        _response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        self.is_active.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        self.is_active.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeConversationProvider;

#[async_trait]
impl RealtimeConversationProvider for FakeConversationProvider {
    async fn connect(
        &self,
        _config: LiveSessionConfig,
        event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError> {
        let is_active = Arc::new(AtomicBool::new(true));
        let active_clone = is_active.clone();

        // Spawn a background task simulating a back-and-forth conversational response
        tokio::spawn(async move {
            let _ = event_sender.send(LiveServerEvent::Connected).await;
            sleep(Duration::from_millis(400)).await;

            if !active_clone.load(Ordering::SeqCst) {
                return;
            }
            let _ = event_sender
                .send(LiveServerEvent::UserTranscript(TranscriptUpdate {
                    text: "Hello Moose".to_string(),
                    is_final: true,
                }))
                .await;

            sleep(Duration::from_millis(500)).await;
            if !active_clone.load(Ordering::SeqCst) {
                return;
            }

            let response_text = "Greetings, human. Still staring at the glowing glass box, I see.";
            let _ = event_sender
                .send(LiveServerEvent::ModelTranscript(TranscriptUpdate {
                    text: response_text.to_string(),
                    is_final: true,
                }))
                .await;

            // Stream a few audio chunks (0.2s each)
            let sample_rate = 24000;
            let chunk_size = 4800; // 0.2s
            for chunk_idx in 0..4 {
                if !active_clone.load(Ordering::SeqCst) {
                    let _ = event_sender.send(LiveServerEvent::Interrupted).await;
                    return;
                }

                let mut pcm = Vec::with_capacity(chunk_size * 2);
                for i in 0..chunk_size {
                    let t = (chunk_idx * chunk_size + i) as f32 / sample_rate as f32;
                    let val = (t * 260.0 * 2.0 * std::f32::consts::PI).sin() * 0.35;
                    let sample = (val * 32767.0) as i16;
                    pcm.extend_from_slice(&sample.to_le_bytes());
                }

                let _ = event_sender.send(LiveServerEvent::AudioData(pcm)).await;
                sleep(Duration::from_millis(200)).await;
            }

            let _ = event_sender.send(LiveServerEvent::TurnComplete).await;
        });

        Ok(Box::new(FakeLiveSession { is_active }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_text_model() {
        let model = FakeTextModel::default();
        let res = model
            .generate(TextRequest {
                prompt: "Test".to_string(),
                system_instruction: None,
                temperature: None,
                max_tokens: None,
            })
            .await
            .unwrap();
        assert!(res.text.contains("terminal"));
    }

    #[tokio::test]
    async fn test_fake_tts() {
        let tts = FakeSpeechSynthesizer;
        let res = tts
            .synthesize(TtsRequest {
                text: "Hello".to_string(),
                voice_name: None,
                speaking_rate: None,
                pitch: None,
            })
            .await
            .unwrap();
        assert!(!res.pcm_bytes.is_empty());
        assert_eq!(res.sample_rate, 24000);
    }

    #[tokio::test]
    async fn test_fake_live_conversation() {
        let (tx, mut rx) = mpsc::channel(16);
        let provider = FakeConversationProvider;
        let _session = provider
            .connect(
                LiveSessionConfig {
                    model: "fake".to_string(),
                    voice_name: None,
                    system_instruction: None,
                    sample_rate_in: 16000,
                    sample_rate_out: 24000,
                    tools: vec![],
                },
                tx,
            )
            .await
            .unwrap();

        let event = rx.recv().await.unwrap();
        match event {
            LiveServerEvent::Connected => {}
            _ => panic!("Expected Connected event"),
        }
    }
}
