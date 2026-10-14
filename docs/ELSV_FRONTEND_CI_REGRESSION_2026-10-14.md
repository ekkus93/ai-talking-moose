# ELSV frontend regression: Google model controls and ASR mode

Date: 2026-10-14

## Verified failure

CI run 38516423237 at master 0c3f84758b0f17f5e355f3f959ffa0e91862c9da failed the Frontend quality job (115696187092), with three failures in `src/test/SettingsModal.test.tsx`. The Rust quality and generated backend contract jobs passed; the Rust test job was still running when the frontend diagnosis was obtained.

The three failures are:
- "preserves model selections and blocks writes while the Google catalog is loading"
- "shows a persisted Google model as unavailable after catalog resolution without rewriting it"
- "shows only current capability-filtered Gemini model options"

The first two tests cannot find the "Realtime Live Voice Model" label; the third cannot find a Gemini Live model option.

## Root cause and precise remediation

The current `AiTab` intentionally renders the Gemini Live model selector only when `settings.asr_mode === "gemini_live_audio"`. Google text with local ASR uses non-Live Talk and must not expose or start Gemini Live. The three tests set `text_provider: "google"` but retain the default local ASR, then assert the Live selector is present.

Fix the test setup rather than weakening product routing. For the three Live-selector tests, initialize both `text_provider: "google"` and `asr_mode: "gemini_live_audio"` before rendering. Add a separate negative test proving that Google text with local ASR shows the Gemini text selector but not the Live selector. Preserve the default Local/KittenTTS behavior.

## Qualification required

Run the focused SettingsModal frontend tests, then the full frontend quality gate. Verify the exact new commit head through Ralph Bridge and require CI success before claiming this is fixed. Do not mark ELSV-400 or ELSV-600 accepted solely from these tests.

## Tool write limitation

An attempted atomic `commit_files` update of the existing SettingsModal test file was rejected by tool safety checks before the bridge accepted a commit. This diagnosis is not a source fix.
