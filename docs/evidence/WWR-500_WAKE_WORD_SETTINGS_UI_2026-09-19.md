# WWR-500 — Wake Word Settings UI evidence

Evidence head reviewed: `01a45ce95720eac25f7fa8f22b788eaadba1786a`.

## Production UI

`src/components/Settings/WakeWordSettingsPanel.tsx` provides the normal Settings Wake Word section and:

- exposes an accessible `Enable wake word` checkbox;
- displays the fixed `Hey, Moose` phrase as text rather than an editable phrase field;
- exposes no sensitivity control;
- states that V1 uses local/offline keyword spotting;
- states that the microphone remains locally active while listening;
- states that wake detection is not full-time cloud transcription;
- states that the wake phrase plus immediate command may enter normal ASR;
- states the V1 no-barge-in limitation;
- renders loading/listening/suspended/error runtime phase labels from privacy-safe diagnostics;
- displays only the sanitized diagnostics error returned by the backend;
- persists enable/disable through the normal settings update path.

## Live runtime application

`src-tauri/src/app/runtime_preferences.rs` binds a persisted Wake Word setting transition to the authoritative `WakeWordApplicationRuntime` through `apply_wake_word_setting_change`. `apply_changed_runtime_preferences` resolves the process-wide runtime only when the enable bit changes, applies the new state before persistence, and rolls the runtime back to the previous setting if a later reversible runtime-preference side effect fails. The settings command already rolls runtime preferences back when persistence fails.

Focused Rust tests cover immediate Disabled → Loading and Listening → Disabled transitions, unchanged-setting stability, and rollback to the previous disabled state.

## Frontend regression coverage

`src/components/Settings/WakeWordSettingsPanel.test.tsx` verifies:

- disabled-by-default rendering;
- fixed phrase is displayed but not editable;
- local/offline, active-microphone, non-cloud-transcription, no-barge-in, and manual-start disclosures;
- enabling persists `wake_word_enabled: true` with canonical `Hey, Moose`;
- disabling persists `wake_word_enabled: false` with canonical `Hey, Moose`.

## Qualification evidence

The documentation/source-backed live-toggle correction merged to master at `01a45ce95720eac25f7fa8f22b788eaadba1786a` with:

- ordinary CI run `35487357684`: success;
- Wake Word privacy audit run `35487357710`: success;
- Wake Word documentation audit run `35487357681`: success.

This evidence closes the implementation/test substance of WWR-500. It does **not** claim WWR-300/310 production microphone routing, real KWS fixture acceptance, or WWR-640 integrated lifecycle soak completion.
