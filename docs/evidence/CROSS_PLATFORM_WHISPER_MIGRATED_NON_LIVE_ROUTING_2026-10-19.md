# WSP-300/400 — persisted Moonshine selection to non-Live Whisper Talk routing

**Observed 2026-10-19 UTC.** Tested source/CI SHA: `4cf36c142a7489d2aca0dd8fbca7e39ac26312c4`.
**Scope:** automated routing and settings tests only. No installed Whisper model, physical microphone, live text provider, external TTS or audible playback is exercised.

## What the regression exercises

Source commit [6255478717b0caed1ca0819f25164bdeb41c3882](https://github.com/ekkus93/ai-talking-moose/commit/6255478717b0caed1ca0819f25164bdeb41c3882) adds `migrated_moonshine_profiles_keep_selected_text_and_tts_non_live` under `src-tauri/src/app/state/tests/talk_routing.rs`.
It serializes **both** legacy Moonshine selections, exercises the persisted settings migration to `whisper_small`, verifies no Moonshine selection survives the normalized serialized output, repeats deserialization without further migration, preserves each selected text/TTS provider and ensures the non-Live adapter rejects microphone PCM without contacting Gemini Live.
Coverage is both legacy selectors × Local/Google/OpenAI/Ollama/llama-server text × Local/Google standalone TTS, i.e. 20 deterministic combinations. It does not require model weights or provider keys.

Workflow commit [4cf36c142a7489d2aca0dd8fbca7e39ac26312c4](https://github.com/ekkus93/ai-talking-moose/commit/4cf36c142a7489d2aca0dd8fbca7e39ac26312c4) includes this test in `.github/workflows/whisper-macos-native-compile.yml` on both macOS ARM64 and Intel native matrices.

## Exact-source observed qualification

- [Native macOS workflow 39210247467](https://github.com/ekkus93/ai-talking-moose/actions/runs/39210247467) was **still in progress when recorded**. ARM64 job `117921894881` completed **success**, including `Qualify macOS Whisper settings and non-Live provider isolation`, fixture-model lifecycle/installer tests, pinned Whisper native linking and release-profile probes.
- Intel x86_64 job `117921895256` had **not yet completed**. The new migration/routing test was still pending at the time of observation; do not interpret this report as an Intel pass.
- [Main CI 39210247491](https://github.com/ekkus93/ai-talking-moose/actions/runs/39210247491) completed **success**, but its change classifier selected **CI-plumbing only** because the tested HEAD was a workflow change. Rust tests and Rust quality were **skipped** there, not falsely counted as a separate acceptance pass.
- [Wake Word audit 39210247557](https://github.com/ekkus93/ai-talking-moose/actions/runs/39210247557) passed.

## Safety gates and remaining acceptance

These tests prove **migration plus routing boundary behavior** with deterministic fixtures. They do **not** prove physical microphone ownership, audio forwarding, actual model loading, genuine Intel Whisper inference, macOS enforced offline egress, model lifetime under live PCM, standalone TTS playback, nor complete WSP-300 or WSP-400 acceptance. Retain Moonshine until the full WSP-200/300/400 cutover gates are satisfied and preserve independent KittenTTS ONNX Runtime. No release or physical WSP-700 gate is closed.

Future updates must re-read the actual final Intel conclusion and exact source SHA rather than assuming it from the ARM64 result.
