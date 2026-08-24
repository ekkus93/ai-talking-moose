use parking_lot::Mutex;
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;

const AMBIENT_QUEUE_CAPACITY: usize = 32;
const MAX_AMBIENT_EVENT_SUMMARY_CHARS: usize = 2_048;
const AMBIENT_SETTLE_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbientEventCategory {
    Manual,
    Application,
    WindowTitle,
    Idle,
    Power,
    Wake,
    System,
    Other,
}

impl AmbientEventCategory {
    pub fn from_event_name(event_name: &str) -> Self {
        let normalized = event_name.trim().to_ascii_lowercase();
        if normalized.contains("window_title") || normalized.contains("window-title") {
            Self::WindowTitle
        } else if normalized.contains("active_app")
            || normalized.contains("active-app")
            || normalized.contains("application")
        {
            Self::Application
        } else if normalized.contains("idle") {
            Self::Idle
        } else if normalized.contains("battery") || normalized.contains("power") {
            Self::Power
        } else if normalized.contains("wake") || normalized.contains("resume") {
            Self::Wake
        } else if normalized.contains("manual") {
            Self::Manual
        } else if normalized.contains("system") {
            Self::System
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone)]
pub struct AmbientEvent {
    pub category: AmbientEventCategory,
    pub summary: String,
    pub importance: f32,
}

impl AmbientEvent {
    pub fn new(event_name: &str, summary: String, importance: f32) -> Self {
        let importance = if importance.is_finite() {
            importance.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let summary = summary
            .chars()
            .take(MAX_AMBIENT_EVENT_SUMMARY_CHARS)
            .collect();
        Self {
            category: AmbientEventCategory::from_event_name(event_name),
            summary,
            importance,
        }
    }

    pub fn fingerprint(&self) -> String {
        let normalized = normalize_summary_for_fingerprint(&self.summary);
        let input = format!("{:?}:{normalized}", self.category);
        let digest = digest(&SHA256, input.as_bytes());
        let mut fingerprint = String::with_capacity(64);
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for &byte in digest.as_ref() {
            fingerprint.push(char::from(HEX[usize::from(byte >> 4)]));
            fingerprint.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        fingerprint
    }
}

fn normalize_summary_for_fingerprint(summary: &str) -> String {
    let mut normalized = String::with_capacity(summary.len().min(256));
    let mut previous_separator = true;
    let mut in_number = false;

    for character in summary.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_digit() {
            if !in_number {
                normalized.push('#');
            }
            in_number = true;
            previous_separator = false;
        } else if character.is_alphanumeric() {
            normalized.push(character);
            in_number = false;
            previous_separator = false;
        } else {
            in_number = false;
            if !previous_separator {
                normalized.push(' ');
                previous_separator = true;
            }
        }

        if normalized.chars().count() >= 192 {
            break;
        }
    }

    normalized.trim().to_string()
}

struct AmbientRequest {
    event: AmbientEvent,
    epoch: u64,
    response: oneshot::Sender<Result<Option<String>, String>>,
}

struct AmbientSchedulerInner {
    sender: mpsc::Sender<AmbientRequest>,
    receiver: Mutex<Option<mpsc::Receiver<AmbientRequest>>>,
    cancellation: CancellationToken,
    current_request: Mutex<Option<CancellationToken>>,
    epoch: AtomicU64,
    started: AtomicBool,
    shutdown_complete: watch::Sender<bool>,
}

struct ShutdownSignal(watch::Sender<bool>);

impl Drop for ShutdownSignal {
    fn drop(&mut self) {
        let _ = self.0.send(true);
    }
}

#[derive(Clone)]
pub struct AmbientScheduler {
    inner: Arc<AmbientSchedulerInner>,
}

impl Default for AmbientScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl AmbientScheduler {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(AMBIENT_QUEUE_CAPACITY);
        let (shutdown_complete, _shutdown_rx) = watch::channel(false);
        Self {
            inner: Arc::new(AmbientSchedulerInner {
                sender,
                receiver: Mutex::new(Some(receiver)),
                cancellation: CancellationToken::new(),
                current_request: Mutex::new(None),
                epoch: AtomicU64::new(0),
                started: AtomicBool::new(false),
                shutdown_complete,
            }),
        }
    }

