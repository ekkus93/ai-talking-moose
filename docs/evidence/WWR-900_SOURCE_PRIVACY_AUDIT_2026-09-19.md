# WWR-900 Source / Privacy / Security Audit

**Audited master:** `e4eebf76d0f7df4288ab49fb15e6e4f21f00d07a`

This is a source-level audit of the Wake Word V1 implementation currently on `master`. It records what source and deterministic tests prove and leaves real platform/inference acceptance open where source inspection is insufficient.

## Ownership and lifecycle

- `src-tauri/src/app/wake_word.rs` is the canonical facade and re-exports one runtime manager implementation from `asr/wake_word_runtime.rs`.
- Legacy duplicate runtime/engine modules are not compiled; architecture regression tests guard that boundary.
- `WakeWordApplicationRuntime` is the application composition owner. Its clone shares the same manager state rather than constructing a second manager.
- The composition boundary owns no microphone stream. Its source explicitly leaves capture ownership with the application `AudioCapture`; Wake Word error recovery does not open/reopen a capture stream.
- Runtime shutdown is idempotent, clears retained audio, enters `ShuttingDown`, and rejects later enable attempts.
- `disable()` clears retained ring/handoff audio and returns to `Disabled`.
- Talking suspension clears retained audio, rejects trigger activation while suspended, and resume clears stale audio before returning to listening.
- The application resume boundary takes the latest enabled setting, so disabling during an interaction wins over an eventual completion callback.

## One-trigger / one-command state invariant

`WakeWordRuntimeManager::accept_trigger` accepts a trigger only while `Listening`. The first accepted trigger changes the phase to `Triggered`, increments the bounded trigger counter, and captures one chronological pre-roll snapshot. Further trigger attempts while `Triggered` or `SuspendedTalking` return `false` and do not increment the count. The pre-roll transfer is destructive: a second `take_triggered_pre_roll` returns `None`.

Deterministic tests prove repeated positive frames create one trigger until interaction reset, and a later phrase after resume can create the next trigger.

This state-machine proof does not replace the pending production audio/lifecycle acceptance in WWR-640.

## Retained audio and diagnostics

Raw Wake PCM is represented only by bounded in-memory `Vec<i16>`/ring-buffer state in the runtime path inspected here. Disable, resume, Talking suspension, runtime error, and shutdown clear retained ring/handoff audio.

`WakeWordDiagnostics` is deliberately structurally incapable of serializing raw PCM, transcripts, credentials, or filesystem paths. It exposes only bounded state/counters, immutable identities, fixed policy, platform/architecture, initialization timing, trigger age/count, retained-sample counts, Talking suspension, and a sanitized error string.

Tests serialize diagnostics and reject privacy-sensitive field/content names including PCM/audio, transcript, credential, and path representations. Runtime errors are reduced to the fixed message `The Wake Word runtime encountered an internal error.`

## Artifact/runtime identity

Diagnostics source binds model identity, model archive SHA-256, model license, keyword SHA-256, runtime ID/license, and platform-specific C API SHA-256 to the frozen manifest constants. Tests assert those identities occur in `wake-word-artifacts.json`.

Earlier WWR-100/110/120 evidence records immutable model/runtime identities, architecture checks, safe extraction, cached-artifact re-verification, and fail-closed unsupported-platform behavior.

## Provider separation / network claims

The authoritative Wake runtime is keyword spotting, not a command-ASR provider, and the current-behavior documentation explicitly states that it does not run an independent always-on full-ASR stream or transcribe idle speech.

The native KWS implementation is designed for local inference and contains no intended network fallback. However, WWR-610/620 real acceptance has not yet demonstrated the final prepared production session with network denied on Linux x86_64 and macOS arm64. Therefore this audit does **not** close the real offline-inference acceptance requirement.

## Documentation claim audit

`docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md` explicitly states that:

- the real redistributable fixture corpus is not yet populated;
- Linux x86_64 real KWS acceptance is not yet complete;
- macOS arm64 real KWS acceptance is not yet complete;
- performance evidence is not yet complete;
- Wake Word V1 must not be described as fully user-ready/accepted until real fixture, platform, lifecycle, performance, and final-audit work completes.

`docs/WAKE_WORD_V1_CI_GATES.md` also states that ordinary CI alone cannot qualify final Wake Word V1 and that a skipped workflow is not acceptance evidence.

## Findings still open

This audit intentionally leaves these mandatory findings open:

1. Real positive/negative redistributable corpus and calibrated recall/false-accept criteria.
2. Linux x86_64 real KWS acceptance with offline-after-preparation proof.
3. macOS arm64 real KWS acceptance with offline-after-preparation proof.
4. Final packaged runtime/architecture acceptance.
5. Integrated repeated lifecycle stability/soak acceptance.
6. Performance evidence.
7. Final whole-feature source/privacy/security re-audit on the exact qualification head after the above work.

Accordingly, WWR-900 acceptance (`No mandatory Wake Word V1 defect remains unresolved`) is **not** claimed complete by this document.
