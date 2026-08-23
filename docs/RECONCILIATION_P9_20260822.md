# Phase P9 reconciliation — 2026-08-22

This overlay reconciles the active V1 tracker without rewriting the large monolithic TODO through the
GitHub contents API.

## Accepted closure: V1R-090 through V1R-093

P9 is **accepted complete** at implementation commit
`f92dcc9fc491abaefb86988bad2871368c8b6fe6` (`feat: close P9 persistence lifecycle`).
GitHub Actions run `32614136156` completed successfully against that exact head SHA on 2026-08-22 PT.

### V1R-090 — Database schema migrations

- [x] Explicit `schema_version` table and current version constant.
- [x] Ordered migrations are applied according to the recorded version.
- [x] The complete pending migration chain is atomic; failure rolls back the startup migration attempt.
- [x] Privacy-preserving backup policy documented: V1 creates no automatic duplicate database containing
      private data; manual stopped-app copies remain possible.
- [x] Persistent database initialization/migration failure aborts startup rather than silently switching to RAM.
- [x] Legacy-schema migration and idempotent reopen tests preserve existing memory/transcript data.
- [x] CI acceptance: run `32614136156` passed at `f92dcc9fc491abaefb86988bad2871368c8b6fe6`.

### V1R-091 — Transcript correctness

- [x] Recent-limit retrieval selects the newest bounded window and returns it chronologically.
- [x] Session ID and role validation provide stable persisted semantics.
- [x] Only final `user`/`moose` roles are accepted; consecutive duplicate finals are idempotent.
- [x] Transcript input, query size, and persisted row retention are explicitly bounded.
- [x] CI acceptance: run `32614136156` passed at `f92dcc9fc491abaefb86988bad2871368c8b6fe6`.

### V1R-092 — Richer memory records

- [x] Records include category, source, confidence, created timestamp, and updated timestamp.
- [x] New explicit memories use source `remember_fact` and confidence `1.0`; legacy rows migrate as `legacy`.
- [x] Re-remembering the same normalized fact updates metadata rather than creating a duplicate.
- [x] Fact/category lengths are bounded.
- [x] No transcript, observer, ambient-event, or inferred-summary auto-ingestion path was added.
- [x] CI acceptance: run `32614136156` passed at `f92dcc9fc491abaefb86988bad2871368c8b6fe6`.

### V1R-093 — Conversation summary decision

- [x] Conversation summaries are deferred for V1.
- [x] No summary table or background summarizer is created.
- [x] The privacy/retention requirements for any future summary feature are documented.
- [x] CI acceptance: run `32614136156` passed at `f92dcc9fc491abaefb86988bad2871368c8b6fe6`.

See `docs/PERSISTENCE_V1_POLICY.md` for the V1 persistence contract.
