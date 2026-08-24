#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_path="${1:-}"
dmg_path="${2:-}"
report_dir="${3:-${TMPDIR:-/tmp}/talking-moose-p13-release-$(date +%Y%m%d-%H%M%S)}"
report="$report_dir/P13_RELEASE_ACCEPTANCE.md"
checksum_manifest="${P13_SHA256SUMS:-}"

fail() { printf 'run_p13_macos_release_acceptance: %s\n' "$*" >&2; exit 1; }
[[ "$(uname -s)" == "Darwin" ]] || fail "this acceptance must run on a real supported Mac"
[[ -n "$app_path" && -d "$app_path" ]] || fail "usage: P13_SHA256SUMS=/path/to/SHA256SUMS.txt $0 '/Applications/Talking Moose AI.app' /path/to/release.dmg [report-dir]"
[[ -n "$dmg_path" && -f "$dmg_path" ]] || fail "release DMG does not exist: $dmg_path"
[[ -n "$checksum_manifest" && -f "$checksum_manifest" ]] || fail "P13_SHA256SUMS must point to the downloaded combined SHA256SUMS.txt"
for command in git sw_vers system_profiler shasum; do command -v "$command" >/dev/null || fail "required command not found: $command"; done
[[ -x /usr/libexec/PlistBuddy ]] || fail "required command not found: /usr/libexec/PlistBuddy"
cd "$repo_root"
source_status="$(git status --porcelain --untracked-files=normal -- . \
  ':(exclude)node_modules/**' \
  ':(exclude)dist/**' \
  ':(exclude)src-tauri/target/**' \
  ':(exclude)src-tauri/icons/**' \
  ':(exclude)src-tauri/native/macos/**')"
if [[ -n "$source_status" ]]; then
  printf '%s\n' "$source_status" >&2
  fail "source-controlled inputs are dirty; only dependency/generated/build trees may differ during release acceptance"
fi
mkdir -p "$report_dir"

commit="$(git rev-parse HEAD)"
expected_commit="${P13_EXPECTED_COMMIT:-$commit}"
expected_commit="$(printf '%s' "$expected_commit" | tr '[:upper:]' '[:lower:]')"
[[ "$expected_commit" =~ ^[0-9a-f]{40}$ ]] || fail "P13_EXPECTED_COMMIT must be a full 40-character Git SHA"
[[ "$commit" == "$expected_commit" ]] || fail "checkout is $commit, expected $expected_commit"

version="$(python3 - <<'PY'
import json
print(json.load(open('package.json', encoding='utf-8'))['version'])
PY
)"
release_tag="v$version"
tag_commit="$(git rev-parse "${release_tag}^{commit}" 2>/dev/null || true)"
[[ "$tag_commit" == "$expected_commit" ]] || fail "release tag $release_tag does not resolve to expected commit $expected_commit"

bundle_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app_path/Contents/Info.plist" 2>/dev/null || true)"
[[ "$bundle_version" == "$version" ]] || fail "installed app version $bundle_version does not match checkout version $version"

provenance="$(bash scripts/verify_app_build_provenance.sh "$app_path" "$expected_commit")"
printf '%s\n' "$provenance"
executable_sha256_start="$(printf '%s\n' "$provenance" | awk -F= '$1 == "executable-sha256" {print $2; exit}')"
[[ "$executable_sha256_start" =~ ^[0-9a-f]{64}$ ]] || fail "application executable provenance hash is missing"

# The automated release verifier is a prerequisite, not a manual checkbox.
bash scripts/verify_macos_release.sh "$app_path" "$dmg_path" "$(uname -m)" "$expected_commit"
dmg_sha256="$(shasum -a 256 "$dmg_path" | awk '{print $1}')"
dmg_name="$(basename "$dmg_path")"
manifest_sha256="$(awk -v name="$dmg_name" '$2 == name || $2 == "*" name {print $1; exit}' "$checksum_manifest")"
[[ "$manifest_sha256" =~ ^[0-9a-fA-F]{64}$ ]] || fail "combined checksum manifest has no entry for $dmg_name"
manifest_sha256="$(printf '%s' "$manifest_sha256" | tr '[:upper:]' '[:lower:]')"
[[ "$dmg_sha256" == "$manifest_sha256" ]] || fail "DMG checksum does not match combined release manifest"

cat > "$report" <<EOF_REPORT
# P13 macOS release acceptance

- Commit: \`$commit\`
- Release tag: \`$release_tag\`
- Installed app: \`$app_path\`
- App executable SHA-256: \`$executable_sha256_start\`
- DMG: \`$dmg_path\`
- DMG SHA-256: \`$dmg_sha256\`
- Combined checksum manifest: \`$checksum_manifest\`
- Checksum-manifest verification: **PASS**
- macOS: \`$(sw_vers -productVersion) ($(sw_vers -buildVersion))\`
- Hardware: \`$(system_profiler SPHardwareDataType | awk -F': ' '/Model Name|Model Identifier|Chip|Processor Name/ {printf "%s=%s; ", $1, $2}' | sed 's/; $//')\`
- Architecture: \`$(uname -m)\`
- Started: \`$(date -u +%Y-%m-%dT%H:%M:%SZ)\`
- Automated signature/notarization/package/provenance verification: **PASS**

EOF_REPORT

prompt_result() {
  local id="$1" text="$2" result notes
  printf '\n[%s] %s\n' "$id" "$text"
  while true; do
    read -r -p 'Result (PASS/FAIL/SKIP): ' result
    result="$(printf '%s' "$result" | tr '[:lower:]' '[:upper:]')"
    case "$result" in PASS|FAIL|SKIP) break ;; *) printf 'Enter PASS, FAIL, or SKIP.\n' ;; esac
  done
  while true; do
    read -r -p 'Evidence/notes (required): ' notes
    [[ -n "${notes//[[:space:]]/}" ]] && break
    printf 'Evidence/notes cannot be blank.\n'
  done
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

provenance_end="$(bash scripts/verify_app_build_provenance.sh "$app_path" "$expected_commit")"
executable_sha256_end="$(printf '%s\n' "$provenance_end" | awk -F= '$1 == "executable-sha256" {print $2; exit}')"
[[ "$executable_sha256_end" == "$executable_sha256_start" ]] || fail "application executable changed during physical acceptance"

status=PASS
if grep -Eq '\*\*[^*]+ — (FAIL|SKIP):' "$report"; then status=OPEN; fi
cat >> "$report" <<EOF_REPORT

## Gate decision

**P13 status: $status**

P13 is accepted only when every required row above is PASS and all previously required physical gates for P1/P2/P3/P3A/P6 are also satisfied for the same release candidate.
EOF_REPORT

printf '\nReport: %s\n' "$report"
cat "$report"
[[ "$status" == PASS ]] || exit 2
