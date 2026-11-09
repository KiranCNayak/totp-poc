# CLAUDE.md

Guidance for AI coding sessions in this repo.

## What this is

A learning proof of concept for TOTP (RFC 6238): a Rust axum server ("MyApp": enroll + verify) and a self-built browser authenticator, both served on http://localhost:3000. HOTP/TOTP is implemented by hand on top of the `hmac` and `sha1` crates. Not meant for production.

## Commands

```sh
cargo build
cargo test     # RFC 4226 / RFC 6238 test vectors in src/totp.rs
cargo run      # serves on port 3000: / and /authenticator
```

## Layout

- `src/totp.rs`: `generate_secret`, `hotp`, `totp`, `verify` (±1 step window, constant-time compare, replay check via last accepted time step) and unit tests.
- `src/main.rs`: axum routes per `docs/api-contract.md`; in-memory `Arc<Mutex<HashMap<String, User>>>`; QR as SVG via the `qrcode` crate; pages embedded with `include_str!`.
- `static/index.html`: "MyApp" (enroll, show QR + URI, verify).
- `static/authenticator.html`: "My Authenticator" (parses `otpauth://`, computes codes with Web Crypto).
- `docs/api-contract.md`: server <-> pages contract. `docs/adr/`: decisions. `docs/rust-notes.md`: Rust notes.

## Invariants

- `docs/api-contract.md` is the source of truth for routes and JSON shapes between server and pages. Change it first (and both sides together) if the API must change.
- TOTP parameters: SHA-1, 6 digits, 30 s step, T0 = 0, verification window ±1 step. These are chosen for authenticator-app compatibility (ADR 0002); don't change them casually.
- Secrets: 20 random bytes, Base32 (RFC 4648) **without** `=` padding.
- Reject replays: a time step already accepted for a user must not be accepted again. Compare codes in constant time.
- Pages stay dependency-free vanilla JS (no frameworks, no CDN scripts, no QR library: the server renders the QR as SVG).
- Keep the RFC test vectors in `src/totp.rs` passing. Don't replace the hand-written algorithm with a TOTP crate (ADR 0001).
- Decisions worth recording go in `docs/adr/` (short Nygard format) and the index `docs/adr/README.md`.

## Working with the owner

The owner is learning Rust and TOTP. Explanations of Rust concepts (ownership, borrowing, `Arc`/`Mutex`, traits, async, error handling, macros) and of the "why" behind TOTP details are welcome. Prefer clear, idiomatic code over clever code.
