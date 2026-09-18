# WWR-500 — Wake Word Settings UI Evidence

**Task:** WWR-500 — Implement Wake Word Settings UI
**Merged implementation:** PR #207
**Merged `master` SHA:** `4c8dc57e850bae182710a9837bbbcaf76c07d96a`
**Exact PR-head SHA:** `7601d7543bfc0ef83f5dd9ace2bd94756af7ebaf`
**Exact PR-head CI:** `35403953141`
**Exact merged-master CI:** `35404033768`

## Implemented UI surface

`src/components/Settings/WakeWordSettingsPanel.tsx` adds a Wake Word section under Settings > General via `src/components/Settings/GeneralTab.tsx`.

The panel provides:

- an `Enable wake word` toggle bound to `wake_word_enabled`;
- fixed phrase display for `Hey, Moose`;
- no editable phrase textbox;
- no V1 sensitivity control;
- local/offline keyword spotting disclosure;
- active local microphone disclosure while listening;
- explicit statement that Wake Word detection is not full-time cloud transcription;
- handoff disclosure that the wake phrase plus immediate command audio may enter normal ASR;
- no-barge-in disclosure while Moose is talking;
- accessible heading, toggle label, phrase label, and live status region.

## Implemented tests

`src/test/WakeWordSettingsPanel.test.tsx` covers:

- default disabled UI;
- fixed phrase rendering;
- absence of a phrase textbox;
- local/offline and active-microphone disclosures;
- no full-time cloud transcription disclosure;
- no-barge-in disclosure;
- persisted enable toggle using the canonical `Hey, Moose` phrase;
- enabled status copy.

## Scope note

This PR establishes the Settings UI foundation. It intentionally does not claim full WWR-500 completion because runtime-status detail and live runtime stop/start semantics depend on the still-open lifecycle/runtime work in WWR-400. The UI does persist the user setting through the normal settings path, but production wake listening is not yet fully activated by the panel alone.

## Remaining WWR-500 dependencies

The following WWR-500 items should remain open until WWR-400/runtime integration exposes authoritative runtime status to the UI:

- loading/listening/suspended/error runtime status beyond settings-level enabled/disabled;
- sanitized runtime error/help text from the live Wake Word runtime;
- proof that toggling off stops/suspends the live Wake Word runtime according to final policy;
- proof that disabling restores manual behavior immediately while a live Wake Word runtime exists.
