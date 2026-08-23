# Phase P12 reconciliation — 2026-08-23

This overlay reconciles the legacy monolithic TODO without rewriting the large tracker file through the GitHub contents API.

## Current closure candidate: V1R-120 through V1R-125

P12 is **implemented in the source candidate but pending GitHub Actions acceptance**. Do not treat P12 as accepted complete until the published implementation commit is reported green.

Implementation candidate: `6657fd337e99e3e55d8a250b4ce69ac892111925` (`test: harden P12 runtime boundaries`).

The current sandbox source ZIP predates the already-accepted P11 commits, so P12 work is limited to files verified unchanged between ZIP commit `bae050ce94b9d56943e80b8e8ba3b0f599864f86` and P11-accepted `master` commit `5499bba34b3119a2018e55710c6fe60fe308c53c`, plus this reconciliation overlay.

Local validation available in this sandbox: `git diff --check`, workflow YAML parsing, and `scripts/validate_moonshine_runtime_manifest.py` pass. The previously supplied standalone Rust toolchain is not mounted in this new chat sandbox, so Rust formatting, Clippy, and tests require GitHub Actions acceptance for this candidate.

## V1R-120 — Resource limits

- [x] Playback queue is hard bounded by `MAX_QUEUED_PLAYBACK_SECONDS`; overflow drops the newest tail and records dropped-sample diagnostics (`playback_queue_has_hard_limit_and_drops_newest_tail`).
- [x] Microphone capture uses bounded channels and drops newest chunks on overload.
- [x] Moonshine ASR ingress is hard bounded (`LOCAL_ASR_QUEUE_CAPACITY_CHUNKS`; `ingress_is_hard_bounded_and_drops_newest`, `small_pipeline_uses_same_bounded_worker_and_reports_small_architecture`).
- [x] Gemini Live reconnect policy has hard retry/backoff bounds and explicit-close cancellation (`reconnect_backoff_is_bounded`, `retry_policy_has_hard_bounds_and_explicit_close_cancels_reconnect`).
- [x] Tool input, output, timeout, and audit history have hard caps in `ToolRouter`.
- [x] Transcript text/query/retention are bounded in SQLite. Desktop observations have no production persistence writer in V1, so persisted observation retention is effectively zero.
- [x] Prompt sections and final system/ambient prompts already have hard character budgets. P12 additionally caps queued ambient summaries at 2,048 characters before scheduler admission and rejects direct text turns above 16,384 characters.
- [x] Model-memory retrieval now issues a bounded SQLite query for only the newest 64 facts before prompt construction (`MAX_MODEL_MEMORY_RECORDS`; `model_memory_retrieval_has_a_hard_record_cap_and_prefers_recent_facts`).

## V1R-121 — Failure matrix

- [x] Missing default microphone and output-device paths fail closed through production helpers, with hardware-free unit coverage (`missing_default_microphone_fails_closed_without_hardware`, `missing_default_output_fails_closed_without_hardware`).
- [x] Microphone NotRequested, Denied, and Unavailable states map to explicit fail-closed errors through the same helper used on macOS (`microphone_permission_states_fail_closed_without_touching_hardware`).
- [x] Stale selected input/output devices are rejected by settings validation (`selected_device_must_come_from_current_enumeration`).
- [x] Output start failure closes the provisional provider session before microphone capture (`audio_output_start_failure_closes_provider_before_microphone_capture`).
- [x] Runtime input/output stream failures transition diagnostics to inactive/not-playing and record the structured stream error (`runtime_input_failure_stops_capture_and_records_diagnostics`, `runtime_output_failure_stops_playback_and_records_bounded_diagnostics`).
- [x] Moonshine missing/corrupt/incompatible/native-runtime failures are fail-closed before cloud fallback or microphone use in the existing engine/session tests.
- [x] Installer interruption, cancellation, checksum/revision mismatch, corrupt repair, disk-probe failure, insufficient disk, untrusted host, and stale partial-directory paths are covered by the existing Moonshine installer suite.
- [x] Google Auth, Quota, Network, Protocol, Closed, and other provider errors have explicit structured categories/retryability. Failed connect never becomes active; provider Closed/Error events converge through centralized cleanup.
- [x] Persistent DB initialization fails closed rather than silently using memory; failed secure-key migration preserves the legacy row; secret-store behavior is covered by backend tests.
- [x] Tool timeout, denied permission, confirmation requirements, oversized input/output, and unknown-tool paths return structured errors.

