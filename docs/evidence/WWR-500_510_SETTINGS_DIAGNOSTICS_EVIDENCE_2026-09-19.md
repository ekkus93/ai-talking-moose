# WWR-500 / WWR-510 Settings and Diagnostics Evidence

Date: 2026-09-19

This note records objective evidence for completed Wake Word Settings UI and privacy-safe diagnostics work. It is intentionally limited to source paths and exact merge/CI evidence already on `master`; it does not claim completion of real-audio corpus, real KWS acceptance, microphone ownership, or wake-to-ASR handoff work.

## Scope

Covered remediation areas:

- WWR-500 — Wake Word Settings UI items that are implemented in the normal Settings surface.
- WWR-510 — privacy-safe Wake Word diagnostics fields that are exposed through the backend command and rendered/used by the Settings UI.

Out of scope:

- WWR-300/310 production microphone routing and wake-to-ASR handoff acceptance.
- WWR-600+ corpus, real fixture, platform acceptance, and performance evidence.
- Final closeout qualification.

## Merge and CI evidence

- PR #236 merged Wake Word Settings UI regression coverage at `c1ac10819c6af4d0d0a103ee58200c47d6f07eca`.
  - Exact final PR head: `e51e36c5110b2efd10a21fce42cfe23c3e6daa2a`.
  - Exact-head ordinary CI: `35474903777`.
  - Exact merged-master ordinary CI: `35475002448`.
- PR #237 merged Wake Word runtime diagnostics exposure at `6fc16cf31cac1b30575673bd997f03fdb22b6203`.
  - Exact-head ordinary CI: `35476943401`.
  - KittenTTS CPU acceptance: `35476943386`.
- PR #238 merged disabled-interaction resume lifecycle coverage at `a49d048774bbb02fca5f21f34f34de47449fc876`.
  - Exact final PR head: `e3f34140675de66d897dddc5ab72452db4464d1c`.
  - Exact-head ordinary CI: `35478870301`.
  - Exact merged-master ordinary CI: `35479150116`.

## WWR-500 evidence

### Normal Settings UI

Source: `src/components/Settings/GeneralTab.tsx` renders `WakeWordSettingsPanel` in the normal Settings General tab.

Source: `src/components/Settings/WakeWordSettingsPanel.tsx` implements:

- `Enable wake word` checkbox.
- Fixed phrase display of `Hey, Moose`.
- No arbitrary phrase edit field.
- No V1 sensitivity control.
- Local/offline keyword spotting disclosure.
- Active local microphone disclosure.
- Disclosure that Wake detection is not full-time cloud transcription.
- No-barge-in disclosure.
- Runtime status row populated from `getWakeWordDiagnostics`.
- Sanitized runtime failure/status text.
- Accessibility linkage through `aria-describedby` and live status regions.

### Toggle persistence and live runtime application

Source: `src/components/Settings/WakeWordSettingsPanel.tsx` applies the toggle through `updateSettingsPatch` with canonical `wake_word_phrase: "Hey, Moose"`.

Source: `src-tauri/src/commands/settings.rs` normalizes live settings updates with `WakeWordSettings::from_app_settings_fields` before persistence.

Source: `src-tauri/src/app/runtime_preferences.rs` applies Wake Word setting changes through `WakeWordApplicationRuntime::apply_enabled_setting` before persisting settings and rolls the runtime back if later runtime/persistence work fails.

The runtime-preferences tests cover immediate enable/disable behavior, unchanged-setting no-op behavior, and rollback to the previous disabled state.

### Regression coverage

Source: `src/components/Settings/WakeWordSettingsPanel.test.tsx` verifies:

- Default UI shows Wake Word disabled.
- The fixed phrase is displayed and not editable.
- Local/offline and active-mic disclosures render.
- The UI does not imply full-time cloud transcription.
- Toggle-on persists `wake_word_enabled: true` and the canonical phrase.
- Toggle-off persists `wake_word_enabled: false` and the canonical phrase.
- Runtime status and sanitized diagnostics behavior settle before persistence assertions.

## WWR-510 evidence

Source: `src-tauri/src/asr/wake_word_diagnostics.rs` defines the privacy-safe `WakeWordDiagnostics` payload. The Rust type intentionally exposes lifecycle/configuration metadata and bounded counters, while raw PCM, transcripts, credentials, and filesystem paths are not representable.

Source: `src-tauri/src/commands/wake_word_diagnostics.rs` exposes authoritative runtime diagnostics through `get_wake_word_diagnostics` using the application Wake Word runtime from `AppState` composition.

Source: `src/lib/tauriBridge.ts` exposes `getWakeWordDiagnostics` to the frontend.

Source: `src/types/wakeWord.ts` contains the frontend Wake Word diagnostics type away from the broad IPC shape gate, preserving existing generated contract checks while still typing the UI bridge.

Source: `src/lib/browserPreviewBridge.ts` and `src/test/setup.ts` provide deterministic preview/test diagnostics without requiring audio hardware.

Implemented diagnostic fields include:

- Wake Word enabled state.
- Authoritative runtime phase.
- Model identity and model archive SHA-256.
- Runtime identity, license, platform, architecture, and platform runtime C API hash where supported.
- Canonical sample rate and channels.
- One-thread inference policy.
- Threshold and score.
- Ring buffer sample and duration capacity.
- Current ring/pre-roll sample counts.
- Trigger count.
- Last-trigger age in bounded millisecond form.
- Initialization duration.
- Talking suspension flag.
- Sanitized last error.

Privacy/security coverage in `wake_word_diagnostics.rs` verifies serialized diagnostics do not contain raw PCM, transcript, credential, or path-like fields for the covered runtime states.

## Remaining unchecked areas

The following remain intentionally open after this evidence note:

- Real positive and negative fixture acceptance.
- Linux/macOS real KWS platform acceptance completion.
- Integrated production microphone ownership and wake-to-ASR handoff acceptance.
- Performance baseline.
- Full original TODO reconciliation and final closeout.
