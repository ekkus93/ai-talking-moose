# Phase P9 reconciliation — 2026-08-22

This overlay reconciles the active V1 tracker without rewriting the large monolithic TODO through the
GitHub contents API.

## Previously accepted closure: V1R-090 through V1R-093

V1R-090 through V1R-093 were accepted at implementation commit
`f92dcc9fc491abaefb86988bad2871368c8b6fe6` (`feat: close P9 persistence lifecycle`).
GitHub Actions run `32614136156` completed successfully against that exact head SHA on 2026-08-22 PT.

The legacy monolithic tracker also contains V1R-094 and V1R-095. Those rows were omitted from the
original P9 reconciliation and are addressed by the 2026-08-23 closure candidate below. P9 must not be
considered fully complete until CI accepts that candidate.

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

## 2026-08-23 closure candidate: V1R-094 and V1R-095

### V1R-094 — Observation retention

- [x] V1 retention decision is zero persistent observation rows; P8 observation state remains transient and bounded in memory.
- [x] Schema version 4 drops the legacy `observations` table, which is the startup pruning/migration path for older profiles.
- [x] There is no production observation-write API, so the persistent hard limit is zero rather than a positive TTL-backed history.
- [x] `Forget Everything` interrupts queued ambient observation work, clears P7 event fingerprints, and resets the live desktop summarizer so derived app/idle/battery history cannot survive a privacy reset.
- [x] Direct schema-v3 migration and runtime-reset regressions cover the legacy-table purge and fresh derived-state behavior.

### V1R-095 — Forget operations

- [x] Delete-one-memory remains supported.
- [x] Transcript clearing has an explicit persistence primitive and full reset clears transcripts.
- [x] Observation persistence is eliminated; full reset defensively drops any legacy observation table and clears live derived observer state.
- [x] Full reset atomically deletes persisted memories/transcripts while preserving ordinary settings; SQLite `secure_delete` is enabled on every connection.
- [x] Google API credentials remain intentionally separate in the OS secure store and are removed only by the explicit credential-clear operation.
- [x] Persistence rollback and command-level regressions verify Forget Everything is atomic for SQLite private rows and preserves preferences/credentials.

### Gate P9

Status: **pending CI acceptance** for the V1R-094/V1R-095 closure candidate.

The original reconciliation's “P9 accepted complete” wording applied only to V1R-090 through V1R-093.
After this candidate passes the full CI matrix, P9 can be treated as accepted complete across V1R-090 through V1R-095.
