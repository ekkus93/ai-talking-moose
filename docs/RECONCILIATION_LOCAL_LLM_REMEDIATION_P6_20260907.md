# Local LLM remediation P6 reconciliation — 2026-09-07

## Status

**P6 implementation is complete and post-merge validated; this record is the separate tracker closeout.**

This record closes `LLMR-600` through `LLMR-603` after exact-head implementation CI, guarded squash merge, and exact post-merge `master` CI. LLMR-601 is closed as **N/A by audit**, not as an implemented application cancellation API.

Implementation base: `185021d2b3baef3c3d57f6a909512e507bb8b908`, the exact P5-closure `master` validated by CI `34094007233`.

Clean implementation head: `31ac0b76b99c28d0f5225c0c6ebcfd8176c78231`.

Implementation PR: #59, `Local LLM: clarify runtime cancellation and template ownership`.

Validation history:

- superseded PR CI `34143707914` reached Rust quality and failed only `cargo fmt --check` on one new negative-test line wrap;
- the exact rustfmt transformation was applied and authoring history was collapsed back to one clean commit;
- replacement exact-head PR CI `34144002755` completed successfully on `31ac0b76b99c28d0f5225c0c6ebcfd8176c78231`, including frontend quality, Rust format/Clippy/tests, backend failure/stress matrices, all three Local LLM compile proofs, dependency/security/release gates, both macOS bundles, and canonical `npm run check:all`;
- PR #59 was squash-merged with the expected-head guard;
- merged `master` SHA is `0782091c8f46bb6471b86a2e724364e10b611077` with the same validated implementation tree;
- exact post-merge `master` CI `34146860974` completed successfully on `0782091c8f46bb6471b86a2e724364e10b611077`, including canonical `npm run check:all` and every ordinary job.

## LLMR-600 — Local generation cancellation ownership audit

Production ownership is intentionally narrower than the decode-loop cancellation mechanism:

1. The provider-neutral `TextModel::generate()` trait has no cancellation parameter.
2. `LocalTextModel::generate()` creates a fresh `CancellationToken` for each normal typed/ambient Local request and passes it into the runtime. No application caller receives or retains that token.
3. `LocalRuntimeManager::generate()` accepts the token and forwards it into the blocking native generation scope.
4. `LlamaEngine::generate()` checks the token before/through the cooperative decode path; direct runtime tests can cancel a supplied token to prove those checkpoints.
5. Model switch is driven by a later serialized generation/load operation; it does not cancel the request currently holding the operation mutex.
6. Model deletion serializes behind generation, unloads the selected model if needed, and only then removes the artifact.
7. Shutdown calls `begin_shutdown()` to reject new work, then waits for the serialized runtime transition subject to the application-level five-second teardown bound. It does not falsely claim to cancel the private request token.

Decision: typed and ambient generation currently have no product Cancel affordance and therefore do **not** require a first-class external generation-cancellation API. Cooperative runtime cancellation and application-exposed cancellation are distinct guarantees.

## LLMR-601 — External cancellation handle decision

**N/A for current product semantics.**

The five implementation-only checklist bullets under the conditional “If LLMR-600 finds a current requirement” are closed in the tracker explicitly as `N/A`, because that condition is false. This does not assert that an application cancellation handle was implemented.

Adding a request-owned public cancellation API would create a new product contract with no caller. P6 therefore does not manufacture one merely because the runtime already supports cooperative cancellation internally. Existing decode-iteration cooperative checks remain intact, model switch/delete ordering remains serialized, shutdown remains bounded, and no unsafe llama.cpp abort FFI is introduced.

The code/docs are corrected so a fresh private token is no longer described as an application-cancellable path. A provider-level test observes that ordinary `LocalTextModel::generate()` supplies a non-cancelled private token to the runtime.

## LLMR-602 — Chat-template ownership truthfulness

The previous `LocalTextModel` comment incorrectly stated that the native llama.cpp layer applies the selected GGUF's embedded chat template.

Actual ownership is now documented consistently:

- `LlamaModel::chat_template(None)` retrieves the embedded template source for compatibility validation only.
- `runtime/chat_template.rs` renders the supported SmolLM2/Qwen ChatML framing in application code.
- SmolLM2 injects its pinned default system instruction when the request omits one.
- Qwen3 receives explicit application-owned non-thinking framing: the base assistant opening is followed by the empty `<think>

</think>

` prefill equivalent to `enable_thinking=false`.
- `runtime/reasoning.rs` strips any unexpected Qwen reasoning trace before visible output can leave the runtime and fails closed on malformed/ambiguous reasoning markup.
- Unsupported embedded templates fail with `ChatTemplate`; there is no generic-family fallback.

Existing family rendering assertions are unchanged, so P6 does not alter prompt framing.

## LLMR-603 — Supported-template identification

P6 evaluated a whole-template fingerprint and rejected it as unnecessarily brittle for this two-artifact scope: harmless Jinja whitespace/trim-marker formatting can change the full source while preserving the family semantics the application relies on.

Instead, validation now uses a deterministic ordered family signature:

- common ChatML start/end tokens are required;
- family-specific semantic anchors must appear in expected order;
- SmolLM2 requires the pinned default system-injection/message-render/generation-prompt sequence and rejects Qwen thinking markers;
- Qwen requires its system/message/generation-prompt/non-thinking sequence and rejects the SmolLM2 default-system sentinel;
- synthetic fragment bags containing the right words in the wrong semantic order fail closed;
- cross-family fixtures fail closed;
- existing canonical SmolLM2 and Qwen fixtures remain accepted.

This discriminator is intentionally not described as a general Jinja semantic-equivalence proof. A new or changed Local artifact/template requires an explicit signature and fixture review.

## P12 decision

No P12 rerun is scheduled for P6 because the actual rendered SmolLM2/Qwen prompt framing did not change. P6 corrects ownership claims and tightens compatibility identification only. If a later change alters rendered prompt bytes or control-token policy, P12 must be reconsidered before final closure.

## Closure result

`LLMR-600`, `LLMR-602`, and `LLMR-603` satisfy every listed task and acceptance item. `LLMR-601` satisfies the tracker’s alternate no-current-requirement branch, with its implementation-only branch explicitly resolved as N/A. The P6 section therefore has no unresolved checklist item after this closeout.
