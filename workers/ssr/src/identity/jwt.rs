//! Minimal, hand-rolled compact-JWT structure handling — RFC-080 §4.1
//! (Handoff 053).
//!
//! Deliberately not a general JWT library, and deliberately not built on
//! one: every wasm32-compatible crate evaluated for this pulled in
//! algorithm support (RSA/PKCS/EC) this package never needs, at real
//! dependency-tree and build-target risk — see the Handoff 053 review
//! request for the concrete failure. This module supports exactly HS256,
//! built on the same RustCrypto primitives (`hmac`, `sha2`) already used
//! by `crypto::hmac_hex`, and is written so there is no "library default"
//! for algorithm selection to research or trust: [`decode_and_verify`]
//! takes the expected algorithm and the key source as explicit parameters
//! supplied by the caller (resolved from the namespace, never from the
//! token), and never lets the token's own header dictate which
//! verification path runs.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// The one algorithm identifier this module accepts. A token whose header
/// names anything else — including this exact string spelled by an
/// attacker hoping the caller's `expected_alg` check is skippable — is
/// rejected by [`decode_and_verify`] before any signature is attempted.
#[allow(dead_code)] // Slice 4b: the namespace-verification-requirements resolver's first caller.
pub(crate) const ALG_HS256: &str = "HS256";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JwtError {
    /// Not three dot-separated, base64url-decodable, JSON-parseable parts.
    Malformed,
    /// The header's own `alg` is not the one JSON string type expected.
    ClaimTypeMismatch,
    /// `alg` is literally `"none"` — rejected unconditionally, regardless
    /// of what the caller's `expected_alg` is (defence in depth: this
    /// path does not exist to be configured away).
    AlgNone,
    /// The header's `alg` does not equal the caller-supplied
    /// `expected_alg`. This is the algorithm-confusion guard: the token
    /// never gets to choose how it is verified.
    AlgorithmMismatch,
    /// The header's `kid` (or the absence of one) does not resolve to a
    /// key the caller's key source recognises.
    UnknownKey,
    /// The key resolved; the signature did not verify under it.
    BadSignature,
}

#[derive(Debug)]
pub(crate) struct DecodedJwt {
    pub claims: serde_json::Value,
}

/// Decode and verify a compact JWT. `expected_alg` and `resolve_key` are
/// both supplied by the caller, resolved from the namespace's own
/// configuration — this function has no notion of "the provider's
/// algorithm" beyond what it is told, and it is told once, by the caller,
/// not by the token.
///
/// `resolve_key` receives the header's `kid` claim (`None` if absent) and
/// returns the verification key bytes, or `None` if the id is unknown.
pub(crate) fn decode_and_verify(
    token: &str,
    expected_alg: &str,
    resolve_key: impl FnOnce(Option<&str>) -> Option<Vec<u8>>,
) -> Result<DecodedJwt, JwtError> {
    let mut parts = token.split('.');
    let (Some(header_b64), Some(payload_b64), Some(sig_b64), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(JwtError::Malformed);
    };

    let header_bytes = URL_SAFE_NO_PAD
        .decode(header_b64)
        .map_err(|_| JwtError::Malformed)?;
    let header: serde_json::Value =
        serde_json::from_slice(&header_bytes).map_err(|_| JwtError::Malformed)?;

    let alg = header
        .get("alg")
        .ok_or(JwtError::Malformed)?
        .as_str()
        .ok_or(JwtError::ClaimTypeMismatch)?;

    // Checked before anything else, unconditionally: no configuration of
    // this function can ever cause an alg:none token to verify.
    if alg.eq_ignore_ascii_case("none") {
        return Err(JwtError::AlgNone);
    }
    if alg != expected_alg {
        return Err(JwtError::AlgorithmMismatch);
    }

    let kid = header.get("kid").and_then(|v| v.as_str());
    let key = resolve_key(kid).ok_or(JwtError::UnknownKey)?;

    let sig_bytes = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| JwtError::Malformed)?;

    let signing_input = format!("{header_b64}.{payload_b64}");
    let mut mac = HmacSha256::new_from_slice(&key).map_err(|_| JwtError::UnknownKey)?;
    mac.update(signing_input.as_bytes());
    mac.verify_slice(&sig_bytes)
        .map_err(|_| JwtError::BadSignature)?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| JwtError::Malformed)?;
    let claims: serde_json::Value =
        serde_json::from_slice(&payload_bytes).map_err(|_| JwtError::Malformed)?;

    Ok(DecodedJwt { claims })
}

#[cfg(test)]
mod tests;
