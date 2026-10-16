# WSP-300 — Cross-platform default selection evidence (2026-10-16)

**Source:** `fe3ddc642b73f57aa88ab8ccd1cb0ad4272f3934` on `master`.

## Implemented and qualified scope

- `src-tauri/src/asr/types.rs` selects `AsrMode::WhisperSmall` as the Rust enum default on both `target_os = "linux"` and `target_os = "macos"`. Other targets retain the explicit fallback default of `GeminiLiveAudio`; the user-facing Linux/Mac selector still offers Gemini Live only as an explicit cloud choice.
- `src-tauri/src/app/state.rs` uses `AsrMode::default()` for fresh `AppSettings`, and `CURRENT_SETTINGS_VERSION = 9`. Its persisted-settings loader rewrites the two historical Moonshine ASR selections to `whisper_small` before enum decoding while retaining explicit Gemini Live.
- Rust tests include `supported_platform_asr_default_is_whisper_small`, `new_settings_use_the_platform_asr_default_and_local_text`, and Moonshine settings-migration/idempotence tests.
- Exact-source main CI `38893284623` completed **success**: `Rust quality`, `Rust tests`, and `Generated backend contract` jobs passed. The frontend-specific UI gate previously passed at `ba6f871acd496db4f6fa584e32852c9f2834ea38`, CI `38893061667`.

## Important independent CI failure and remaining work

The **separate** `ASR-015 Native Acceptance` workflow `38893284686` failed on the same SHA in `Prepare pinned Moonshine native runtime`: fallback ONNX Runtime archive size was **35,138,784** bytes versus **27,623,992** expected. This is the historical Moonshine/LFS dependency targeted for replacement under WSP-500/600; it is **not** a passing native gate. Do not silently weaken the gate or delete KittenTTS's independent ONNX Runtime.

This evidence qualifies the **fresh-profile Rust default** checkbox only. It does not prove full app settings migration, a clean Mac installation, Mac microphone, packaged app, egress denial, provider/TTS matrix or safe Moonshine deletion. Backend settings-write and IPC rejection of new retired Moonshine selections should be verified explicitly during WSP-300/400; persisted migration alone does not prove that direct stale IPC writes cannot reintroduce old modes.
