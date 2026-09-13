#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
backend_tmp="$(mktemp)"
local_tts_tmp="$(mktemp)"
trap 'rm -f "${backend_tmp}" "${local_tts_tmp}"' EXIT

cd "${repo_root}"
if [[ ! -f src-tauri/icons/icon.png ]]; then
  python3 scripts/generate_app_icons.py
fi
cargo run --quiet --manifest-path src-tauri/Cargo.toml --features frontend-contract-export --bin export_frontend_contract \
  > "${backend_tmp}"
cargo run --quiet --manifest-path src-tauri/Cargo.toml --features frontend-contract-export --bin export_local_tts_frontend_contract \
  > "${local_tts_tmp}"
mv "${backend_tmp}" src/generated/backendContract.json
mv "${local_tts_tmp}" src/generated/localTtsBackendContract.json
trap - EXIT
