#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
arch="${1:-$(uname -m)}"
work_root="${2:-${RUNNER_TEMP:-/tmp}/talking-moose-asr015}"

fail() {
  printf 'run_asr015_macos_acceptance: %s\n' "$*" >&2
  exit 1
}

[[ "$(uname -s)" == "Darwin" ]] || fail "this acceptance runner must execute on macOS"
[[ "$arch" == "arm64" || "$arch" == "x86_64" ]] || fail "unsupported architecture: $arch"
[[ "$(uname -m)" == "$arch" ]] || fail "requested $arch but host is $(uname -m)"

for command in cargo git lipo python3; do
  command -v "$command" >/dev/null 2>&1 || fail "required command not found: $command"
done

commit_sha="$(git -C "$repo_root" rev-parse HEAD)"
[[ -n "$commit_sha" ]] || fail "unable to determine repository commit"
if [[ -n "$(git -C "$repo_root" status --porcelain --untracked-files=no)" ]]; then
  fail "tracked repository files are dirty; run acceptance against an exact commit"
fi

# A local acceptance run must build the pinned runtime from the manifest rather
# than silently trusting dylibs left behind by an older checkout. The dedicated
# GitHub workflow stages the same runtime in a prior step and opts into reuse to
# avoid compiling Moonshine twice.
if [[ "${TALKING_MOOSE_ASR015_RUNTIME_PREPARED:-0}" != "1" ]]; then
  python3 "$repo_root/scripts/validate_moonshine_runtime_manifest.py"
  bash "$repo_root/scripts/prepare_moonshine_macos.sh" "$arch"
fi

runtime_manifest="$repo_root/src-tauri/native/moonshine-runtime.json"
ort_version="$(python3 - "$runtime_manifest" <<'PY_RUNTIME'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    print(json.load(handle)["onnxruntime"]["version"])
PY_RUNTIME
)"
moonshine_dylib="$repo_root/src-tauri/native/macos/libmoonshine.dylib"
onnxruntime_dylib="$repo_root/src-tauri/native/macos/libonnxruntime.$ort_version.dylib"
for dylib in "$moonshine_dylib" "$onnxruntime_dylib"; do
  [[ -f "$dylib" ]] || fail "prepared native runtime is missing: $dylib"
  dylib_arches="$(lipo -archs "$dylib")"
  [[ " $dylib_arches " == *" $arch "* ]] || fail "$dylib does not contain architecture $arch"
done

# tauri::generate_context!() reads the configured icon set at compile time.
# Release icons are deterministic ignored build inputs, so create them on clean
# local checkouts before invoking Cargo.
python3 "$repo_root/scripts/generate_app_icons.py"

mkdir -p "$work_root"
model_root="$work_root/models"
pcm_path="$work_root/asr015-corpus.pcm"
corpus_metadata="$work_root/asr015-corpus.json"
hardware_metadata="$work_root/asr015-hardware.json"
records="$work_root/asr015-records.jsonl"
report="$work_root/ASR015_SUPPORTED_MAC_ACCEPTANCE.md"
: > "$records"

python3 "$repo_root/scripts/prepare_asr015_corpus.py" "$pcm_path" "$corpus_metadata"

cpu_brand="$(sysctl -n machdep.cpu.brand_string 2>/dev/null || true)"
if [[ -z "$cpu_brand" ]]; then
  cpu_brand="$(system_profiler SPHardwareDataType 2>/dev/null | awk -F': ' '/Chip:|Processor Name:/ {print $2; exit}')"
fi
[[ -n "$cpu_brand" ]] || cpu_brand="unknown"
hardware_model="$(sysctl -n hw.model 2>/dev/null || printf 'unknown')"
physical_cpu_count="$(sysctl -n hw.physicalcpu)"
logical_cpu_count="$(sysctl -n hw.logicalcpu)"
memory_bytes="$(sysctl -n hw.memsize)"
macos_version="$(sw_vers -productVersion)"
macos_build="$(sw_vers -buildVersion)"
low_power_mode="$(pmset -g 2>/dev/null | awk '/lowpowermode/ {print $2; exit}')"
[[ -n "$low_power_mode" ]] || low_power_mode="unknown"

python3 - "$hardware_metadata" "$hardware_model" "$cpu_brand" "$physical_cpu_count" \
  "$logical_cpu_count" "$memory_bytes" "$macos_version" "$macos_build" "$arch" "$low_power_mode" <<'PY'
import json
import pathlib
import sys
(
    output,
    hardware_model,
    cpu_brand,
    physical_cpu_count,
    logical_cpu_count,
    memory_bytes,
    macos_version,
    macos_build,
    architecture,
    low_power_mode,
) = sys.argv[1:]
data = {
    "hardware_model": hardware_model,
    "cpu_brand": cpu_brand,
    "physical_cpu_count": int(physical_cpu_count),
    "logical_cpu_count": int(logical_cpu_count),
    "memory_bytes": int(memory_bytes),
    "macos_version": macos_version,
    "macos_build": macos_build,
    "architecture": architecture,
    "low_power_mode": low_power_mode,
}
pathlib.Path(output).write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
print(json.dumps(data, sort_keys=True))
PY

export TALKING_MOOSE_ASR_BENCHMARK=1
export TALKING_MOOSE_ASR_BENCHMARK_INSTALL=1
export TALKING_MOOSE_ASR_BENCHMARK_MODEL_ROOT="$model_root"
export TALKING_MOOSE_ASR_BENCHMARK_PCM="$pcm_path"
export TALKING_MOOSE_MOONSHINE_LIB_DIR="$repo_root/src-tauri/native/macos"
export DYLD_LIBRARY_PATH="$repo_root/src-tauri/native/macos${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
export CARGO_INCREMENTAL=0

run_one() {
  local architecture="$1"
  local test_name="$2"
  local phase="$3"
  local run_number="$4"
  local log="$work_root/${architecture}-${phase}-${run_number}.log"
  export TALKING_MOOSE_ASR_BENCHMARK_PHASE="$phase"
  export TALKING_MOOSE_ASR_BENCHMARK_RUN="$run_number"

  printf '\n=== %s %s run %s ===\n' "$architecture" "$phase" "$run_number"
  cargo test --locked --release --manifest-path "$repo_root/src-tauri/Cargo.toml" --lib \
    "$test_name" -- --ignored --nocapture --test-threads=1 2>&1 | tee "$log"

  record="$(grep -o 'ASR015_BENCHMARK_JSON=.*' "$log" | tail -n 1 | sed 's/^ASR015_BENCHMARK_JSON=//')"
  [[ -n "$record" ]] || fail "benchmark JSON record missing from $log"
  python3 -c 'import json,sys; json.loads(sys.argv[1])' "$record"
  printf '%s\n' "$record" >> "$records"
}

run_model() {
  local architecture="$1"
  local test_name="$2"
  run_one "$architecture" "$test_name" warmup 0
  for run_number in 1 2 3 4 5; do
    run_one "$architecture" "$test_name" measured "$run_number"
  done
}

run_model tiny_streaming asr015_cpu_benchmark_tiny_on_supported_mac
run_model small_streaming asr015_cpu_benchmark_small_on_supported_mac

python3 "$repo_root/scripts/render_asr015_benchmark_report.py" \
  --records "$records" \
  --hardware "$hardware_metadata" \
  --corpus "$corpus_metadata" \
  --output "$report" \
  --commit "$commit_sha"

grep -F 'Status: **PASS**' "$report" >/dev/null || fail "acceptance report did not pass"
printf '\nASR015_ACCEPTANCE_REPORT=%s\n' "$report"
cat "$report"
