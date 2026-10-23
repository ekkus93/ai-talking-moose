---
name: lint-n-test
description: Lint the files and run all repository test suites and verification gates. Use when asked to lint and test, run all tests, or invoke /lint-n-test.
model: haiku
context: fork
agent: general-purpose
background: false
---

# Lint the Files and Run All Tests

Work from the repository root. Read applicable `AGENTS.md` and `CLAUDE.md`
instructions, `package.json`, and CI definitions to identify the current gates.

For Talking Moose AI, run these gates separately so an earlier failure does not
hide results from the remaining suites:

1. `npm run check:frontend` — frontend formatting, lint, types, contracts, all
   Vitest tests, and production build.
2. `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 npm run check:rust` — Rustfmt, Clippy,
   and all Rust test targets/features. Use this prescribed gate rather than
   substituting narrower Cargo commands.
3. `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 npm run check:generated-backend-contract`
   — regenerate Rust-derived frontend contracts and check for drift.
4. Run additional applicable CI policy, script, and regression suites identified
   from the repository instructions and changed files.

Continue with independent gates after a failure and record each exit status.
If a gate stops at its first failing stage, run its remaining independent stages
so every applicable test suite receives a result. Preserve existing changes;
report generated-contract drift without staging or committing it to hide failure.

Do not run overlapping Cargo jobs. On a constrained host, use
`VITEST_MAX_WORKERS=2` for the complete frontend suite. Do not narrow test coverage
or increase timeouts to conceal failures. Diagnose failures before retrying.

Record unavailable platform, hardware, credential, or real-model acceptance
checks as not run, with the reason. Running this skill does not authorize paid
API calls, model downloads, workflow dispatch, commits, or pushes.

Return a concise pass/fail/not-run result for every gate, test totals where
available, and actionable diagnostics for failures. Distinguish the initial
failure from a successful rerun; never claim a suite passed if it did not finish.
