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
fn constant_time_eq_requires_equal_strings() {
    assert!(constant_time_eq("same-secret", "same-secret"));
    assert!(!constant_time_eq("same-secret", "other-secret"));
    assert!(!constant_time_eq("same-secret", "short"));
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

#[test]
fn pepper_validation_rejects_missing_and_invalid_values() {
    assert_eq!(PepperConfigError::Missing.category(), "missing");
    assert!(matches!(validate_pepper(""), Err(PepperConfigError::Empty)));
    assert!(matches!(
        validate_pepper(" \t\n"),
        Err(PepperConfigError::Empty)
    ));
    assert!(matches!(
        validate_pepper(" aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        Err(PepperConfigError::SurroundingWhitespace)
    ));
    assert!(matches!(
        validate_pepper("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa "),
        Err(PepperConfigError::SurroundingWhitespace)
    ));
    for sentinel in LEGACY_SENTINELS {
        assert!(matches!(
            validate_pepper(sentinel),
            Err(PepperConfigError::LegacySentinel)
        ));
    }
    assert!(matches!(
        validate_pepper(&"a".repeat(31)),
        Err(PepperConfigError::TooShort)
    ));
    assert!(matches!(
        validate_pepper(&"a".repeat(4097)),
        Err(PepperConfigError::TooLong)
    ));
}

#[test]
fn subject_lookup_is_deterministic() {
    assert_eq!(
        subject_lookup("pepper", "ns1", "sub-123"),
        subject_lookup("pepper", "ns1", "sub-123")
    );
}

#[test]
fn subject_lookup_different_pepper_gives_different_digest() {
    assert_ne!(
        subject_lookup("pepper-a", "ns1", "sub-123"),
        subject_lookup("pepper-b", "ns1", "sub-123")
    );
}

#[test]
fn subject_lookup_is_case_sensitive_no_normalisation() {
    // A case difference must change the digest — proves the subject is
    // hashed as-is, never lowercased/normalised first (RFC-080 §3.3: doing
    // so would silently merge two distinct people).
    assert_ne!(
        subject_lookup("pepper", "ns1", "Sub-123"),
        subject_lookup("pepper", "ns1", "sub-123")
    );
}

#[test]
fn subject_lookup_different_namespace_gives_different_digest() {
    // RFC-080 §3.1: two different namespaces must never be inferable as
    // the same person. Without the namespace mixed into the digest input,
    // the same subject linked under two namespaces would produce the
    // identical digest — this is the property the Slice 2 review's
    // required correction exists to guarantee.
    assert_ne!(
        subject_lookup("pepper", "ns1", "sub-123"),
        subject_lookup("pepper", "ns2", "sub-123")
    );
}

#[test]
fn subject_lookup_never_contains_the_raw_subject() {
    let subject = "very-identifiable-subject-value";
    let digest = subject_lookup("pepper", "ns1", subject);
    assert!(!digest.contains(subject));
    // The digest is hex-encoded SHA-256 output (32 bytes -> 64 hex chars),
    // same shape as hmac_hex — confirms this routes through the one
    // existing hashing primitive rather than a second path.
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn pepper_validation_uses_utf8_byte_length_and_preserves_input() {
    let thirty_one_bytes = format!("{}a", "界".repeat(10));
    assert_eq!(thirty_one_bytes.len(), 31);
    assert!(matches!(
        validate_pepper(&thirty_one_bytes),
        Err(PepperConfigError::TooShort)
    ));

    let thirty_two_bytes = format!("{}aa", "界".repeat(10));
    assert_eq!(thirty_two_bytes.len(), 32);
    assert_eq!(
        validate_pepper(&thirty_two_bytes)
            .expect("32-byte non-hex key must be accepted")
            .as_str(),
        thirty_two_bytes
    );

    let maximum = "z".repeat(4096);
    assert_eq!(
        validate_pepper(&maximum)
            .expect("maximum key size must be accepted")
            .as_str(),
        maximum
    );
}
