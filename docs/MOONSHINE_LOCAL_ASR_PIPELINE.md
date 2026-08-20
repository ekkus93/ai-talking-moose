# Moonshine Local ASR Pipeline

Status: **V1R-ASR-009 implementation contract; V1R-ASR-012 production lifecycle integrated**
Recorded: 2026-08-20

## Ownership

Talking Moose has one production microphone owner: `AudioCapture`. Local Moonshine does not open CPAL or create a second microphone stream. `LocalAsrPipeline::start_capture` starts the caller-owned `AudioCapture` with the local pipeline's bounded ingress sender.

Cloud Gemini Live audio and local Moonshine are separate modes. The local pipeline has no provider/session handle and contains no cloud fallback path.

## PCM contract

`AudioCapture` owns device-format conversion:

1. CPAL reads the negotiated device format (`f32`, `i16`, or `u16`).
2. Capture converts to `f32` as necessary.
3. Interleaved input is downmixed to mono.
4. Mono input is linearly resampled to **16,000 Hz**.
5. Capture emits **100 ms** chunks as signed **16-bit little-endian mono PCM**.
6. The dedicated local-ASR worker converts each accepted chunk to mono `f32` before calling `MoonshineTinyEngine::push_pcm`.

The Moonshine engine boundary therefore remains exactly **mono `f32`, 16 kHz**. ASR-009 does not perform a second resample in the inference worker.

## Bounded queue and overload policy

The local inference ingress is the same bounded `tokio::sync::mpsc` channel type already consumed by production `AudioCapture`, with a hard capacity of **8 chunks**. At 100 ms per capture chunk this caps queued microphone audio at approximately **800 ms**. The receiver is owned exclusively by the dedicated OS inference thread and is polled synchronously; no inference work runs on Tokio.

The CPAL callback is never allowed to block on inference. It uses `try_send`:

- if space is available, the chunk is queued;
- if the queue is full, the **newest** chunk is dropped;
- existing queued audio is retained;
- `AudioCaptureDiagnostics.dropped_chunks` remains the authoritative rejected-microphone-chunk counter;
- pipeline queue depth is derived from the bounded sender capacity and reports zero once pipeline ingress is closed.

Dropping newest audio matches the existing production microphone overload policy and avoids replacing older speech with a discontinuous future fragment.

## Worker model

Moonshine model verification, native transcriber/stream construction, and every `push_pcm` inference call run on a named dedicated OS thread (`moonshine-tiny-asr`). Long native inference therefore does not run on:

- the CPAL callback thread;
- a Tokio async-runtime worker;
- the Tauri/UI thread.

Pipeline startup uses an async one-shot readiness handshake. A missing/corrupt model or native startup failure is returned before microphone capture is started.

Native Moonshine line updates are consumed by the ASR-010 transcript state machine inside the worker. The callback boundary exposes only provider-neutral `AsrEvent` values; partial/final replacement and speech lifecycle semantics are documented in `MOONSHINE_TRANSCRIPT_STATE.md`.

## Shutdown and cancellation

`stop_and_join`:

1. sets the cooperative stop flag;
2. closes the pipeline-owned ingress sender;
3. causes queued-but-not-yet-inferred audio to be discarded rather than drained;
4. asks the Tiny engine to stop after the currently executing native call returns;
5. joins the OS worker through `tokio::task::spawn_blocking`, so joining cannot block a Tokio runtime worker;
6. is idempotent.

`LocalAsrPipeline` implements `LocalAsrResource`, allowing ASR-012 to attach the real worker to the already-established authoritative conversation lifecycle. The conversation shutdown order remains microphone stop first, then local-ASR worker stop/join.

A native inference function already executing cannot be preempted safely by Rust; cancellation is observed immediately before the next queue receive. This is preferable to abandoning a native worker or freeing its resources concurrently.

## Production conversation integration

`ConversationManager::start_session` now owns the ASR-012 Tiny startup transaction:

1. verify/open the Tiny pipeline before opening the Gemini Live session;
2. fail closed before microphone capture if the model is missing/corrupt or the native runtime cannot load;
3. open the provider and output only after local-ASR prerequisites are valid;
4. start the one authoritative `AudioCapture` directly on the pipeline's bounded ingress queue;
5. attach the running pipeline to `LocalAsrLifecycle` for the current generation;
6. publish the session as active/Listening only after microphone startup and lifecycle attachment succeed;
7. route final local transcripts through `ConversationManager::handle_local_asr_event`, which sends exactly one Gemini text turn after generation/session checks.

Gemini cloud-audio mode retains its separate microphone-to-provider queue. Tiny mode never creates that queue, so there is no accidental provider audio-upload path to fall back to. `shutdown_locked` stops microphone capture first and then stops/joins the attached local-ASR worker for Stop, Mute, Dismiss, provider loss, mode replacement, and application exit. Barge-in flushes/suppresses the stale Moose response while deliberately leaving the current local recognizer attached.

Moonshine Small remains fail-closed until V1R-ASR-008 supplies a production Small engine/pipeline path.

## Diagnostics

`LocalAsrPipelineDiagnostics` exposes:

- required input sample rate;
- current queue depth;
- hard queue capacity;
- capture-side dropped chunk count remains available from `AudioCaptureDiagnostics`;
- worker running state;
- last typed terminal ASR error.

ASR-015 will aggregate these values into the user-facing diagnostics surface.
