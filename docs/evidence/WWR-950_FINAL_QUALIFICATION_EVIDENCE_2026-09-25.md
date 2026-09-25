# WWR-950 Final Qualification Evidence — 2026-09-25

**Initial exact master:** `201dabffcef346ec4dd85baafa67d8b37f531ba9`

This evidence file exists to bind final Wake Word V1 qualification to an exact PR head and exact-master follow-up evidence.

## Scope

The final qualification slice is evidence-only plus workflow-comment trigger updates. It does not alter production runtime, ASR, TTS, model/runtime manifests, corpus fixtures, artifact identities, or acceptance thresholds.

## Required exact-head gates to record

- ordinary CI
- deterministic corpus manifest and corpus contract
- Linux x86_64 and macOS arm64 real KWS acceptance
- native packaging / architecture policy
- integrated lifecycle stability
- performance evidence validation
- privacy/security source audit
- documentation audit
- required-gates audit
- Wake artifact verification

## Notes

The workflow-comment changes are intentionally behavior-neutral and exist only to select every mandatory final-closeout workflow through normal pull-request and post-merge push path filters. A skipped workflow is not counted as passing evidence.
