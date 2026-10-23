#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
config="$repo_root/src-tauri/tauri.conf.json"
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

max_macos_target="$(python3 - "$config" <<'PYCONFIG'
import json, sys
print(json.load(open(sys.argv[1], encoding="utf-8"))["bundle"]["macOS"]["minimumSystemVersion"])
PYCONFIG
)"
[[ -n "$max_macos_target" ]] || fail "missing macOS deployment floor"
if find "$app_path/Contents" \
    \( -iname '*moonshine*' -o -name 'ggml-small.bin' \) -print -quit | grep -q .; then
  fail "bundle contains retired ASR runtime/resources or Whisper model weights"
fi

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

for binary in "$executable"; do
  archs="$(lipo -archs "$binary")"
  [[ " $archs " == *" $arch "* ]] || fail "$binary does not contain architecture $arch"
  bad_load_paths="$(developer_load_paths "$binary")"
  if [[ -n "$bad_load_paths" ]]; then
    printf '%s\n' "$bad_load_paths" >&2
    fail "$binary contains a developer-machine load path"
  fi
done

# Every shipped Mach-O must remain compatible with the deployment floor pinned in
# the application bundle configuration. Support both modern LC_BUILD_VERSION and
# older LC_VERSION_MIN_MACOSX load commands.
for binary in "$executable"; do
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

if otool -L "$executable" | grep -Ei 'lib(moonshine|whisper|ggml[^/]*)\.dylib|libonnxruntime\.1\.23\.2\.dylib'; then
  fail "executable loads retired ASR libraries or dynamically linked Whisper"
fi

notice_root="$app_path/Contents/Resources/native/macos/notices"
[[ -f "$notice_root/TALKING_MOOSE_LICENSE" ]] || fail "Talking Moose project license is missing from bundle"
[[ -f "$notice_root/THIRD_PARTY_NOTICES.md" ]] || fail "third-party notice inventory is missing from bundle"
[[ -f "$notice_root/WhisperRuntime/WHISPER_CPP_LICENSE" ]] \
  || fail "pinned Whisper runtime license is missing from bundle"

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
    "$executable" --whisper-native-smoke-test
)"
printf '%s\n' "$smoke_output"
grep -E '^whisper-native-runtime-version=.+$' <<<"$smoke_output" >/dev/null \
  || fail "bundled executable failed Whisper native runtime smoke check"

# An ad-hoc executable signature alone does not seal the application resources.
bundle_seal="$app_path/Contents/_CodeSignature/CodeResources"
if [[ -f "$bundle_seal" ]]; then
  codesign --verify --strict --deep "$app_path"
elif [[ "$require_signature" == '--require-signature' ]]; then
  fail "signed bundle required, but application has no resource-sealed signature"
else
  printf 'Bundle has no resource-sealed signature; unsigned app verified for smoke CI.\n'
fi

printf 'Verified static Whisper runtime bundle for macOS %s.\n' "$arch"