    pub fn start<F, Fut>(&self, handler: F) -> Result<(), String>
    where
        F: Fn(AmbientEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Option<String>, String>> + Send + 'static,
    {
        let mut receiver = self
            .inner
            .receiver
            .lock()
            .take()
            .ok_or_else(|| "ambient scheduler is already started".to_string())?;
        self.inner.started.store(true, AtomicOrdering::SeqCst);
        let inner = self.inner.clone();
        let cancellation = inner.cancellation.clone();
        let shutdown_signal = ShutdownSignal(inner.shutdown_complete.clone());

        tauri::async_runtime::spawn(async move {
            let _shutdown_signal = shutdown_signal;
            loop {
                let request = tokio::select! {
                    _ = cancellation.cancelled() => break,
                    request = receiver.recv() => match request {
                        Some(request) => request,
                        None => break,
                    },
                };

                if request.epoch != inner.epoch.load(AtomicOrdering::SeqCst) {
                    let _ = request.response.send(Ok(None));
                    continue;
                }

                let request_cancellation = CancellationToken::new();
                *inner.current_request.lock() = Some(request_cancellation.clone());
                if request.epoch != inner.epoch.load(AtomicOrdering::SeqCst) {
                    request_cancellation.cancel();
                }

                let settled = tokio::select! {
                    _ = cancellation.cancelled() => false,
                    _ = request_cancellation.cancelled() => false,
                    _ = tokio::time::sleep(AMBIENT_SETTLE_DELAY) => true,
                };
                if !settled {
                    *inner.current_request.lock() = None;
                    let _ = request.response.send(Ok(None));
                    if cancellation.is_cancelled() {
                        break;
                    }
                    continue;
                }

                let result = tokio::select! {
                    _ = cancellation.cancelled() => None,
                    _ = request_cancellation.cancelled() => None,
                    result = handler(request.event) => Some(result),
                };
                *inner.current_request.lock() = None;
                match result {
                    Some(result) => {
                        let _ = request.response.send(result);
                    }
                    None => {
                        let _ = request.response.send(Ok(None));
                        if cancellation.is_cancelled() {
                            break;
                        }
                    }
                }
            }

            while let Ok(request) = receiver.try_recv() {
                let _ = request.response.send(Ok(None));
            }
        });

        Ok(())
    }

    pub fn interrupt(&self) {
        self.inner.epoch.fetch_add(1, AtomicOrdering::SeqCst);
        if let Some(cancellation) = self.inner.current_request.lock().as_ref() {
            cancellation.cancel();
        }
    }

    pub fn try_submit_background(&self, event: AmbientEvent) -> Result<bool, String> {
        if self.inner.cancellation.is_cancelled() {
            return Ok(false);
        }
        if !self.inner.started.load(AtomicOrdering::SeqCst) {
            return Err("ambient scheduler is not running".to_string());
        }

        let (response_tx, _response_rx) = oneshot::channel();
        let request = AmbientRequest {
            event,
            epoch: self.inner.epoch.load(AtomicOrdering::SeqCst),
            response: response_tx,
        };
        match self.inner.sender.try_send(request) {
            Ok(()) => Ok(true),
            Err(mpsc::error::TrySendError::Full(_)) => Ok(false),
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err("ambient scheduler is not running".to_string())
            }
        }
    }

    pub async fn submit(&self, event: AmbientEvent) -> Result<Option<String>, String> {
        if self.inner.cancellation.is_cancelled() {
            return Ok(None);
        }
        if !self.inner.started.load(AtomicOrdering::SeqCst) {
            return Err("ambient scheduler is not running".to_string());
        }

        let (response_tx, response_rx) = oneshot::channel();
        let epoch = self.inner.epoch.load(AtomicOrdering::SeqCst);
        self.inner
            .sender
            .send(AmbientRequest {
                event,
                epoch,
                response: response_tx,
            })
            .await
            .map_err(|_| "ambient scheduler is not running".to_string())?;

        response_rx
            .await
            .map_err(|_| "ambient scheduler stopped before completing the request".to_string())?
    }

