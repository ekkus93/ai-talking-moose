# MCR-720 Placeholder settings rollback test remediation evidence

**Evidence source SHA:** `3da82eace72e45613918bba1b029e4f408e2e28e`
**Recorded:** 2026-09-30

## Substantive rollback regression

`src/stores/mooseStore.settingsRollback.test.ts` is no longer a pass-by-construction placeholder. It forces `tauriBridge.updateSettings` to reject, supplies a different authoritative persisted snapshot through `getSettings`, invokes the production `updateSettingsPatch` path, and asserts all of the following falsifiable behavior:

- the caller receives the typed `rolled_back` result and bounded message;
- the failed candidate write was actually attempted;
- authoritative settings were reloaded exactly once;
- store state equals the authoritative persisted snapshot after reconciliation; and
- the rejected optimistic value is absent after rollback.

Additional rollback/result and queue-rebase coverage exists in `src/test/mooseStore.test.ts`.

## Repository placeholder policy

`scripts/check_no_placeholder_tests.mjs` recursively scans frontend production test/spec files and rejects known pass-by-construction forms including `expect(true).toBe(true)` and `expect(false).toBe(false)`. The checker is wired into ordinary CI, so a regression is a CI failure rather than a review-only convention.

## Qualification

Ordinary CI passed on implementation source SHA `5be626f0c4581301d73a8c8e148638c29d34e603` as run `36803769184`. The documentation-only MCR-710 evidence commit advanced master to `3da82eace72e45613918bba1b029e4f408e2e28e`; exact-head CI run `36804762038` was in progress when this evidence was prepared. Final MCR-950/MCR-960 qualification remains bound to the eventual final exact head/master.

## MCR-720 conclusion

The MCR-720 implementation and substantive rollback regression requirements are satisfied at the evidence source SHA. Final closeout still requires the final exact-head placeholder policy gate and exact-master verification.
