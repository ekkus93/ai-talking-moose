#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp="$(mktemp)"
trap 'rm -f "${tmp}"' EXIT

cd "${repo_root}"
if [[ ! -f src-tauri/icons/icon.png ]]; then
  python3 scripts/generate_app_icons.py
fi
cargo run --quiet --manifest-path src-tauri/Cargo.toml --features frontend-contract-export --bin export_frontend_contract \
  | node node_modules/prettier/bin/prettier.cjs --parser json > "${tmp}"
mv "${tmp}" src/generated/backendContract.json
trap - EXIT
