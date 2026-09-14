# AI Talking Moose — Local TTS Post-Review Hardening Implementation Evidence

**Date:** 2026-09-14
**TODO:** `docs/LOCAL_TTS_POST_REVIEW_HARDENING_TODO_2026-09-14.md`
**Spec:** `docs/LOCAL_TTS_POST_REVIEW_HARDENING_SPEC_2026-09-14.md`
**Implementation base:** `542b47b26f7cd86f0d669821e1a5120b58197328`
**Status:** Complete — implementation merged and exact-head/exact-master validation passed

This record captures the final implementation and validation state. The companion TODO contains the authoritative per-task checkmarks and final closeout evidence.

## Implemented review findings

- **LTR-100:** repaired the restart-policy regression test so Local standalone voice actually changes from the current default to another valid catalog voice (`Leo`) and the test explicitly proves the values differ.
- **LTR-200 / LTR-210:** reconciled the completed PR #111 post-closeout hardening spec/TODO with the later PR #114/#115 owner-approved ASR-proxy decision and `Luna` default while preserving historical truth.
- **LTR-220:** reconciled additional current-looking KittenTTS closeout/remediation/handoff documents and added historical/superseded context where appropriate.
- **LTR-300:** completed the previously unchecked PR #115 exact-head, guarded-merge, and merged-master evidence in the ASR-proxy default-voice closeout record.
- **LTR-400:** froze the settings policy: `Luna` is the new/missing-field default; existing valid persisted Local voice selections such as `Bella` remain preserved. Regression tests cover both behaviors; no schema bump or forced Bella-to-Luna migration is introduced.
- **LTR-500:** selected **Path A — strengthen**. Runtime waveform tensors are accepted only as mono `[N]` or single-batch mono `[1, N]`, with the dimension required to match the extracted sample count. Empty, multi-batch, higher-rank, and mismatched shapes fail closed as sanitized inference failures. Unit tests cover the predicate. Real KittenTTS CPU acceptance records `KITTENTTS_WAVEFORM_SHAPE` so the pinned production model supplies observed runtime-shape evidence.

## Preserved boundaries

- KittenTTS model/runtime/G2P pins are unchanged.
- Local production inference-thread policy remains 2 threads.
- Local-to-Google and Google-to-Local fallback behavior is unchanged.
- Google standalone, Local standalone, and Gemini Live voice ownership remains separate.
- No subjective human listening claim is introduced; `KCR-330` / `KTT-805` remains closed for V1 specifically by owner-approved ASR-proxy evidence.
- `duration` remains a presence-only ONNX contract sentinel; only `waveform` is semantically consumed.

## Validation required before merge

Because Rust tests and the Local TTS runtime engine changed, the final exact PR head must pass ordinary CI and real KittenTTS production CPU acceptance on Linux x86_64 and macOS arm64. ASR intelligibility smoke is also required because waveform output validation changed. The ASR voice-audition workflow may run due repository path policy; if it runs, its result is recorded, but the implementation does not change voice ranking policy.

Rust formatting was applied after the first exact-head CI attempt reported only `cargo fmt --check` differences. Clippy then identified one explicit auto-deref in waveform-shape extraction; the implementation now relies on Rust auto-deref for that borrow. The final validation head must therefore re-run the complete required gate set rather than relying on any pre-format or pre-Clippy head.


## Final qualification and merge evidence

- PR #118 exact tested head: `2e238385ff7f47bd0eff8982be0408773d7742f4`.
- PR-head CI `34872268664` — PASS.
- PR-head production CPU acceptance `34872268709` — PASS.
- PR-head ASR intelligibility smoke `34872268631` — PASS.
- PR-head ASR voice audition `34872268820` — PASS.
- Guarded squash merge master: `cfd32b06f106829a209a053891d155e822e58a10`.
- Post-merge CI `34873555675` — PASS.
- Post-merge production CPU acceptance `34873555672` — PASS.
- Post-merge ASR intelligibility smoke `34873555704` — PASS.
- Post-merge ASR voice audition `34873555711` — PASS.

## LTR-500 pinned-model waveform evidence

Dedicated evidence run `34876293975` on disposable branch head `7a776e88c585d700ffdcd6804df8e346726b5909` passed on Linux x86_64 and macOS arm64 while rooted at merged master `cfd32b06f106829a209a053891d155e822e58a10`. It observed the pinned ONNX model contract as `waveform: Float32 [-1]` with symbolic dimension `num_samples` and `duration: Int64 [-1]`. Runtime waveform tensors were one-dimensional (`[76200]` Linux, `[75600]` macOS), directly supporting the selected Path A validator.
