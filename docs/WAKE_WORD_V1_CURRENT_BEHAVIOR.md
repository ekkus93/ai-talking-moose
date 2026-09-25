# Wake Word V1 Current Behavior

**Status:** implementation/reference documentation for the current `master` behavior.

Wake Word V1 is a local keyword-spotting feature for the fixed phrase **`Hey, Moose`**. This document records what the repository currently implements, which acceptance evidence has passed, and what is still intentionally not claimed.

## Authoritative subsystem

- The application owns one authoritative Wake Word application runtime through the application composition layer.
- Startup initializes that runtime from persisted settings before normal runtime preference application.
- The Wake Word runtime manager owns lifecycle state, ring-buffer state, trigger debounce state, pre-roll retention, and privacy-safe diagnostics.
- Wake Word does not define a second command-ASR provider. After a wake trigger, the existing normal command interaction path remains the intended downstream path.
- The Wake Word runtime does **not** open an independent always-on full ASR stream, and it does not transcribe idle speech.

## Settings and user-visible behavior

- Wake Word defaults to disabled.
- The phrase is fixed to **`Hey, Moose`**.
- V1 does not expose arbitrary phrase editing.
- V1 does not expose a sensitivity control.
- The Settings UI can enable or disable Wake Word and persists the setting through the normal settings transaction.
- Live enable/disable changes are applied to the authoritative runtime without requiring an app restart through `apply_changed_runtime_preferences`.
- Wake runtime changes are reversible with the other runtime preferences if a later preference side effect or persistence step fails.
- When Wake Word is disabled, manual listen/start behavior remains available.

## Local/offline microphone behavior

When Wake Word is enabled and listening, microphone samples are intended to be consumed locally for keyword spotting. The idle Wake Word path is not full-time cloud transcription.

V1 disclosure requirements:

- The microphone may remain locally active while listening for the wake phrase.
- The wake phrase and immediate spoken command may enter the normal command ASR path after a trigger.
- Raw Wake Word PCM is retained only in bounded in-memory ring/pre-roll buffers.
- Wake Word diagnostics do not serialize or expose raw PCM.

The one-stream production microphone routing is the authoritative design and deterministic source/tests cover the shared-capture boundary. Documentation and source/privacy audit evidence are merged for the current evidence boundaries. Remaining final closeout still depends on WWR-630 measured performance acceptance and WWR-950/960 exact final qualification/merge verification.

## Lifecycle policy

The authoritative runtime manager implements these state-machine rules:

- Listening begins only after the runtime has been enabled and loaded.
- One accepted wake event moves the runtime to the triggered/handoff state.
- Repeated positive frames while already triggered do not create duplicate trigger counts.
- Talking suspends Wake Word activation at the runtime boundary.
- Entry to Talking clears retained Wake Word audio.
- Resume paths clear stale pre-roll before returning to listening when enabled.
- Disabling Wake Word during a triggered or suspended interaction leaves the runtime disabled and prevents a later completion/resume path from unintentionally restoring listening.
- Wake Word V1 does **not** implement wake-word barge-in while Moose is talking.

Integrated lifecycle stability evidence now exists for repeated wake→ASR→Thinking→Talking→wake cycles, bounded retained audio, repeated TTS terminal outcomes, repeated disable/enable cycles, shutdown while listening, and shutdown during handoff. Exact-master run `36090294124` on `755d02b738742513419778043b524e8b94f9c340` also records WWR-630 repeated-cycle resource behavior: 100 cycles, zero ring-buffer sample delta, zero handoff pre-roll sample delta, final phase `Listening`, and capacity bounded at 32,000 samples. That evidence is still not a substitute for final WWR-950/960 closeout.

## Artifacts, model, runtime, and licenses

The production KWS policy is frozen to 16 kHz mono, feature dimension 80, one inference thread, phrase `HEY MOOSE`, score 1.0, threshold 0.25, and two seconds of pre-roll.

Model/runtime identity and provenance are recorded in `wake-word-artifacts.json`, `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`, `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`, and `docs/licenses/SHERPA_ONNX_RUNTIME_NOTICE.md`.

Runtime and model licensing are tracked separately. The pinned sherpa-onnx runtime is Apache-2.0. The selected GigaSpeech KWS model provenance/license evidence is documented in the artifact manifest and WWR-100 evidence.

## Diagnostics and troubleshooting

Wake Word diagnostics expose privacy-safe state useful for lifecycle and artifact troubleshooting, including enabled state, authoritative runtime phase, exact model/runtime identity, platform/architecture, one-thread policy, canonical sample rate/channels, bounded ring/pre-roll counts, threshold/score, trigger count, last-trigger age, initialization duration, Talking suspension state, and sanitized last error.

Optional measured CPU, memory, inference, and handoff timing fields remain empty until accepted measurements exist. Partial WWR-630 evidence exists for idle KWS CPU, memory, inference timing, wake→command-ASR activation timing, pre-roll startup timing, repeated-cycle resource behavior, and a continuous-ASR idle comparison diagnostic. Accepted performance closeout remains pending until the required measurements are reconciled into accepted platform baselines.

Diagnostics intentionally do not expose raw PCM, transcripts, credentials, or private audio content. The WWR-510 privacy audits cover Wake Word diagnostics, production error strings, and production logging surfaces for credentials, unnecessary filesystem paths, and audio content.

## Corpus and acceptance status

`docs/wake-word-corpus.json` defines the deterministic Wake Word corpus manifest and versioned acceptance criteria schema. The current deterministic generated corpus covers multiple positive variants, command-following positives, background/noise variants, ordinary-speech negatives, near-misses, and media/background-style negatives generated from repository-authored text.

Linux x86_64 and macOS arm64 real native KWS acceptance have passed on exact merged master evidence recorded in `docs/evidence/WWR-610_620_REAL_KWS_ACCEPTANCE_2026-09-24.md`. Deterministic corpus acceptance is recorded in `docs/evidence/WWR-600_DETERMINISTIC_CORPUS_ACCEPTANCE_2026-09-24.md`.

Current limitations:

- WWR-630 measured performance acceptance is still pending.
- WWR-910 original TODO reconciliation has a merged evidence matrix, but final reconciliation cannot close until WWR-630 and WWR-950/960 are complete.
- WWR-950/960 exact-head and exact-master final closeout are still pending.

Do not describe Wake Word V1 as fully user-ready or fully accepted until the measured performance, original TODO final reconciliation, and exact final closeout tasks are complete.
