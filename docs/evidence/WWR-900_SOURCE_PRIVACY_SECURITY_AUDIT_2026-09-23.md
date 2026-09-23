# WWR-900 — Source/privacy/security audit evidence

Audited commit: `fb0316a8380b63e39f0dbe7fc82b5f4399a86a87`

This evidence records the source/privacy/security audit state for Wake Word V1. It is not a final WWR-900 closeout claim: production wake-trigger orchestration, real native KWS acceptance, integrated lifecycle acceptance, and measured performance acceptance remain open in `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`.

## Source ownership and microphone routing

Audited source paths:

- `src-tauri/src/app/state.rs`
- `src-tauri/src/app/wake_word_composition.rs`
- `src-tauri/src/app/wake_word_authoritative_capture.rs`
- `src-tauri/src/app/wake_word_pcm_router.rs`
- `src-tauri/src/app/wake_word_command_handoff.rs`
- `src-tauri/src/app/wake_word_command_asr_ingress.rs`
- `src-tauri/src/app/wake_word_command_activation.rs`
- `src-tauri/src/conversation/session.rs`
- `src-tauri/src/conversation/session/local_asr.rs`
- `src-tauri/src/asr/pipeline.rs`

Findings:

- `AppState` remains the authoritative application composition point for the Wake runtime and shared `AudioCapture` owner.
- The Wake capture owner is represented by `AuthoritativeWakeCaptureOwner`; it is intended to borrow the shared application capture rather than constructing a second long-lived microphone owner.
- `CanonicalWakePcmRouter` is the canonical 16 kHz mono PCM boundary for Wake pre-roll, KWS feed, and handoff retention.
- `WakeCommandHandoffAudio` owns memory-only chronological PCM for the wake phrase plus immediate post-trigger command audio and performs no V1 acoustic trimming.
- `WakeCommandAsrHandoff` and the activation boundary keep handoff delivery single-use at the component seam.
- `LocalAsrPipeline::prime_wake_handoff` is the existing bounded Moonshine command-ASR ingress for priming Wake handoff audio before command capture appends later microphone PCM.

Open source-ownership risk:

- The real accepted-trigger production path still needs to pass the single-use Wake handoff into the normal `ConversationManager::start_session` path before command microphone capture starts. Until that wiring is merged and qualified, WWR-300/310/400 remain open.

## Lifecycle and disabled-state audit

Findings:

- The current command lifecycle helpers define explicit suspend/resume behavior for Wake activation during command interaction.
- The component activation boundary has tests for one-shot activation, normal-command-start failure recovery, and disabled-during-failure resolution.
- `start_conversation` suspends Wake before ordinary manual command start and resumes on failure/terminal lifecycle through the shared runtime helper.

Open lifecycle risk:

- Integrated production tests still need to prove an accepted real Wake trigger starts exactly one normal command interaction, suppresses repeated positive frames during transition, remains suspended through Thinking/Talking/TTS, resumes after success/cancel/recoverable failure, and resolves to `Disabled` when disabled during the interaction.

## Provider separation and cloud fallback audit

Findings:

- Idle Wake Word KWS is represented separately from command ASR selection.
- Wake diagnostics and qualification evidence describe local/offline KWS as separate from normal command ASR provider behavior.
- The local Moonshine command-ASR path uses `LocalAsrPipeline` and final transcript events to send text turns to the active provider; partial transcripts and local speech lifecycle events remain local.

Open provider-separation risk:

- Real/integrated acceptance still needs to prove that idle Wake KWS does not silently start continuous full ASR or cloud ASR merely for wake detection.

## Artifact/runtime and native architecture audit

Findings:

- `wake-word-artifacts.json` remains the pinned model/runtime identity source.
- `scripts/validate_wake_word_artifact_manifest.py` and `scripts/prepare_wake_word_runtime.py` are covered by the native packaging policy workflow.
- `tests/test_wake_word_runtime_artifacts.py` exercises architecture rejection, corrupt archive/library rejection, path traversal rejection, and cache re-verification.
- The WWR-800 evidence matrix records that native packaging/architecture policy is implemented, while Linux/macOS real KWS inference acceptance remains separate and pending.

Open native-acceptance risk:

- Linux x86_64 and macOS arm64 platform support claims cannot close until real positive/negative sherpa KWS inference acceptance is captured on those platforms.

## Privacy/logging audit

Findings:

- Wake handoff PCM remains an owned memory payload, not a serialized diagnostic artifact.
- The architecture and gate evidence explicitly prohibit treating docs, component tests, or skipped workflows as real acceptance.
- The qualification report validator requires privacy fields for no raw audio, no secrets, and no unnecessary paths in reports.

Open privacy/security risk:

- Final WWR-900 closeout still requires an exact final-head audit after production activation wiring and real acceptance reports land. This file is an intermediate audit artifact and must not be used as final closeout evidence by itself.

## Evidence status

Current supporting exact-master runs already recorded elsewhere:

- PR #412 merged at `b6c33cd999191bfedcc6358f4f80165b5c9e937c` with exact-master ordinary CI `35888332561`, Wake qualification report `35888332515`, and Wake required-gates audit `35888332562`.
- PR #413 merged at `fb0316a8380b63e39f0dbe7fc82b5f4399a86a87` with exact-master ordinary CI `35890855819`.

Remaining mandatory evidence before WWR-900 can close:

- Production accepted-trigger orchestration wiring merged and exact-head qualified.
- Real Linux x86_64 KWS acceptance.
- Real macOS arm64 KWS acceptance.
- Integrated repeated lifecycle acceptance.
- Measured performance acceptance.
- Final source/privacy/security audit on the exact final feature head.
