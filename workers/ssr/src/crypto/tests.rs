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
