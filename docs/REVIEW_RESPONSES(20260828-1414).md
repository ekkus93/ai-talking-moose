# Responses to Follow-up Review Questions — 2026-08-28 14:14

**Reviewing document:** `REVIEW_QUESTIONS(20260828-1414).md`
**Documents under review:** `SPEC(20260828-125605).md`, `TODO(20260828-125605).md`, `REVIEW_RESPONSES(20260828-1303).md`
**Snapshot reviewed:** `1f8e44e194442aa51fef7db19eb02e8830829e1b`
**Disposition:** Agree with alternative ×1, Agree ×1. Neither item disputed.

Both items were re-verified against source before responding. Both hold. Item 1 identifies a genuine over-claim in the amended documents — the same class of error the previous round corrected in `V1R-212`, reintroduced one section later in its replacement.

No application source was changed.

---

## Verification performed before responding

| Claim | Verified | Evidence |
| --- | --- | --- |
| §6.2 names signature drift but prescribes a name-only gate | Yes | Problem statement said "a rename, a signature change, or a removed registration"; required end state said only "a command name invoked by the frontend is not registered in Rust" |
| The exporter emits values, not shapes | Yes | `src/bin/export_frontend_contract.rs` serializes `AppSettings::default()` plus the two catalogs — 25 lines, plain `serde::Serialize` |
| Frontend types are a hand-maintained parallel surface | Yes | `src/types/moose.ts` declares 30 interfaces/type aliases |
| The cast discards mismatches | Yes | `src/lib/backendContract.ts` returns `{...backendContract.settings} as AppSettings` |
| No schema/binding crate is present | Yes | `src-tauri/Cargo.toml` declares `serde` and `serde_json`; no `schemars`, `ts-rs`, or `specta` |
| Shapes currently agree | Yes | Rust-derived `settings` 33 keys, TypeScript `AppSettings` 33 fields, set difference empty both directions |
| `isTauri()` is a single choke point | Yes | Referenced only inside `src/lib/tauriBridge.ts`; no other occurrence anywhere in `src/` |
| Fail-closed is an established repo idiom | Yes | `commands/ambient.rs:37`, `app/state.rs:171`, `character/prompt.rs:8`, plus the non-macOS observer branch |

---

## 1. `V1R-213` scope — **Agree with alternative**

The over-claim is real and is confirmed. §6.2's problem statement named "a rename, a signature change, or a removed registration," while the required end state it prescribed detects only unregistered names. That is precisely the failure mode the previous round corrected in `V1R-212` — claiming an invariant the mechanism cannot prove — reappearing in the requirement written to replace it. It must be fixed regardless of which scope is chosen.

The reviewer's concern about fixture drift is also concrete rather than theoretical. `V1R-212` makes frontend tests run the real `invoke` path against hand-written fixtures, and `src/types/moose.ts` hand-maintains 30 types mirroring Rust. Nothing ties either to Rust. `src/lib/backendContract.ts` casts the generated settings with `as AppSettings`, which actively discards mismatches in both directions: a field added in Rust is invisible to the interface, and a field removed in Rust leaves the interface still declaring it. Without a shape gate, `V1R-212` would move the parallel implementation up one layer into the tests meant to eliminate it.

**Neither Option A nor Option B as written.** Option A leaves a defect the reviewer correctly identifies. Option B, read literally as generated bindings or JSON Schema, means adding `schemars`, `ts-rs`, or `specta` — none of which is currently a dependency — which is the disproportionate framework change the review itself asked to avoid if the cost were high.

**Proposed alternative, closer to B at roughly A's cost.** Extend the existing exporter rather than introducing a schema framework:

1. `export_frontend_contract` emits a representative instance of each IPC-crossing type alongside the values it already emits.
2. A frontend check compares each hand-written interface in `src/types/moose.ts` against the emitted key set and per-key JSON value type.

This reuses machinery that already exists, is already invoked by `scripts/generate_frontend_contract.sh`, and that `V1R-214` is independently placing under a CI regeneration gate — so extending the exporter means the shape data inherits that gate for free. Marginal cost is one exporter change plus one test, with no new dependency. It catches the dominant drift modes: added, removed, renamed, and retyped fields.

