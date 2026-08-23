# Desktop observation policy — V1

## Scope

V1 desktop observation is local, typed, minimized, and fail-closed. Production code must never substitute a placeholder application name, battery level, idle duration, window title, or power event when the operating system cannot provide a real value.

Every observer returns the common `ObserverResult<T>` contract with one of these states:

- `available` — contains a real local observation;
- `denied` — the user privacy gate is off and the OS is not queried;
- `unavailable` — the platform supports the source but no value currently exists;
- `unsupported` — this build/platform deliberately does not implement the source;
- `error` — a platform API returned an invalid or unusable result.

Diagnostics serialize only observer kind, status, and a fixed error code. Observation values are never part of diagnostic metadata.

## V1 macOS sources

- **Idle time:** CoreGraphics `CGEventSourceSecondsSinceLastEventType` using the combined session and any-input event type. Non-finite/negative results fail closed; downstream summarization bounds retained idle duration to 24 hours and emits at most one event per five-minute bucket.
- **Sleep/wake:** `IORegisterForSystemPower` on a dedicated bounded-run-loop thread. `kIOMessageCanSystemSleep` and `kIOMessageSystemWillSleep` are acknowledged as required. The observer is deregistered, its notification port is destroyed, and the root power connection is closed on runtime shutdown.
- **Battery:** IOPowerSources (`IOPSCopyPowerSourcesInfo`, `IOPSCopyPowerSourcesList`, and `IOPSGetPowerSourceDescription`). Only an internal battery is accepted. Capacity and charging values are type-checked; a desktop/no-battery machine reports `unavailable` rather than a fabricated percentage.
- **Active application:** AppKit `NSWorkspace.sharedWorkspace.frontmostApplication.localizedName`. The method returns `denied` before any AppKit query unless `active_app_observation` is enabled. The active-application privacy gate is checked again by the P7 ambient-delivery path before model generation/delivery.

No desktop observer opens the microphone or calls an AI provider. Available observations flow only into the bounded local summarizer and then into the existing P7 ambient scheduler. Built-in battery and active-application tools use the same typed observer contract: unsupported/denied/unavailable/error results return status metadata instead of fabricated values, and the active-application tool cannot query the OS while its opt-in is off.

## Window-title decision — V1R-085

V1 deliberately does **not** implement window-title observation.

Window titles are higher-sensitivity content than application identity and a general implementation can require broader Accessibility/Screen Recording-style access. The persisted compatibility field remains in the settings schema, but the production P7 privacy gate treats `WindowTitle` as denied and `SystemDesktopMonitor::get_window_title(true)` returns `unsupported`. A stale legacy `true` value therefore cannot expose a title to model context, tools, or normal logs.

Reconsidering window titles requires a separate design review, explicit user opt-in, a narrow OS permission model, minimized/bounded title handling, and dedicated privacy regressions.

## Local summarization and retention

The summarizer accepts only values extracted from `ObserverResult::Available`.

- Active-application names are whitespace-normalized and capped at 80 characters before transient event construction.
- Only a SHA-256 fingerprint of the last application identity is retained for change detection; prior names are not kept in history.
- Rapid-switch history retains timestamps only, is limited to a 120-second window, and clears after the six-switch pattern event. The pattern summary contains a count, not historical application names.
- Idle time is reduced to five-minute buckets and bounded to 24 hours.
- Battery state retains only the previous percentage and emits on the 20% and 10% downward thresholds.
- Sleep/wake has no retained payload beyond the event currently being submitted.

P7 still applies deduplication, annoyance budget, quiet hours, conversation/mute checks, and privacy gating before any generated remark is delivered.
