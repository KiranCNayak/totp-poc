# Rust notes: concepts used in this project

A guided tour of the Rust features in this codebase, in the order you meet them.
Read alongside [`src/totp.rs`](../src/totp.rs) and [`src/main.rs`](../src/main.rs).

## 1. Cargo and crates

- **Cargo** is Rust's build tool and package manager. `cargo new` made the project,
  `cargo build` compiles, `cargo test` runs tests, `cargo run` builds and starts the binary.
- A **crate** is a compilation unit (a library or a binary). Third-party crates come from
  [crates.io](https://crates.io) and are listed under `[dependencies]` in `Cargo.toml`.
  `Cargo.lock` pins the exact versions that were resolved.
- `"0.12"` in `Cargo.toml` means "any 0.12.x" (semver-compatible). Features (e.g.
  `features = ["svg"]`) switch optional parts of a crate on or off.
- This crate is **both a library and a binary**: `src/lib.rs` is the library root
  (`totp_poc`), `src/main.rs` is the binary and imports the library with
  `use totp_poc::totp`. That lets `cargo test --lib` test the logic without the server.
- `pub mod totp;` in `lib.rs` tells the compiler that a module lives in `src/totp.rs`
  and makes it public. Items inside are private unless marked `pub`.

## 2. Types: integers, arrays, slices

| In the code | Meaning |
|---|---|
| `u8`, `u32`, `u64` | Unsigned integers of 8/32/64 bits. No implicit conversions: you write `as usize`, `as u64`. |
| `[u8; 20]` | A fixed-size array of exactly 20 bytes, stored inline (no heap). |
| `&[u8]` | A **slice**: a borrowed view into some bytes of any length (array, `Vec`, string bytes). |
| `Vec<u8>` | A growable, heap-allocated byte buffer that *owns* its data. |
| `b"1234..."` | A byte-string literal, type `&[u8; N]`. |

`hotp(secret: &[u8], ...)` takes a slice so callers can pass an array, a `Vec`, or a
literal. `counter.to_be_bytes()` turns a `u64` into its 8 big-endian bytes, and
`u32::from_be_bytes([...])` does the reverse: exactly what RFC 4226 needs.

`0x7fff_ffff`: underscores in number literals are just for readability.

## 3. Ownership and borrowing

The core idea of Rust. Every value has exactly one **owner**; when the owner goes out of
scope, the value is freed. No garbage collector, no manual `free`.

- **Borrowing** (`&x`) lends a value without giving up ownership. Many `&` (shared)
  borrows may exist at once, *or* one `&mut` (exclusive) borrow, never both.
  The compiler (the "borrow checker") enforces this, which rules out data races.
- `totp::to_base32(&secret)`: we lend `secret`, and can still use it afterwards.
- `secret.to_vec()` copies the array into a new owned `Vec<u8>` so the `User` struct
  can own it.
- `username.clone()` in `enroll`: `insert` takes ownership of the key, but we still need
  `username` for the response, so we give `insert` a copy.
- `String` vs `&str`: `String` is an owned, growable string; `&str` is a borrowed view
  of one. Functions usually *take* `&str` (flexible) and *return* `String` (the caller owns it).

## 4. Error handling: `Result`, `Option`, `?`, `match`

Rust has no exceptions. Fallible functions return:

- `Option<T>`: `Some(value)` or `None` (e.g. `last_used_step: Option<u64>`: no code accepted yet).
- `Result<T, E>`: `Ok(value)` or `Err(error)` (e.g. `verify` returns `Result<u64, VerifyError>`).

Handling them:

- `match` must cover every case; the compiler checks it. See the end of `verify` in
  `main.rs`, which maps each `VerifyError` variant to a message.
- `Some(last) if step <= last =>` is a **match guard**: a pattern plus a condition.
- `let Some(user) = users.get_mut(..) else { return ...; };` (**let-else**) unwraps or
  returns early.
- `?` (in `enroll`) means "if this is `Err`, return it from this function now;
  otherwise unwrap the `Ok` value". `.map_err(..)` converts the error type first.
- `.unwrap()` / `.expect("why")` crash the program on `Err`/`None`. Fine for "this
  cannot happen" cases and tests, avoid it for real input.

## 5. Enums and `#[derive]`

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError { InvalidCode, Replayed }
```

Rust enums are "one of these variants" types (variants can also carry data, like
`Option`'s `Some(T)`). `#[derive(...)]` auto-generates trait implementations:
`Debug` (printable with `{:?}`), `PartialEq` (`==`, needed by `assert_eq!`).

## 6. Traits and generics

A **trait** is like an interface. `use hmac::{Hmac, Mac};` imports the `Mac` trait;
methods like `new_from_slice`, `update` and `finalize` come from it, and are only
callable when the trait is in scope.

`Hmac<Sha1>` is a **generic** type: HMAC is written once for any hash, and we plug in
SHA-1. `type HmacSha1 = Hmac<Sha1>;` is just an alias.

`#[derive(Serialize, Deserialize)]` (from `serde`) implements traits that convert our
structs to and from JSON. axum uses them for `Json<T>` request and response bodies.

## 7. Iterators and closures

```rust
a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
```

- `.iter()` yields items one at a time; `.zip(b)` pairs them with `b`'s items.
- `|acc, (x, y)| ...` is a **closure** (anonymous function); `(x, y)` destructures the pair.
- `.fold` combines everything into one value. Here: OR together the XOR of every byte
  pair; the result is 0 only if all bytes are equal. No early exit, so the time taken
  doesn't depend on where the first mismatch is (constant-time comparison).

Other examples: `code.bytes().all(|b| b.is_ascii_digit())`, and `percent_encode`'s
`.map(...).collect()` which builds a `String` from an iterator of `String`s.

Ranges: `first..=now + WINDOW` is inclusive (`..` would exclude the end).
`saturating_sub` subtracts but stops at 0 instead of overflowing.

## 8. Formatting and macros

Anything ending in `!` is a **macro** (code that generates code at compile time):
`format!`, `println!`, `assert_eq!`, `include_str!`, `json!`.

- `format!("{:0width$}", code, width = 6)`: zero-pad to 6 characters.
- `format!("%{b:02X}")`: variables can be named inline; `02X` = 2-digit uppercase hex.
- `include_str!("../static/index.html")` reads the file **at compile time** and embeds it
  in the binary as a `&'static str`.

`'static` is a **lifetime**: the data lives for the whole program (string literals do).
That's why `VerifyResponse.message` can be `&'static str` without owning a `String`.

## 9. Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rfc4226_appendix_d_hotp_vectors() { ... }
}
```

- `#[cfg(test)]` compiles the module only for `cargo test`.
- `use super::*;` imports everything from the parent module, including private items,
  so tests can call private helpers like `format_code`.
- `assert_eq!(got, want, "message {x}")` fails the test with both values printed.

## 10. Async, tokio, and shared state (the server)

- `async fn` returns a *future*: work that runs when awaited. `.await` pauses this task
  without blocking the thread, so one thread can serve many connections.
- Rust has no built-in async runtime. **tokio** is the runtime; `#[tokio::main]` sets it
  up and runs `main` on it.
- **axum** maps routes to handler functions. Handler arguments are **extractors**:
  `State(users)` gets the shared state, `Json(req)` parses the body into our struct.
  Return values implement `IntoResponse` (`Json<T>`, `Html<..>`, `(StatusCode, Json<..>)`).
- `Arc<Mutex<HashMap<..>>>`:
  - `HashMap` is the users table.
  - `Mutex` gives exclusive access: `.lock()` returns a guard; the lock is released
    automatically when the guard goes out of scope (RAII). We use `std::sync::Mutex`
    because we never `.await` while holding it.
  - `Arc` (atomically reference-counted pointer) lets many handlers share the same map;
    it's freed when the last `Arc` is dropped.
- `|| async { ... }` is a closure returning a future, used for the two tiny page routes.

## Where to go next

- *The Rust Programming Language* ("the book"): https://doc.rust-lang.org/book/
  (chapters 4 ownership, 6 enums, 9 errors, 10 generics/traits, 16 concurrency).
- *Rust by Example*: https://doc.rust-lang.org/rust-by-example/
- `cargo clippy` for lint suggestions, `cargo fmt` to format, `cargo doc --open` for docs.
