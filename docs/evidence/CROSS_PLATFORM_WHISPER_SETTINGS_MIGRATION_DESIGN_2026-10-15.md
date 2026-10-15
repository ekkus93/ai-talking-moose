# Whisper-Only ASR Settings Migration — Implementation Design

**Date:** 2026-10-15  
**State:** Design accepted for implementation after WSP-200; **no production migration is active yet**.  
**Source inspected:** `src-tauri/src/app/state.rs` on `master` `a46005cd3166e4f772900907e57251030d0e7226`.  
**Tracks:** WSP-300, `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md`.

## Existing persisted-settings contract

`AppSettings` uses Serde `#[serde(default)]` and `#[serde(flatten)] future_settings`. `AppSettings::from_persisted_json` first reads a `serde_json::Value`, rejects a `settings_version` above `CURRENT_SETTINGS_VERSION` (currently 8), then deserializes `AppSettings` before normalizing select legacy fields. It currently:

- Treats **no `asr_mode` key** in a legacy profile as an intentional old Gemini Live audio selection, because that was historically the only ASR mode; never reinterpret absence in a *persisted* record as a new Whisper choice.
- On Linux only, rewrites persisted Moonshine Tiny/Small modes to the platform default Whisper mode; Mac currently leaves those choices unchanged.
- Preserves a saved explicit `whisper_small` or `gemini_live_audio` selector and all independent text, TTS, devices, memory, transcript, wake-word and unknown additive settings (except credential-shaped future keys intentionally scrubbed).
- Writes normalized `AppSettings` JSON back into the `app_settings` SQLite row when the migration flag is true. Fresh profiles use the platform-specific `AsrMode::default()` without automatically installing the selected model.

## Cutover requirements

1. **Only after WSP-200 evidence is accepted**, change fresh-profile `AsrMode::default()` to `WhisperSmall` on supported Linux/macOS while keeping `GeminiLiveAudio` explicit.
2. Before `serde_json::from_value::<AppSettings>`, examine the original persisted `asr_mode` key in the raw JSON `Value`:
   - `moonshine_tiny_streaming` or `moonshine_small_streaming`: replace **that field only** with `whisper_small`; set `asr_migrated = true`.
   - `whisper_small`: preserve; no ASR-specific rewrite.
   - `gemini_live_audio`: preserve; never transfer audio between local and cloud providers.
   - Missing key: preserve the pre-existing legacy no-selector -> Gemini Live behavior; mark migrated if appropriate.
   - Unknown string, wrong type, invalid JSON, or future schema version: reject explicitly through the existing typed error boundary; do not silently default.
3. Remove the Linux-only `unsupported_asr_mode` behavior once old enum variants are no longer used. Do **not** remove the migration's pre-deserialization raw-value handling: otherwise old profiles will fail to deserialize after deleting Moonshine enum variants.
4. Increment `CURRENT_SETTINGS_VERSION` from 8 to **9** at the actual semantic cutover, updating schema fixtures and generated contracts as appropriate. Preserve rejection of values newer than 9 and normal migration of all older versions; do not accept unknown future profiles.
5. Return `migrated = true` when the raw Moonshine selector was rewritten, even if the stored document already carries version 9 (defensive compatibility). A second read of the persisted normalized JSON must return the same semantic object with `migrated = false` when no other migration remains.
6. Never inherit a Moonshine model path as Whisper weights, download a 488 MB model during migration, delete user model data automatically, call Google, open Gemini Live, or change text/TTS providers. After migration, Whisper may be **selected but uninstalled**: Settings must display explicit Download & Verify, and Talk must fail locally and early until verification succeeds.
7. Preserve unknown additive settings and the recursive secret/credential scrubber exactly. Keep `wake_word_enabled`, `wake_word_phrase`, saved devices, audio, provider configuration, credentials (stored elsewhere), consent/privacy, rate and voice fields untouched.
8. After backend cutover update frontend `AsrMode` union, generated contract, test fixtures, migration snapshots, diagnostics, settings labels, and Live/non-Live routing assertions; do not leave obsolete Moonshine values in active selector options.

## Required regression cases

| Input persisted `asr_mode` | Post-upgrade value | First read migrated? | Network/model installation |
| --- | --- | --- | --- |
| `moonshine_tiny_streaming` | `whisper_small` | Yes | None |
| `moonshine_small_streaming` | `whisper_small` | Yes | None |
| `whisper_small` | `whisper_small` | Only for other legacy reasons | None |
| `gemini_live_audio` | `gemini_live_audio` | Only for other legacy reasons | None |
| Missing in legacy record | `gemini_live_audio` | Yes | None |
| Unsupported/invalid ASR value | Error | No write | None |
| Future `settings_version` | `FutureVersion` error | No write | None |

Test both Tiny and Small on Mac and Linux using fully populated current- and old-version fixtures; prove exact serialized output, `future_settings` preservation with nested credential scrubbing, database persistence, migration idempotence across restart, preservation of independently selected text/TTS/voice and privacy fields, and no runtime installer/network calls during migration. Test install state separately after migration, including missing and corrupt Whisper models.

**Do not check WSP-300 implementation/migration acceptance boxes until these tests execute against the real cutover and both supported Mac architectures have passed WSP-200.**
