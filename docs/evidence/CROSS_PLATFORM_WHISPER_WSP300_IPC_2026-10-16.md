# WSP-300 — Settings IPC and persisted migration regression evidence

**Checked:** 2026-10-16 UTC
**Repository:** `ekkus93/ai-talking-moose`, direct-to-`master`
**Authoritative checklist:** `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md`

## Source and exact-head CI

- `bde643a71498e22f9b5d06632d03eadc74dca906`: shared `validate_app_settings` rejects retired Moonshine modes from new settings IPC while retaining legacy-deserialization compatibility; removes obsolete rejection of Whisper on macOS.
- `305209f221e2d34fea4623fd5810e0cbac9f6cbc`: fixes Rust formatting after the initial quality gate.
- `58bd8117fa57b866ab2874f50462014d29aaf284`: adds command-level Whisper startup preflight test with local text provider.
- `4bf0e638c4f77da2f95b2998a068c22cb7e113a5`: adds conversation-level Whisper missing-model negative test.
- `ab30cca6438d88b9853ac2deccc7b5cda6d3ff27`: fixes formatting, tests SQLite startup normalization of both old ASR choices and rejects invalid ASR strings.

At exact source `ab30cca6438d88b9853ac2deccc7b5cda6d3ff27`, CI run [38897533479](https://github.com/ekkus93/ai-talking-moose/actions/runs/38897533479) completed **successfully**. The observed successful jobs are **Rust tests**, **Rust quality** (including formatting and Clippy), and **Generated backend contract**. Separate [Wake Word source security audit 38897533475](https://github.com/ekkus93/ai-talking-moose/actions/runs/38897533475) and [Wake Word lifecycle stability 38897533476](https://github.com/ekkus93/ai-talking-moose/actions/runs/38897533476) also passed at this exact SHA.

These tests are code/fixture evidence on Linux CI, **not** physical Mac app, real microphone, packaging, model performance, or strict offline egress-denial evidence. Superseded cancelled workflows do not count as successful tests.

## Behavioral scope proved

- Both legacy serialized values `moonshine_tiny_streaming` and `moonshine_small_streaming` still decode on historical settings load, and are rewritten to `whisper_small` in SQLite.
- Unknown serialized ASR strings fail closed. A fresh/direct IPC write with either retired Moonshine selection is rejected before settings persistence; new Whisper and explicitly selected Gemini Live are valid.
- Existing text/TTS provider selections, privacy/transcript/memory preferences, and unrelated settings are preserved in migration. Re-reading normalized SQLite settings is idempotent.
- Legacy migration never installs the Whisper model implicitly. Without verified model weights, Whisper startup fails before microphone capture and before connecting to a text provider; no model or cloud fallback was introduced.
- Frontend/backend catalog and original Moonshine compatibility fields are left intact until full WSP-300/WSP-400 release qualification; no premature Moonshine removal or changes to KittenTTS ONNX Runtime.

## Still open

- Live Mac settings save/restart, clean-machine model states, and model lease/installer full matrix.
- Whisper final transcript routing across every supported text provider and standalone TTS, physical mic/wake handoff, and Gemini Live negative isolation at app level.
- Enforced offline macOS inference, full native lifecycle/performance, packaged app, signed release and physical acceptance.

These gaps must remain open in the authoritative checklist; this evidence is not approval to remove the old runtime or mark WSP-300 complete.
