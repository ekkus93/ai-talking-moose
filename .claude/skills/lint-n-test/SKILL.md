---
name: lint-n-test
description: Lint the frontend and backend code and run the full test suite. Invoked as /lint-n-test.
---

# Lint and test

Run this project's lint and test steps by delegating to a Haiku subagent, so the
work happens off the main conversation's model.

Use the Agent tool with `model: "haiku"` (subagent_type `general-purpose`) and give
it a prompt instructing it to run, in order, from the repo root — continuing to the
next step even if an earlier one fails, not stopping early:

1. `npm run lint` — ESLint (frontend)
2. `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` — Clippy (backend)
3. `npm run test` — Vitest (frontend)
4. `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features` — cargo test (backend)

Ask the subagent to return a concise pass/fail line for each of the 4 steps, plus
the full error output for any step that failed.

Relay the subagent's report to the user as-is — do not re-run the commands yourself.
