# Wake Word V1

Wake Word V1 is the local keyword-spotting subsystem for the fixed phrase **Hey, Moose**. It is disabled by default. When enabled and fully initialized, the application keeps the selected microphone active locally so it can detect that phrase without continuously transcribing microphone audio or sending idle microphone audio to a cloud service.

## Current implementation and qualification status

The authoritative implementation is the consolidated Wake Word subsystem exposed through `src-tauri/src/app/wake_word/` and its application integration modules. `WakeWordRuntimeManager` is the single runtime state machine. `WakeWordApplicationRuntime` is the application-level runtime owner stored in `AppState`. `AppState::audio_capture` remains the authoritative physical microphone owner; Wake Word does not own an independent competing microphone capture object.

The repository contains the real pinned sherpa-onnx native KWS session, immutable model/runtime manifests, canonical PCM routing, pre-roll/live handoff primitives, command-ASR ingress, lifecycle guards, settings UI, diagnostics, and deterministic component/lifecycle tests. **Wake Word V1 is not yet fully production-qualified.** Production lifecycle startup wiring, deterministic corpus qualification, real Linux x86_64 and macOS arm64 native KWS acceptance, integrated performance evidence, and final end-to-end qualification remain tracked in `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`. Do not interpret the presence of platform artifacts as a completed support claim until the corresponding real acceptance section is complete.

## Fixed V1 policy

V1 recognizes only `Hey, Moose`; arbitrary wake phrases and sensitivity controls are intentionally not exposed. KWS input is canonical 16 kHz mono PCM. The pinned policy uses one inference thread, score `1.0`, threshold `0.25`, and a two-second in-memory pre-roll window. V1 has no acoustic wake-phrase trimming and no barge-in while Moose is speaking.

Wake-triggered command ASR is explicitly limited to local Moonshine streaming modes in Wake Word V1: `MoonshineTinyStreaming` and `MoonshineSmallStreaming`. Wake cannot be enabled, started, or kept active when an unsupported command ASR mode such as Gemini Live audio is selected. A user can still use unsupported command ASR modes through ordinary manual interaction; the restriction applies only to Wake-triggered command activation.

The exact model and native-runtime identities are defined by `wake-word-artifacts.json`. Preparation and loading are fail-closed: required files are checked against frozen byte sizes and SHA-256 identities, and native libraries are architecture-checked before use. Runtime and model licensing/provenance are recorded separately in `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`, `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`, and the repository license notices.

## Audio ownership and privacy

Wake Word uses the same authoritative `AppState::audio_capture` object used by normal microphone interaction. Canonical Wake PCM is routed chronologically to both the bounded ring buffer and KWS. On a trigger, the handoff path preserves the chronological pre-roll plus subsequent live PCM so local Moonshine command ASR can receive the wake phrase and immediate command without intentionally trimming `Hey, Moose`.

Wake pre-roll and handoff PCM are memory-only application state. Wake diagnostics do not serialize raw PCM. Idle KWS is designed as local/offline inference; it must not silently fall back to cloud transcription or continuously running full ASR merely to detect the wake phrase. Once a wake-triggered command is handed to local Moonshine command ASR, that command path processes the wake phrase and following prompt locally through the supported Wake V1 command-ASR boundary.

## Lifecycle

The runtime states are Disabled, Loading, Listening, Triggered, SuspendedTalking, Error, and ShuttingDown. Persisted disabled state remains Disabled. Persisted enabled state begins in Loading and must not claim Listening until the verified native KWS session is actually ready.

V1's lifecycle policy is to suspend wake activation for an active command interaction and throughout Moose speech/TTS. Entering the talking suspension clears retained pre-roll. Terminal success, cancellation, and recoverable failure resolve through the same policy boundary and honor the latest enabled setting; disabling Wake Word during an interaction must result in Disabled rather than an unintended return to Listening. KWS state is reset before a new listening epoch where required. Wake errors must fail Wake closed without disabling ordinary manual interaction.

These lifecycle rules describe the required and implemented state-machine policy, but full production end-to-end lifecycle qualification remains open until WWR-400 and WWR-640 are completed.

## Settings and user-visible behavior

Settings exposes an **Enable wake word** toggle and the fixed phrase `Hey, Moose`. The UI explains that keyword spotting is local/offline, that the microphone remains locally active while Wake Word is listening, and that Wake-triggered commands require local Moonshine command ASR. The runtime status is reported as loading, listening, suspended, disabled, or error using sanitized user-facing error text. Disabling Wake Word preserves normal manual-listen behavior.

## Diagnostics and troubleshooting

Wake diagnostics expose privacy-safe state needed to diagnose initialization and lifecycle problems: enabled/runtime state, model/runtime identity, platform/architecture, canonical audio policy, ring capacity, threshold/score, trigger count and approved trigger timing, initialization timing, Talking suspension, and sanitized last error. Raw audio, credentials, and unnecessary absolute filesystem paths are not diagnostic payloads.

If Wake Word cannot initialize, first verify the pinned model/runtime artifacts, selected local Moonshine command ASR mode, and platform architecture rather than bypassing identity checks. Artifact verification failures and unsupported ASR-mode failures are intentional fail-closed behavior. A capture/device failure moves Wake to a recoverable error state; recovery must reuse the authoritative application capture owner rather than opening a second microphone stream.

## Supported-platform claims

The manifest currently contains pinned native runtime identities for Linux x86_64 and macOS arm64. Those entries define the only architectures eligible for V1 qualification; they do **not** by themselves constitute final support claims. Real native KWS acceptance is tracked separately as WWR-610 (Linux x86_64) and WWR-620 (macOS arm64). Documentation must not claim either platform as production-qualified until its acceptance evidence passes on the exact qualified feature head.

## Developer references

The remediation specification is `docs/WAKE_WORD_V1_REMEDIATION_SPEC_2026-09-17.md` and the live qualification queue is `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`. Architecture consolidation evidence is in `docs/evidence/WWR-020_WAKE_WORD_ARCHITECTURE_CONSOLIDATION_2026-09-17.md`; capture ownership/routing evidence is in the WWR-300 evidence files; handoff evidence is `docs/evidence/WWR-310_HANDOFF_BOUNDARY_2026-09-22.md`; diagnostics/privacy evidence is linked from WWR-510 in the remediation TODO.
