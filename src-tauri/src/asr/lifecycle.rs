use crate::asr::types::LocalAsrRuntimeDiagnostics;
use crate::asr::AsrError;
use async_trait::async_trait;
use tokio::sync::Mutex;

#[async_trait]
pub trait LocalAsrResource: Send {
    async fn stop(&mut self) -> Result<(), AsrError>;

    fn diagnostics(&self) -> Option<LocalAsrRuntimeDiagnostics> {
        None
    }
}

struct ActiveLocalAsrResource {
    generation: u64,
    resource: Box<dyn LocalAsrResource>,
}

#[derive(Default)]
pub struct LocalAsrLifecycle {
    operation: Mutex<()>,
    active: Mutex<Option<ActiveLocalAsrResource>>,
}

impl LocalAsrLifecycle {
    pub async fn attach(
        &self,
        generation: u64,
        mut resource: Box<dyn LocalAsrResource>,
    ) -> Result<(), AsrError> {
        let _operation_guard = self.operation.lock().await;
        let previous = self.active.lock().await.take();
        if let Some(mut previous) = previous {
            if let Err(error) = previous.resource.stop().await {
                let _ = resource.stop().await;
                return Err(error);
            }
        }
        *self.active.lock().await = Some(ActiveLocalAsrResource {
            generation,
            resource,
        });
        Ok(())
    }

    pub async fn stop_and_clear(&self) -> Result<(), AsrError> {
        let _operation_guard = self.operation.lock().await;
        let current = self.active.lock().await.take();
        let Some(mut current) = current else {
            return Ok(());
        };
        current.resource.stop().await
    }

    pub async fn is_active(&self) -> bool {
        self.active.lock().await.is_some()
    }

    pub async fn diagnostics(&self) -> Option<LocalAsrRuntimeDiagnostics> {
        self.active
            .lock()
            .await
            .as_ref()
            .and_then(|active| active.resource.diagnostics())
    }

    pub async fn accepts_callback(&self, generation: u64) -> bool {
        self.active
            .lock()
            .await
            .as_ref()
            .is_some_and(|active| active.generation == generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CountingResource {
        stop_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LocalAsrResource for CountingResource {
        async fn stop(&mut self) -> Result<(), AsrError> {
            self.stop_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn replacing_and_stopping_resources_is_idempotent() {
        let lifecycle = LocalAsrLifecycle::default();
        let first_stop = Arc::new(AtomicUsize::new(0));
        let second_stop = Arc::new(AtomicUsize::new(0));

        lifecycle
            .attach(
                1,
                Box::new(CountingResource {
                    stop_count: first_stop.clone(),
                }),
            )
            .await
            .unwrap();
        lifecycle
            .attach(
                2,
                Box::new(CountingResource {
                    stop_count: second_stop.clone(),
                }),
            )
            .await
            .unwrap();

        assert_eq!(first_stop.load(Ordering::SeqCst), 1);
        assert!(lifecycle.accepts_callback(2).await);
        assert!(!lifecycle.accepts_callback(1).await);

        lifecycle.stop_and_clear().await.unwrap();
        lifecycle.stop_and_clear().await.unwrap();
        assert_eq!(second_stop.load(Ordering::SeqCst), 1);
        assert!(!lifecycle.is_active().await);
    }
}
