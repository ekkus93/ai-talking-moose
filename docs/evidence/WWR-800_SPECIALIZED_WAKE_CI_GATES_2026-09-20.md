# WWR-800 — Specialized Wake CI gate evidence

Evidence head reviewed: `beee25b095185b36fb8c17e819073851dc7f4cd9`.

## Gates present on master

Wake Word validation is no longer represented by ordinary CI alone. The repository contains dedicated exact-ref workflows for distinct Wake acceptance boundaries:

- `.github/workflows/wake-word-corpus.yml` validates the deterministic corpus manifest and fixture policy when corpus inputs change and is manually dispatchable.
- `.github/workflows/wake-word-lifecycle-stability.yml` runs the focused `wake_word_stability` Rust acceptance suite on Wake lifecycle/runtime changes and is manually dispatchable.
- `.github/workflows/wake-word-performance-evidence.yml` validates the versioned performance evidence policy/report and is manually dispatchable. Its checker explicitly distinguishes a valid evidence schema from completed measurements; pending measurements cannot be represented as accepted performance evidence.
- `.github/workflows/wake-word-native-packaging.yml` validates frozen artifact/runtime identities and target architecture policy on both `ubuntu-latest` / Linux x86_64 and `macos-15` / macOS arm64. Its job summary explicitly states that packaging/architecture validation is not real KWS inference acceptance.
- `.github/workflows/wake-word-privacy-audit.yml` enforces the privacy/security diagnostics boundary.
- `.github/workflows/wake-word-documentation-audit.yml` enforces truthful Wake documentation claims.

Each workflow checks out the triggering ref/commit and is triggered by relevant pull-request/push paths (plus `workflow_dispatch` where defined), so evidence is attributable to an exact Git head rather than a floating workspace.

## Passing merged-master evidence

Recent merged-master specialized evidence includes:

- Wake Word lifecycle stability run `35485439525` on `fd3b13505db116532e43a6271f6e3176cc10dcdd`: success.
- Wake Word performance evidence policy run `35486070649` on `ed3e12385c6ad1a3a623dfbb0c34ad4bbfe630f9`: success; the log states `measurements remain pending`, so this does not close WWR-630.
- Wake Word native packaging architecture run `35486871769` on `374dc8bad85d3d05a78bb9f4cbbc112979fce81d`: success.
- Wake Word privacy audit run `35487357710` on `01a45ce95720eac25f7fa8f22b788eaadba1786a`: success.
- Wake Word documentation audit run `35487357681` on `01a45ce95720eac25f7fa8f22b788eaadba1786a`: success.

## Acceptance boundary still open

This evidence closes the *definition* of the deterministic corpus, native packaging/architecture, repeated lifecycle, performance-report-policy, privacy, and documentation specialized gates. It does not convert skipped or unimplemented real-inference acceptance into a pass.

WWR-610 Linux x86_64 real positive/negative KWS inference and WWR-620 macOS arm64 real positive/negative KWS inference remain open. The corpus manifest also still has no real redistributable positive/negative audio fixtures, and WWR-630 measured performance values remain pending. Consequently final feature qualification must continue to reject ordinary-CI-only evidence and must not claim WWR-950 complete until those real acceptance gates have passing exact-head evidence.
