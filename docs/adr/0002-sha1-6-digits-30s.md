# 2. Use SHA-1, 6 digits, 30-second step

Status: Accepted

Date: 2026-11-10

## Context

RFC 6238 allows HMAC-SHA1, HMAC-SHA256, or HMAC-SHA512, a configurable number of digits, and a configurable time step. The `otpauth://` URI can carry `algorithm`, `digits`, and `period` parameters.

In practice, many authenticator apps ignore those parameters (or support only some values) and always compute SHA-1, 6 digits, 30 s. If the server used anything else, those apps would silently show wrong codes. We want the QR code to work with Google Authenticator and similar apps as a cross-check.

SHA-1 is broken for collision resistance (practical collisions have been demonstrated). HMAC's security, however, does not depend on collision resistance of the hash; it relies on the hash's compression function behaving like a pseudorandom function. HMAC-SHA1 is still considered secure for this use, and it is what RFC 4226 specifies.

## Decision

Fix the parameters to SHA-1, 6 digits, 30-second step, T0 = 0. The server still writes `algorithm=SHA1&digits=6&period=30` into the `otpauth://` URI so the values are explicit.

## Consequences

- Works with essentially every authenticator app, including our own browser authenticator.
- The RFC test vectors for SHA-1 apply directly.
- 6 digits means 10^6 possible codes; combined with the ±1 window (ADR 0003), brute force is only prevented by rate limiting, which this POC doesn't have (listed as not production-ready).
- Switching algorithms later would require re-enrolling users, since the algorithm is bound to the secret in their app.
