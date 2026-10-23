#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_root/src-tauri/Cargo.toml"
export TALKING_MOOSE_ALLOW_LIVE_API=0
export CARGO_NET_OFFLINE=true

cases=(
  "no microphone|audio::capture::tests::missing_default_microphone_fails_closed_without_hardware"
  "microphone permission denied/not requested|audio::capture::tests::microphone_permission_states_fail_closed_without_touching_hardware"
  "selected input/output missing|app::settings_policy::tests::selected_device_must_come_from_current_enumeration"
  "output start failure|conversation::session::diagnostics_tests::audio_output_start_failure_closes_provider_before_microphone_capture"
  "output runtime failure|audio::playback::tests::runtime_output_failure_stops_playback_and_records_bounded_diagnostics"
  "Whisper error taxonomy|asr::whisper::installer::tests::public_error_mapping_covers_all_whisper_engine_failure_classes"
  "Whisper model missing|conversation::session::tests::lifecycle::missing_whisper_model_fails_before_provider_connect_or_microphone_capture"
  "Whisper corrupt model|conversation::session::tests::lifecycle::corrupt_whisper_model_fails_before_provider_connect_or_microphone_capture"
  "Whisper manifest provenance|asr::whisper::manifest::tests::whisper_pins_are_fixed"
  "model download interrupted|asr::whisper::installer::tests::transport_failures_clean_staging_and_preserve_error_categories"
  "model checksum mismatch|asr::whisper::installer::tests::size_and_sha256_failures_never_promote_fixture_artifacts"
  "model out of disk|asr::whisper::installer::tests::insufficient_space_fails_before_transport_starts"
  "Google auth failure|app::state::tests::app_state_runtime::configured_google_live_without_secret_fails_auth_instead_of_using_fake_provider"
  "Google quota/network categories|ai::google::tts::tests::http_errors_map_to_structured_safe_provider_categories"
  "Google protocol failure|ai::google::live::tests::malformed_known_frame_is_protocol_error_without_private_payload"
  "provider disconnect|conversation::session::tests::lifecycle::provider_closed_event_loop_converges_through_centralized_cleanup"
  "database failure|app::state::tests::app_state_runtime::persistent_database_init_failure_never_falls_back_to_memory"
  "secret-store failure|app::state::tests::app_state_runtime::failing_secure_store_during_legacy_migration_does_not_abort_startup"
  "tool permission denial|tools::router::tests::router_enforces_privacy_schema_and_memory_opt_in"
  "tool timeout|tools::execution::tests::pending_async_worker_obeys_the_same_timeout_contract"
  "network denial/text|ai::google::text::tests::network_denial_harness_blocks_text_before_http_send"
  "network denial/TTS|ai::google::tts::tests::network_denial_harness_blocks_tts_before_http_send"
  "network denial/Live|ai::google::live::lifecycle_tests::network_denial_harness_blocks_live_before_websocket_connect"
  "ordinary Whisper verification never downloads|commands::asr_models::tests::whisper_descriptor_reports_missing_model_without_fabricating_installation"
  "standalone queue overload|audio::speech::tests::standalone_speech_queue_overload_is_bounded_and_reported"
)

list_file="$(mktemp)"
trap 'rm -f "$list_file"' EXIT
cargo test --offline --manifest-path "$manifest" --lib --all-features -- --list >"$list_file"

for entry in "${cases[@]}"; do
  scenario="${entry%%|*}"
  test_name="${entry#*|}"
  if ! grep -Fqx "$test_name: test" "$list_file"; then
    printf 'backend_failure_matrix: missing test for %s: %s\n' "$scenario" "$test_name" >&2
    exit 1
  fi
done

if [[ "${TALKING_MOOSE_MATRIX_VERIFY_ONLY:-0}" == "1" ]]; then
  printf 'backend_failure_matrix: verified all %d scenario tests are present; complete Rust test suite owns execution\n' "${#cases[@]}"
  exit 0
fi

for entry in "${cases[@]}"; do
  scenario="${entry%%|*}"
  test_name="${entry#*|}"
  printf 'backend_failure_matrix: %s -> %s\n' "$scenario" "$test_name"
  cargo test --offline --manifest-path "$manifest" --lib --all-features \
    "$test_name" -- --exact
done

printf 'backend_failure_matrix: all %d scenarios passed\n' "${#cases[@]}"
