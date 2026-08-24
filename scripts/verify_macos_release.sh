#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_path="${1:-}"
dmg_path="${2:-}"
arch="${3:-$(uname -m)}"
expected_commit="${4:-}"

fail() {
  printf 'verify_macos_release: %s\n' "$*" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "this script must run on macOS"
[[ -n "$app_path" && -d "$app_path" ]] || fail "usage: $0 /path/to/App.app /path/to/App.dmg [arm64|x86_64] [EXPECTED_FULL_GIT_SHA]"
[[ -n "$dmg_path" && -f "$dmg_path" ]] || fail "DMG does not exist: $dmg_path"
case "$arch" in arm64|x86_64) ;; *) fail "unsupported architecture: $arch" ;; esac

if [[ -n "$expected_commit" ]]; then
  bash "$repo_root/scripts/verify_app_build_provenance.sh" "$app_path" "$expected_commit"
fi

bash "$repo_root/scripts/verify_macos_bundle.sh" "$app_path" "$arch" --require-signature

signature_details="$(codesign -dvvv "$app_path" 2>&1)"
printf '%s\n' "$signature_details"
grep -E 'Authority=Developer ID Application:' <<<"$signature_details" >/dev/null \
  || fail "application is not signed with a Developer ID Application certificate"
grep -E 'flags=.*runtime' <<<"$signature_details" >/dev/null \
  || fail "application signature does not enable the hardened runtime"
grep -E '^Timestamp=' <<<"$signature_details" >/dev/null \
  || fail "application signature has no secure timestamp"

spctl -a -t exec -vv "$app_path"
xcrun stapler validate "$app_path"

# A direct-download DMG is the primary V1 distribution artifact. Require its own
# Developer ID signature and secure timestamp before validating the stapled ticket.
codesign --verify --strict --verbose=2 "$dmg_path"
dmg_signature_details="$(codesign -dvvv "$dmg_path" 2>&1)"
printf '%s\n' "$dmg_signature_details"
grep -E 'Authority=Developer ID Application:' <<<"$dmg_signature_details" >/dev/null \
  || fail "DMG is not signed with a Developer ID Application certificate"
grep -E '^Timestamp=' <<<"$dmg_signature_details" >/dev/null \
  || fail "DMG signature has no secure timestamp"
xcrun stapler validate "$dmg_path"
spctl -a -t open --context context:primary-signature -v "$dmg_path"

# Guard against accidental development material in the bundle.
if find "$app_path" -type f \( \
  -name '.env' -o \
  -name '*.sqlite' -o \
  -name '*.db' -o \
  -name '*.pem' -o \
  -name '*.p12' -o \
  -name 'AuthKey_*.p8' \
\) -print -quit | grep -q .; then
  fail "bundle contains a development secret/database-shaped file"
fi

notice_root="$app_path/Contents/Resources/native/macos/notices"
[[ -f "$notice_root/TALKING_MOOSE_LICENSE" ]] || fail "project license is not bundled"
[[ -f "$notice_root/Dependencies/DEPENDENCY_LICENSES.md" ]] || fail "dependency license inventory is not bundled"
if grep -q '\*\*MISSING\*\*' "$notice_root/Dependencies/DEPENDENCY_LICENSES.md"; then
  fail "dependency license inventory contains missing notice files"
fi

printf 'Verified signed, hardened, notarized macOS %s release artifacts.\n' "$arch"