    pub async fn stop(&self) {
        let mut shutdown_complete = self.inner.shutdown_complete.subscribe();
        self.inner.cancellation.cancel();
        if !self.inner.started.load(AtomicOrdering::SeqCst) || *shutdown_complete.borrow() {
            return;
        }
        let _ = shutdown_complete.changed().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn background_submission_is_hard_bounded_and_drops_newest_when_full() {
        let scheduler = AmbientScheduler::new();
        scheduler.inner.started.store(true, AtomicOrdering::SeqCst);

        for index in 0..AMBIENT_QUEUE_CAPACITY {
            assert!(scheduler
                .try_submit_background(AmbientEvent::new("desktop", format!("event {index}"), 0.5,))
                .unwrap());
        }
        assert!(!scheduler
            .try_submit_background(AmbientEvent::new("desktop", "drop newest".to_string(), 0.5,))
            .unwrap());
    }

    #[test]
    fn event_summary_is_bounded_before_entering_the_scheduler_queue() {
        let oversized = format!(
            "PRIVATE:{}",
            "x".repeat(MAX_AMBIENT_EVENT_SUMMARY_CHARS * 4)
        );
        let event = AmbientEvent::new("manual", oversized, 0.8);

        assert_eq!(
            event.summary.chars().count(),
            MAX_AMBIENT_EVENT_SUMMARY_CHARS
        );
        assert!(event.summary.starts_with("PRIVATE:"));
    }

    #[test]
    fn event_categories_and_fingerprints_are_normalized_without_retaining_text() {
        let first = AmbientEvent::new(
            "active_app_changed",
            "VS Code 123 -- README.md".to_string(),
            0.8,
        );
        let second = AmbientEvent::new("application", "vs code 456, README md!!!".to_string(), 0.8);

        assert_eq!(first.category, AmbientEventCategory::Application);
        assert_eq!(second.category, AmbientEventCategory::Application);
        assert_eq!(first.fingerprint(), second.fingerprint());
        assert!(!first.fingerprint().contains("readme"));
    }

    #[tokio::test]
    async fn scheduler_cancels_an_in_flight_handler() {
        let scheduler = AmbientScheduler::new();
        let handler_started = Arc::new(tokio::sync::Notify::new());
        let handler_finished = Arc::new(AtomicUsize::new(0));
        let started_for_task = handler_started.clone();
        let finished_for_task = handler_finished.clone();
        scheduler
            .start(move |_event| {
                let started = started_for_task.clone();
                let finished = finished_for_task.clone();
                async move {
                    started.notify_one();
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    finished.fetch_add(1, Ordering::SeqCst);
                    Ok(Some("unexpected".to_string()))
                }
            })
            .unwrap();

        let pending_scheduler = scheduler.clone();
        let pending = tokio::spawn(async move {
            pending_scheduler
                .submit(AmbientEvent::new("manual", "cancel me".to_string(), 1.0))
                .await
        });
        handler_started.notified().await;
        scheduler.stop().await;

        let result = tokio::time::timeout(Duration::from_secs(1), pending)
            .await
            .expect("cancelled scheduler request should resolve promptly")
            .expect("submit task should not panic")
            .expect("scheduler cancellation is not an error");
        assert!(result.is_none());
        assert_eq!(handler_finished.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn scheduler_interrupt_cancels_current_request_but_keeps_running() {
        let scheduler = AmbientScheduler::new();
        let handler_started = Arc::new(tokio::sync::Notify::new());
        let started_for_task = handler_started.clone();
        scheduler
            .start(move |event| {
                let started = started_for_task.clone();
                async move {
                    started.notify_one();
                    if event.summary == "first" {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    }
                    Ok(Some(event.summary))
                }
            })
            .unwrap();

        let first_scheduler = scheduler.clone();
        let first = tokio::spawn(async move {
            first_scheduler
                .submit(AmbientEvent::new("manual", "first".to_string(), 1.0))
                .await
        });
        handler_started.notified().await;
        scheduler.interrupt();

        assert!(first.await.unwrap().unwrap().is_none());

        let later = scheduler
            .submit(AmbientEvent::new("manual", "later".to_string(), 1.0))
            .await
            .unwrap();
        assert_eq!(later.as_deref(), Some("later"));
        scheduler.stop().await;
    }

    #[tokio::test]
    async fn scheduler_serializes_requests_and_stops_fail_closed() {
        let scheduler = AmbientScheduler::new();
        let handled = Arc::new(AtomicUsize::new(0));
        let handled_for_task = handled.clone();
        scheduler
            .start(move |event| {
                let handled = handled_for_task.clone();
                async move {
                    handled.fetch_add(1, Ordering::SeqCst);
                    Ok(Some(event.summary))
                }
            })
            .unwrap();

        let result = scheduler
            .submit(AmbientEvent::new("manual", "safe event".to_string(), 1.0))
            .await
            .unwrap();
        assert_eq!(result.as_deref(), Some("safe event"));
        assert_eq!(handled.load(Ordering::SeqCst), 1);

        scheduler.stop().await;
        let stopped = scheduler
            .submit(AmbientEvent::new("manual", "later".to_string(), 1.0))
            .await
            .unwrap();
        assert!(stopped.is_none());
    }
}
