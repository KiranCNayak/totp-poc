# 3. ±1 step verification window, constant-time compare, replay check

Status: Accepted

Date: 2026-11-10

## Context

If the server only accepted the code for the exact current time step, users would regularly fail: a code read at second 29 of a step and submitted at second 31 belongs to the previous step, and phone clocks drift from server clocks.

RFC 6238 §5.2 recommends allowing at most one previous step to cover transmission delay, and says an OTP that has been accepted once must not be accepted again. §6 discusses tolerating a few steps of clock drift in either direction. Accepting the previous, current, and next step (±1) is the widespread convention.

Two further concerns:

- **Replay.** Within the window a code stays valid for up to ~90 s. Someone who sees it (shoulder surfing, phishing proxy, logs) could reuse it.
- **Timing.** A comparison that stops at the first differing character leaks, through response time, how many leading digits were right.

## Decision

- `verify` checks the submitted code against the codes for steps `T-1`, `T`, `T+1`.
- Each comparison is constant-time (no early exit on mismatch).
- Each user record stores the last accepted time step. A code whose matching step is not newer than the last accepted step is rejected with `"Code already used"`. On success the stored step is updated.
- A code outside the window is rejected with `"Invalid code"`.

## Consequences

- Normal delays and small clock drift are tolerated; drift beyond ~30 s will cause failures (we don't do per-user drift tracking or resynchronization).
- Each code can be used at most once, and after a login the user must wait for the next step to log in again. This is the expected TOTP behaviour.
- Three candidate codes are valid at any moment, which slightly improves a guessing attacker's odds (3 in 10^6 per attempt). Rate limiting would be the real fix; not in scope.
- The "last accepted step" must be read and updated atomically with verification. Here the `Mutex` around the user store guarantees that.
