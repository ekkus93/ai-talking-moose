#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work_root="${1:-${TMPDIR:-/tmp}/talking-moose-p2-p3-physical-$(date +%Y%m%d-%H%M%S)}"
report="$work_root/P2_P3_PHYSICAL_ACCEPTANCE.md"
audio_inventory="$work_root/macos-audio-inventory.txt"

fail() {
  printf 'run_p2_p3_macos_physical_acceptance: %s\n' "$*" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "this acceptance runner must execute on macOS"
command -v git >/dev/null 2>&1 || fail "git is required to identify the tested commit"
command -v system_profiler >/dev/null 2>&1 || fail "system_profiler is required"

mkdir -p "$work_root"

commit_sha="$(git -C "$repo_root" rev-parse HEAD)"
[[ -n "$commit_sha" ]] || fail "unable to determine repository commit"
if [[ -n "$(git -C "$repo_root" status --porcelain --untracked-files=no)" ]]; then
  fail "tracked repository files are dirty; run acceptance against an exact commit"
fi

macos_version="$(sw_vers -productVersion)"
macos_build="$(sw_vers -buildVersion)"
hardware_model="$(sysctl -n hw.model 2>/dev/null || printf 'unknown')"
arch="$(uname -m)"
cpu_brand="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || true)"
if [[ -z "$cpu_brand" ]]; then
  cpu_brand="$(system_profiler SPHardwareDataType 2>/dev/null | awk -F': ' '/Chip:|Processor Name:/ {print $2; exit}')"
fi
[[ -n "$cpu_brand" ]] || cpu_brand="unknown"

system_profiler SPAudioDataType > "$audio_inventory"

printf '\nTalking Moose P2/P3 supported-Mac physical acceptance\n'
printf 'Commit: %s\n' "$commit_sha"
printf 'macOS: %s (%s)\n' "$macos_version" "$macos_build"
printf 'Hardware: %s / %s / %s\n' "$hardware_model" "$cpu_brand" "$arch"
printf 'Evidence directory: %s\n\n' "$work_root"
printf 'Keep the packaged Talking Moose application and its Diagnostics/Privacy settings available while answering.\n'
printf 'For the clean permission test, you may need: tccutil reset Microphone com.talkingmoose.ai\n'
printf 'This runner will NOT execute that command for you.\n\n'

escape_md() {
  printf '%s' "$1" | tr '\n\r' '  ' | sed 's/|/\\|/g'
}

prompt_result() {
  local id="$1"
  local title="$2"
  local instructions="$3"
  local result notes

  printf '\n=== %s — %s ===\n%s\n' "$id" "$title" "$instructions"
  while true; do
    read -r -p 'Result [PASS/FAIL/SKIP]: ' result
    result="$(printf '%s' "$result" | tr '[:lower:]' '[:upper:]')"
    case "$result" in
      PASS|FAIL|SKIP) break ;;
      *) printf 'Enter PASS, FAIL, or SKIP.\n' ;;
    esac
  done
  read -r -p 'Evidence/notes: ' notes
  printf '| %s | %s | %s | %s |\n' \
    "$(escape_md "$id")" \
    "$(escape_md "$title")" \
    "$(escape_md "$result")" \
    "$(escape_md "$notes")" >> "$report"
}

cat > "$report" <<EOF_REPORT
# P2/P3 supported-Mac physical acceptance report

- Tested commit: \`$commit_sha\`
- macOS: $macos_version ($macos_build)
- Hardware model: $hardware_model
- CPU/SoC: $cpu_brand
- Architecture: $arch
- Audio inventory: \`$(basename "$audio_inventory")\`
- Generated: $(date -u +'%Y-%m-%dT%H:%M:%SZ')

| ID | Check | Result | Evidence / notes |
| --- | --- | --- | --- |
EOF_REPORT

prompt_result "V1R-020" "single production speech owner" \
  "Exercise conversation plus standalone/canned or ambient speech. Confirm one audible speech path only, then Stop/Dismiss and confirm no second engine continues."
prompt_result "V1R-021-44100" "44.1 kHz real output" \
  "Use a real output configuration that Diagnostics reports as 44.1 kHz. Run the generated tone and normal speech; verify correct pitch/speed and truthful device/rate."
prompt_result "V1R-021-48000" "48 kHz real output" \
  "Use a real output configuration that Diagnostics reports as 48 kHz. Run the generated tone and normal speech; verify correct pitch/speed and truthful device/rate."
prompt_result "V1R-022" "real CPAL output format" \
  "Record the real output sample format/channels from Diagnostics and verify generated-tone plus speech playback without silent device substitution."
prompt_result "V1R-023" "bounded overload and stale-audio barge-in" \
  "Create queued/long speech. Confirm queue remains within its hard limit. Barge in while audio is queued; verify immediate queue/level reset and that old audio never resumes. Record dropped-sample behavior if exercised."
prompt_result "V1R-024" "truthful microphone failures" \
  "Exercise denied permission, stale selected/disconnected USB microphone, and runtime disconnect where practical. Verify explicit errors, no silent device substitution, and inactive capture after failure."
prompt_result "V1R-025-BUILTIN" "built-in microphone" \
  "Select the built-in microphone; run local mic diagnostics and a normal conversation. Record actual device/rate/format/channels and verify intelligible input."
prompt_result "V1R-025-USB" "USB microphone" \
  "Select a real USB microphone; run diagnostics and conversation, then disconnect it and verify the selected device is not silently replaced."
prompt_result "V1R-026-NOTREQUESTED" "clean-profile not-requested state" \
  "After resetting TCC for com.talkingmoose.ai, confirm Privacy reports not_requested and normal conversation start does not accidentally trigger the macOS prompt."
prompt_result "V1R-026-DENIED" "explicit prompt then denial" \
  "Request microphone permission only from the explicit Settings action, deny it, and confirm denied guidance with no repeated automatic prompt."
prompt_result "V1R-026-GRANTLATER" "grant later in System Settings" \
  "Grant microphone access in macOS System Settings, return to the app, Refresh status, and verify granted state plus successful real capture."
prompt_result "V1R-027" "real diagnostics UI" \
  "Verify configured/actual devices, rate/format/channels, mic active/input level, output level, queue depth/limit, drop counters, last typed errors, mic test, tone test, and active-conversation ownership refusal."
prompt_result "V1R-037" "real barge-in latency" \
  "Measure and record the interval in milliseconds between barge-in and audible playback stop on real hardware. Repeat several times and verify no stale pre-interruption audio returns. No hard SLA is imposed yet; record the measured value/method."

if grep -Eq '\| (FAIL|SKIP) \|' "$report"; then
  status="OPEN"
else
  status="PASS"
fi

cat >> "$report" <<EOF_REPORT

## Gate status

**$status**

The gate is PASS only when every required row above is PASS. This report does not close the separate P1 Keychain restart, P3A native-model/ASR-015 benchmark, P6 voice audition, or P13 release gates.
EOF_REPORT

printf '\nP2_P3_PHYSICAL_ACCEPTANCE_REPORT=%s\n' "$report"
printf 'P2_P3_PHYSICAL_ACCEPTANCE_STATUS=%s\n' "$status"
cat "$report"

[[ "$status" == "PASS" ]] || exit 2
