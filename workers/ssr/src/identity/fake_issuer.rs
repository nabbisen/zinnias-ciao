//! In-process, test-only issuer — RFC-080 §10 (Handoff 053). A required
//! deliverable, not a test fixture bolted on afterward: the whole
//! verification contract must be exercisable with no provider account, no
//! secrets, and no network.
//!
//! Test-only structurally, not by convention: `identity/mod.rs` declares
//! `#[cfg(test)] mod fake_issuer;`, so this file is absent from any
//! non-test build entirely — checked by
//! `identity_fake_issuer_is_test_only` in `release_gates.rs`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub(super) struct FakeIssuer {
    pub key: Vec<u8>,
    pub kid: String,
    pub issuer: String,
    pub audience: String,
}

pub(super) struct Claims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub nonce: String,
    pub exp: i64,
    pub iat: i64,
}

impl FakeIssuer {
    /// A fresh, random key every call — never committed, per RFC-080 §10 /
    /// Handoff 053 §10's evidence constraint.
    pub(super) fn new(issuer: &str, audience: &str) -> Self {
        let mut key = vec![0u8; 32];
        getrandom::fill(&mut key).expect("getrandom failed");
        Self {
            key,
            kid: "fake-key-1".to_owned(),
            issuer: issuer.to_owned(),
            audience: audience.to_owned(),
        }
    }

    pub(super) fn valid_claims(&self, subject: &str, nonce: &str, now: i64) -> Claims {
        Claims {
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            sub: subject.to_owned(),
            nonce: nonce.to_owned(),
            exp: now + 300,
            iat: now,
        }
    }

    /// A validly-signed HS256 token for the given claims, using this
    /// issuer's own `kid`.
    pub(super) fn mint(&self, claims: &Claims) -> String {
        self.mint_with_header(claims, "HS256", Some(self.kid.as_str()))
    }

    /// Full control over the header's `alg`/`kid`. Signing is always
    /// actually performed with HMAC-SHA256 under this issuer's key — this
    /// project has no other verified signing algorithm to sign with — so
    /// a token minted with `alg` set to something else proves the
    /// verifier rejects on the header-vs-namespace mismatch alone, before
    /// any signature is even computed; the signature bytes present are
    /// never reached.
    pub(super) fn mint_with_header(&self, claims: &Claims, alg: &str, kid: Option<&str>) -> String {
        let mut header = serde_json::json!({ "alg": alg, "typ": "JWT" });
        if let Some(kid) = kid {
            header["kid"] = serde_json::Value::String(kid.to_owned());
        }
        let (header_b64, payload_b64) = self.encode_header_and_payload(&header, claims);
        let signing_input = format!("{header_b64}.{payload_b64}");
        let sig_b64 = self.sign(&signing_input);
        format!("{signing_input}.{sig_b64}")
    }

    /// `alg: none` — no signature segment carries real weight for this
    /// algorithm, so an empty one is the honest representation of what a
    /// none-attack token actually presents.
    pub(super) fn mint_alg_none(&self, claims: &Claims) -> String {
        let header = serde_json::json!({ "alg": "none", "typ": "JWT" });
        let (header_b64, payload_b64) = self.encode_header_and_payload(&header, claims);
        format!("{header_b64}.{payload_b64}.")
    }

    /// A structurally valid, validly-signed-shaped token whose signature
    /// has been corrupted after signing — decodes fine, verifies against
    /// nothing.
    pub(super) fn mint_malformed_signature(&self, claims: &Claims) -> String {
        let mut token = self.mint(claims);
        let last = token.pop().expect("signed token is non-empty");
        // Any distinct valid base64url character changes the decoded
        // signature bytes without changing the token's length or making
        // it fail to base64-decode.
        let replacement = if last == 'A' { 'B' } else { 'A' };
        token.push(replacement);
        token
    }

    /// Full control over both header and payload JSON, for cases the
    /// typed [`Claims`] shape can't express — a claim present with the
    /// wrong JSON type, or a claim missing outright.
    pub(super) fn mint_raw(&self, header_json: &str, payload_json: &str) -> String {
        let header_b64 = URL_SAFE_NO_PAD.encode(header_json.as_bytes());
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        let signing_input = format!("{header_b64}.{payload_b64}");
        let sig_b64 = self.sign(&signing_input);
        format!("{signing_input}.{sig_b64}")
    }

    fn encode_header_and_payload(
        &self,
        header: &serde_json::Value,
        claims: &Claims,
    ) -> (String, String) {
        let payload = serde_json::json!({
            "iss": claims.iss,
            "aud": claims.aud,
            "sub": claims.sub,
            "nonce": claims.nonce,
            "exp": claims.exp,
            "iat": claims.iat,
        });
        let header_b64 =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(header).expect("header serializes"));
        let payload_b64 =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).expect("payload serializes"));
        (header_b64, payload_b64)
    }

    fn sign(&self, signing_input: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(signing_input.as_bytes());
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    }
}
