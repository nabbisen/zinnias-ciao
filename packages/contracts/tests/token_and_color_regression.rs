//! Regression tests for RFC-020 v1.2 implementation decisions:
//!   - Status token fg colors pass WCAG AA (≥4.5:1 on white).
//!   - All new admin handler routes have token_purpose constants.

use zinnias_ciao_contracts::auth::token_purpose;

// ── WCAG AA contrast guard ────────────────────────────────────────────────
//
// Contrast ratio = (L1 + 0.05) / (L2 + 0.05)  where L1 ≥ L2 (relative luminance).
// Luminance of sRGB channel c: c/12.92 if c≤0.04045 else ((c+0.055)/1.055)^2.4
// We test the three status foreground colors against white (#FFFFFF, L=1.0).

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055_f64).powf(2.4)
    }
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    let r = srgb_to_linear(r as f64 / 255.0);
    let g = srgb_to_linear(g as f64 / 255.0);
    let b = srgb_to_linear(b as f64 / 255.0);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn contrast_on_white(r: u8, g: u8, b: u8) -> f64 {
    let l_white = 1.0_f64;
    let l_fg    = relative_luminance(r, g, b);
    let l1 = l_white.max(l_fg);
    let l2 = l_white.min(l_fg);
    (l1 + 0.05) / (l2 + 0.05)
}

fn parse_hex_color(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    assert_eq!(h.len(), 6, "expected 6-digit hex, got: {hex}");
    let r = u8::from_str_radix(&h[0..2], 16).unwrap();
    let g = u8::from_str_radix(&h[2..4], 16).unwrap();
    let b = u8::from_str_radix(&h[4..6], 16).unwrap();
    (r, g, b)
}

/// AA normal-text threshold = 4.5:1.
const AA_MIN: f64 = 4.5;

#[test]
fn status_going_fg_passes_wcag_aa() {
    let (r, g, b) = parse_hex_color("#005BBB");
    let ratio = contrast_on_white(r, g, b);
    assert!(
        ratio >= AA_MIN,
        "going fg #005BBB: contrast {ratio:.2}:1 < AA {AA_MIN}:1"
    );
}

#[test]
fn status_not_going_fg_passes_wcag_aa() {
    let (r, g, b) = parse_hex_color("#B42318");
    let ratio = contrast_on_white(r, g, b);
    assert!(
        ratio >= AA_MIN,
        "not-going fg #B42318: contrast {ratio:.2}:1 < AA {AA_MIN}:1"
    );
}

#[test]
fn status_attended_fg_passes_wcag_aa() {
    let (r, g, b) = parse_hex_color("#167A34");
    let ratio = contrast_on_white(r, g, b);
    assert!(
        ratio >= AA_MIN,
        "attended fg #167A34: contrast {ratio:.2}:1 < AA {AA_MIN}:1"
    );
}

#[test]
fn status_no_answer_fg_passes_wcag_aa() {
    let (r, g, b) = parse_hex_color("#6E6E73");
    let ratio = contrast_on_white(r, g, b);
    assert!(
        ratio >= AA_MIN,
        "no-answer fg #6E6E73: contrast {ratio:.2}:1 < AA {AA_MIN}:1"
    );
}

/// Old raw iOS colors that the RFC-020 v1.2 triplets replace must *not* pass AA —
/// confirming we actually needed the fix.
#[test]
fn old_ios_status_colors_fail_wcag_aa_on_text() {
    let ios_going    = parse_hex_color("#007AFF"); // was used for status text
    let ios_attended = parse_hex_color("#34C759");
    assert!(
        contrast_on_white(ios_going.0,    ios_going.1,    ios_going.2)    < AA_MIN,
        "expected #007AFF to fail AA on white (it's a decorative-only color)"
    );
    assert!(
        contrast_on_white(ios_attended.0, ios_attended.1, ios_attended.2) < AA_MIN,
        "expected #34C759 to fail AA on white (it's a decorative-only color)"
    );
}

// ── New admin handler token_purpose coverage ─────────────────────────────

#[test]
fn edit_event_token_purpose_exists_and_is_valid() {
    let p = token_purpose::EDIT_EVENT;
    assert!(!p.is_empty());
    assert!(!p.contains(' '));
}

#[test]
fn attendance_override_token_purpose_exists_and_is_valid() {
    let p = token_purpose::ATTENDANCE_OVERRIDE;
    assert!(!p.is_empty());
    assert!(!p.contains(' '));
}

#[test]
fn admin_hide_note_token_purpose_exists_and_is_valid() {
    let p = token_purpose::ADMIN_HIDE_NOTE;
    assert!(!p.is_empty());
    assert!(!p.contains(' '));
}

/// All purposes must be unique strings (no accidental re-use that would let
/// one form token be replayed on a different action).
#[test]
fn all_token_purposes_are_unique() {
    use std::collections::HashSet;
    let purposes = [
        token_purpose::SET_STATUS,
        token_purpose::SAVE_NOTE,
        token_purpose::DELETE_NOTE,
        token_purpose::CREATE_EVENT,
        token_purpose::EDIT_EVENT,
        token_purpose::CANCEL_EVENT,
        token_purpose::ATTENDANCE_OVERRIDE,
        token_purpose::ADMIN_HIDE_NOTE,
        token_purpose::REVOKE_INVITE,
        token_purpose::CALENDAR_REGENERATE,
        token_purpose::CALENDAR_REVOKE,
        token_purpose::COMMUNITY_EXPORT,
        token_purpose::REDEEM_INVITE,
        token_purpose::JOIN_PROFILE,
        token_purpose::LOGOUT,
    ];
    let set: HashSet<&str> = purposes.iter().copied().collect();
    assert_eq!(
        set.len(), purposes.len(),
        "duplicate token_purpose detected — each action must have a unique string"
    );
}
