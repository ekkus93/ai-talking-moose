# MCR-720 Placeholder settings rollback test remediation evidence

**Implementation source SHA:** `5be626f0c4581301d73a8c8e148638c29d34e603`
**Latest qualified master SHA:** `4296c35823c8a094632aa7bba7f12dfcfa7b5844`
**Recorded:** 2026-09-30
**Revalidated:** 2026-10-01

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

Initial ordinary CI passed on implementation source SHA `5be626f0c4581301d73a8c8e148638c29d34e603` as run `36803769184`. The documentation-only MCR-710 evidence commit advanced master to `3da82eace72e45613918bba1b029e4f408e2e28e`; the latest direct-to-master head `4296c35823c8a094632aa7bba7f12dfcfa7b5844` revalidated the accumulated remediation through ordinary CI run `36826031723` and Wake source-security audit run `36826031739`; both passed.

The current `master` still contains the substantive rollback test replacement and the placeholder-test policy checker. Later commits between the implementation source SHA and latest qualified master did not reintroduce the placeholder test.

## MCR-720 conclusion

The MCR-720 implementation and substantive rollback regression requirements are satisfied. Final MCR-950/MCR-960 qualification remains bound to the eventual final exact head/master.
