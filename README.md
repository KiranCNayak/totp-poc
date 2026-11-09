# TOTP proof of concept

A small, self-contained demo of **time-based one-time passwords** (TOTP, RFC 6238), the 6-digit codes that apps like Google Authenticator show for two-factor login.

It has two sides, both served by one Rust ([axum](https://github.com/tokio-rs/axum)) server:

- **MyApp** (`/`): a pretend website. You enroll a username, it generates a secret and shows it as a QR code and an `otpauth://` URI, then you can verify 6-digit codes against it.
- **My Authenticator** (`/authenticator`): a self-built authenticator app in plain JavaScript. Paste the `otpauth://` URI in and it shows the current code with a countdown, computed in the browser with the Web Crypto API.

The HOTP/TOTP algorithm is implemented from the RFCs (not pulled from a TOTP crate) and checked against the official test vectors. This is a learning project, not something to deploy. See [Not production-ready](#not-production-ready).

## Quick start

Requires a Rust toolchain (`rustup`, `cargo`).

```sh
cargo test    # runs the RFC 4226 / RFC 6238 test vectors
cargo run     # starts the server on http://localhost:3000
```

Then open these two side by side in a browser:

- http://localhost:3000 (MyApp)
- http://localhost:3000/authenticator (My Authenticator)

## Demo walkthrough

1. **Enroll.** In MyApp, type a username (for example `alice`) and click enroll. The server creates a random 160-bit secret and returns it as Base32 text, as an `otpauth://totp/...` URI, and as a QR code.
2. **Add to the authenticator.** Copy the `otpauth://` URI and paste it into My Authenticator. It parses the URI, decodes the Base32 secret, and shows a 6-digit code that changes every 30 seconds.
3. **Verify.** Type the current code into MyApp's verify form. Expected results:
   - the current code gives `Code accepted`;
   - submitting the same code again gives `Code already used` (replay protection);
   - a made-up code gives `Invalid code`;
   - an unknown username gives `Unknown user`.
4. **Optional cross-check with a real app.** Scan the QR code in MyApp with Google Authenticator (or any TOTP app). It should show the same code as My Authenticator at the same moment. That confirms both our server and our JS authenticator agree with an independent implementation.

Enrolling the same username again replaces its secret, so old authenticator entries stop working.

## Project layout

```
Cargo.toml               dependencies (axum, tokio, serde, hmac, sha1, data-encoding, rand, qrcode)
src/totp.rs              the algorithm: generate_secret, hotp, totp, verify (±1 step window,
                         constant-time compare, replay check); unit tests with RFC vectors
src/main.rs              axum server: routes, in-memory user store, QR rendering, embedded pages
static/index.html        "MyApp": enroll + verify UI
static/authenticator.html "My Authenticator": browser authenticator using Web Crypto
docs/api-contract.md     the HTTP API shared by server and pages (source of truth)
docs/adr/                architecture decision records (why things are the way they are)
docs/rust-notes.md       notes on the Rust concepts used in this project
```

Some details of `src/main.rs`:

- Users live in an `Arc<Mutex<HashMap<String, User>>>`: a hash map shared between request handlers (`Arc`) and guarded so only one handler changes it at a time (`Mutex`). Each `User` holds the secret and the last accepted time step. Everything is lost when the server stops.
- The QR code is rendered on the server as an SVG string (`qrcode` crate) and returned in the JSON, so the page just inserts it. No JS QR library needed.
- The two HTML pages are compiled into the binary with `include_str!`, so `cargo run` is all you need.

Routes are described in [docs/api-contract.md](docs/api-contract.md).

## How TOTP works

### The shared secret

At enrollment the server generates 20 random bytes (160 bits) and shares them once with the user's authenticator app, typically through a QR code holding an `otpauth://` URI. The secret is written in Base32 (letters A-Z and digits 2-7) because it survives being typed by hand or embedded in a URL. After that, server and app never talk to each other again: both compute codes independently from the same secret and the same clock.

### HOTP: the building block (RFC 4226)

HOTP is the **HMAC-based One-Time Password**. It turns a secret and a counter into a short number:

1. **HMAC.** Compute `HMAC-SHA1(secret, counter)`, where the counter is an unsigned 64-bit integer written as 8 bytes, big-endian. The result is 20 bytes.
2. **Dynamic truncation.** Pick 4 of those 20 bytes:
   - `offset = last_byte & 0x0F` (a number 0..15, taken from the low 4 bits of byte 19);
   - read bytes `offset..offset+4` as a big-endian 32-bit integer;
   - `& 0x7FFFFFFF` to clear the top bit, so the value is a non-negative 31-bit number regardless of signed/unsigned handling across languages.
3. **Reduce to digits.** `code = value mod 10^6`, then left-pad with zeros to 6 digits (`"004211"` is a valid code).

The offset is "dynamic" because it depends on the HMAC output itself, so an attacker can't predict which bytes end up in the code.

In Rust this is roughly 20 lines; see `hotp` in `src/totp.rs`.

### TOTP: HOTP with the clock as the counter (RFC 6238)

TOTP is just HOTP where the counter is derived from the current time:

```
T = floor(unix_time / 30)
code = HOTP(secret, T)
```

(`unix_time` is seconds since 1970-01-01 UTC; the RFC allows a different start time T0 and step size, we use T0 = 0 and 30 s.)

Why divide by 30?

- **Stability.** The code has to stay the same long enough for a person to read it on one device and type it into another.
- **Clock tolerance.** Phone and server clocks are never perfectly in sync. Coarse 30 s buckets mean small differences usually land in the same step.
- **It's what makes time a counter.** Dividing and rounding down turns a constantly changing timestamp into an integer that increases by exactly 1 every step, which is exactly what HOTP wants.
- **It's the standard.** RFC 6238 recommends 30 s as the default step, balancing usability against how long a stolen code stays valid. Most authenticator apps assume it.

### The verification window

The server doesn't only accept the code for the current step `T`. It also accepts `T-1` and `T+1`:

- **Network and typing delay.** The user may read a code just before the step rolls over and submit it just after. RFC 6238 §5.2 suggests allowing at most one step of delay for this.
- **Clock drift.** The authenticator's clock may run a bit fast or slow. RFC 6238 §6 discusses accepting a few steps backward or forward and even tracking drift per device. Accepting one step in each direction (±1) is the common practice and what we do.

So a code is valid for roughly 60 to 90 seconds in total. Wider windows are more forgiving but give a guessed or stolen code more time to work.

**Replay protection.** §5.2 also says a verifier should not accept the same OTP a second time once it has been accepted. We store the last accepted time step per user and reject any code whose step is not newer than that (`Code already used`). Without this, someone looking over your shoulder could reuse your code within the window.

**Constant-time comparison.** The submitted code is compared to the expected one without exiting early on the first mismatching character, so response timing doesn't leak how many digits were right.

See [ADR 0003](docs/adr/0003-verification-window-and-replay.md) for the reasoning.

### Gotcha: Web Crypto needs a secure context

My Authenticator computes HMAC-SHA1 with `crypto.subtle`, which browsers only expose in a **secure context**: pages served over `https://`, or from `localhost` / `127.0.0.1`. On `http://localhost:3000` it works.

If you open the authenticator page from your phone at `http://192.168.x.x:3000/authenticator`, that's plain HTTP on a non-localhost address, so `crypto.subtle` is `undefined` and the page can't compute codes. To use a phone, either scan the QR code with a real authenticator app instead, or put the server behind HTTPS (for example a local TLS proxy or tunnel).

## Not production-ready

This is a teaching demo. Things a real system would need:

- **Secrets are kept in memory and unencrypted.** They vanish on restart, and anyone who can read the process memory gets them. Real systems store them encrypted at rest (or in an HSM / KMS).
- **No rate limiting.** A 6-digit code has 1,000,000 possibilities and ±1 window accepts 3 of them, so unlimited guessing will eventually succeed. Real systems lock out or slow down after a few failures.
- **No password step.** TOTP is meant to be the *second* factor; here it's the only one.
- **No backup / recovery codes.** Lose the authenticator and the account is gone.
- **No enrollment confirmation.** A real app asks for one valid code before turning 2FA on, to prove the user's app was set up correctly. Here enrollment is immediately active.
- **No HTTPS.** The secret travels over plain HTTP during enrollment.
- **No authentication on enroll.** Anyone can (re-)enroll any username and replace its secret.

## Further reading

- [docs/adr/](docs/adr/README.md): architecture decision records
- [docs/rust-notes.md](docs/rust-notes.md): Rust concepts used in this project
- [docs/api-contract.md](docs/api-contract.md): HTTP API
- RFC 4226 (HOTP) and RFC 6238 (TOTP)
