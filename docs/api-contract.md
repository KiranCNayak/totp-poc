# API contract

The Rust server (`src/main.rs`) and the two HTML pages (`static/`) agree on this contract.

Server listens on `http://localhost:3000`.

## Pages

| Route | Serves | Purpose |
|---|---|---|
| `GET /` | `static/index.html` | "MyApp": enroll a user, show QR + URI, verify a code |
| `GET /authenticator` | `static/authenticator.html` | "My Authenticator": a browser-based authenticator app |

Pages are embedded into the binary with `include_str!`, so the server needs no static file directory at runtime.

## `POST /api/enroll`

Request:

```json
{ "username": "alice" }
```

Response `200`:

```json
{
  "username": "alice",
  "secret": "JBSWY3DPEHPK3PXP...",
  "otpauth_uri": "otpauth://totp/TOTP-POC:alice?secret=...&issuer=TOTP-POC&algorithm=SHA1&digits=6&period=30",
  "qr_svg": "<svg ...>...</svg>"
}
```

- `secret` is 20 random bytes, Base32 (RFC 4648) without `=` padding.
- Enrolling an existing username replaces the old secret.
- `400` with `{ "error": "..." }` if the username is empty.

## `POST /api/verify`

Request:

```json
{ "username": "alice", "code": "123456" }
```

Response `200`, always JSON:

```json
{ "ok": true, "message": "Code accepted" }
```

`ok` is `false` with one of these messages:

- `"Unknown user"`
- `"Invalid code"` (wrong code, or outside the T-1..T+1 window)
- `"Code already used"` (replay: its time step was already accepted)

## TOTP parameters

SHA-1, 6 digits, 30-second step, T0 = 0, verification window ±1 step.
