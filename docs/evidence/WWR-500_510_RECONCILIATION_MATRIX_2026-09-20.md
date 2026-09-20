# WWR-500 / WWR-510 reconciliation matrix

Date: 2026-09-20
Baseline master: `2094c14a85730c7b43d9f7acf867f456f34f0495`

## WWR-500 — Wake Word Settings UI

Objective source/evidence:

- `src/components/Settings/WakeWordSettingsPanel.tsx`
- `src/components/Settings/GeneralTab.tsx`
- `src/components/Settings/SettingsModalBase.tsx`
- `src/components/Settings/WakeWordSettingsPanel.test.tsx`
- `src/test/WakeWordSettingsPanel.test.tsx`
- `docs/evidence/WWR-500_WAKE_WORD_SETTINGS_UI_2026-09-20.md`
- PR #270 exact-head ordinary CI `35498154276`
- PR #270 exact-head Wake Word documentation audit `35498154265`
- exact merged-master ordinary CI `35498219780`
- exact merged-master Wake Word documentation audit `35498219843`

Reconciled requirements covered by the merged source/tests/evidence:

- Add Wake Word section under Settings.
- Add `Enable wake word` toggle.
- Display fixed phrase `Hey, Moose`.
- Do not expose arbitrary phrase editing.
- Do not expose sensitivity in V1.
- Explain local/offline keyword spotting.
- Explain microphone remains locally active while listening for Wake Word.
- Show useful runtime status: loading/listening/suspended/error.
- Show sanitized failure/help text.
- Apply toggle to runtime without app restart when safe.
- Ensure disabling restores manual behavior immediately/boundedly.
- Ensure UI never implies full-time cloud transcription.
- Add accessibility labels and keyboard behavior consistent with Settings conventions.
- Default UI shows disabled.
- Toggle persists enabled state.
- Toggle updates runtime.
- Toggle off stops/suspends Wake Word according to policy.
- Phrase is displayed but not editable.
- Local/offline and active-mic disclosures render.
- Runtime error state is displayed without raw path/secret leakage.
- User can enable/disable Wake Word entirely through normal Settings UI.

Limits not claimed: this evidence does not claim production microphone routing, real KWS fixture acceptance, wake-to-ASR handoff, or full lifecycle acceptance.

## WWR-510 — Privacy-safe diagnostics

Objective source/evidence:

- `src-tauri/src/asr/wake_word_diagnostics.rs`
- `src-tauri/src/commands/wake_word_diagnostics.rs`
- `docs/evidence/WWR-510_PRIVACY_SAFE_DIAGNOSTICS_2026-09-20.md`
- PR #271 exact PR-head ordinary CI `35499561565`
- PR #271 exact merged-master ordinary CI `35499705029`

Reconciled requirements covered by the merged source/tests/evidence:

- Expose Wake Word enabled state.
- Expose authoritative runtime state.
- Expose exact model identity.
- Expose exact runtime version/identity.
- Expose platform/architecture.
- Expose one-thread policy.
- Expose canonical sample rate/channels.
- Expose ring duration/capacity.
- Expose threshold/score.
- Expose trigger count.
- Expose last-trigger age/timestamp in approved form.
- Expose initialization duration.
- Expose Talking suspension.
- Expose sanitized last error.
- Ensure raw PCM cannot be represented/serialized.
- Diagnostics can troubleshoot lifecycle/artifact/performance issues without exposing audio or secrets.

Requirements intentionally left open:

- Optional measured CPU/memory/inference/handoff timing fields, because no accepted measurements exist yet.
- Repository-wide log/error audits for credentials, unnecessary absolute paths, and audio content, because those remain WWR-900/final-audit scope.

## Final checklist reconciliation

The merged WWR-500 and WWR-510 evidence also supports checking these final-remediation checklist entries when the canonical TODO is updated:

- Settings UI can enable/disable Wake Word.
- UI discloses local/offline KWS and active microphone behavior.
- Diagnostics are privacy-safe.

This matrix is deliberately bounded to already-merged evidence and must not be used to infer completion of WWR-300, WWR-310, WWR-400 lifecycle acceptance, WWR-600+ corpus/real-KWS acceptance, WWR-630 performance, WWR-640 lifecycle stability, or WWR-900 final audit.
