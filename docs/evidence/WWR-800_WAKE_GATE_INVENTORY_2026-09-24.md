# WWR-800 — Wake Word required-gate inventory

Date: 2026-09-24
Base master: `8ab53480ebae0d4f8e447d3d7a4a413884d3768b`
Scope: gate inventory and gap analysis only. This does not define final merge policy by itself and does not mark WWR-800 complete.

## Purpose

WWR-800 requires Wake Word V1 to have specialized gates beyond ordinary CI. This inventory records the current known gate state so final qualification cannot accidentally treat ordinary CI as sufficient.

## Current ordinary gates

### `CI`

Observed latest exact-master success:

- master SHA: `8ab53480ebae0d4f8e447d3d7a4a413884d3768b`
- run ID: `35985747766`
- conclusion: success

Use: ordinary repository build/test/lint signal.

Limitation: ordinary CI is not Wake Word final acceptance. It does not by itself satisfy deterministic corpus, real native KWS, platform acceptance, performance, lifecycle soak, documentation audit, or final privacy/security audit requirements.

## Current Wake-related specialized gates/evidence paths

### Wake artifact verification

Existing evidence from earlier remediation work records model/runtime identity freezer and artifact verification coverage, including non-placeholder identity checks, manifest validation, safe extraction/path traversal rejection, and cache re-verification.

Current status: useful specialized gate family exists for artifacts/runtime identity, but final WWR-800 still must ensure the required artifact/package gate is exact-head bound for the final feature head and is required by final merge policy.

### Wake Word lifecycle stability

The Wake lifecycle stability workflow exists and, after PR #423, watches the dedicated Wake modules and the conversation-session side of the Wake-to-command-ASR boundary. Exact-master gate evidence for the path-filter expansion:

- master SHA: `145bf21d9baeb2b5f58fb50ae079e3094d11f4c6`
- ordinary CI run: `35983938327`, success
- Wake lifecycle stability run: `35983938243`, success
- Wake source-security audit run: `35983938252`, success
- Wake required-gates audit run: `35983938148`, success

Current status: deterministic lifecycle gate exists and relevant conversation paths are covered, but final WWR-640 acceptance still requires the defined integrated acceptance/soak scope and exact-head/exact-master evidence for the final implementation.

### Wake source security/privacy audit

Existing source audit evidence covers deterministic/source-level privacy and security concerns, including raw PCM exclusion from diagnostics, sanitized errors, artifact identity checks, architecture checks, no silent full-ASR fallback, and documentation overclaim prevention. Current source/privacy/security audit pass 1 is recorded in `docs/evidence/WWR-900_SOURCE_PRIVACY_AUDIT_PASS1_2026-09-24.md`.

Current status: audit evidence exists as an input to final WWR-900, but final source/privacy/security audit must be rerun against the exact final feature head.

### Wake required-gates audit

The Wake required-gates audit workflow exists and passed on the WWR-800 path-filter expansion merge. It is useful for preventing skipped/ordinary-only specialized Wake qualification from being mistaken for completion.

Current status: useful gate-policy audit exists, but final WWR-800 still must ensure all required gates are implemented and final merge eligibility requires the exact gate set listed below.

## Required gates still missing or incomplete

### Deterministic corpus CI/validation gate

Required by WWR-600/800/950.

Current gap:

- PR #395 adds a real KWS acceptance harness/corpus path, but that branch is not merged;
- the unmerged branch passed Wake Word real KWS acceptance at `1399dca15cd30073e13957b021b72d5ee2b7f0d5`, but ordinary CI remains blocked by the RustSec `rustls` lockfile audit failure;
- no merged final corpus version is recorded for final qualification;
- no merged final report proves positive detections, false rejects, and negative false accepts across the required corpus categories.

### Linux x86_64 real KWS acceptance gate

Required by WWR-610/800/950.

Current gap:

- PR #395 has exact-head real-KWS evidence but remains unmerged;
- final merged evidence must include exact commit, manifest revision, runner/platform details, run ID, positive fixture, negative fixture, offline inference result, one-thread policy, CPU-only path, and privacy-safe diagnostics.

### macOS arm64 real KWS acceptance gate

Required by WWR-620/800/950.

Current gap:

- PR #395 has exact-head real-KWS evidence but remains unmerged;
- final merged evidence must include exact commit, manifest revision, runner/platform details, run ID, positive fixture, negative fixture, offline inference result, one-thread policy, CPU-only path, and privacy-safe diagnostics.

### Native packaging/architecture gate

Required by WWR-800/950.

Current gap:

- artifact tooling verifies runtime identity and architecture, but final packaging/architecture gate must be explicitly bound to the final feature head and must not be silently skipped when packaging inputs change.

### Performance evidence gate/report policy

Required by WWR-630/800/950.

Current gap:

- optional diagnostics placeholders exist;
- final idle CPU, memory overhead, inference timing, wake-to-ASR latency, pre-roll replay/startup timing, repeated-cycle resource behavior, and KWS-versus-full-ASR comparison are not yet recorded on merged master;
- final policy must define which performance report is required and how it is bound to an exact head.

### Final documentation audit gate

Required by WWR-700/900/950/960.

Current gap:

- `docs/WAKE_WORD_V1.md` and WWR-700 audit intentionally avoid unsupported claims;
- final README/user-facing update remains deferred until the feature is usable;
- final documentation audit must run after production wiring and native acceptance so user-facing support statements match actual master behavior.

## Exact-head policy required before final closeout

Final Wake Word V1 qualification must require, at minimum:

1. ordinary CI on exact final PR head;
2. deterministic corpus gate on exact final PR head;
3. Linux x86_64 real KWS gate on exact final PR head or exact approved platform head;
4. macOS arm64 real KWS gate on exact final PR head or exact approved platform head;
5. native packaging/architecture gate on exact final PR head;
6. integrated lifecycle stability gate on exact final PR head;
7. performance evidence report bound to exact final PR head;
8. privacy/security audit bound to exact final PR head;
9. documentation audit bound to exact final PR head;
10. guarded merge using only the tested head SHA;
11. exact-master ordinary CI after merge;
12. exact-master Wake gates required by final diff/policy after merge.

A skipped gate must never be treated as passed. A gate that is not configured for a platform must be recorded as unavailable/incomplete, not as success.

## Current WWR-800 conclusion

This inventory advances WWR-800 planning and prevents ordinary CI from being confused with final Wake qualification. It does not close WWR-800. The remaining work is to merge the corpus/native acceptance branch after its dependency-audit blocker is resolved, implement/record the missing performance/package/documentation gate definitions, wire them into required final qualification policy, and record exact-head/exact-master run IDs during WWR-950/960.
