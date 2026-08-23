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

P12's implementation candidate is `6657fd337e99e3e55d8a250b4ce69ac892111925`; its reconciliation remains pending CI acceptance until the user reports the published checkpoint green. This overlay does not promote P12 by itself.

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
