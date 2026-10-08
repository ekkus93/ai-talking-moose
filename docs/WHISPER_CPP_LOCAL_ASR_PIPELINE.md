# Whisper.cpp Local ASR Pipeline

**Status:** Current production behavior has passed local gates, ordinary CI, and real-CPU acceptance on Linux x86_64.
**Production source:** `29759a18d7c91c26113d33e16a55c6d7f80bdb0d`; exact acceptance checkout `cf3280ea1d4e91d298828e4935e3369f8cbd6be6` is a documentation-only descendant.
**Initial qualification:** run `37744371559`, job `113202252940`; see `docs/evidence/WHISPER_CPP_LOCAL_ASR_QUALIFICATION_2026-10-08.md`.
**Post-qualification review:** completed in `docs/WHISPER_CPP_LOCAL_ASR_POST_QUALIFICATION_REVIEW_TODO_2026-10-08.md`; evidence is in `docs/evidence/WHISPER_CPP_LOCAL_ASR_POST_QUALIFICATION_2026-10-08.md` (run `37756211710`, job `113241297405`).

This document describes the current production pipeline behavior. Historical qualification evidence applies only to its recorded source checkout and does not qualify later source changes.
It supersedes implementation-state claims in `docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md`; the dated document remains historical context.

## Provenance and runtime identity

The production Whisper runtime separates two identities that were previously conflated:

- **Native source identity:** the tracked `third_party/whisper.cpp` gitlink revision used to build the linked runtime.
- **Model artifact identity:** the pinned Hugging Face `ggml-small.bin` model revision, byte count, and SHA-256 used by the installer and verifier.

`src-tauri/src/asr/whisper/manifest.rs` is the authoritative runtime manifest for these production pins. `scripts/check_whisper_provenance.py` verifies that the manifest, tracked native source revision, license documents, and build policy remain mutually consistent.

## Install layout

The canonical on-disk Whisper Small layout is:

```text
<app-data>/models/whisper/whisper-small/
```

The model artifact and model-specific metadata live under that per-model directory. The production installer is responsible for creating the model root and parents before staging, probing disk space, verifying artifacts, or writing canonical metadata. Workflow or test harnesses must not pre-create this directory as a substitute for production behavior.

Existing deployed profiles that used the older `<app-data>/models/whisper/` layout are migrated through the installer compatibility path when a persistent application database path is present. Migration is fail-closed: a legacy artifact is accepted only after verification against current immutable pins; otherwise normal `NotInstalled`/`Corrupt` behavior applies.

## Installed-model verification

Whisper model verification is streaming and bounded-memory:

- SHA-256 is computed incrementally.
- Exact byte count is checked while streaming.
- Required magic/header validation is preserved.
- Integrity failures map to corrupt-model diagnostics rather than runtime/load failures.

Descriptor retrieval from `get_asr_models()` uses blocking isolation before hashing installed model artifacts, so a full installed-model verification does not run directly on the Tokio/Tauri async command executor.

## Whisper ASR error mapping

Model verification has one explicit mapping boundary in `map_verification_error()`:

| Verification result | Public `AsrErrorKind` |
|---|---|
| No installed model | `ModelNotInstalled` (handled directly by engine open) |
| Size, SHA-256, magic, or installed-marker integrity mismatch | `ModelCorrupt` |
| Invalid bundled manifest or filesystem access failure | `Internal` |
| Native runtime not linked | `RuntimeUnavailable` |
| Verified model path/native model load failure | `ModelLoadFailed` |
| Invalid/non-finite PCM | `AudioInput` |
| Native inference or segment extraction failure | `Inference` |
| Audio after stop or other invalid lifecycle use | `InvalidState` |
| Worker invariant/panic/join failure | `Internal` |

Cancellation, download, HTTP, disk-space, and promotion failures belong to the explicit installer API and do not arise through Whisper engine startup; the mapper treats such variants reaching verification as `Internal`.

## Startup and dependency isolation

`prepare_local_asr()` resolves mode-specific installer/runtime dependencies inside the selected ASR-mode branch. Whisper startup requires the Whisper installer plus shared local-ASR pipeline dependencies; it must not fail merely because a Moonshine-only installer dependency is unavailable.

The production startup path is fail-closed before microphone capture. A selected local ASR mode must create and verify its pipeline before the existing authoritative microphone capture object is started.

## Pipeline worker model

Whisper uses the shared `LocalAsrPipeline` bounded-worker path:

- One authoritative capture source feeds a bounded local-ASR ingress queue.
- Moonshine ingress has capacity eight chunks (about 800 ms). Whisper has capacity 56 chunks (about 5.6 seconds) because each partial runs synchronous whole-utterance inference; the larger Whisper-only queue absorbs that bounded CPU pause while preserving the Moonshine queue budget.
- Native model loading and inference run on a dedicated OS worker, not on the async caller thread and not on the CPAL callback thread.
- The worker emits provider-neutral `AsrEvent` values into the conversation layer.