**What it does not prove, published rather than implied closed.** Recording this is the point of the item, so the documents do not repeat the error being corrected:

- narrowing within one JSON type — `u32`, `i32`, and `f64` all serialize as JSON numbers;
- enum variant sets, unless a representative instance of each variant is emitted;
- optionality, where `None` and an absent field serialize identically;
- argument shapes for commands whose parameters are primitives rather than named types.

Closing that remainder is a candidate for a later cycle and is recorded as open.

**Timing.** Both halves start green — 37/37 command names, 33/33 settings keys — so this adds gates to a clean surface rather than opening a remediation front.

**Changes:** §6.2 retitled "No cross-language IPC contract exists" and split into §6.2.1 registration integrity and §6.2.2 shape integrity, each independently acceptable, with the mechanism, the explicit non-adoption of a schema dependency, and the residual-gap list. `V1R-213` retitled "Verify the Rust↔frontend IPC contract" and restructured into Part 1 / Part 2 / Shared, with acceptance for both. Added a row to remove or justify the `as AppSettings` cast once shapes are checked. Spec §11 gains a corresponding acceptance line.

---

## 2. `V1R-215` production isolation — **Agree**

Accepted in full, including all five suggested acceptance cases. The reasoning is right and the gap was real: the plan required explicit selection and separate preview tests, but nowhere established that production *cannot* select the adapter. That was left as an assumption in exactly the place this cycle exists to eliminate assumptions.

The point about consolidation deserves emphasis, because it inverts a natural reading of the refactor. Today's forty scattered fallbacks are individually bad but collectively inert in production, since `__TAURI_INTERNALS__` is always present in a packaged build. Consolidating them behind one adapter is the right design, and it also concentrates every fabricated result — synthetic credential state, connection success, session ids, persistence mutations, model-install state, provider replies — behind a single switch. The refactor improves the architecture and raises the consequence of one mistake at that switch. A negative test is therefore worth more after the change than before it.

Verified that this is cheap to establish now: `isTauri()` is referenced only inside `src/lib/tauriBridge.ts` and nowhere else in `src/`. Selection is a single choke point today, and will be harder to constrain once an adapter is threaded through the application.

The invariant is adopted as written. The implementation mechanism is deliberately left open — compile-time or environment gating at the entry point, dependency injection, or a preview module production never imports — with the required property being that the packaged application fails closed. That matches the established repository idiom rather than introducing a new convention: `commands/ambient.rs:37` fails closed on a stale legacy settings value, `app/state.rs:171` normalizes an unsupported field fail-closed, and the non-macOS desktop observer branch fails closed rather than fabricating observations.

One constraint added beyond the review's text: selection must not be reachable through ordinary runtime state — no query parameter, `localStorage` value, browser global, or environment variable that could ship in a production bundle. The review named these as risks; making it an explicit prohibition keeps a later implementer from satisfying "explicitly selected" with a runtime flag that technically qualifies.

**Changes:** new spec §6.4.1 with the invariant, the rationale, the prohibition on runtime-state selection, and all five acceptance cases. `V1R-215` gains a "Production isolation" section with four implementation rows and four new acceptance rows.

---

## Summary of document changes

| Item | Disposition | Primary change |
| --- | --- | --- |
| 1 | Agree with alternative | §6.2 split into registration + shape integrity; exporter extension instead of a schema dependency; residual gaps published |
| 2 | Agree | New §6.4.1 fail-closed production-isolation invariant, tested rather than assumed |

Item count is unchanged at 9; severities remain S1 ×2, S2 ×4, S3 ×2, S4 ×1. Both changes add scope inside existing rows rather than new rows, because both are properties of gates already specified.

One process observation worth carrying forward: item 1 is the second instance in two rounds of a requirement describing a broader problem than its acceptance criterion can detect. The first was caught in `V1R-212`; this one appeared in the requirement written to fix it. A requirement's problem statement and its acceptance criterion should be checked against each other explicitly before publication — the failure mode is not carelessness about the code, but a gap between how a defect is described and what the proposed gate actually measures.
