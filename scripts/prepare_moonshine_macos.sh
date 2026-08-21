#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_root/src-tauri/native/moonshine-runtime.json"
stage_dir="$repo_root/src-tauri/native/macos"
notice_dir="$stage_dir/notices"
arch="${1:-$(uname -m)}"

fail() {
  printf 'prepare_moonshine_macos: %s\n' "$*" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "this script must run on macOS"
case "$arch" in
  arm64|x86_64) ;;
  *) fail "unsupported macOS architecture: $arch" ;;
esac
[[ "$(uname -m)" == "$arch" ]] || fail "requested architecture $arch does not match host $(uname -m)"

for command in git cmake python3 lipo otool install_name_tool; do
  command -v "$command" >/dev/null || fail "required command not found: $command"
done
command -v git-lfs >/dev/null || git lfs version >/dev/null 2>&1 || fail "git-lfs is required"

IFS=$'\t' read -r runtime_release source_commit ort_version ort_sha256 ort_bytes < <(python3 - "$manifest" "$arch" <<'PY'
import json, sys
manifest_path, arch = sys.argv[1:]
data = json.load(open(manifest_path, encoding="utf-8"))
entry = data["macos"][arch]
print("\t".join([
    data["runtime"]["release"],
    data["runtime"]["source_commit"],
    data["onnxruntime"]["version"],
    entry["onnxruntime_sha256"],
    str(entry["onnxruntime_bytes"]),
]))
PY
)
[[ -n "$runtime_release" && -n "$source_commit" && -n "$ort_version" && -n "$ort_sha256" && -n "$ort_bytes" ]] || fail "invalid provenance manifest"

workspace="$(mktemp -d "${TMPDIR:-/tmp}/talking-moose-moonshine.XXXXXX")"
cleanup() { rm -rf "$workspace"; }
trap cleanup EXIT
source_dir="$workspace/moonshine"
build_dir="$workspace/build"

printf 'Preparing Moonshine %s (%s) for macOS %s\n' "$runtime_release" "$source_commit" "$arch"
mkdir -p "$source_dir"
git -C "$source_dir" init -q
git -C "$source_dir" remote add origin https://github.com/moonshine-ai/moonshine.git
GIT_LFS_SKIP_SMUDGE=1 git -C "$source_dir" fetch --depth 1 origin "$source_commit"
git -C "$source_dir" checkout -q --detach FETCH_HEAD
[[ "$(git -C "$source_dir" rev-parse HEAD)" == "$source_commit" ]] || fail "source commit mismatch"

ort_rel="core/third-party/onnxruntime/lib/macos/$arch/libonnxruntime.$ort_version.dylib"
git -C "$source_dir" lfs pull --include="$ort_rel"
ort_source="$source_dir/$ort_rel"
[[ -f "$ort_source" ]] || fail "pinned ONNX Runtime dylib was not materialized: $ort_rel"

python3 - "$ort_source" "$ort_sha256" "$ort_bytes" <<'PY'
import hashlib, pathlib, sys
path = pathlib.Path(sys.argv[1])
expected_sha = sys.argv[2]
expected_size = int(sys.argv[3])
data = path.read_bytes()
actual_sha = hashlib.sha256(data).hexdigest()
if len(data) != expected_size:
    raise SystemExit(f"ONNX Runtime size mismatch: expected {expected_size}, got {len(data)}")
if actual_sha != expected_sha:
    raise SystemExit(f"ONNX Runtime SHA-256 mismatch: expected {expected_sha}, got {actual_sha}")
PY

cmake \
  -S "$source_dir/core" \
  -B "$build_dir" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_OSX_ARCHITECTURES="$arch" \
  -DCMAKE_OSX_DEPLOYMENT_TARGET=10.15
cmake --build "$build_dir" --config Release --target moonshine --parallel

moonshine_source="$(find "$build_dir" -type f -name 'libmoonshine.dylib' -print -quit)"
[[ -n "$moonshine_source" && -f "$moonshine_source" ]] || fail "libmoonshine.dylib was not produced"

rm -f "$stage_dir/libmoonshine.dylib" "$stage_dir/libonnxruntime.$ort_version.dylib"
cp "$moonshine_source" "$stage_dir/libmoonshine.dylib"
cp "$ort_source" "$stage_dir/libonnxruntime.$ort_version.dylib"

install_name_tool -id '@rpath/libmoonshine.dylib' "$stage_dir/libmoonshine.dylib"
ort_dependencies=()
while IFS= read -r dependency; do
  ort_dependencies+=("$dependency")
done < <(otool -L "$stage_dir/libmoonshine.dylib" | awk '/libonnxruntime[^ ]*\.dylib/ { print $1 }')
[[ ${#ort_dependencies[@]} -eq 1 ]] || fail "expected exactly one ONNX Runtime dependency in libmoonshine.dylib"
install_name_tool -change "${ort_dependencies[0]}" "@rpath/libonnxruntime.$ort_version.dylib" "$stage_dir/libmoonshine.dylib"
install_name_tool -id "@rpath/libonnxruntime.$ort_version.dylib" "$stage_dir/libonnxruntime.$ort_version.dylib"

for dylib in "$stage_dir/libmoonshine.dylib" "$stage_dir/libonnxruntime.$ort_version.dylib"; do
  archs="$(lipo -archs "$dylib")"
  [[ " $archs " == *" $arch "* ]] || fail "$dylib does not contain architecture $arch"
  if otool -L "$dylib" | grep -E '/opt/homebrew|/usr/local|/Users/|/private/tmp|/var/folders' >/dev/null; then
    otool -L "$dylib" >&2
    fail "$dylib contains a developer-machine load path"
  fi
done

otool -L "$stage_dir/libmoonshine.dylib" | grep -F "@rpath/libonnxruntime.$ort_version.dylib" >/dev/null \
  || fail "libmoonshine.dylib does not load ONNX Runtime through @rpath"

rm -rf "$notice_dir/MoonshineRuntime"
mkdir -p "$notice_dir/MoonshineRuntime/source"
cp "$repo_root/docs/THIRD_PARTY_NOTICES.md" "$notice_dir/THIRD_PARTY_NOTICES.md"
cp "$manifest" "$notice_dir/MoonshineRuntime/moonshine-runtime.json"
cp "$source_dir/LICENSE" "$notice_dir/MoonshineRuntime/MOONSHINE_LICENSE"

while IFS= read -r -d '' license; do
  rel="${license#"$source_dir/"}"
  dest="$notice_dir/MoonshineRuntime/source/$rel"
  mkdir -p "$(dirname "$dest")"
  cp "$license" "$dest"
done < <(
  find "$source_dir/core/third-party" "$source_dir/core/cpp-annote" \
    -type f \( -iname 'LICENSE*' -o -iname 'COPYING*' -o -iname 'NOTICE*' \) \
    -print0
)

license_count="$(find "$notice_dir/MoonshineRuntime" -type f | wc -l | tr -d ' ')"
(( license_count >= 5 )) || fail "unexpectedly small native notice set: $license_count files"

printf 'Prepared %s and ONNX Runtime %s for macOS %s (%s notice files).\n' \
  "$runtime_release" "$ort_version" "$arch" "$license_count"
