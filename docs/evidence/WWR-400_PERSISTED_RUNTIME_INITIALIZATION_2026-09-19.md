# WWR-400 persisted runtime initialization evidence

Date: 2026-09-19
Baseline master: `94fb74ffe778f45aaf2a1eb024e8c2c1f1c66aae`

## Source evidence

`AppState::new_with_secret_store` loads and normalizes persisted `app_settings` from SQLite before constructing `WakeWordApplicationRuntime`. The runtime is then constructed from the loaded `settings` value and stored directly in `AppState::wake_word_runtime`.

`WakeWordApplicationRuntime::from_settings` has fail-closed startup semantics:

- `wake_word_enabled == false` leaves the authoritative runtime in `Disabled`.
- `wake_word_enabled == true` calls `WakeWordRuntimeManager::begin_enable`, entering `Loading` rather than falsely claiming `Listening` before the pinned KWS runtime is loaded.

Focused composition tests cover disabled startup and enabled startup through `Loading -> Listening` after `mark_loaded`. AppState tests cover persisted settings loading/migration independently.

## Qualification

The AppState ownership implementation was merged by PR #267 as exact master `94fb74ffe778f45aaf2a1eb024e8c2c1f1c66aae`.

Exact-master validation passed:

- ordinary CI run `35494035068`
- KittenTTS production CPU acceptance run `35494035070`
- Wake Word lifecycle stability run `35494035071`

This evidence closes only the WWR-400 requirements that the authoritative runtime is initialized from persisted settings and remains disabled when the persisted setting is false. It does not claim production KWS loading, microphone routing, trigger activation, or full interaction lifecycle integration; those remain open.
