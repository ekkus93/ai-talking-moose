# Wake Word V1 Current Behavior

**Status:** implementation/reference documentation for the current `master` behavior.

Wake Word V1 is a local keyword-spotting feature for the fixed phrase **`Hey, Moose`**. This document records what the repository currently implements and what is still intentionally not claimed.

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

The one-stream production microphone routing and continuous wake→command-ASR handoff remain remediation work; these statements describe the required behavior and implemented runtime boundaries, not completed real-audio acceptance.

## Lifecycle policy

The authoritative runtime manager implements these state-machine rules, while full production audio/lifecycle integration acceptance remains pending:

- Listening begins only after the runtime has been enabled and loaded.
- One accepted wake event moves the runtime to the triggered/handoff state.
- Repeated positive frames while already triggered do not create duplicate trigger counts.
- Talking suspends Wake Word activation at the runtime boundary.
- Entry to Talking clears retained Wake Word audio.
- Resume paths clear stale pre-roll before returning to listening when enabled.
- Disabling Wake Word during a triggered or suspended interaction leaves the runtime disabled and prevents a later completion/resume path from unintentionally restoring listening.
- Wake Word V1 does **not** implement wake-word barge-in while Moose is talking.

These runtime-manager invariants must not be described as proof that the production microphone/conversation/TTS graph is fully integrated; WWR-300/310/400/410 and WWR-640 remain the acceptance authority for that claim.

## Artifacts, model, runtime, and licenses

The production KWS policy is frozen to 16 kHz mono, feature dimension 80, one inference thread, phrase `HEY MOOSE`, score 1.0, threshold 0.25, and two seconds of pre-roll.

Model/runtime identity and provenance are recorded in `wake-word-artifacts.json`, `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`, `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`, and `docs/licenses/SHERPA_ONNX_RUNTIME_NOTICE.md`.

Runtime and model licensing are tracked separately. The pinned sherpa-onnx runtime is Apache-2.0. The selected GigaSpeech KWS model provenance/license evidence is documented in the artifact manifest and WWR-100 evidence.

## Diagnostics and troubleshooting

Wake Word diagnostics expose privacy-safe state useful for lifecycle and artifact troubleshooting, including enabled state, authoritative runtime phase, exact model/runtime identity, platform/architecture, one-thread policy, canonical sample rate/channels, bounded ring/pre-roll counts, threshold/score, trigger count, last-trigger age, initialization duration, Talking suspension state, and sanitized last error.

Diagnostics intentionally do not expose raw PCM, transcripts, credentials, or private audio content.

## Corpus and acceptance status

`docs/wake-word-corpus.json` defines the deterministic Wake Word corpus manifest and versioned acceptance criteria schema. `.github/workflows/wake-word-corpus.yml` validates that manifest when the corpus, checker, fixture tree, or workflow changes.

Current limitations:

- The manifest currently contains no real redistributable audio fixtures.
- Positive recall and negative false-accept thresholds are intentionally `null` until real fixtures are added and calibrated.
- Linux x86_64 real KWS acceptance is not yet claimed complete.
- macOS arm64 real KWS acceptance is not yet claimed complete.
- Integrated one-stream microphone routing and wake→command-ASR handoff are not yet claimed complete.
- Integrated production lifecycle stability acceptance is not yet claimed complete.
- Performance measurements are not yet claimed.

Do not describe Wake Word V1 as fully user-ready or fully accepted until the real fixture, platform acceptance, production integration, lifecycle stability, performance, and final audit tasks are complete.
