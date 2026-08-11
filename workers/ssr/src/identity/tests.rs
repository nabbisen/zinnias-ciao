use super::fake_issuer::FakeIssuer;
use super::*;

const PEPPER: &str = "test-pepper-at-least-32-bytes-long-000000";
const NOW: i64 = 1_700_000_000;

fn nonce_hmac(nonce: &str) -> String {
    crate::crypto::hmac_hex(PEPPER, nonce)
}

fn namespace(issuer: &FakeIssuer) -> NamespaceVerification {
    NamespaceVerification {
        expected_alg: "HS256",
        expected_issuer: issuer.issuer.clone(),
        expected_audience: issuer.audience.clone(),
        keys: vec![(issuer.kid.clone(), issuer.key.clone())],
    }
}

fn issuer() -> FakeIssuer {
    FakeIssuer::new("https://fake.local.test", "client-under-test")
}

// ── The positive case, and the control the whole matrix depends on ──────

#[test]
fn valid_token_is_accepted() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    let token = issuer.mint(&claims);

    let result = verify_id_token(
        &token,
        "idns_test",
        &ns,
        &nonce_hmac("nonce-1"),
        PEPPER,
        NOW,
    )
    .expect("a validly-signed, correctly-claimed token must verify");

    assert_eq!(result.subject, "sub-1");
    assert_eq!(result.identity_namespace_id, "idns_test");
    assert_eq!(result.authenticated_at, "2023-11-14T22:13:20.000Z");
}

// ── Algorithm pinning — the security core of this package ───────────────

#[test]
fn correct_algorithm_is_accepted() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    let token = issuer.mint_with_header(&claims, "HS256", Some(&issuer.kid));
    assert!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW
        )
        .is_ok()
    );
}

#[test]
fn every_other_algorithm_is_rejected_including_one_the_header_claims_is_correct() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    // The header itself asserts "this is the right algorithm" by naming
    // HS256's usual peers — none of them are what the namespace expects
    // here would be a genuine provider-registered algorithm, so this is
    // exactly the "header lies about being correct" case.
    for alg in ["RS256", "ES256", "HS384", "HS512", "hs256"] {
        let token = issuer.mint_with_header(&claims, alg, Some(&issuer.kid));
        let err = verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW,
        )
        .expect_err(&format!("alg={alg} must be rejected"));
        assert_eq!(
            err,
            VerificationError::Jwt(jwt::JwtError::AlgorithmMismatch),
            "alg={alg} must be rejected specifically as an algorithm mismatch"
        );
    }
}

#[test]
fn alg_none_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    let token = issuer.mint_alg_none(&claims);
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW
        )
        .unwrap_err(),
        VerificationError::Jwt(jwt::JwtError::AlgNone)
    );
}

#[test]
fn unknown_key_id_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    let token = issuer.mint_with_header(&claims, "HS256", Some("a-kid-the-namespace-never-issued"));
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW
        )
        .unwrap_err(),
        VerificationError::Jwt(jwt::JwtError::UnknownKey)
    );
}

#[test]
fn malformed_signature_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    let token = issuer.mint_malformed_signature(&claims);
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW
        )
        .unwrap_err(),
        VerificationError::Jwt(jwt::JwtError::BadSignature)
    );
}

// ── Claim validation — each mismatch its own rejection ───────────────────

#[test]
fn wrong_issuer_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let mut claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    claims.iss = "https://not-the-registered-issuer.test".to_owned();
    let token = issuer.mint(&claims);
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW
        )
        .unwrap_err(),
        VerificationError::IssuerMismatch
    );
}

#[test]
fn wrong_audience_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let mut claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    claims.aud = "some-other-client".to_owned();
    let token = issuer.mint(&claims);
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW
        )
        .unwrap_err(),
        VerificationError::AudienceMismatch
    );
}

#[test]
fn wrong_nonce_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let claims = issuer.valid_claims("sub-1", "nonce-that-does-not-match", NOW);
    let token = issuer.mint(&claims);
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("the-transaction-actually-expected-this-one"),
            PEPPER,
            NOW,
        )
        .unwrap_err(),
        VerificationError::NonceMismatch
    );
}

#[test]
fn expired_token_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    let token = issuer.mint(&claims);
    // Past exp + the clock-skew allowance.
    let far_future = claims.exp + CLOCK_SKEW_SECONDS + 1;
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            far_future
        )
        .unwrap_err(),
        VerificationError::Expired
    );
}

#[test]
fn token_within_clock_skew_of_expiry_is_still_accepted() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    let claims = issuer.valid_claims("sub-1", "nonce-1", NOW);
    let token = issuer.mint(&claims);
    let just_within_skew = claims.exp + CLOCK_SKEW_SECONDS;
    assert!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            just_within_skew
        )
        .is_ok()
    );
}

#[test]
fn issued_in_the_future_beyond_skew_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    // iat is far ahead of "now" — beyond any plausible clock drift.
    let claims = issuer.valid_claims("sub-1", "nonce-1", NOW + 10_000);
    let token = issuer.mint(&claims);
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW
        )
        .unwrap_err(),
        VerificationError::IssuedInFuture
    );
}

#[test]
fn claim_of_wrong_json_type_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    // exp present, but as a string instead of a number — a claim of the
    // wrong type is not the same defect as a missing claim, and must be
    // reported as its own reason.
    let token = issuer.mint_raw(
        &format!(r#"{{"alg":"HS256","kid":"{}"}}"#, issuer.kid),
        r#"{"iss":"https://fake.local.test","aud":"client-under-test","sub":"sub-1","nonce":"nonce-1","exp":"not-a-number","iat":1700000000}"#,
    );
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW
        )
        .unwrap_err(),
        VerificationError::ClaimTypeMismatch
    );
}

// ── §5.6: the revoked-identity decision ───────────────────────────────

#[test]
fn active_identity_is_authenticatable() {
    assert!(identity_lookup_is_authenticatable("active"));
}

#[test]
fn revoked_identity_is_not_authenticatable() {
    // The decision itself: a revoked identity authenticates nobody. This
    // function's boolean result is deliberately the *only* fact a caller
    // gets — not "revoked" vs "not found", just "no".
    assert!(!identity_lookup_is_authenticatable("revoked"));
}

#[test]
fn missing_claim_is_rejected() {
    let issuer = issuer();
    let ns = namespace(&issuer);
    // No `sub` at all.
    let token = issuer.mint_raw(
        &format!(r#"{{"alg":"HS256","kid":"{}"}}"#, issuer.kid),
        r#"{"iss":"https://fake.local.test","aud":"client-under-test","nonce":"nonce-1","exp":1700000300,"iat":1700000000}"#,
    );
    assert_eq!(
        verify_id_token(
            &token,
            "idns_test",
            &ns,
            &nonce_hmac("nonce-1"),
            PEPPER,
            NOW
        )
        .unwrap_err(),
        VerificationError::MissingClaim
    );
}
