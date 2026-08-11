use super::*;

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn sign(key: &[u8], signing_input: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(signing_input.as_bytes());
    b64(&mac.finalize().into_bytes())
}

fn build(header_json: &str, payload_json: &str, key: &[u8]) -> String {
    let header_b64 = b64(header_json.as_bytes());
    let payload_b64 = b64(payload_json.as_bytes());
    let signing_input = format!("{header_b64}.{payload_b64}");
    let sig_b64 = sign(key, &signing_input);
    format!("{signing_input}.{sig_b64}")
}

const KEY: &[u8] = b"test-key-not-a-real-secret-value";

fn resolve(kid: Option<&str>) -> Option<Vec<u8>> {
    match kid {
        None | Some("kid-1") => Some(KEY.to_vec()),
        _ => None,
    }
}

#[test]
fn valid_hs256_token_decodes() {
    let token = build(
        r#"{"alg":"HS256","kid":"kid-1"}"#,
        r#"{"sub":"user-1"}"#,
        KEY,
    );
    let decoded = decode_and_verify(&token, ALG_HS256, resolve).unwrap();
    assert_eq!(decoded.claims["sub"], "user-1");
}

#[test]
fn wrong_number_of_segments_is_malformed() {
    assert_eq!(
        decode_and_verify("only.two", ALG_HS256, resolve).unwrap_err(),
        JwtError::Malformed
    );
    assert_eq!(
        decode_and_verify("a.b.c.d", ALG_HS256, resolve).unwrap_err(),
        JwtError::Malformed
    );
}

#[test]
fn header_not_valid_base64_or_json_is_malformed() {
    assert_eq!(
        decode_and_verify("not-base64!!!.payload.sig", ALG_HS256, resolve).unwrap_err(),
        JwtError::Malformed
    );
    let bad_json_header = b64(b"not json");
    let token = format!("{bad_json_header}.{}.sig", b64(b"{}"));
    assert_eq!(
        decode_and_verify(&token, ALG_HS256, resolve).unwrap_err(),
        JwtError::Malformed
    );
}

#[test]
fn missing_alg_is_malformed() {
    let token = build(r#"{"typ":"JWT"}"#, r#"{"sub":"user-1"}"#, KEY);
    assert_eq!(
        decode_and_verify(&token, ALG_HS256, resolve).unwrap_err(),
        JwtError::Malformed
    );
}

#[test]
fn alg_of_wrong_json_type_is_claim_type_mismatch() {
    let token = build(r#"{"alg":123}"#, r#"{"sub":"user-1"}"#, KEY);
    assert_eq!(
        decode_and_verify(&token, ALG_HS256, resolve).unwrap_err(),
        JwtError::ClaimTypeMismatch
    );
}

#[test]
fn alg_none_is_rejected_unconditionally() {
    let token = build(r#"{"alg":"none"}"#, r#"{"sub":"user-1"}"#, KEY);
    assert_eq!(
        decode_and_verify(&token, ALG_HS256, resolve).unwrap_err(),
        JwtError::AlgNone
    );
}

#[test]
fn alg_none_is_rejected_even_if_the_caller_expects_it() {
    // Defence in depth: nothing about how this function is called can
    // ever cause an alg:none token to verify.
    let token = build(r#"{"alg":"none"}"#, r#"{"sub":"user-1"}"#, KEY);
    assert_eq!(
        decode_and_verify(&token, "none", resolve).unwrap_err(),
        JwtError::AlgNone
    );
}

#[test]
fn header_alg_different_from_expected_is_rejected_before_verification() {
    // Signed correctly under HS256, but the header claims RS256 — this
    // must be rejected on the mismatch alone, not by attempting (and
    // failing) verification under either algorithm.
    let token = build(
        r#"{"alg":"RS256","kid":"kid-1"}"#,
        r#"{"sub":"user-1"}"#,
        KEY,
    );
    assert_eq!(
        decode_and_verify(&token, ALG_HS256, resolve).unwrap_err(),
        JwtError::AlgorithmMismatch
    );
}

#[test]
fn unknown_kid_is_rejected_before_verification() {
    let token = build(
        r#"{"alg":"HS256","kid":"unknown-kid"}"#,
        r#"{"sub":"user-1"}"#,
        KEY,
    );
    assert_eq!(
        decode_and_verify(&token, ALG_HS256, resolve).unwrap_err(),
        JwtError::UnknownKey
    );
}

#[test]
fn bad_signature_is_rejected() {
    let mut token = build(
        r#"{"alg":"HS256","kid":"kid-1"}"#,
        r#"{"sub":"user-1"}"#,
        KEY,
    );
    let last = token.pop().unwrap();
    token.push(if last == 'A' { 'B' } else { 'A' });
    assert_eq!(
        decode_and_verify(&token, ALG_HS256, resolve).unwrap_err(),
        JwtError::BadSignature
    );
}

#[test]
fn signature_valid_under_a_different_key_is_rejected() {
    let other_key = b"a-completely-different-key-value";
    let token = build(
        r#"{"alg":"HS256","kid":"kid-1"}"#,
        r#"{"sub":"user-1"}"#,
        other_key,
    );
    assert_eq!(
        decode_and_verify(&token, ALG_HS256, resolve).unwrap_err(),
        JwtError::BadSignature
    );
}

#[test]
fn malformed_payload_is_malformed() {
    let header_b64 = b64(br#"{"alg":"HS256","kid":"kid-1"}"#);
    let payload_b64 = b64(b"not json");
    let signing_input = format!("{header_b64}.{payload_b64}");
    let sig_b64 = sign(KEY, &signing_input);
    let token = format!("{signing_input}.{sig_b64}");
    assert_eq!(
        decode_and_verify(&token, ALG_HS256, resolve).unwrap_err(),
        JwtError::Malformed
    );
}
