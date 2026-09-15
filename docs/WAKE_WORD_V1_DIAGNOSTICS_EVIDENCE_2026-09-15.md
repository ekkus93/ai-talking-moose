# Wake Word V1 diagnostics evidence

**Date:** 2026-09-15
**Scope:** WW-700 privacy-safe diagnostics and the diagnostics-facing portion of WW-710 performance instrumentation.

## Merged evidence

The Wake Word V1 diagnostics boundary is implemented in `src-tauri/src/asr/wake_word_diagnostics.rs` and is fed by the authoritative runtime snapshot in `src-tauri/src/asr/wake_word_runtime.rs`.

Merged PR evidence:

- PR #139, head `186e44ab123f3519576384454b5fd48573fd3cf7`, ordinary CI `35022948290`: introduced the privacy-safe diagnostics projection.
- PR #140, head `295ca770d5ec2e708e664785f986fe4edefe1ca6`, ordinary CI `35027297993`: added fixed engine identity plus compile-target platform and architecture.
- PR #142, head `d893c4387b1e27cbb183b11e7dd364a96e2a2757`, ordinary CI `35034233032`: added runtime load-to-listening initialization timing.

Current merged master evidence after PR #142:

- `master`: `afaf64dfac451e8ffa8015aa171dacecd9a9d535`
- Merge commit: `afaf64dfac451e8ffa8015aa171dacecd9a9d535`

## Fields provided

`WakeWordDiagnostics` intentionally exposes only bounded lifecycle, configuration, and performance fields:

- wake enabled/disabled diagnostic derived from runtime phase
- runtime phase
- fixed engine id: `sherpa-onnx-kws`
- compile target platform and architecture
- canonical 16 kHz mono audio contract
- one-thread inference policy
- fixed V1 threshold and score
- ring-buffer capacity in samples and milliseconds
- retained ring-buffer sample count
- retained handoff pre-roll sample count
- trigger count
- last-trigger age in milliseconds
- runtime initialization duration in milliseconds
- Talking-suspension indicator
- sanitized last-error text

## Privacy boundary

The diagnostics type cannot represent raw PCM, transcripts, credentials, or filesystem paths. Unit tests serialize diagnostics after injecting representative PCM sample values and assert that those sample values do not appear in the JSON. Error diagnostics expose only the sanitized Wake Word runtime message.

## Remaining WW-700 work

The source diagnostics boundary is in place, but WW-700 is not fully closed until final source/privacy/security audit confirms that all logs, metrics, and errors outside these types also avoid utterance, audio, credential, and unnecessary filesystem-path leakage.

## Remaining WW-710 work

The runtime initialization duration is recorded, but broader WW-710 performance acceptance still needs real KWS runtime memory overhead, idle-listening CPU utilization, inference real-time behavior, wake-to-ASR activation latency, ring-buffer replay/startup timing, repeated-cycle stability evidence, and comparison against the full ASR path.
