#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_root/src-tauri/native/moonshine-runtime.json"
app_path="${1:-}"
arch="${2:-$(uname -m)}"
require_signature="${3:-}"

fail() {
  printf 'verify_macos_bundle: %s\n' "$*" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "this script must run on macOS"
[[ -n "$app_path" && -d "$app_path" ]] || fail "usage: $0 /path/to/App.app [arm64|x86_64] [--require-signature]"
case "$arch" in arm64|x86_64) ;; *) fail "unsupported architecture: $arch" ;; esac

IFS=$'\t' read -r ort_version max_macos_target < <(python3 - "$manifest" "$arch" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
entry = data["macos"][sys.argv[2]]
print(f"{data['onnxruntime']['version']}\t{entry['minimum_macos']}")
PY
)
[[ -n "$ort_version" && -n "$max_macos_target" ]] || fail "invalid native runtime provenance manifest"
frameworks="$app_path/Contents/Frameworks"
moonshine="$frameworks/libmoonshine.dylib"
ort="$frameworks/libonnxruntime.$ort_version.dylib"
[[ -f "$moonshine" ]] || fail "bundle is missing $moonshine"
[[ -f "$ort" ]] || fail "bundle is missing $ort"

executables=()
while IFS= read -r executable_path; do
  executables+=("$executable_path")
done < <(find "$app_path/Contents/MacOS" -maxdepth 1 -type f -perm -111 -print)
[[ ${#executables[@]} -eq 1 ]] || fail "expected exactly one executable under Contents/MacOS"
executable="${executables[0]}"

developer_load_paths() {
  local binary="$1"
  {
    # Exclude otool's first-line filename header; only dependency install names
    # and LC_RPATH entries participate in runtime dynamic-library resolution.
    otool -L "$binary" | awk 'NR > 1 { print $1 }'
    otool -l "$binary" | awk '
      $1 == "cmd" && $2 == "LC_RPATH" { want_path = 1; next }
      want_path && $1 == "path" { print $2; want_path = 0 }
    '
  } | grep -E '/opt/homebrew|/usr/local|/Users/|/private/tmp|/var/folders' || true
}

for binary in "$executable" "$moonshine" "$ort"; do
  archs="$(lipo -archs "$binary")"
  [[ " $archs " == *" $arch "* ]] || fail "$binary does not contain architecture $arch"
  bad_load_paths="$(developer_load_paths "$binary")"
  if [[ -n "$bad_load_paths" ]]; then
    printf '%s\n' "$bad_load_paths" >&2
    fail "$binary contains a developer-machine load path"
  fi
done

# Every shipped Mach-O must remain compatible with the deployment floor pinned in
# the native runtime provenance manifest. Support both modern LC_BUILD_VERSION and
# older LC_VERSION_MIN_MACOSX load commands.
for binary in "$executable" "$moonshine" "$ort"; do
  min_version="$(otool -l "$binary" | awk '
    $1 == "cmd" && $2 == "LC_BUILD_VERSION" { build = 1; legacy = 0; next }
    $1 == "cmd" && $2 == "LC_VERSION_MIN_MACOSX" { legacy = 1; build = 0; next }
    build && $1 == "minos" { print $2; exit }
    legacy && $1 == "version" { print $2; exit }
  ')"
  [[ -n "$min_version" ]] || fail "could not determine deployment target for $binary"
  python3 - "$binary" "$min_version" "$max_macos_target" <<'PY2'
import sys
path, actual, maximum = sys.argv[1:]
def parts(value):
    pieces = [int(piece) for piece in value.split('.')]
    return tuple((pieces + [0, 0, 0])[:3])
if parts(actual) > parts(maximum):
    raise SystemExit(f"{path} requires macOS {actual}, above supported {maximum} for this architecture")
PY2
done

otool -L "$executable" | grep -F '@rpath/libmoonshine.dylib' >/dev/null \
  || fail "application executable does not load Moonshine through @rpath"
otool -L "$moonshine" | grep -F "@rpath/libonnxruntime.$ort_version.dylib" >/dev/null \
  || fail "Moonshine does not load ONNX Runtime through @rpath"

notice_root="$app_path/Contents/Resources/native/macos/notices"
[[ -f "$notice_root/TALKING_MOOSE_LICENSE" ]] || fail "Talking Moose project license is missing from bundle"
[[ -f "$notice_root/THIRD_PARTY_NOTICES.md" ]] || fail "third-party notice inventory is missing from bundle"
license_count="$(find "$notice_root/MoonshineRuntime" -type f 2>/dev/null | wc -l | tr -d ' ')"
(( license_count >= 5 )) || fail "bundled native notice set is incomplete"

local_llm_notice="$notice_root/LocalLlmRuntime/LLAMA_CPP_LICENSE"
local_llm_binding_notice="$notice_root/LocalLlmRuntime/LLAMA_CPP_RS_LICENSE_MIT"
local_llm_readme="$notice_root/LocalLlmRuntime/README.md"
dependency_inventory="$notice_root/Dependencies/DEPENDENCY_LICENSES.md"
[[ -f "$local_llm_notice" ]] || fail "llama.cpp native runtime license is missing from bundle"
[[ -f "$local_llm_binding_notice" ]] || fail "llama-cpp-rs binding license is missing from bundle"
[[ -f "$local_llm_readme" ]] || fail "Local LLM native runtime notice metadata is missing from bundle"
[[ -f "$dependency_inventory" ]] || fail "dependency license inventory is missing from bundle"
grep -F '| cargo | `llama-cpp-2` | `0.1.154` |' "$dependency_inventory" >/dev/null \
  || fail "llama-cpp-2 is missing from bundled dependency license inventory"
grep -F '| cargo | `llama-cpp-sys-2` | `0.1.154` |' "$dependency_inventory" >/dev/null \
  || fail "llama-cpp-sys-2 is missing from bundled dependency license inventory"
if grep -q '\*\*MISSING\*\*' "$dependency_inventory"; then
  fail "dependency license inventory contains missing notice evidence"
fi

if find "$app_path" -type f -iname '*.gguf' -print -quit | grep -q .; then
  fail "bundle contains GGUF model weights; Local LLM weights must remain external"
fi

smoke_output="$(
  env -i \
    HOME="${HOME:-/tmp}" \
    PATH='/usr/bin:/bin:/usr/sbin:/sbin' \
    "$executable" --moonshine-native-smoke-test
)"
printf '%s\n' "$smoke_output"
grep -E '^moonshine-runtime-version=[0-9]+$' <<<"$smoke_output" >/dev/null \
  || fail "bundled executable failed Moonshine native runtime smoke check"

# The linker can leave an ad-hoc signature on Contents/MacOS/<binary> even when
# the .app bundle itself has never been resource-sealed. `codesign -dv App.app`
# may therefore succeed for an otherwise unsigned bundle. Treat the bundle as
# signed only when the resource seal exists; always verify the staged native
# dylib signatures created by prepare_moonshine_macos.sh.
codesign --verify --strict "$moonshine"
codesign --verify --strict "$ort"

bundle_seal="$app_path/Contents/_CodeSignature/CodeResources"
if [[ -f "$bundle_seal" ]]; then
  codesign --verify --strict --deep "$app_path"
elif [[ "$require_signature" == '--require-signature' ]]; then
  fail "signed bundle required, but application has no resource-sealed signature"
else
  printf 'Bundle has no resource-sealed signature; native dylib signatures verified for smoke CI.\n'
fi

printf 'Verified self-contained Moonshine runtime bundle for macOS %s.\n' "$arch"
