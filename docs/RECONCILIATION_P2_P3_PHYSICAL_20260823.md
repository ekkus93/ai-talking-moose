# P2/P3 physical-acceptance reconciliation — 2026-08-23

This overlay reconciles the stale source-level rows in `docs/TODO(20260818-163801).md` before the final supported-Mac physical acceptance tranche. It does **not** claim physical evidence that has not been run.

## Source/test rows already closed by later phases

The following legacy unchecked rows are now covered by implementation/tests introduced after the monolithic tracker was written:

- **V1R-023 sustained overload:** playback is hard bounded and deterministic overload coverage verifies newest-tail dropping, counters, and flush behavior; P12 extends the stress/failure matrix around bounded queues.
- **V1R-023 stale pre-interruption audio:** barge-in flushes a non-empty real playback abstraction immediately and the conversation tests suppress stale audio/non-audio response callbacks until the next user-turn boundary.
- **V1R-025 representative format/channel regression:** resampler tests cover stereo downmix, signed 16-bit round-trip conversion, unsigned 16-bit endpoint/midpoint conversion, while production capture routes `f32`, `i16`, and `u16` through the same mono/resample processor.
- **V1R-031 command-level invalid transition:** `set_character_state_ipc_rejects_invalid_transition_without_mutating_state` exercises the Tauri IPC command boundary and verifies authoritative state is unchanged.
- **V1R-032 setup readiness:** P4 V1R-041 explicitly parses setup acknowledgement and prevents Listening readiness before it; structured provider error categories are implemented under P4.
- **V1R-034 Listening/Talking mute integration:** `mute_ipc_tears_down_listening_and_talking_and_unmute_stays_passive` exercises both states through Tauri IPC and verifies mic/playback teardown plus passive unmute.
- **V1R-035 final hide/cooldown + Listening/Talking integration:** P7 closes appearance/cooldown semantics and `dismiss_ipc_tears_down_listening_and_talking_hides_and_records_cooldown` exercises both states through Tauri IPC.
- **V1R-038 ambient shutdown:** P7 application shutdown cancels/awaits the central ambient scheduler. Later window-position persistence work serializes the final shutdown write so an older debounce cannot overwrite it.

P12 is now accepted on current master through follow-up commit `e03cec29e2b65473fb7e2eee94c81017bfe64d70`; GitHub Actions run `32699957139` completed successfully with Rust formatting, Clippy, Rust tests, frontend quality, dependency/security audits, release metadata, and both macOS bundle smoke jobs green.

## 2026-08-24 physical-evidence integrity hardening

Before asking a supported Mac to close the remaining rows, the evidence path was audited for reproducibility. The prior runner recorded repository `HEAD` but could not prove that the `.app` under test came from that commit. It also allowed a PASS row with blank evidence notes.

The physical runner now:

- [x] requires source-controlled inputs to match the tested commit, including untracked-file detection, while excluding only known dependency/generated/build trees that may legitimately differ during a local package build;
- [x] requires a packaged `.app` and verifies `com.talkingmoose.ai`, bundle version, executable path, and embedded full Git build commit before any manual evidence is accepted;
- [x] fails if the packaged binary commit differs from repository `HEAD` or optional `P2_P3_EXPECTED_COMMIT`;
- [x] records the validated executable SHA-256 and verifies it again at the end of the run;
- [x] launches the exact validated bundle with `open -n` before physical checks; and
- [x] requires nonblank evidence/notes for every PASS/FAIL/SKIP result so a nominal PASS cannot carry an empty evidence record.

The binary exposes this provenance through a non-GUI `--build-info` probe. `build.rs` prefers explicit `TALKING_MOOSE_BUILD_COMMIT`, then GitHub Actions `GITHUB_SHA`, then the clean checkout's Git `HEAD`; local acceptance instructions explicitly set the expected/build SHA.

This hardening does not claim any physical row PASS. The preparation changes require ordinary CI acceptance at their publication head, and P2/P3 remains open until the validated supported-Mac report is completed.

## Physical rows that remain open

P2 still requires real supported-Mac evidence for:

- V1R-020 single Rust-owned production speech path;
- V1R-021 real 44.1/48 kHz output correctness;
- V1R-022 real CPAL output-format/device behavior;
- V1R-023 real overload/barge-in stale-audio behavior;
- V1R-024 permission, stale-device, and device-disconnect behavior;
- V1R-025 built-in and USB microphone behavior;
- V1R-026 clean-profile prompt, denial, and grant-later flow;
- V1R-027 complete Diagnostics UI/device acceptance.

P3 still requires:

- V1R-037 a real Mac interruption-to-audible-playback-stop latency measurement and confirmation that stale speech never resumes.

Use `docs/MACOS_P2_P3_PHYSICAL_ACCEPTANCE.md` and `scripts/run_p2_p3_macos_physical_acceptance.sh` to collect this evidence against an exact candidate commit.

## Deliberately separate manual gates

This tranche does not subsume the previously identified physical/manual gates for P1 Keychain restart persistence, P3A Tiny/Small native transcription plus ASR-015 CPU benchmark/minimum-CPU decision, or P6 intentional final-voice listening comparison. Those retain their own acceptance status.

## Gate status

**P2/P3 physical acceptance remains open until a supported-Mac report passes every required row.** No sandbox, mock-audio test, ordinary CI run, or macOS bundle smoke job can substitute for that report.