The normal producer policy is non-blocking. When capture cannot enqueue because the bounded queue is full, the newest chunk is dropped by the capture/producer side and diagnostics own the authoritative overload counter.

## Whisper utterance state

Whisper no longer treats every approximately 300 ms batch as a final utterance. It maintains an explicit utterance state:

- `WHISPER_PARTIAL_INTERVAL_SAMPLES = 80_000` at 16 kHz, five seconds. Each partial re-transcribes the accumulated utterance, so this cadence gives the worker time to accept queued microphone audio between inference calls.
- `WHISPER_ENDPOINT_SILENCE_SAMPLES = 8_000` at 16 kHz, approximately 500 ms.
- `WHISPER_MAX_UTTERANCE_SAMPLES = 480_000` at 16 kHz, approximately 30 seconds.
- `WHISPER_SPEECH_RMS_THRESHOLD = 0.008` for local endpointing.

The partial interval is a refresh cadence only; it is not finality. An active utterance keeps a stable segment identity across partial updates until one of the terminal conditions is reached:

- endpoint silence,
- explicit stop/finalization,
- maximum utterance duration.

`WhisperEngineConfig` exposes the partial interval, endpoint silence, maximum utterance, and RMS threshold as runtime configuration. Production currently selects its documented defaults above; focused tests override cadence to verify that changing it does not change finality.

A final update resets the active utterance state only after the pipeline has applied the update and synchronously emitted its events. The worker then acknowledges delivery to the engine, which releases the utterance PCM and advances its identity.

## Conversation-layer finality contract

The provider/conversation boundary treats local-ASR updates as follows:

- Partial transcripts update the user partial display only.
- Partial transcripts must not commit provider user turns.
- One final transcript commits at most one provider user turn.
- Empty or whitespace-only final text closes local speech state without committing a user turn.
- Multiple partial updates for the same logical utterance collapse into the later final utterance.

No production path logs raw PCM or transcript payloads as part of the local-ASR regression tests or normal pipeline behavior.

## Stop, drain, and abort behavior

Normal stop is graceful:

1. Close/stop producer input.
2. Signal no-more-input to the worker.
3. Drain PCM chunks already accepted into the bounded queue.
4. Feed drained chunks through the active engine.
5. Finalize the utterance.
6. Forward any final transcript through the normal transcript path.
7. Join/retire the worker.
8. Wait for every emitted event's conversation/provider delivery acknowledgement or terminal error disposition.
9. Release the verified model lease with worker termination.

Drop remains an emergency safety-net abort path. Abort may discard accepted queued audio because it runs without an async caller that can await graceful drain. This is intentionally distinct from normal conversation stop.

Stop/finalization is idempotent: repeated stop calls must not duplicate the final transcript.

## Diagnostics and performance evidence

Runtime diagnostics expose the selected local-ASR architecture, input sample rate, queue depth/capacity, running state, last error, first partial/final latency, last transcription latency, processed audio duration, inference wall time, real-time factor, process CPU time, average CPU utilization, baseline RSS, current RSS, and peak RSS where the operating system supports those metrics.

Real-CPU performance claims below apply only to the exact historical checkout in the qualification evidence. Changes to installer, runtime, or stop delivery require a successful manual acceptance workflow at the exact new source SHA before they can be described as qualified.

## Acceptance workflow contract

The real-CPU workflow must remain manually invoked and separate from ordinary CI. It may download the real model only in that explicit acceptance context. Ordinary CI may compile and syntax-check the embedded evidence validator, but it must not download real Whisper model artifacts.

Acceptance evidence must record, at minimum:

- repository SHA under test,
- actual tracked `third_party/whisper.cpp` source revision,
- expected canonical Whisper source revision,
- model revision,
- model SHA-256,
- model byte count,
- test-audio identity/hash where practical,
- transcript output evidence,
- first partial latency,
- final latency,
- real-time factor,
- CPU utilization,
- true/high-water RSS,
- nominal and overload dropped-chunk behavior,
- partial interval, endpoint threshold, maximum utterance duration, and queue capacity.

Human summaries must be generated from verified machine-readable evidence rather than unverified manifest claims.

## Follow-up qualification boundary

The initial qualification remains a historical result for its recorded checkout. The follow-up fixes for stop-time event delivery, diagnostics, installed-model state classification, migration recovery, user cancellation, and native rebuild invalidation passed ordinary CI and a fresh exact-source real-CPU run; see the post-qualification evidence record.
