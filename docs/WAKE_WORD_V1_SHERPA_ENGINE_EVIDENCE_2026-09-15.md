# Wake Word V1 — Sherpa Engine Boundary Evidence

**Date:** 2026-09-15
**Task area:** WW-300
**Repository:** `ekkus93/ai-talking-moose`

## Qualified merged slices

### PR #146 — Initial sherpa KWS engine boundary

- Exact PR head: `e352ce45b1011cccd36b92833e38119e6bda26ae`
- Ordinary CI run: `35046005074`
- Merge commit: `2ebca3ace891a58bd874faf55f8a0be7f885c90c`

Evidence established:

- V1 engine identity is fixed to `sherpa-onnx-kws`.
- Canonical keyword representation is fixed to `HEY MOOSE`.
- Canonical input contract is 16 kHz mono PCM.
- KWS inference policy is one thread.
- V1 threshold/score are sourced from the Wake Word settings policy.
- Public errors are sanitized and do not expose artifact paths or PCM content.
- Engine shutdown is idempotent at the Rust boundary.

### PR #147 — Native streaming-session boundary

- Initial exact PR head: `d5c5eaf45063c6caa00c76e61e0437f922c4f492`
- Initial ordinary CI run: `35049147724`
- Initial result: formatting-only failure in `src-tauri/src/asr/wake_word_sherpa.rs`
- Corrected exact PR head: `bf7577d6b4e488922ddd21366893e8e3c78350cc`
- Ordinary CI run: `35050180504`
- Merge commit: `afbaf7c2af9a9c027a627dea25e4394a8ff47f17`

Evidence established:

- `NativeKwsSession` is private to the sherpa engine boundary and accepts only canonical `f32` PCM frames.
- `SherpaKwsEngine::feed_canonical_pcm` returns bounded `NoTrigger` / `WakeDetected` outcomes without raw audio payloads.
- A native-session trigger resets the KWS stream immediately after the accepted detection.
- Non-triggering frames do not reset the stream.
- Missing native-session attachment fails with a sanitized artifact error.
- Native-session shutdown is performed once even if engine shutdown is requested repeatedly.

## WW-300 items covered by these slices

Covered at the Rust boundary:

- Narrow sherpa-specific KWS engine abstraction.
- Canonical `HEY MOOSE` keyword configuration contract.
- One-thread inference policy.
- Explicit threshold/score configuration plumbing.
- Canonical streaming PCM feed path.
- Bounded wake-detected event representation without raw audio.
- KWS stream reset after recognition.
- Sanitized artifact/runtime/shutdown errors.
- Idempotent shutdown boundary.
- No full-transcription API exposed by the wake engine boundary.
- No networking API exposed by the wake engine boundary.

## Still open for WW-300 closeout

These items are intentionally not claimed complete by the current evidence:

- Production attachment to real sherpa-onnx native runtime objects.
- Exact pinned model/runtime/tokenizer artifact identities in `wake-word-artifacts.json`.
- Real positive fixture acceptance for `Hey, Moose`.
- Real negative fixture acceptance.
- Network-is-unnecessary proof after production artifacts are prepared.
- Platform acceptance on Linux x86_64 and macOS arm64.

## Privacy notes

The evidence above is source and CI evidence only. It records no PCM contents, no transcripts, no credentials, and no local filesystem paths beyond repository-relative source paths already visible in the project.
