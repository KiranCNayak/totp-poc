# 5. axum server, in-memory store, embedded pages, server-side QR

Status: Accepted

Date: 2026-11-10

## Context

The POC needs a small HTTP server with two JSON endpoints and two HTML pages, minimal setup (ideally just `cargo run`), and pages that stay simple enough to read in one sitting.

## Decision

- **Server:** axum on the tokio async runtime. It is a widely used, well-documented Rust web framework with typed JSON extraction via serde, which keeps handlers short.
- **Storage:** an in-memory `HashMap<String, User>` wrapped in `Arc<Mutex<...>>` and shared as axum state. `Arc` lets every handler hold a reference to the same map; `Mutex` ensures one handler at a time reads or modifies it (including the replay check in ADR 0003). No database.
- **Pages:** `static/index.html` and `static/authenticator.html` are compiled into the binary with `include_str!`, so the server needs no static-file directory at runtime.
- **QR code:** rendered on the server as an SVG string with the `qrcode` crate and returned in the enroll response. The page inserts it directly, so no JavaScript QR library is needed.
- **Authenticator page:** plain JavaScript with the browser's Web Crypto API (`crypto.subtle`) for HMAC-SHA1, plus a small hand-written Base32 decoder. No libraries.

## Consequences

- One binary, one command, no external services.
- All users and secrets are lost when the server restarts. Acceptable for a demo; re-enroll after restart.
- A single global `Mutex` serializes all requests touching the store. Fine at demo scale, not for real load.
- Editing an HTML page requires a rebuild (`cargo run` again) because the page is baked into the binary.
- The authenticator page only works in a secure context (https or localhost), because `crypto.subtle` is unavailable elsewhere. Opening it from a phone over `http://<LAN-IP>` fails.
- The pages stay dependency-free and readable end to end.
