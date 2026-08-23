# Phase P11 reconciliation — 2026-08-23

This overlay reconciles the legacy monolithic TODO without rewriting the large tracker file through the GitHub contents API.

## Accepted closure: V1R-110 through V1R-117

P11 is **accepted complete**.

Implementation commit: `a203381d78c52aae75fac33a9680ad8008271e26` (`feat: close P11 UX and accessibility`).

Follow-up CI fixes:

- `939fb3d5b32da36afdb4d9373f62e43ef5c61acb` (`style: apply rustfmt module ordering`).
- `b38bcacc9bb3b222d21aa8c263dc7c100d50ca10` (`fix: remove unused Tauri Manager import`).

Acceptance evidence: GitHub Actions CI run `32629218483` completed successfully on `master` at `b38bcacc9bb3b222d21aa8c263dc7c100d50ca10` on 2026-08-23.

Local frontend acceptance completed before publication: TypeScript, ESLint with zero warnings, Prettier, 31/31 Vitest tests, and the production Vite build passed. The final CI run supplied the required Rust formatting/compilation acceptance that was unavailable in the sandbox.

### V1R-110 — Onboarding

- [x] Explains that microphone capture is bounded to an active conversation or explicit microphone test and closes when the session stops.
- [x] Explains the Tiny/Small local-audio boundary versus explicit Gemini Live cloud microphone upload, including the no-silent-cloud-fallback guarantee.
- [x] Offers Moonshine Tiny download as an optional onboarding step that can be skipped.
- [x] Supports secure Google API-key setup without storing the key in the settings database.
- [x] States conservative defaults for active-app observation, cross-conversation memory, and transcript retention.

### V1R-111 — Privacy dashboard

- [x] Keeps the existing live microphone permission card/action.
- [x] Adds an active summary for microphone use, active-app observation, window-title policy, memory count/state, and transcript retention.
- [x] Adds direct `Forget stored data` and `Reset privacy defaults` actions.
- [x] Privacy reset disables active-app observation, window-title observation, memory, and transcript retention.

### V1R-112 — Menu bar/system tray

- [x] Adds a bounded Tauri tray/menu with Show Moose and Hide Moose actions.
- [x] Adds explicit Mute Moose and Unmute Moose actions.
- [x] Adds explicit Start Conversation and Stop Conversation actions routed through the existing store/backend lifecycle.
- [x] Adds Open Settings and left-click-to-show behavior.
- [x] Quit uses `AppHandle::exit`, which enters the existing `ExitRequested` shutdown path that stops desktop/ambient runtimes, persists position, closes conversation/audio resources, then exits.
- [x] `show_in_menu_bar` controls tray visibility at startup and at runtime.

### V1R-113 — Quiet-hours UI

- [x] Keeps the existing opt-in quiet-hours toggle.
- [x] Adds explicit start/end selectors for all 24 local hours with 12-hour display labels.
- [x] Explains same-day, overnight, and equal-start/end semantics in user-facing copy.

### V1R-114 — Talkativeness UI

- [x] Keeps the existing slider and runtime behavior-engine synchronization.
- [x] Explains that lower values raise the event-importance threshold and higher values make Moose more willing to comment, while hourly caps and quiet hours still apply.

### V1R-115 — Keyboard shortcuts

- [x] Defines focused-window shortcuts in Settings.
- [x] `Ctrl/Cmd+Enter` starts or stops a conversation when no modal/panel is blocking it.
- [x] `Ctrl/Cmd+Shift+M` mutes or unmutes Moose.
- [x] `Ctrl/Cmd+,` opens Settings and Escape closes the active panel.
- [x] Show/hide stays available from the tray so a hidden window can be restored.
- [x] No global keyboard hook/capture is registered; shortcuts are renderer-window local and suppressed while editing controls.

### V1R-116 — Accessibility

- [x] Moose/speech-bubble/panel controls use keyboard-operable native controls.
- [x] Adds a strong global `:focus-visible` outline.
- [x] Adds dialog/tab/status/meter semantics and labels for important icon-only/form controls.
- [x] Honors `prefers-reduced-motion` by collapsing animation/transition durations and disabling smooth scrolling.
- [x] Raises low-contrast helper copy in the touched P11 settings/onboarding paths while retaining the retro palette.

### V1R-117 — Window/pixel renderer polish

- [x] Existing ambient presentation remains presentation-only and does not raise/focus the native window.
- [x] Moose sprite sizing is quantized to integer multiples of its 32×32 source grid when renderer space permits.
- [x] Removes fractional active-scale animation from the Moose sprite.
- [x] Enforces pixelated/crisp-edge image and SVG shape rendering for the Moose artwork.
