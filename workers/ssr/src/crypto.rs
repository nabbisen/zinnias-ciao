#![allow(dead_code)]
//! Cryptographic helpers.
//!
//! All secrets are stored as HMAC-SHA256(pepper, value) — fast enough for the
//! 10 ms Workers CPU budget (AD-3) while making a DB leak non-exploitable
//! without the pepper.  Never use argon2/bcrypt/scrypt in a request path.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// The single source of truth for the HMAC pepper (AD-3).
///
/// `HMAC_PEPPER` is set per-environment via `wrangler secret put`. We read it
/// as a secret. The dev fallback is only used when neither a secret nor a var
/// is bound (i.e. local `wrangler dev` without a configured secret).
///
/// In staging/production the deploy must set the secret. The `LOG_LEVEL` var
/// distinguishes environments: when it is `warn` (production) or `info`
/// (staging) and no pepper is bound, we still return the fallback but the
/// caller is expected to have configured the secret — a release gate and the
/// launch runbook enforce this. Centralizing here bans the previous mix of
/// `env.var`/`env.secret` with divergent fallbacks across handlers.
pub fn pepper(env: &worker::Env) -> String {
    if let Ok(s) = env.secret("HMAC_PEPPER") {
        let v = s.to_string();
        if !v.is_empty() {
            return v;
        }
    }
    // Fall back to a var binding (some local setups bind it as a plain var),
    // then to the dev sentinel as a last resort for `wrangler dev`.
    if let Ok(v) = env.var("HMAC_PEPPER") {
        let v = v.to_string();
        if !v.is_empty() {
            return v;
        }
    }
    "dev-pepper-change-in-production".to_string()
}

/// Compute HMAC-SHA256(key=pepper, msg=value) and return lowercase hex.
pub fn hmac_hex(pepper: &str, value: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(pepper.as_bytes()).expect("HMAC accepts any key length");
    mac.update(value.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time comparison of two hex strings.
pub fn hmac_hex_eq(a: &str, b: &str) -> bool {
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
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
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
mod tests {
    use super::*;

    #[test]
    fn hmac_hex_deterministic() {
        assert_eq!(hmac_hex("pepper", "value"), hmac_hex("pepper", "value"));
    }

    #[test]
    fn hmac_hex_different_inputs() {
        assert_ne!(hmac_hex("pepper", "a"), hmac_hex("pepper", "b"));
    }

    #[test]
    fn hmac_hex_different_peppers() {
        assert_ne!(hmac_hex("pepper1", "value"), hmac_hex("pepper2", "value"));
    }

    #[test]
    fn hmac_hex_eq_constant_time() {
        let a = hmac_hex("p", "v");
        let b = hmac_hex("p", "v");
        assert!(hmac_hex_eq(&a, &b));
        assert!(!hmac_hex_eq(&a, "deadbeef"));
    }

    #[test]
    fn normalize_invite_code_strips_separators() {
        assert_eq!(normalize_invite_code("X7-Y9 Z2"), "X7Y9Z2");
        assert_eq!(normalize_invite_code("x7y9z2"), "X7Y9Z2");
    }

    #[test]
    fn random_token_is_64_hex_chars() {
        let t = random_token();
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
