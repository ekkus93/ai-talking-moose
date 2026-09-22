# WWR-800 required Wake gates inventory evidence — 2026-09-22

## Scope

This evidence records the current Wake Word V1 required-gates inventory and audit policy. It distinguishes implemented policy/source gates from pending specialized acceptance gates and does not treat skipped workflows as passing evidence.

## Source evidence

`docs/wake-word-required-gates.json` defines the final-closeout gate inventory. The manifest states:

- ordinary CI alone is not final Wake Word V1 qualification;
- skipped workflow conclusions do not count as pass;
- every final-closeout gate requires exact-head evidence;
- implemented gates reference concrete workflow files;
- pending gates do not point at a passing workflow until specialized/integrated acceptance exists.

Implemented gates currently include ordinary CI, required-gates manifest audit, deterministic corpus manifest/contract gates, native packaging/architecture policy gate, lifecycle stability policy gate, performance evidence policy gate, privacy audit, documentation audit, and source/security ownership audit.

Pending gates remain explicit for Linux real KWS acceptance, macOS real KWS acceptance, integrated production lifecycle acceptance, and measured performance acceptance.

`scripts/check_wake_word_required_gates.mjs` audits the manifest and fails closed when required gates are missing, implemented workflows are absent, `continue-on-error: true` is present, path-filtered skipped conclusions are treated as pass, or documentation omits required truthfulness statements.

`.github/workflows/wake-word-required-gates.yml` runs the manifest audit on pull requests and pushes that touch the manifest, CI-gates documentation, the audit script, or workflow files.

## Validation

Exact merged-master validation for `412c246c197b342f2fa71eb38b76655da2f8efaf` passed ordinary CI `35766454955`.

## Non-claims

This evidence does not claim the pending specialized acceptance gates have passed. Linux/macOS real KWS inference, integrated production lifecycle soak, and measured performance acceptance remain open until exact-head evidence exists for those gates.

This evidence also does not claim final qualification or closeout. WWR-950 and WWR-960 remain open.