#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_root/src-tauri/Cargo.toml"
export TALKING_MOOSE_ALLOW_LIVE_API=0

# Dependencies are fetched/compiled by the ordinary Rust quality gate before this
# suite runs in CI. Keep Cargo itself offline here so the failure matrix cannot
# accidentally turn a missing cache into network activity.
export CARGO_NET_OFFLINE=true

cases=(
  "no microphone|audio::capture::tests::missing_default_microphone_fails_closed_without_hardware"
  "microphone permission denied/not requested|audio::capture::tests::microphone_permission_states_fail_closed_without_touching_hardware"
  "selected input/output missing|app::settings_policy::tests::selected_device_must_come_from_current_enumeration"
  "output runtime failure|audio::playback::tests::runtime_output_failure_stops_playback_and_records_bounded_diagnostics"
  "Moonshine runtime unavailable|asr::moonshine::engine::tests::native_load_error_is_typed_and_not_replaced_with_fallback"
  "Moonshine corrupt model|asr::moonshine::engine::tests::production_open_reports_corrupt_install_without_native_fallback"
  "model download interrupted|asr::moonshine::installer::tests::interrupted_download_cleans_partial_and_can_be_retried_from_scratch"
  "model checksum mismatch|asr::moonshine::installer::tests::sha256_mismatch_cleans_staging_and_preserves_existing_install"
  "model out of disk|asr::moonshine::installer::tests::insufficient_disk_space_fails_before_network"
  "Google auth failure|app::state::tests::configured_google_live_without_secret_fails_auth_instead_of_using_fake_provider"
  "Google quota/network categories|ai::google::tts::tests::http_errors_map_to_structured_safe_provider_categories"
  "Google protocol failure|ai::google::live::tests::malformed_known_frame_is_protocol_error_without_private_payload"
  "provider disconnect|conversation::session::tests::provider_closed_event_loop_converges_through_centralized_cleanup"
  "database failure|app::state::tests::persistent_database_init_failure_never_falls_back_to_memory"
  "secret-store failure|app::state::tests::failing_secure_store_during_legacy_migration_does_not_abort_startup"
  "tool permission denial|tools::router::tests::router_enforces_privacy_schema_and_memory_opt_in"
  "tool timeout|tools::router::tests::timeout_wrapper_returns_structured_timeout"
  "network denial/text|ai::google::text::tests::network_denial_harness_blocks_text_before_http_send"
  "network denial/TTS|ai::google::tts::tests::network_denial_harness_blocks_tts_before_http_send"
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
  printf 'backend_failure_matrix: %s -> %s\n' "$scenario" "$test_name"
  cargo test --offline --manifest-path "$manifest" --lib --all-features \
    "$test_name" -- --exact
done

printf 'backend_failure_matrix: all %d scenarios passed\n' "${#cases[@]}"
