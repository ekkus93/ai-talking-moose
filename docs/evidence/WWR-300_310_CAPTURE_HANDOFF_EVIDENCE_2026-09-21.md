# WWR-300 / WWR-310 capture and handoff evidence

Date: 2026-09-21
Baseline master: `d4dfdc835ce1deaa4f7a28d9a2db7dc1ecab44f7`
Baseline merged-master CI: `35645739218` (success)

This matrix records implementation already present on master so the remediation TODO can be reconciled without treating component evidence as broader production acceptance.

## WWR-300 authoritative capture path

Production application state in `src-tauri/src/app/state.rs` owns exactly one `Arc<Mutex<AudioCapture>>` and one `WakeWordApplicationRuntime`. `WakeWordApplicationRuntime` deliberately owns no microphone stream.

`src-tauri/src/app/wake_word_capture_orchestrator.rs` routes Wake Word through a borrowed `&mut AudioCapture`; it never constructs a competing physical capture owner. Its tests objectively demonstrate:

- `start_uses_the_existing_capture_owner_at_the_canonical_rate`: the existing owner is opened at the V1 canonical rate.
- `replacing_orchestrator_reuses_same_capture_owner_instead_of_multiplying_owners`: replacement reuses the same capture owner rather than creating another owner.
- `disable_stops_single_capture_owner_and_clears_wake_state`: disable stops that owner and clears retained Wake state.
- `command_return_restarts_capture_through_same_owner`: command return reopens through the same owner and returns the runtime to Listening after resetting KWS.
- `closed_capture_queue_fails_wake_runtime_closed_without_reopening_capture`: capture closure enters sanitized Error and clears retained audio without an implicit retry/open loop.
- `reconnect_reuses_same_capture_owner_and_returns_error_runtime_to_listening`: explicit reconnect reuses the same owner and recovers Error -> Loading -> Listening.

`src-tauri/src/app/wake_word_pcm_router.rs` routes the same canonical sample slice to ring retention and KWS in serial chronological order, and preserves post-trigger chunks in the live handoff instead of re-feeding KWS.

These are objective component/ownership facts. They do not by themselves prove that the production application startup currently constructs and drives `WakeCaptureOrchestrator`; that production-composition item remains open until an executable production path is identified and qualified.

## WWR-310 wake -> command ASR handoff

The current handoff implementation has deterministic evidence for most sample-integrity requirements:

- `WakeWordRuntimeManager::accept_trigger` snapshots the bounded ring chronologically at the accepted trigger.
- `CanonicalWakePcmRouter` creates `WakeAsrHandoff` from that snapshot and appends subsequent canonical chunks as live handoff audio while KWS is no longer fed.
- `handoff_audio_preserves_wake_phrase_tail_and_first_command_word_contiguously` proves synthetic wake-phrase and immediate-command samples remain contiguous and ordered.
- `WakeCommandHandoffAudio` performs no V1 acoustic trimming and preserves exact sample order.
- `WakeCommandAsrHandoff::deliver_once` is single-use and feeds the provider-neutral payload to the selected command-ASR ingress exactly once.
- `synthetic_snapshot_live_boundary_is_exact_without_gap_duplicate_or_inversion` proves a monotonic pre-roll/live boundary arrives at ingress without a gap, duplicate range, or inversion.
- `failed_startup_consumes_payload_instead_of_replaying_stale_audio` proves failed ASR startup cannot replay stale handoff audio on a later attempt.
- `CanonicalWakePcmRouter::clear_handoff` and return-to-listening paths clear stale handoff state; the return path resets KWS before resuming.
- `LocalAsrPipeline` implements `WakeCommandAsrIngress` through `prime_wake_handoff`, using the existing local command-ASR pipeline rather than creating a second ASR pipeline or microphone stream.

This evidence supports deterministic component-level reconciliation for chronological snapshot/live retention, no trimming, exact sample-order boundary behavior, single-use transfer, and stale-handoff clearing. It does not satisfy the real/reproducible `Hey Moose, tell me the time` acceptance, nor does it prove that every production ASR mode is wired through this handoff from the physical microphone. Those acceptance items remain open.

## Qualification boundary

The baseline master passed ordinary CI run `35645739218`. Earlier implementation slices were also individually exact-head qualified before merge. This document intentionally makes no real-audio, Linux/macOS native KWS, specialized-runner, performance, or end-to-end production-startup claim.
