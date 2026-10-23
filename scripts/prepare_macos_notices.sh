#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
python3 scripts/check_whisper_provenance.py
notice_dir="$repo_root/src-tauri/native/macos/notices"
# Remove generated resources left by a checkout predating the ASR replacement.
rm -rf "$notice_dir/MoonshineRuntime"
rm -f "$repo_root/src-tauri/native/macos/libmoonshine.dylib" \
  "$repo_root/src-tauri/native/macos/libonnxruntime.1.23.2.dylib"
mkdir -p "$notice_dir/WhisperRuntime"
cp "$repo_root/LICENSE" "$notice_dir/TALKING_MOOSE_LICENSE"
cp "$repo_root/docs/THIRD_PARTY_NOTICES.md" "$notice_dir/THIRD_PARTY_NOTICES.md"
cp "$repo_root/third_party/whisper.cpp/LICENSE"   "$notice_dir/WhisperRuntime/WHISPER_CPP_LICENSE"
printf 'Prepared application and pinned Whisper runtime notices.\n'
