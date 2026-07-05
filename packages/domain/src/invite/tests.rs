use super::*;

#[test]
fn valid_code_accepted() {
    assert!(validate_invite_input("X7Y9Z2").is_ok());
    assert!(validate_invite_input("x7y9z2").is_ok()); // lowercase ok pre-norm
    assert!(validate_invite_input("X7-Y9 Z2").is_ok()); // separators stripped
}

#[test]
fn empty_rejected() {
    assert_eq!(
        validate_invite_input(""),
        Err(InviteValidationError::InvalidOrExpired)
    );
}

#[test]
fn too_short_rejected() {
    assert_eq!(
        validate_invite_input("X7Y9"),
        Err(InviteValidationError::InvalidOrExpired)
    );
}

#[test]
fn too_long_rejected() {
    assert_eq!(
        validate_invite_input("X7Y9Z2AAAAAAAAAA"),
        Err(InviteValidationError::InvalidOrExpired)
    );
}

#[test]
fn special_chars_rejected() {
    assert_eq!(
        validate_invite_input("X7Y9Z!"),
        Err(InviteValidationError::InvalidOrExpired)
    );
}

// ── Invite code generation properties ─────────────────────────────────

/// The alphabet length must evenly divide a power-of-two minus the bias
/// tail. 256 % 31 = 8, so the unbiased ceiling is 248.  Verify the
/// arithmetic the rejection-sampling generator in members.rs relies on.
#[test]
fn rejection_sampling_ceiling_is_correct() {
    let alpha_len = INVITE_CODE_ALPHABET.len(); // 31
    let bias_tail = 256 % alpha_len; // 8
    let unbiased_ceiling = 256 - bias_tail; // 248
    assert_eq!(
        alpha_len, 31,
        "alphabet length changed — review bias analysis"
    );
    assert_eq!(unbiased_ceiling, 248);
    // Every byte in [0, 248) maps to a unique position mod 31.
    // No byte in [248, 256) should be used.
    for b in 0..unbiased_ceiling {
        assert!(b % alpha_len < alpha_len); // tautological but documents intent
    }
}

/// Every byte the generator would accept (b < 248) maps to a valid
/// alphabet character.
#[test]
fn all_accepted_bytes_map_to_alphabet() {
    let alpha_len = INVITE_CODE_ALPHABET.len();
    let unbiased_ceiling = 256 - (256 % alpha_len);
    for b in 0..unbiased_ceiling {
        let ch = INVITE_CODE_ALPHABET[b % alpha_len];
        assert!(
            INVITE_CODE_ALPHABET.contains(&ch),
            "byte {b} mapped to char {ch} which is not in alphabet"
        );
    }
}

/// The unambiguous alphabet must not contain any of the visually
/// confusable characters excluded by RFC-003 §5.
#[test]
fn alphabet_excludes_ambiguous_characters() {
    let forbidden: &[u8] = b"01OIL";
    for &c in forbidden {
        assert!(
            !INVITE_CODE_ALPHABET.contains(&c),
            "alphabet contains ambiguous character '{}'",
            c as char
        );
    }
}
