# MCR-710 Frontend event-listener exception-safety evidence

**Evidence source SHA:** `5be626f0c4581301d73a8c8e148638c29d34e603`
**Recorded:** 2026-09-30

## Implementation

`src/stores/mooseStore.ts` now registers frontend event listeners through a single helper that records each disposer immediately after successful registration. A shared idempotent cleanup path drains the recorded disposers in reverse order, invokes each at most once, and catches cleanup exceptions so the original initialization failure is replaced only by the bounded privacy-safe `Application event listeners could not be initialized.` error. If cleanup occurs while a registration is pending, a late disposer is invoked immediately instead of escaping as a leaked handler.

`src/windows/MooseWindow.tsx` handles rejected `loadSettings` / `initEventListeners` startup work and disposes a listener set that resolves after component unmount, preventing unhandled startup rejection and late-registration leaks.

## Deterministic regression coverage

`src/test/mooseStore.test.ts` exercises partial-registration failure at the first registration (`moose://state`), a middle registration (`moose://transcript/moose`), and the final registration (`moose://tray/action`). Every previously registered disposer is asserted to run exactly once. The same suite verifies successful cleanup is idempotent.

`src/test/MooseWindow.shortcuts.test.tsx` verifies that a listener-registration promise resolving after unmount has its returned cleanup invoked exactly once, and verifies that listener setup failure surfaces only the sanitized user-facing message rather than private failure detail.

## Exact-head CI

GitHub Actions ordinary CI run `36803769184` passed on exact source SHA `5be626f0c4581301d73a8c8e148638c29d34e603`. The same exact head also passed Wake source-security audit run `36803769116`, Wake real KWS acceptance run `36803769150`, and Wake performance evidence run `36803769106`.

## MCR-710 conclusion

The implementation and deterministic tests satisfy the MCR-710 task/test requirements at the evidence source SHA. Final MCR-950/MCR-960 closeout still requires qualification of the eventual final exact head/master; this evidence does not substitute for those final gates.
