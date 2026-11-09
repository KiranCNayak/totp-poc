//! HOTP (RFC 4226) and TOTP (RFC 6238), built directly on HMAC-SHA1.
//!
//! Parameters used everywhere in this POC: SHA-1, 6 digits, 30-second step,
//! T0 = 0 (Unix epoch), verification window of ±1 step.

use hmac::{Hmac, Mac};
use sha1::Sha1;

/// `Hmac<Sha1>` is a generic type: the HMAC construction, specialised to SHA-1.
type HmacSha1 = Hmac<Sha1>;

pub const DIGITS: u32 = 6;
pub const STEP_SECONDS: u64 = 30;
/// How many time steps before/after "now" we accept (RFC 6238 §5.2 and §6).
pub const WINDOW: u64 = 1;

/// 20 random bytes (160 bits), the key length RFC 4226 recommends for HMAC-SHA1.
pub fn generate_secret() -> [u8; 20] {
    rand::random()
}

/// Base32 without `=` padding: the format authenticator apps expect.
pub fn to_base32(secret: &[u8]) -> String {
    data_encoding::BASE32_NOPAD.encode(secret)
}

/// HOTP: HMAC-SHA1(secret, counter) -> dynamic truncation -> `digits` decimal digits.
pub fn hotp(secret: &[u8], counter: u64, digits: u32) -> u32 {
    // HMAC accepts keys of any length, so this can't fail in practice.
    let mut mac = HmacSha1::new_from_slice(secret).expect("HMAC accepts any key length");
    // The counter is hashed as 8 bytes, big-endian (RFC 4226 §5.2).
    mac.update(&counter.to_be_bytes());
    let hash = mac.finalize().into_bytes(); // 20 bytes

    // Dynamic truncation (RFC 4226 §5.3): the low 4 bits of the last byte pick
    // an offset, we read 4 bytes from there and drop the top (sign) bit.
    let offset = (hash[hash.len() - 1] & 0x0f) as usize;
    let bin = u32::from_be_bytes([
        hash[offset],
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ]) & 0x7fff_ffff;

    bin % 10u32.pow(digits)
}

/// The TOTP counter: which 30-second bucket `unix_seconds` falls into.
pub fn time_step(unix_seconds: u64) -> u64 {
    unix_seconds / STEP_SECONDS
}

/// TOTP at a given Unix time, zero-padded to `DIGITS` characters.
pub fn totp(secret: &[u8], unix_seconds: u64) -> String {
    format_code(hotp(secret, time_step(unix_seconds), DIGITS), DIGITS)
}

fn format_code(code: u32, digits: u32) -> String {
    format!("{:0width$}", code, width = digits as usize)
}

#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError {
    /// Wrong code, or a code from outside the ±WINDOW steps.
    InvalidCode,
    /// The code matches, but its time step was already accepted once.
    Replayed,
}

/// Checks `code` against the steps T-WINDOW..=T+WINDOW.
///
/// `last_used_step` is the step of the last accepted code for this user (if any).
/// On success returns the matched step, which the caller must store as the new
/// `last_used_step` so the same code can't be used twice.
pub fn verify(
    secret: &[u8],
    code: &str,
    unix_seconds: u64,
    last_used_step: Option<u64>,
) -> Result<u64, VerifyError> {
    let code = code.trim();
    if code.len() != DIGITS as usize || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(VerifyError::InvalidCode);
    }

    let now = time_step(unix_seconds);
    let first = now.saturating_sub(WINDOW);
    for step in first..=now + WINDOW {
        let expected = format_code(hotp(secret, step, DIGITS), DIGITS);
        if constant_time_eq(expected.as_bytes(), code.as_bytes()) {
            return match last_used_step {
                Some(last) if step <= last => Err(VerifyError::Replayed),
                _ => Ok(step),
            };
        }
    }
    Err(VerifyError::InvalidCode)
}

/// Compares without returning early, so timing doesn't reveal how many leading
/// characters were right.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// `otpauth://totp/Issuer:account?secret=...` — the string encoded in the QR code.
/// Format: https://github.com/google/google-authenticator/wiki/Key-Uri-Format
pub fn otpauth_uri(issuer: &str, account: &str, secret_b32: &str) -> String {
    let issuer = percent_encode(issuer);
    format!(
        "otpauth://totp/{issuer}:{account}?secret={secret_b32}&issuer={issuer}\
         &algorithm=SHA1&digits={DIGITS}&period={STEP_SECONDS}",
        account = percent_encode(account),
    )
}

/// Minimal URL encoding: keep RFC 3986 unreserved characters, escape the rest.
fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test secret from both RFCs: ASCII "12345678901234567890".
    const RFC_SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn rfc4226_appendix_d_hotp_vectors() {
        let expected = [
            755224, 287082, 359152, 969429, 338314, 254676, 287922, 162583, 399871, 520489,
        ];
        for (counter, want) in expected.iter().enumerate() {
            assert_eq!(hotp(RFC_SECRET, counter as u64, 6), *want, "counter {counter}");
        }
    }

    #[test]
    fn rfc6238_appendix_b_sha1_vectors() {
        let cases = [
            (59, "94287082"),
            (1111111109, "07081804"),
            (1111111111, "14050471"),
            (1234567890, "89005924"),
            (2000000000, "69279037"),
            (20000000000, "65353130"),
        ];
        for (time, want) in cases {
            let got = format_code(hotp(RFC_SECRET, time_step(time), 8), 8);
            assert_eq!(got, want, "time {time}");
        }
    }

    #[test]
    fn accepts_current_and_neighbouring_steps() {
        let now = 1_700_000_000;
        for offset in [-30i64, 0, 30] {
            let code = totp(RFC_SECRET, (now as i64 + offset) as u64);
            assert!(verify(RFC_SECRET, &code, now, None).is_ok(), "offset {offset}");
        }
    }

    #[test]
    fn rejects_codes_outside_window() {
        let now = 1_700_000_000;
        for offset in [-60i64, 60] {
            let code = totp(RFC_SECRET, (now as i64 + offset) as u64);
            assert_eq!(
                verify(RFC_SECRET, &code, now, None),
                Err(VerifyError::InvalidCode),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn rejects_replay_of_accepted_code() {
        let now = 1_700_000_000;
        let code = totp(RFC_SECRET, now);
        let step = verify(RFC_SECRET, &code, now, None).unwrap();
        assert_eq!(verify(RFC_SECRET, &code, now, Some(step)), Err(VerifyError::Replayed));
    }

    #[test]
    fn rejects_malformed_codes() {
        for bad in ["", "12345", "1234567", "abcdef", "12 456"] {
            assert_eq!(verify(RFC_SECRET, bad, 0, None), Err(VerifyError::InvalidCode));
        }
    }

    #[test]
    fn base32_round_trip_and_uri() {
        assert_eq!(to_base32(RFC_SECRET), "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        let uri = otpauth_uri("TOTP-POC", "alice@example.com", "ABC");
        assert_eq!(
            uri,
            "otpauth://totp/TOTP-POC:alice%40example.com?secret=ABC&issuer=TOTP-POC\
             &algorithm=SHA1&digits=6&period=30"
        );
    }
}
