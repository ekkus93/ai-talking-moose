use crate::ai::types::*;
use async_trait::async_trait;
use tokio::sync::mpsc;

#[async_trait]
pub trait TextModel: Send + Sync {
    async fn generate(&self, request: TextRequest) -> Result<TextResponse, String>;
}

#[async_trait]
pub trait SpeechSynthesizer: Send + Sync {
    async fn synthesize(&self, request: TtsRequest) -> Result<AudioStreamData, String>;
}

#[async_trait]
pub trait LiveSession: Send + Sync {
    async fn send_audio_chunk(&mut self, pcm_bytes: &[u8]) -> Result<(), ProviderError>;
    async fn send_text_turn(&mut self, text: &str) -> Result<(), ProviderError>;
    async fn send_tool_response(&mut self, response: ToolCallResponse)
        -> Result<(), ProviderError>;
    async fn interrupt(&mut self) -> Result<(), ProviderError>;
    async fn close(&mut self) -> Result<(), ProviderError>;
}

#[async_trait]
pub trait RealtimeConversationProvider: Send + Sync {
    async fn connect(
        &self,
        config: LiveSessionConfig,
        event_sender: mpsc::Sender<LiveServerEvent>,
    ) -> Result<Box<dyn LiveSession>, ProviderError>;
}
