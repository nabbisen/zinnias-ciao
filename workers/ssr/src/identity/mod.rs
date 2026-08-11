//! The verified-identity boundary — RFC-080 §4/§4.1 (Handoff 053, external-
//! identity Slice 4a). This module itself makes no network call and
//! reaches no route directly; `handlers::identity` (Handoff 054) is what
//! wires a real callback to it.

mod jwt;

#[cfg(test)]
mod fake_issuer;

#[cfg(feature = "dev_fake_issuer")]
pub(crate) mod dev_fake_issuer;

use jwt::{JwtError, decode_and_verify};

/// The only shape that crosses from provider-specific verification into
/// identity logic (RFC-080 §4). `provider_authentication_context` — RFC-080's
/// own optional field, "only if reviewed and needed" — is deliberately
/// omitted: it has no consumer yet, and this project has just spent four
/// packages removing fields in exactly that shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedExternalIdentity {
    pub identity_namespace_id: String,
    pub subject: String,
    pub authenticated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerificationError {
    Jwt(JwtError),
    IssuerMismatch,
    AudienceMismatch,
    Expired,
    IssuedInFuture,
    NonceMismatch,
    MissingClaim,
    ClaimTypeMismatch,
}

/// What a namespace requires to verify a token — resolved from the
/// namespace's own configuration, never from the token. Held in code, not
/// a database column: this slice recognises exactly one namespace
/// (`idns_local_fake`); a real provider's registration is a future
/// provider RFC's concern (RFC-080 §3.2), not a column this slice invents
/// ahead of the registration flow that would populate it.
pub(crate) struct NamespaceVerification {
    pub expected_alg: &'static str,
    pub expected_issuer: String,
    pub expected_audience: String,
    /// `(kid, key bytes)` pairs. A token with no `kid` resolves only if
    /// there is exactly one key — the common shared-secret-namespace
    /// case; a token naming a `kid` must match one exactly.
    pub keys: Vec<(String, Vec<u8>)>,
}

/// Resolve verification requirements for a namespace — from code, never
/// from the token. Handoff 054 §3: `idns_local_fake` is recognised only
/// under the `dev_fake_issuer` feature; without it, this returns `None`
/// for every namespace unconditionally, which is what makes a production
/// build structurally incapable of verifying a token against it — there
/// is no configuration flag that re-enables this at runtime, only a
/// cargo feature that changes what got compiled.
#[cfg(feature = "dev_fake_issuer")]
pub(crate) fn resolve_namespace_verification(namespace_id: &str) -> Option<NamespaceVerification> {
    if namespace_id != "idns_local_fake" {
        return None;
    }
    Some(NamespaceVerification {
        expected_alg: jwt::ALG_HS256,
        expected_issuer: dev_fake_issuer::ISSUER.to_owned(),
        expected_audience: dev_fake_issuer::AUDIENCE.to_owned(),
        keys: vec![(
            dev_fake_issuer::KID.to_owned(),
            dev_fake_issuer::shared_key(),
        )],
    })
}

#[cfg(not(feature = "dev_fake_issuer"))]
pub(crate) fn resolve_namespace_verification(_namespace_id: &str) -> Option<NamespaceVerification> {
    None
}

impl NamespaceVerification {
    fn resolve_key(&self, kid: Option<&str>) -> Option<Vec<u8>> {
        match kid {
            Some(kid) => self
                .keys
                .iter()
                .find(|(k, _)| k == kid)
                .map(|(_, key)| key.clone()),
            None if self.keys.len() == 1 => Some(self.keys[0].1.clone()),
            None => None,
        }
    }
}

/// RFC 7519's registered claims tolerate up to a few minutes of clock
/// drift between issuer and verifier in practice; this is the bound RFC-080
/// §4.1 asks this package to enforce rather than leaving unbounded.
const CLOCK_SKEW_SECONDS: i64 = 120;

