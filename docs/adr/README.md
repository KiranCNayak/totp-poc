# Architecture decision records

Short records of why the project is built the way it is, in the Nygard format (Context, Decision, Consequences). Add a new numbered file for each new decision; don't rewrite accepted ones, supersede them.

| # | Title | Status | Date |
|---|---|---|---|
| [0001](0001-implement-totp-from-rfc.md) | Implement HOTP/TOTP ourselves from the RFCs | Accepted | 2026-11-10 |
| [0002](0002-sha1-6-digits-30s.md) | Use SHA-1, 6 digits, 30-second step | Accepted | 2026-11-10 |
| [0003](0003-verification-window-and-replay.md) | ±1 step verification window, constant-time compare, replay check | Accepted | 2026-11-10 |
| [0004](0004-no-domain-separation-in-secret.md) | No domain separation (HKDF derivation) in the secret | Accepted | 2026-11-10 |
| [0005](0005-axum-in-memory-store-embedded-pages.md) | axum server, in-memory store, embedded pages, server-side QR | Accepted | 2026-11-10 |
