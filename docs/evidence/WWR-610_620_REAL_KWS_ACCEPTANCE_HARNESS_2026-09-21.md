# WWR-610/620 — Real KWS acceptance harness evidence

**Date:** 2026-09-21

`src-tauri/tests/wake_word_real_kws_acceptance.rs` is an ignored, fail-closed integration harness for the exact production `NativeKwsSession`. It requires explicitly prepared pinned model/runtime directories plus the repository's redistributable corpus manifest, validates PCM16 mono 16 kHz WAV input, runs every fixture through the real native KWS session, resets between fixtures, records per-fixture detection outcomes, and enforces calibrated positive-recall and negative-false-accept criteria.

The harness is ignored in ordinary CI because the current corpus is intentionally empty and its criteria remain `pending_real_fixture_calibration`. It therefore does not create a false platform-support claim. WWR-610/620 remain open until real redistributable fixtures are added, criteria are calibrated, and this harness passes on Linux x86_64 and macOS arm64 with exact run evidence.