## V1R-122 — Prompt/data boundary tests

- [x] Memory Off yields no memory facts for model prompt context; re-enabling restores them (`ambient_prompt_memories_obey_memory_setting_and_restore_on_reenable`).
- [x] Active-app observation Off fails closed before ambient model work (`observer_categories_fail_closed_when_privacy_settings_are_off`).
- [x] Transcript retention Off writes no records (`transcript_retention_off_writes_no_records`).
- [x] Moonshine modes never call the provider raw-audio upload API (`moonshine_mode_never_calls_provider_audio_upload_api`).
- [x] Window-title observation remains unsupported in V1 even if a legacy setting is true, and ambient privacy rejects WindowTitle events.
- [x] Prompt builder hard bounds memory, desktop-observation, event, rules, and final prompt sizes (`oversized_context_is_bounded_with_stable_first_in_truncation`, `ambient_event_is_bounded_in_both_context_and_task`).

## V1R-123 — Logging privacy suite

- [x] Central secret redaction covers plaintext, bearer/header-like, JSON, and query-string appearances.
- [x] Tool audit records contain no raw arguments/results and are bounded.
- [x] Normal tool logs are regression-tested against transcript, system-prompt, API-secret, memory-fact, window-title, active-app, and raw-audio/base64 sentinels (`private_tool_payloads_and_unknown_names_never_enter_normal_logs`).
- [x] Ambient policy diagnostics carry reason/category metadata rather than event-summary text (`policy_diagnostics_do_not_carry_event_summary`).
- [x] Provider/session logging uses structured error categories rather than transcript/audio payload bodies in the covered normal paths.

## V1R-124 — Offline/default-test guarantee

- [x] The live Gemini integration test remains `#[ignore]` and requires explicit `TALKING_MOOSE_ALLOW_LIVE_API=1` plus a dedicated key.
- [x] Ordinary CI now explicitly sets `TALKING_MOOSE_ALLOW_LIVE_API=0` globally so inherited runner configuration cannot opt into the live test.
- [x] Provider network failure is exercised with deterministic fake providers rather than external networking (`failed_provider_connect_never_becomes_active`).
- [x] Moonshine verification/missing-model tests perform no network I/O; installer tests use fixture transports rather than production model downloads.
- [x] Explicit mock microphone and P12 mock playback allow lifecycle/stress tests to run without physical microphone/output hardware; production constructors remain real-device paths.

## V1R-125 — Audio/session stress tests

- [x] Repeated transactional start/stop is exercised for 16 hardware-free cycles, asserting every provider session closes and capture returns inactive (`repeated_start_stop_cycles_are_hardware_free_and_release_each_session`).
- [x] Concurrent teardown requests are serialized through the conversation operation lock and close the provider exactly once (`concurrent_stop_requests_are_serialized_and_close_provider_once`). Existing mute and dismiss IPC tests prove both user actions route through that same teardown primitive.
- [x] Barge-in is now exercised with a non-empty playback queue and verifies immediate queue/level/play-state reset while the current local ASR remains alive (`barge_in_keeps_current_local_asr_active`).
- [x] Provider close/error while capture is active converges through centralized shutdown (`provider_closed_event_loop_converges_through_centralized_cleanup`, `provider_error_event_loop_converges_to_failed_cleanup`).
- [x] Playback, microphone, and Moonshine ingress overload policies are hard bounded with drop/flush diagnostics tests.
- [x] Synthetic runtime microphone/output device failure updates state deterministically without hardware.
- [x] Moonshine stop/cancellation discards queued audio, prevents reuse, and stops lifecycle resources exactly once in the existing pipeline/engine tests.

## Acceptance gate

P12 may be promoted from closure candidate to accepted only after the published commit passes the repository CI gates, especially:

- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features`
- ordinary frontend quality/build and dependency-audit jobs
- macOS Tauri bundle smoke jobs
