---
name: phase-gate-check
description: Audit a numbered project phase (P0-P13+) against its SPEC/TODO acceptance criteria and existing RECONCILIATION doc, and report gaps. Use when the user asks to check, close, or reconcile a phase gate, or asks "is P<N> done?".
---

# Phase gate check

Talking Moose AI tracks delivery in numbered phases against `docs/SPEC(*).md` and
`docs/TODO(*).md`, with `docs/RECONCILIATION_P<N>_<date>.md` files recording the
acceptance-criteria audit for each phase. This skill reproduces that audit.

## Inputs

`$ARGUMENTS` names the phase, e.g. `P13` or `P2 P3` (some reconciliations cover more
than one phase at once — check existing filenames like
`RECONCILIATION_P2_P3_PHYSICAL_20260823.md` for that pattern). If no phase is given,
ask which phase to check.

## Steps

1. **Find the sources of truth.** Locate the relevant `docs/SPEC(*).md` / `docs/TODO(*).md`
   entries for the phase, and any existing `docs/RECONCILIATION_P<N>_*.md` file(s) for it
   (there may be more than one, dated — the most recent is the current state).
2. **Enumerate required items.** Pull every acceptance-criteria item for the phase out of
   the SPEC/TODO doc. Note anything explicitly marked as a manual/physical check (e.g.
   macOS hardware acceptance) — those stay open until actually run; never mark them
   complete from source inspection alone.
3. **Check current state against each item.** For each item, inspect the actual source,
   tests, CI workflows, and scripts referenced (not just doc prose) to determine PASS /
   FAIL / SKIP(deferred). Re-run or point to the relevant `npm run check:*` / `cargo test`
   / verification script output where that's what the item requires.
4. **Diff against the existing RECONCILIATION doc**, if one exists: what it already
   recorded as done vs. what's newly true or newly broken on current `master`.
5. **Report gaps** in the same style as the existing RECONCILIATION docs: a short status
   line, then per-requirement-ID findings (what's implemented, what's still open, what's
   explicitly deferred and why). Do not mark a physical/manual acceptance row complete
   just because the source-level work behind it is done.
6. If asked to close the gate, offer to write/update the `docs/RECONCILIATION_P<N>_<date>.md`
   file with the findings, following the structure of the most recent existing one — but
   confirm with the user before creating a new dated reconciliation file.
