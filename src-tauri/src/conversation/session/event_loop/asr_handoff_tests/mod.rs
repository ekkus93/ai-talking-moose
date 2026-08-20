use super::super::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

struct RecordingSession {
    audio_upload_count: Arc<AtomicUsize>,
    text_turns: Arc<SyncMutex<Vec<String>>>,
}

#[async_trait]
impl LiveSession for RecordingSession {
    async fn send_audio_chunk(&mut self, _pcm_bytes: &[u8]) -> Result<(), ProviderError> {
        self.audio_upload_count.fetch_add(1, AtomicOrdering::SeqCst);
        Ok(())
    }

    async fn send_text_turn(&mut self, text: &str) -> Result<(), ProviderError> {
        self.text_turns.lock().push(text.to_string());
        Ok(())
    }

    async fn send_tool_response(
        &mut self,
        _response: ToolCallResponse,
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn interrupt(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn close(&mut self) -> Result<(), ProviderError> {
        Ok(())
    }
}

struct CountingLocalAsrResource {
    stop_count: Arc<AtomicUsize>,
}

#[async_trait]
impl crate::asr::lifecycle::LocalAsrResource for CountingLocalAsrResource {
    async fn stop(&mut self) -> Result<(), crate::asr::AsrError> {
        self.stop_count.fetch_add(1, AtomicOrdering::SeqCst);
        Ok(())
    }
}

async fn attach_counting_local_asr(
    manager: &ConversationManager,
    generation: u64,
) -> Arc<AtomicUsize> {
    let stop_count = Arc::new(AtomicUsize::new(0));
    manager
        .local_asr_lifecycle()
        .attach(
            generation,
            Box::new(CountingLocalAsrResource {
                stop_count: stop_count.clone(),
            }),
        )
        .await
        .unwrap();
    stop_count
}

mod privacy;
mod routing;
mod serialization;
mod stale;
