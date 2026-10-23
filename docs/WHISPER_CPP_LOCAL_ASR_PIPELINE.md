# Whisper.cpp Local ASR Pipeline

**Latest reconciliation (2026-10-23):** Moonshine runtime removal is implemented under the owner's override. Both Mac architectures passed unsigned Whisper app smoke at `c3d32fc2`; KittenTTS-to-Whisper real-model roundtrip also passed on arm64. Seatbelt-denied real Whisper inference passed earlier on both Macs at `6009d63c`. These distinct source-bound results, Intel performance limits and remaining physical/release gates are recorded in [closeout evidence](evidence/CROSS_PLATFORM_WHISPER_CLOSEOUT_2026-10-23.md). The older source checkpoints below remain historical evidence.

**Status:** Historical Linux x86_64 real-CPU acceptance and separate macOS arm64/x86_64 model-backed CPU inference are recorded at their exact source SHAs. At `8b4c505c28b22b5c37885d513e4eaa1115569b43`, [main CI 39378458759](https://github.com/ekkus93/ai-talking-moose/actions/runs/39378458759), [Mac native compile 39378458637](https://github.com/ekkus93/ai-talking-moose/actions/runs/39378458637), and [unsigned Mac app 39378458746](https://github.com/ekkus93/ai-talking-moose/actions/runs/39378458746) passed. These are **not** current exact-head real-weight transcription, verified Seatbelt-denied inference, sustained Intel performance, physical mic/TTS, or signed release acceptance; see `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md` and `docs/evidence/CROSS_PLATFORM_WHISPER_WSP200_REPEAT_REAL_CPU_2026-10-18.md`.
**Production source:** `29759a18d7c91c26113d33e16a55c6d7f80bdb0d`; exact acceptance checkout `cf3280ea1d4e91d298828e4935e3369f8cbd6be6` is a documentation-only descendant.
**Initial qualification:** run `37744371559`, job `113202252940`; see `docs/evidence/WHISPER_CPP_LOCAL_ASR_QUALIFICATION_2026-10-08.md`.
**Post-qualification review:** completed in `docs/WHISPER_CPP_LOCAL_ASR_POST_QUALIFICATION_REVIEW_TODO_2026-10-08.md`; evidence is in `docs/evidence/WHISPER_CPP_LOCAL_ASR_POST_QUALIFICATION_2026-10-08.md` (run `37756211710`, job `113241297405`).

This document describes the production Whisper pipeline on Linux and macOS. New Linux/macOS profiles select Whisper Small, but select/install is not permission for a download or cloud fallback. Historical model-backed qualification applies only to its recorded source checkout and does not qualify later changes.
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

`prepare_local_asr()` starts the Whisper installer/runtime only for local ASR; explicit Gemini Live audio bypasses local inference.

The production startup path is fail-closed before microphone capture. A selected local ASR mode must create and verify its pipeline before the existing authoritative microphone capture object is started.

## Pipeline worker model

Whisper uses the shared `LocalAsrPipeline` bounded-worker path:

- One authoritative capture source feeds a bounded local-ASR ingress queue.
- Whisper ingress has capacity 56 chunks (about 5.6 seconds) because each partial runs synchronous whole-utterance inference; the queue absorbs that bounded CPU pause.
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

The real-CPU workflows remain manually invoked and separate from ordinary CI. They may download the real model only in that explicit acceptance context. The separate macOS arm64/Intel workflow must demonstrate both kernel Seatbelt network-denial evidence and the fail-closed socket denial **during genuine weight-backed transcription**; policy/unit checks alone do not qualify macOS offline acceptance. The configured Ralph Bridge profile `whisper_macos_real_cpu` has repeatedly returned generic `INVALID_ARGUMENT` with no manual run receipt (see `docs/evidence/RALPH_BRIDGE_INCIDENTS_2026-10-19.md`); treat the enforced offline gate as OPEN. Ordinary CI may compile and syntax-check the embedded validator but must not download real Whisper model artifacts.

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
