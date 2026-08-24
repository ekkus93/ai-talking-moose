#!/usr/bin/env bash
set -euo pipefail

app_path="${1:-}"
expected_commit="${2:-}"

fail() {
  printf 'verify_app_build_provenance: %s\n' "$*" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "this verifier must execute on macOS"
[[ -n "$app_path" ]] || fail "usage: $0 /path/to/Talking\\ Moose\\ AI.app EXPECTED_FULL_GIT_SHA"
[[ -n "$expected_commit" ]] || fail "expected full Git SHA is required"
expected_commit="$(printf '%s' "$expected_commit" | tr '[:upper:]' '[:lower:]')"
[[ "$expected_commit" =~ ^[0-9a-f]{40}$ ]] || fail "expected commit must be a full 40-character Git SHA"
[[ -d "$app_path" ]] || fail "app bundle not found: $app_path"
[[ -x /usr/libexec/PlistBuddy ]] || fail "/usr/libexec/PlistBuddy is required"
command -v shasum >/dev/null 2>&1 || fail "shasum is required"

info_plist="$app_path/Contents/Info.plist"
[[ -f "$info_plist" ]] || fail "app bundle is missing Contents/Info.plist"

plist_value() {
  /usr/libexec/PlistBuddy -c "Print :$1" "$info_plist" 2>/dev/null
}

bundle_id="$(plist_value CFBundleIdentifier)"
bundle_version="$(plist_value CFBundleShortVersionString)"
bundle_executable="$(plist_value CFBundleExecutable)"
[[ "$bundle_id" == "com.talkingmoose.ai" ]] || fail "unexpected bundle identifier '$bundle_id'"
[[ -n "$bundle_version" ]] || fail "bundle version is missing"
[[ -n "$bundle_executable" ]] || fail "CFBundleExecutable is missing"

executable_path="$app_path/Contents/MacOS/$bundle_executable"
[[ -x "$executable_path" ]] || fail "bundle executable is missing or not executable: $executable_path"

build_info="$("$executable_path" --build-info)" || fail "bundle executable did not return build provenance"
app_commit="$(printf '%s\n' "$build_info" | awk -F= '$1 == "build-commit" {print $2; exit}')"
app_version="$(printf '%s\n' "$build_info" | awk -F= '$1 == "version" {print $2; exit}')"
app_commit="$(printf '%s' "$app_commit" | tr '[:upper:]' '[:lower:]')"
[[ "$app_commit" =~ ^[0-9a-f]{40}$ ]] || fail "bundle build provenance is missing or invalid: '$app_commit'"
[[ "$app_commit" == "$expected_commit" ]] || fail "bundle was built from $app_commit, expected $expected_commit"
[[ "$app_version" == "$bundle_version" ]] || fail "binary version '$app_version' does not match bundle version '$bundle_version'"

executable_sha256="$(shasum -a 256 "$executable_path" | awk '{print $1}')"
[[ "$executable_sha256" =~ ^[0-9a-f]{64}$ ]] || fail "unable to hash bundle executable"

printf 'bundle-id=%s\n' "$bundle_id"
printf 'bundle-version=%s\n' "$bundle_version"
printf 'bundle-executable=%s\n' "$bundle_executable"
printf 'build-commit=%s\n' "$app_commit"
printf 'executable-sha256=%s\n' "$executable_sha256"
