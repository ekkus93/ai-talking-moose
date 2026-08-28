#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_root/src-tauri/Cargo.toml"
export TALKING_MOOSE_ALLOW_LIVE_API=0
export CARGO_NET_OFFLINE=true

cases=(
  "mute/dismiss race|conversation::session::diagnostics_tests::mute_and_dismiss_teardown_win_races_against_in_flight_start"
  "microphone queue overload|audio::capture::tests::microphone_queue_overload_drops_newest_and_counts_drop"
  "playback queue overload|audio::playback::tests::playback_queue_has_hard_limit_and_drops_newest_tail"
  "Moonshine ingress overload|asr::pipeline::tests::ingress_is_hard_bounded_and_drops_newest"
  "standalone speech overload|audio::speech::tests::standalone_speech_queue_overload_is_bounded_and_reported"
  "runtime microphone device error|audio::capture::tests::runtime_input_failure_stops_capture_and_records_diagnostics"
  "runtime output device error|audio::playback::tests::runtime_output_failure_stops_playback_and_records_bounded_diagnostics"
  "application shutdown|conversation::session::tests::application_shutdown_is_idempotent_and_closes_backend_resources"
)

list_file="$(mktemp)"
trap 'rm -f "$list_file"' EXIT
cargo test --offline --manifest-path "$manifest" --lib --all-features -- --list >"$list_file"

for entry in "${cases[@]}"; do
  scenario="${entry%%|*}"
  test_name="${entry#*|}"
  if ! grep -Fqx "$test_name: test" "$list_file"; then
    printf 'backend_stress_matrix: missing test for %s: %s\n' "$scenario" "$test_name" >&2
    exit 1
  fi
  printf 'backend_stress_matrix: %s -> %s\n' "$scenario" "$test_name"
  cargo test --offline --manifest-path "$manifest" --lib --all-features \
    "$test_name" -- --exact
done

printf 'backend_stress_matrix: all %d scenarios passed\n' "${#cases[@]}"
