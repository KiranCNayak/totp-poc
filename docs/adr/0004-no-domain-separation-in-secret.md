# 4. No domain separation (HKDF derivation) in the secret

Status: Accepted

Date: 2026-11-10

## Context

We considered deriving each user's TOTP secret from a server master key with a domain-separation label, for example `secret = HKDF(master_key, info = "app|scope|user")`, so that secrets for different apps, scopes, or users are cryptographically separated.

Points against it for this POC:

- **Authenticator apps only accept a raw Base32 secret.** They have no notion of a master key or `info` string. The derivation would happen entirely on the server and the app would just receive the derived bytes. It adds no extra factor or protection on the client side.
- **Random per-enrollment secrets already separate.** Each enrollment draws 160 fresh random bits. The chance that any two secrets collide is governed by the birthday bound, around 2^80 enrollments before a collision becomes likely, which is far beyond any realistic number.
- **A master key is a single point of compromise.** Anyone who obtains it can recompute every user's secret. With independent random secrets, an attacker has to steal the stored secrets themselves.

## Decision

Generate each secret as 20 independent random bytes from a CSPRNG and store it per user. No master key, no HKDF.

## Consequences

- Simpler code and one less key to protect.
- The server must store every secret (here: in memory; in production, encrypted at rest).
- Re-enrolling produces a new unrelated secret, as expected.

When derivation could make sense: if we wanted **stateless secret storage**, i.e. not storing per-user secrets at all and recomputing them from a master key (kept in an HSM/KMS) plus a stored per-user nonce or version. That trades the database of secrets for the master key as the critical asset, and needs a versioning scheme so a user can be re-enrolled. Revisit if storage of secrets becomes the main concern.
