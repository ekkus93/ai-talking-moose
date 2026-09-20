# WWR-500 — Wake Word settings UI evidence

Date: 2026-09-20
Baseline master: `b6e438fe62e480191c00d90cae088ec097e318b8`

## Implemented in this slice

`src/components/Settings/WakeWordSettingsPanel.tsx` adds a dedicated Settings tab for Wake Word V1.

The panel provides:

- an accessible `Enable wake word` checkbox backed by `updateSettingsPatch`;
- fixed, read-only phrase display for `Hey, Moose`;
- explicit text that V1 does not expose arbitrary phrase editing or sensitivity controls;
- local/offline keyword-spotting disclosure;
- active local microphone disclosure for the enabled/listening policy;
- explicit text that Wake Word does not imply full-time cloud transcription;
- privacy-safe runtime status from `get_wake_word_diagnostics`;
- sanitized user-visible error presentation using backend diagnostic strings only.

`src/components/Settings/SettingsModalBase.tsx` adds the Wake Word tab to the normal Settings sidebar and renders the panel in the main content area.

## Limits intentionally not claimed

This UI slice does not claim that production microphone routing, real KWS fixture acceptance, wake→ASR handoff, or full lifecycle acceptance is complete. Those remain tracked by WWR-200, WWR-300, WWR-310, WWR-400, and WWR-600+.
