# WWR-310 handoff boundary evidence — 2026-09-22

## Scope

This evidence records deterministic source and unit-test coverage for the current Wake Word pre-roll/live command-ASR handoff boundary.

## Source evidence

`src-tauri/src/app/wake_word_pcm_router.rs` routes one canonical 16-kHz mono microphone timeline through `CanonicalWakePcmRouter`. Before trigger acceptance, the same validated PCM chunks are retained for pre-roll and fed to KWS in order. After trigger acceptance, subsequent canonical chunks are appended to the live handoff buffer and are not re-fed to KWS.

`transfer_handoff_audio_to_asr` returns a provider-neutral `WakeCommandHandoffAudio` payload and consumes the buffered handoff exactly once. `return_to_wake_listening` clears stale handoff state, resets KWS stream state, and resumes the shared Wake runtime.

## Regression coverage

Existing deterministic tests in `wake_word_pcm_router.rs` cover:

- chronological ring retention and KWS feed order;
- invalid PCM mutating neither ring retention nor KWS;
- accepted trigger transitioning runtime out of Listening and starting handoff;
- post-trigger live chunks being retained for command ASR without re-feeding KWS;
- validated command-ASR payload sample rate, sample content, and PCM16LE serialization;
- wake phrase tail plus immediate first command word being contiguous in downstream handoff audio;
- repeated positive frames after trigger not creating duplicate command activation;
- handoff transfer being single-use;
- return-to-listening resetting KWS stream state and allowing a later second phrase without cooldown;
- invalid post-trigger PCM not mutating live handoff;
- clearing stale handoff after ASR startup failure/cancellation paths.

Exact merged-master validation for `f3e7fa9494ab75000cf3d67006b94293e2a41ed5` passed ordinary CI `35758604895`.

## Non-claims

This evidence does not claim a full production conversation has consumed the handoff payload yet. WWR-400 remains open for one wake trigger starting exactly one normal command interaction, and WWR-640 remains open for repeated integrated wake→ASR→Thinking→Talking→wake lifecycle soak.

This evidence also does not claim real/reproducible `Hey Moose, tell me the time` audio fixture acceptance or downstream ASR transcription of the first command word. Those acceptance items remain open until a deterministic corpus/real-audio gate exists.
