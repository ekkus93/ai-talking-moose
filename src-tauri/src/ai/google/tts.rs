use crate::ai::google::auth::GoogleAuth;
use crate::ai::traits::SpeechSynthesizer;
use crate::ai::types::{AudioStreamData, TtsRequest};
use async_trait::async_trait;
use base64::Engine;
use reqwest::Client;
use serde_json::json;
use tracing::{info, warn};

pub struct GoogleSpeechSynthesizer {
    auth: GoogleAuth,
    default_voice: String,
    client: Client,
}

impl GoogleSpeechSynthesizer {
    pub fn new(auth: GoogleAuth, default_voice: String) -> Self {
        Self {
            auth,
            default_voice,
            client: Client::new(),
        }
    }

    fn generate_cartoon_synth_audio(text: &str) -> AudioStreamData {
        let sample_rate = 24000;
        // Generate duration proportional to text length (~60ms per character)
        let duration_secs = (text.len() as f32 * 0.065).clamp(0.8, 6.0);
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut samples = Vec::with_capacity(num_samples);

        for (i, _) in (0..num_samples).enumerate() {
            let t = i as f32 / sample_rate as f32;
            // Wobbly pitch cadence characteristic of 80s/90s Mac speech
            let base_freq = 130.0 + (t * 8.0).sin() * 25.0 + (t * 22.0).sin() * 15.0;
            let formant1 = (t * base_freq * 2.0 * std::f32::consts::PI).sin();
            let formant2 = (t * base_freq * 2.5 * 2.0 * std::f32::consts::PI).sin() * 0.5;
            let val = ((formant1 + formant2) * 0.3).clamp(-0.9, 0.9);
            samples.push((val * 32767.0) as i16);
        }

        let mut pcm_bytes = Vec::with_capacity(samples.len() * 2);
        for s in samples {
            pcm_bytes.extend_from_slice(&s.to_le_bytes());
        }

        AudioStreamData {
            pcm_bytes,
            sample_rate,
        }
    }
}

#[async_trait]
impl SpeechSynthesizer for GoogleSpeechSynthesizer {
    async fn synthesize(&self, request: TtsRequest) -> Result<AudioStreamData, String> {
        if !self.auth.is_valid() {
            return Ok(Self::generate_cartoon_synth_audio(&request.text));
        }

        let voice = request
            .voice_name
            .unwrap_or_else(|| self.default_voice.clone());

        // 1. Try Gemini 2.0 Flash audio generation endpoint (works with AI Studio API keys)
        let gemini_url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-preview-tts:generateContent?key={}",
            self.auth.api_key
        );

        let gemini_body = json!({
            "contents": [
                {
                    "parts": [
                        { "text": format!("Speak the following line in character as a cartoon moose: {}", request.text) }
                    ]
                }
            ],
            "generationConfig": {
                "responseModalities": ["AUDIO"],
                "speechConfig": {
                    "voiceConfig": {
                        "prebuiltVoiceConfig": {
                            "voiceName": voice
                        }
                    }
                }
            }
        });

        if let Ok(resp) = self
            .client
            .post(&gemini_url)
            .json(&gemini_body)
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(json_val) = resp.json::<serde_json::Value>().await {
                    if let Some(parts) = json_val["candidates"][0]["content"]["parts"].as_array() {
                        for part in parts {
                            if let Some(b64) = part["inlineData"]["data"].as_str() {
                                if let Ok(pcm_bytes) =
                                    base64::engine::general_purpose::STANDARD.decode(b64)
                                {
                                    info!(
                                        "Gemini generated audio stream successfully ({} bytes)",
                                        pcm_bytes.len()
                                    );
                                    return Ok(AudioStreamData {
                                        pcm_bytes,
                                        sample_rate: 24000,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Fallback to vintage MacinTalk-style cartoon synth tones if API key doesn't support cloud TTS
        warn!("Falling back to cartoon synth audio for remark");
        Ok(Self::generate_cartoon_synth_audio(&request.text))
    }
}
