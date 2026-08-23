#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_path="${1:-}"
dmg_path="${2:-}"
report_dir="${3:-${TMPDIR:-/tmp}/talking-moose-p13-release-$(date +%Y%m%d-%H%M%S)}"
report="$report_dir/P13_RELEASE_ACCEPTANCE.md"

fail() { printf 'run_p13_macos_release_acceptance: %s\n' "$*" >&2; exit 1; }
[[ "$(uname -s)" == "Darwin" ]] || fail "this acceptance must run on a real supported Mac"
[[ -n "$app_path" && -d "$app_path" ]] || fail "usage: $0 '/Applications/Talking Moose AI.app' /path/to/release.dmg [report-dir]"
[[ -n "$dmg_path" && -f "$dmg_path" ]] || fail "release DMG does not exist: $dmg_path"
for command in git sw_vers system_profiler shasum; do command -v "$command" >/dev/null || fail "required command not found: $command"; done
[[ -x /usr/libexec/PlistBuddy ]] || fail "required command not found: /usr/libexec/PlistBuddy"
cd "$repo_root"
[[ -z "$(git status --porcelain --untracked-files=no)" ]] || fail "tracked working tree must be clean"
mkdir -p "$report_dir"

commit="$(git rev-parse HEAD)"
version="$(python3 - <<'PY'
import json
print(json.load(open('package.json', encoding='utf-8'))['version'])
PY
)"
bundle_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app_path/Contents/Info.plist" 2>/dev/null || true)"
[[ "$bundle_version" == "$version" ]] || fail "installed app version $bundle_version does not match checkout version $version"

# The automated release verifier is a prerequisite, not a manual checkbox.
bash scripts/verify_macos_release.sh "$app_path" "$dmg_path" "$(uname -m)"
dmg_sha256="$(shasum -a 256 "$dmg_path" | awk '{print $1}')"

cat > "$report" <<EOF
# P13 macOS release acceptance

- Commit: \`$commit\`
- Expected release tag: \`v$version\`
- Installed app: \`$app_path\`
- DMG: \`$dmg_path\`
- DMG SHA-256: \`$dmg_sha256\`
- macOS: \`$(sw_vers -productVersion) ($(sw_vers -buildVersion))\`
- Hardware: \`$(system_profiler SPHardwareDataType | awk -F': ' '/Model Name|Model Identifier|Chip|Processor Name/ {printf "%s=%s; ", $1, $2}' | sed 's/; $//')\`
- Architecture: \`$(uname -m)\`
- Started: \`$(date -u +%Y-%m-%dT%H:%M:%SZ)\`
- Automated signature/notarization/package verification: **PASS**

EOF

prompt_result() {
  local id="$1" text="$2" result notes
  printf '\n[%s] %s\n' "$id" "$text"
  while true; do
    read -r -p 'Result (PASS/FAIL/SKIP): ' result
    result="$(printf '%s' "$result" | tr '[:lower:]' '[:upper:]')"
    case "$result" in PASS|FAIL|SKIP) break ;; *) printf 'Enter PASS, FAIL, or SKIP.\n' ;; esac
  done
  read -r -p 'Evidence/notes: ' notes
  printf -- '- **%s — %s:** %s — %s\n' "$id" "$result" "$text" "$notes" >> "$report"
}

printf 'P13 acceptance requires the exact signed/notarized release candidate.\n'
printf 'Do not mark physical checks PASS from source tests or ordinary CI.\n'

prompt_result 'V1R-130' 'Confirm Finder/System Information shows final Talking Moose AI name/version/icon and the app launches on this supported macOS version.'
prompt_result 'V1R-132' 'Confirm draft release notes/checksums match the downloaded architecture artifact and the bundled license/notices are inspectable.'
prompt_result 'V1R-133' 'Upgrade a representative legacy profile: DB/settings migrate, plaintext key moves to Keychain, legacy ASR routing remains Gemini Live, and no surprise Moonshine download occurs.'
prompt_result 'V1R-134-FIRST-RUN' 'On a clean install: launch, onboarding, secure key restart, and explicit microphone permission flow all work.'
prompt_result 'V1R-134-LOCAL-ASR' 'Fresh profile defaults Tiny; Tiny and Small download/verify/install/transcribe/remove work; packet/log evidence shows Moonshine raw mic audio stays local; missing/corrupt model has no cloud fallback; explicit Gemini Live switch works.'
prompt_result 'V1R-134-AUDIO' 'Input/output selection, rate/pitch, transcripts, mouth animation, barge-in, Stop, Mute shutdown, and Dismiss shutdown work.'
prompt_result 'V1R-134-PRIVACY' 'Quiet hours, talkativeness, active-app Off/On, observer truthfulness, memory Off/On/forget, transcript Off, and clear/reset flows work.'
prompt_result 'V1R-134-FAILURE' 'Invalid key/network, no-device/permission denial, Moonshine missing/corrupt/ABI failure, interrupted/out-of-disk model install, and Google outage leave truthful recoverable UI/settings.'
prompt_result 'V1R-134-PACKAGING' 'Clean-machine DMG install works; packaged Moonshine/ONNX loads without developer paths; Tiny transcribes; no development secrets/logs/test DB are bundled.'

status=PASS
if grep -Eq '\*\*[^*]+ — (FAIL|SKIP):' "$report"; then status=OPEN; fi
cat >> "$report" <<EOF

## Gate decision

**P13 status: $status**

P13 is accepted only when every required row above is PASS and all previously required physical gates for P1/P2/P3/P3A/P6 are also satisfied for the same release candidate.
EOF

printf '\nReport: %s\n' "$report"
cat "$report"
[[ "$status" == PASS ]] || exit 2
