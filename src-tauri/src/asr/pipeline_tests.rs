use super::*;
use crate::test_support::{assert_log_capture_live, capture_logs};
use base64::Engine as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex as StdMutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct PushGate {
    state: StdMutex<(bool, bool)>,
    changed: Condvar,
}

impl PushGate {
    fn block_until_released(&self) {
        let mut state = self.state.lock().unwrap();
        state.0 = true;
        self.changed.notify_all();
        while !state.1 {
            state = self.changed.wait(state).unwrap();
        }
    }

    fn wait_until_entered(&self) {
        let mut state = self.state.lock().unwrap();
        while !state.0 {
            state = self.changed.wait(state).unwrap();
        }
    }

    fn release(&self) {
        let mut state = self.state.lock().unwrap();
        state.1 = true;
        self.changed.notify_all();
    }
}

#[derive(Default)]
struct FakeState {
    pushes: AtomicUsize,
    stops: AtomicUsize,
    received_pcm: StdMutex<Vec<Vec<f32>>>,
    fail_push: StdMutex<Option<AsrError>>,
    updates: StdMutex<Vec<StreamingTranscriptUpdate>>,
    stop_updates: StdMutex<Vec<StreamingTranscriptUpdate>>,
    push_gate: StdMutex<Option<Arc<PushGate>>>,
    block_push: AtomicBool,
    worker_thread: StdMutex<Option<thread::ThreadId>>,
}

struct FakeEngine {
    state: Arc<FakeState>,
    sample_rate: u32,
}

impl PipelineEngine for FakeEngine {
    fn input_sample_rate_hz(&self) -> u32 {
        self.sample_rate
    }

    fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        *self.state.worker_thread.lock().unwrap() = Some(thread::current().id());
        self.state.pushes.fetch_add(1, Ordering::SeqCst);
        self.state.received_pcm.lock().unwrap().push(pcm.to_vec());
        // Clone the optional gate while holding the mutex, then release the
        // mutex before blocking. Keeping the temporary lock guard alive across
        // block_until_released() deadlocks tests that clear the gate before
        // releasing the worker.
        let push_gate = self.state.push_gate.lock().unwrap().clone();
        if let Some(gate) = push_gate {
            gate.block_until_released();
        }
        while self.state.block_push.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));
        }
        if let Some(error) = self.state.fail_push.lock().unwrap().clone() {
            return Err(error);
        }
        Ok(std::mem::take(&mut *self.state.updates.lock().unwrap()))
    }

    fn stop(&mut self) -> Result<Vec<StreamingTranscriptUpdate>, AsrError> {
        self.state.stops.fetch_add(1, Ordering::SeqCst);
        Ok(std::mem::take(
            &mut *self.state.stop_updates.lock().unwrap(),
        ))
    }
}

fn callback_events() -> (
    LocalAsrPipelineEventCallback,
    Arc<StdMutex<Vec<LocalAsrPipelineEvent>>>,
) {
    let events = Arc::new(StdMutex::new(Vec::new()));
    let callback_events = events.clone();
    let callback: LocalAsrPipelineEventCallback = Arc::new(move |event, _ack| {
        callback_events.lock().unwrap().push(event);
    });
    (callback, events)
}

async fn fake_pipeline(state: Arc<FakeState>) -> LocalAsrPipeline {
    fake_pipeline_for_architecture(LocalAsrArchitecture::MoonshineTinyStreaming, state).await
}

async fn fake_pipeline_for_architecture(
    architecture: LocalAsrArchitecture,
    state: Arc<FakeState>,
) -> LocalAsrPipeline {
    let (callback, _) = callback_events();
    LocalAsrPipeline::start_architecture(
        architecture,
        local_asr_queue_capacity(architecture),
        move || {
            Ok(Box::new(FakeEngine {
                state,
                sample_rate: LOCAL_ASR_INPUT_SAMPLE_RATE_HZ,
            }))
        },
        callback,
        WORKER_STARTUP_TIMEOUT,
        None,
    )
    .await
    .unwrap()
}

fn wait_until(predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out waiting for worker");
        thread::sleep(Duration::from_millis(1));
    }
}

async fn wait_until_async(predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out waiting for worker");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

#[path = "pipeline_tests/architecture.rs"]
mod architecture;
#[path = "pipeline_tests/audio.rs"]
mod audio;
#[path = "pipeline_tests/privacy.rs"]
mod privacy;
#[path = "pipeline_tests/queue.rs"]
mod queue;
#[path = "pipeline_tests/readiness.rs"]
mod readiness;
#[path = "pipeline_tests/shutdown.rs"]
mod shutdown;
#[path = "pipeline_tests/transcripts.rs"]
mod transcripts;
