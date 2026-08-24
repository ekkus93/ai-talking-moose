# Phase P5 reconciliation — 2026-08-23

This overlay supersedes the remaining unchecked P5 rows in the legacy monolithic TODO. The original P5 implementation was accepted in `4bece8465255e55795e07e0786c07deb8a71ba85` / `bf733d7928073177cddbf27e72c47d2512836041`; later P6–P8 work completed the dependencies that P5 intentionally left open.

## Closure candidate: V1R-050 through V1R-056 and Gate P5

Status: **implementation complete; pending GitHub Actions acceptance for this closure commit**.

### V1R-050 — Complete settings schema/version/validation

- [x] Versioned settings, ASR migration, enum/range/model/device validation, additive-field migration, and transactional persistence were already accepted.
- [x] General runtime preferences now participate in the transaction: reversible OS/window changes are applied before persistence and rolled back if SQLite persistence fails.
- [x] The legacy `microphone_permission_granted` field remains compatibility-only; `get_settings`, permission commands, and diagnostics overwrite/query live OS permission state rather than trusting the persisted cache.

### V1R-050A — Runtime ASR selection synchronization

- [x] Current ASR selection is resolved at conversation start.
- [x] ASR/provider/device/privacy changes stop the active graph and require a fresh explicit Start action.
- [x] Missing local models fail explicitly and never fall back to Gemini microphone upload.

### V1R-051 — Talkativeness/runtime behavior settings

- [x] Persisted behavior/personality values seed the Rust `BehaviorEngine` at startup.
- [x] Settings updates apply to the live engine immediately.
- [x] Existing backend/frontend regression coverage maps the behavior sliders/toggles into runtime policy.

### V1R-052 — Quiet hours in local timezone

- [x] Runtime converts UTC policy time to the machine's local wall clock before evaluating quiet hours.
- [x] Same-day, overnight, equal-boundary, and Pacific-offset cases are covered.
- [x] Deterministic DST regressions now cover Pacific spring-forward (01:30 PST → 03:30 PDT) and both occurrences of the repeated 01:30 hour during fall-back. Policy follows local wall-clock hour in both cases, so no separate manual DST acceptance is required.

### V1R-053 — Active-app setting enforcement

- [x] Dispatch-time tool privacy gating was already accepted.
- [x] P8 replaced the fabricated observer with real macOS `NSWorkspace` active-application observation and fail-closed typed results.
- [x] Turning observation Off prevents the OS query and clears derived active-app fingerprint/switch history; re-enable begins from fresh state.

### V1R-054 — Memory setting enforcement

- [x] Memory mutation is gated at Rust tool policy and again inside the built-in memory tool.
- [x] Production model-context reads are limited to the conversation and ambient prompt paths; both check `memory_enabled` before retrieving facts.
- [x] Data-management reads/deletes (`get_memories`, `delete_memory`, `forget_everything`) remain explicit user controls and do not feed disabled memory into model context.
- [x] Repo-wide audit after P6/P7/P8 found no additional production model-memory read path that bypasses the setting.

### V1R-055 — Transcript retention enforcement

- [x] Shared final-transcript persistence helper remains the only conversation/text retention path.
- [x] Live partials are not persisted and final user/Moose roles are gated by `save_transcripts`.
- [x] Existing final repo-wide write-call audit remains valid after P6–P8.

### V1R-056 — Restore Moose window position

- [x] Move capture, persistence, restart restore, visible-display clamping, and disconnected-display behavior remain accepted.

## Gate P5 — persisted UI settings affect runtime behavior

The current visible settings surface maps to runtime behavior as follows:

| UI setting | Runtime owner/effect |
| --- | --- |
| Launch at login | macOS per-user `com.talkingmoose.ai` LaunchAgent, atomically installed/removed by Rust |
| Show in menu bar | Tauri tray visibility at startup and live settings updates |
| Always on top | Tauri main-window `set_always_on_top` at startup and live settings updates |
| Restore position | persisted/clamped native window position at startup/shutdown/move |
| Unsolicited comments / talkativeness / hourly cap / quiet hours | live Rust `BehaviorEngine` / P7 scheduler policy |
| Input/output device | conversation capture/playback, standalone TTS, and diagnostics |
| Moose voice | Gemini TTS / conversation / ambient / audition paths |
| ASR mode | authoritative conversation ASR graph selection |
| Personality sliders | live character config and prompt/behavior policy |
| Live model | Gemini Live session configuration |
| Text model | text-turn and ambient-generation model selection |
| Active-app observation | P8 observer + tool + P7 delivery privacy gates |
| Memory | memory mutation and model-context read gates |
| Transcript retention | final transcript persistence gate |

Fields retained in the schema for internal/default/migration compatibility but not exposed as editable UI controls (for example `provider`, `tts_model`, the legacy microphone-permission cache, and the V1-disabled window-title field) are not counted as interactive P5 controls.

When the closure commit passes the normal CI matrix, **Gate P5 is accepted complete**.
