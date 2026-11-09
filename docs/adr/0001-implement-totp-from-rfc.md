# 1. Implement HOTP/TOTP ourselves from the RFCs

Status: Accepted

Date: 2026-11-10

## Context

Rust has ready-made TOTP crates (for example `totp-rs`) that handle secret generation, `otpauth://` URIs, and verification. The main goal of this project, though, is to learn how TOTP works and to learn Rust along the way. A library would hide exactly the part we want to understand.

The algorithm itself is small: HMAC-SHA1 over an 8-byte counter, dynamic truncation, modulo 10^6, with the counter derived from the clock. Both RFCs publish test vectors (RFC 4226 Appendix D for HOTP, RFC 6238 Appendix B for TOTP) that make it easy to check a hand-written implementation.

## Decision

Implement `hotp`, `totp`, and `verify` in `src/totp.rs` ourselves, using only low-level crates for the cryptographic primitives: `hmac` + `sha1` for HMAC-SHA1, `rand` for random secrets, `data-encoding` for Base32. We do not write our own HMAC or SHA-1.

Unit tests check the implementation against the RFC 4226 and RFC 6238 (SHA-1) test vectors.

## Consequences

- The core algorithm is visible and readable in about 20 lines, which serves the learning goal.
- Correctness is anchored by the official test vectors, and can additionally be cross-checked against Google Authenticator via the QR code.
- We own the details a library would handle for us (truncation, zero-padding, window and replay logic). Mistakes there are our responsibility, and the tests must stay green.
- Only SHA-1 / 6 digits / 30 s is supported (see ADR 0002); adding SHA-256/512 or 8 digits would be extra work.
