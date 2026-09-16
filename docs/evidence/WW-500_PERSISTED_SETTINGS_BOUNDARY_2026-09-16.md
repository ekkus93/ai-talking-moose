# WW-500 persisted settings boundary evidence

Date: 2026-09-16
Qualified master before this record: `609558b7739575723b88b9983bf3baf492cff631`
Exact-master CI: `35059747906` (`CI`, success)

## Implemented and qualified

`src-tauri/src/app/wake_word_settings.rs` now defines the engine-independent Wake Word V1 persisted-settings boundary:

- `wake_word_enabled` and `wake_word_phrase` are the frozen persisted field identities.
- Wake mode defaults disabled.
- The canonical V1 phrase is `Hey, Moose`.
- Missing wake fields project to disabled/default values.
- Enabled state is preserved by the projection boundary.
- Case/whitespace variants of the canonical phrase normalize deterministically.
- Arbitrary phrases and invalid persisted field types fail closed.
- V1 threshold/score remain fixed rather than being exposed as unstable persisted sensitivity controls.
- The implementation is independent of sherpa runtime/model types.

These behaviors are covered by deterministic Rust unit tests and were qualified on exact master by CI run `35059747906`.

## Deliberately not claimed complete

WW-500 is not complete yet. The authoritative `AppSettings` schema in `src-tauri/src/app/state.rs` still needs the wake fields wired into serialization/default/migration behavior, and `src/generated/backendContract.json` must then be regenerated/updated from the authoritative Rust defaults. The existing generated-contract test intentionally prevents the frontend/backend settings shape from drifting.

The next WW-500 source slice must therefore make the `AppSettings` and generated-contract changes atomically and prove:

1. new/default settings serialize wake disabled with `Hey, Moose`;
2. legacy/missing wake fields migrate to those defaults without changing unrelated ASR/TTS settings;
3. persisted enabled state survives load/restart normalization;
4. invalid wake configuration fails closed through `PersistedSettingsError::Invalid`;
5. the generated frontend/backend contract exactly matches authoritative Rust defaults.

This evidence record exists to prevent the already-qualified projection boundary from being mistaken for full WW-500 completion.
