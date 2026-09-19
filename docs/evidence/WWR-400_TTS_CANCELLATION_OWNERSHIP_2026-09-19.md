# WWR-400 TTS cancellation ownership — 2026-09-19

Base commit: `21f2a71585c751797ff701fcf7e2ef9a362ac699`

## Production race reproduced by source inspection

The standalone speech controller stores one authoritative `CancellationToken`. `begin()` cancels the previous token and replaces it. `cancel_if_current()` cancels the token while leaving that same token in the slot. `is_current()` deliberately returns false for a cancelled token.

That means a terminal TTS cleanup cannot use `is_current()` to distinguish these two materially different cases:

1. **current utterance cancelled** — Wake Word must resume when still enabled;
2. **old utterance superseded by a newer utterance** — the old cleanup must not resume Wake Word underneath the newer Talking playback.

Blindly resuming on every cancelled completion violates the no-self-wake requirement. Never resuming on cancellation leaves Wake Word permanently suspended after a real cancellation. This is the ownership condition identified in the preceding TTS lifecycle audit.

## Required atomic ownership primitive

`StandaloneSpeechController` needs a read-only ownership predicate that compares token identity without rejecting a cancelled token, for example `owns_slot(token)`. It must take the same controller lock used by `begin`, `cancel`, `cancel_if_current`, and `with_current`.

The intended semantics are:

- active current token: `is_current == true`, `owns_slot == true`;
- cancelled current token: `is_current == false`, `owns_slot == true`;
- superseded old token: `is_current == false`, `owns_slot == false`.

This is not a second speech owner; it exposes identity of the existing authoritative slot solely for terminal cleanup.

## TTS lifecycle wiring contract

For non-ambient standalone speech:

1. Suspend Wake Word before publishing `CharacterState::Talking`.
2. If publishing Talking fails, restore Wake Word immediately.
3. On successful playback completion, restore Wake Word only from the current playback completion path.
4. On cancellation, restore Wake Word only when the cancelled playback still `owns_slot`.
5. If a newer utterance superseded it, the old cleanup performs no Wake Word transition.
6. If the user disabled Wake Word during Talking, the runtime is already `Disabled`; terminal cleanup must not force it back to Listening.
7. A suspension failure must fail closed for wake activation (Error/Disabled), without breaking manual speech.

Ambient speech has its own presentation lease and cancellation guard. It must be wired in the same atomic change as its completion/cleanup path; partially suspending ambient speech without lease-aware resume would create a new permanent-suspension regression.

## Required regression tests

- successful current playback: Listening -> SuspendedTalking -> Listening;
- current playback cancellation: SuspendedTalking -> Listening when enabled;
- disable during Talking: terminal cleanup remains Disabled;
- old playback superseded by newer playback: old cleanup cannot resume Wake Word;
- Talking surfacing failure: suspension is restored;
- ambient cancellation/supersession obeys both presentation lease ownership and standalone token ownership.

## Acceptance impact

WWR-400 remains open. This evidence fixes the exact cancellation/supersession invariant that the production patch must preserve and prevents implementing the superficially simple but incorrect "resume on any cancellation" behavior.