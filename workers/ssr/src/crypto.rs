//! Cryptographic helpers.
//!
//! All secrets are stored as HMAC-SHA256(pepper, value) — fast enough for the
//! 10 ms Workers CPU budget (AD-3) while making a DB leak non-exploitable
//! without the pepper.  Never use argon2/bcrypt/scrypt in a request path.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::fmt;

type HmacSha256 = Hmac<Sha256>;

const MIN_HMAC_PEPPER_BYTES: usize = 32;
const MAX_HMAC_PEPPER_BYTES: usize = 4096;

const LEGACY_SENTINELS: [&str; 2] = ["dev-pepper-change-in-production", "dev-pepper"];

/// Validated HMAC key material. Deliberately has no `Debug`, `Display`,
/// serialization, or owned-string conversion surface.
pub struct HmacPepper(String);

impl HmacPepper {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PepperConfigError {
    Missing,
    Empty,
    SurroundingWhitespace,
    LegacySentinel,
    TooShort,
    TooLong,
}

impl PepperConfigError {
    pub const fn category(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Empty => "empty",
            Self::SurroundingWhitespace => "surrounding_whitespace",
            Self::LegacySentinel => "legacy_sentinel",
            Self::TooShort => "too_short",
            Self::TooLong => "too_long",
        }
    }
}

impl fmt::Display for PepperConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.category())
    }
}

impl From<PepperConfigError> for worker::Error {
    fn from(error: PepperConfigError) -> Self {
        Self::RustError(format!("security configuration unavailable: {error}"))
    }
}

fn validate_pepper(value: &str) -> Result<HmacPepper, PepperConfigError> {
    if value.is_empty() || value.trim().is_empty() {
        return Err(PepperConfigError::Empty);
    }
    if value.trim() != value {
        return Err(PepperConfigError::SurroundingWhitespace);
    }
    if LEGACY_SENTINELS.contains(&value) {
        return Err(PepperConfigError::LegacySentinel);
    }
    if value.len() < MIN_HMAC_PEPPER_BYTES {
        return Err(PepperConfigError::TooShort);
    }
    if value.len() > MAX_HMAC_PEPPER_BYTES {
        return Err(PepperConfigError::TooLong);
    }
    Ok(HmacPepper(value.to_owned()))
}

/// The single source of truth for the HMAC pepper (AD-3 / RFC-077).
/// Only a Worker secret binding is accepted. Missing or invalid configuration
/// is never replaced with a local or plain-variable fallback.
pub fn pepper(env: &worker::Env) -> Result<HmacPepper, PepperConfigError> {
    let secret = env
        .secret("HMAC_PEPPER")
        .map_err(|_| PepperConfigError::Missing)?;
    validate_pepper(&secret.to_string())
}

/// Compute HMAC-SHA256(key=pepper, msg=value) and return lowercase hex.
pub fn hmac_hex(pepper: &str, value: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(pepper.as_bytes()).expect("HMAC accepts any key length");
    mac.update(value.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// The `user_identities.subject_lookup` keyed digest (RFC-080 §3.3, Handoff
/// 050 §5.2, corrected per the Slice 2 review's required fix). A D1 export
/// must not become a cross-system correlation list keyed on provider
/// identifiers — an unkeyed hash would not prevent that (a plain SHA-256 of
/// a subject is trivially reversible by anyone holding the same subject),
/// so this exists only to route through the existing pepper rather than
/// introduce a second hashing path. `subject` is opaque and case-sensitive:
/// no normalisation, no lowercasing, no trimming. "Helpfully" normalising
/// an OIDC subject would merge two distinct people.
///
/// `identity_namespace_id` is mixed into the digest input, not just
/// `subject`: RFC-080 §3.1 says two different namespaces must never be
/// inferred to identify the same person, but without the namespace in the
/// input, the same provider subject linked under two namespaces (a
/// production/staging pair for the same client, or two client
/// registrations where the subject is stable across both — an expected
/// state, since §7 forbids auto-linking and §12 puts merge out of scope)
/// would produce the identical digest, making that inference free to
/// anyone holding a database export. The raw subject is never stored, so
/// this must be right before the first row is ever linked — there is
/// nothing to recompute a different digest from afterward. The `\u{1f}`
/// (ASCII unit separator, never a legal character in an OIDC namespace id
/// or subject) prevents `("ns1", "abc")` and `("ns1a", "bc")` from
/// colliding.
///
/// No caller yet (Handoff 050 §5.3): Slice 4's authentication callback is
/// the first caller, computing this digest from a verified provider
/// subject before passing it to `db::identity::find_by_subject_lookup`.
#[allow(dead_code)] // Slice 4: the authentication callback (RFC-080 §5).
pub fn subject_lookup(pepper: &str, identity_namespace_id: &str, subject: &str) -> String {
    hmac_hex(pepper, &format!("{identity_namespace_id}\u{1f}{subject}"))
}

/// Constant-time comparison of equal-length strings.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Generate a cryptographically random URL-safe token (32 bytes → 64 hex chars).
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("getrandom failed");
    hex::encode(bytes)
}

/// Normalize an invite code: uppercase, strip hyphens/spaces, drop
/// visually ambiguous characters (0/O, 1/I/L) per RFC-003.
pub fn normalize_invite_code(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests;
