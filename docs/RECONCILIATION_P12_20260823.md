# Phase P12 reconciliation — 2026-08-23

This overlay reconciles the legacy monolithic TODO without rewriting the large tracker file through the GitHub contents API.

## Accepted closure: V1R-120 through V1R-125

P12 is **accepted complete**. The user reported GitHub Actions run `32635509276` green on 2026-08-23 at master commit `38682fc6c89cc6fa1465d57a6bd70241a71a1fd4`. That head contains the P12 implementation candidate plus the follow-up rustfmt, rusqlite lifetime, and stale diagnostics-test corrections required to restore all repository gates.

Implementation candidate: `6657fd337e99e3e55d8a250b4ce69ac892111925` (`test: harden P12 runtime boundaries`). Accepted CI head: `38682fc6c89cc6fa1465d57a6bd70241a71a1fd4`.

The current sandbox source ZIP predates the already-accepted P11 commits, so P12 work is limited to files verified unchanged between ZIP commit `bae050ce94b9d56943e80b8e8ba3b0f599864f86` and P11-accepted `master` commit `5499bba34b3119a2018e55710c6fe60fe308c53c`, plus this reconciliation overlay.

Original P12 local validation covered `git diff --check`, workflow YAML parsing, and `scripts/validate_moonshine_runtime_manifest.py`. For the 2026-08-24 current-master regression pass, the user-supplied Rust toolchain is mounted: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` and `git diff --check` pass. A focused offline Cargo test cannot resolve the local dependency graph because this sandbox has no cached crates.io package set (`base64` is the first missing package), so GitHub Actions remains authoritative for compilation, Clippy, and Rust tests.

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
- [x] Barge-in playback flush is exercised with a non-empty queue and verifies immediate queue/level/play-state reset plus exactly one provider interrupt (`barge_in_flushes_buffered_output_and_interrupts_once`). Local-ASR continuity is proven separately by `barge_in_keeps_current_local_asr_active`, which asserts the active Moonshine lifecycle is not stopped and its callback remains current.
- [x] Provider close/error while capture is active converges through centralized shutdown (`provider_closed_event_loop_converges_through_centralized_cleanup`, `provider_error_event_loop_converges_to_failed_cleanup`).
- [x] Playback, microphone, and Moonshine ingress overload policies are hard bounded with drop/flush diagnostics tests.
- [x] Synthetic runtime microphone/output device failure updates state deterministically without hardware.
- [x] Moonshine stop/cancellation discards queued audio, prevents reuse, and stops lifecycle resources exactly once in the existing pipeline/engine tests.

## 2026-08-24 current-master regression audit

A current-master audit after P6/P7/P8/P9/P10/P11 confirmed that the accepted failure matrix, prompt/data boundaries, logging privacy suite, offline-default test gate, and audio/session stress coverage remain intact. Ordinary CI still pins `TALKING_MOOSE_ALLOW_LIVE_API=0`, and the live Gemini integration test remains ignored behind explicit opt-in. P9 strengthened local-data deletion and P10 added a hard four-call tool-execution concurrency cap without widening provider authority.

The audit did find two coupled resource-limit regressions introduced by the later desktop-observer runtime:

- [x] macOS sleep/wake notifications no longer enter an unbounded Tokio channel; the observer now uses an eight-event bounded queue and nonblocking `try_send`, so a stalled async consumer cannot create unbounded memory growth or block the IOKit callback thread;
- [x] desktop observations no longer spawn one waiting task per event when the ambient scheduler is saturated; a new nonblocking background-admission path uses the existing bounded ambient queue and drops the newest observation when that queue is full; and
- [x] regression tests prove both the power-event queue capacity and the ambient background-admission hard limit.

This current-master P12 hardening remains **pending GitHub Actions acceptance**. The existing user-facing observation behavior and explicit/user ambient submission semantics are unchanged.

## 2026-08-28 V1R-182 test-integrity correction

The remediation audit corrected two proof-quality defects without changing the accepted P12 behavior:

- the tray regression no longer proves the length of a literal it constructs itself; `tray_action_parser_maps_every_supported_action_and_rejects_unknown_ids` now exercises the production `parse_menu_action` allowlist used by tray dispatch and proves every supported action id maps to the intended effect category while unknown ids fail closed; and
- the V1R-125 barge-in row now cites the production-path test that actually seeds and flushes buffered playback (`barge_in_flushes_buffered_output_and_interrupts_once`) separately from the test that proves local Moonshine ASR remains active (`barge_in_keeps_current_local_asr_active`).

A repository-wide source sweep reviewed Rust `#[test]`/`#[tokio::test]` assertions and frontend Vitest assertions for the same literal-only anti-pattern. The tray test was the only test found that asserted the size/property of a literal assembled solely inside the test. Other count/length assertions found by the sweep are derived from production output (for example resampler output, the serialized TTS voice catalog, rendered select options, persistence query results, or bounded queues) and therefore remain capable of failing when production behavior regresses.

## Acceptance gate

Satisfied by user-reported successful GitHub Actions run `32635509276` at accepted head `38682fc6c89cc6fa1465d57a6bd70241a71a1fd4`, covering:

- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features`
- ordinary frontend quality/build and dependency-audit jobs
- macOS Tauri bundle smoke jobs