/// Verify a compact JWT against one namespace's pinned requirements and
/// one transaction's expected nonce, entirely as pure computation — no
/// D1, no network, no wall-clock read (the caller supplies `now_unix`
/// explicitly, which is what makes this natively unit-testable without
/// the wasm/worker harness, the same reasoning as
/// `authz::decide_membership_scope`).
///
/// `expected_nonce_hmac` and `pepper` implement the same digest-at-rest
/// discipline as `auth_transactions.nonce_hmac`: the raw nonce this
/// transaction was created with is never passed in — only its digest, to
/// compare against the token's own `nonce` claim, HMACed the same way.
#[allow(dead_code)] // Slice 4b: the callback handler is the first caller.
pub(crate) fn verify_id_token(
    token: &str,
    namespace_id: &str,
    namespace: &NamespaceVerification,
    expected_nonce_hmac: &str,
    pepper: &str,
    now_unix: i64,
) -> Result<VerifiedExternalIdentity, VerificationError> {
    let decoded = decode_and_verify(token, namespace.expected_alg, |kid| {
        namespace.resolve_key(kid)
    })
    .map_err(VerificationError::Jwt)?;

    let claims = decoded.claims;

    let iss = claim_str(&claims, "iss")?;
    if iss != namespace.expected_issuer {
        return Err(VerificationError::IssuerMismatch);
    }

    let aud = claim_str(&claims, "aud")?;
    if aud != namespace.expected_audience {
        return Err(VerificationError::AudienceMismatch);
    }

    let exp = claim_i64(&claims, "exp")?;
    if now_unix > exp + CLOCK_SKEW_SECONDS {
        return Err(VerificationError::Expired);
    }

    let iat = claim_i64(&claims, "iat")?;
    if iat > now_unix + CLOCK_SKEW_SECONDS {
        return Err(VerificationError::IssuedInFuture);
    }

    let nonce = claim_str(&claims, "nonce")?;
    let nonce_hmac = crate::crypto::hmac_hex(pepper, nonce);
    if !crate::crypto::constant_time_eq(&nonce_hmac, expected_nonce_hmac) {
        return Err(VerificationError::NonceMismatch);
    }

    let subject = claim_str(&claims, "sub")?.to_owned();

    Ok(VerifiedExternalIdentity {
        identity_namespace_id: namespace_id.to_owned(),
        subject,
        authenticated_at: unix_to_iso8601(now_unix),
    })
}

fn claim_str<'a>(claims: &'a serde_json::Value, key: &str) -> Result<&'a str, VerificationError> {
    match claims.get(key) {
        None => Err(VerificationError::MissingClaim),
        Some(v) => v.as_str().ok_or(VerificationError::ClaimTypeMismatch),
    }
}

fn claim_i64(claims: &serde_json::Value, key: &str) -> Result<i64, VerificationError> {
    match claims.get(key) {
        None => Err(VerificationError::MissingClaim),
        Some(v) => v.as_i64().ok_or(VerificationError::ClaimTypeMismatch),
    }
}

/// RFC-080 §9's collision policy for a returning subject, decided here
/// (Handoff 053 §5.6, carried from the Slice 2 review's note that
/// `db::identity::find_by_subject_lookup` returns `'revoked'` rows
/// deliberately unfiltered — this is where that decision is made real).
///
/// **Decision: a revoked identity authenticates nobody, and is
/// indistinguishable to the caller from an identity that was never linked
/// at all.** Same generic outcome either way; nothing discloses that a
/// link once existed. This mirrors the fail-closed, no-disclosure shape
/// `authz::decide_membership_scope` already uses for a bound session
/// reaching an out-of-scope community (RFC-081 §2) — a revoked identity
/// reaching for authentication is the same class of "this used to work,
/// it doesn't now, and the caller learns nothing more than that."
///
/// Slice 4b's callback must call this rather than reading `status`
/// itself, so the decision has exactly one place it can be made — not
/// re-derived, and not silently assumed, at each new call site.
#[allow(dead_code)] // Slice 4b: the callback route, after find_by_subject_lookup.
pub(crate) fn identity_lookup_is_authenticatable(status: &str) -> bool {
    status == "active"
}

fn unix_to_iso8601(secs: i64) -> String {
    let (y, mo, d, h, mi, s) = crate::db::epoch_to_ymd_hms(secs.max(0) as u64);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.000Z")
}

#[cfg(test)]
mod tests;
