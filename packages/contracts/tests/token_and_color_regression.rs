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
    let l_fg = relative_luminance(r, g, b);
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

// ── RFC-075 Slice 2: read the shipped colour, not a copy ─────────────────
//
// Before this slice, each test below called `parse_hex_color` on a hex
// literal held *by this test file*, while `render/status.rs` separately
// defined its own `CZ_STATUS_*_FG` constant with (hopefully) the same value.
// Nothing linked them: changing the shipped constant to a low-contrast
// colour left every test here passing, because each was checking its own
// copy. Slice 2 moved status colour out of Rust and into `app.css` classes
// (`cz-status-text--{suffix}`, backed by `--cz-status-*-fg` custom
// properties) — so these tests now read the actual shipped rule from
// `app.css`, resolve the custom property it references, and run the AA
// maths on *that* value. A wrong value in either the class rule or the
// `:root` token definition now fails the test that is supposed to guard it.

const APP_CSS_SOURCE: &str = include_str!("../../../workers/ssr/static/app.css");

/// Extract a top-level CSS rule's declaration body by selector, e.g.
/// `css_rule_body(css, ".cz-tab--active")` returns the text between that
/// selector's `{` and its matching `}`. Panics if the selector isn't found —
/// callers use this only to assert on rules that must exist. (Same helper as
/// `release_gates.rs`'s of the same name; duplicated because integration
/// test binaries do not share code with each other.)
///
/// Handoff 040 §7.3: tolerates any run of whitespace (including none)
/// between the selector and its opening brace — a rule written with
/// aligned braces (`.foo       { color: … }`) is not a different selector,
/// and the helper's job is to tolerate that formatting, not constrain it.
/// Still not a real CSS parser: if `selector` occurs as a substring not
/// immediately followed by whitespace-then-`{`, this reports "not found"
/// rather than searching further, the same narrowness the exact-string
/// version had.
fn css_rule_body<'a>(css: &'a str, selector: &str) -> &'a str {
    let start = css
        .find(selector)
        .unwrap_or_else(|| panic!("selector `{selector}` not found in app.css"));
    let after_selector = &css[start + selector.len()..];
    let brace_offset = after_selector
        .find('{')
        .filter(|&i| after_selector[..i].chars().all(char::is_whitespace))
        .unwrap_or_else(|| panic!("selector `{selector}` not found in app.css"));
    let after_brace = start + selector.len() + brace_offset + 1;
    let close = css[after_brace..]
        .find('}')
        .unwrap_or_else(|| panic!("selector `{selector}` has no closing brace in app.css"));
    &css[after_brace..after_brace + close]
}

/// Extract a single declaration's value from a rule body, e.g.
/// `css_property_value(body, "color")` on `"color: var(--x); margin: 0;"`
/// returns `"var(--x)"`. Panics if the property isn't declared in this body.
fn css_property_value<'a>(rule_body: &'a str, property: &str) -> &'a str {
    let needle = format!("{property}:");
    let start = rule_body
        .find(&needle)
        .unwrap_or_else(|| panic!("property `{property}` not declared in rule body: {rule_body}"));
    let after = start + needle.len();
    let end = rule_body[after..]
        .find(';')
        .unwrap_or_else(|| panic!("property `{property}` has no terminating ';' in: {rule_body}"));
    rule_body[after..after + end].trim()
}

/// Resolve a `--cz-*` custom property's literal value from `:root` in
/// `app.css`. Panics if the property is never defined.
fn css_custom_property_value<'a>(css: &'a str, var_name: &str) -> &'a str {
    let needle = format!("{var_name}:");
    let start = css
        .find(&needle)
        .unwrap_or_else(|| panic!("custom property `{var_name}` not found in app.css"));
    let after = start + needle.len();
    let end = css[after..]
        .find(';')
        .unwrap_or_else(|| panic!("custom property `{var_name}` has no terminating ';'"));
    css[after..after + end].trim()
}

/// Resolve a status class selector's `color` declaration to its final hex
/// value: reads the class rule, follows `color: var(--x)` to `--x`'s
/// `:root` definition, and returns that literal hex string. This is the
/// actual value a browser renders for that class — not a value asserted to
/// equal it.
fn resolve_status_fg_hex(css: &'static str, class_selector: &str) -> (u8, u8, u8) {
    let rule = css_rule_body(css, class_selector);
    let color_value = css_property_value(rule, "color");
    let hex = match color_value
        .strip_prefix("var(")
        .and_then(|s| s.strip_suffix(')'))
    {
        Some(var_name) => css_custom_property_value(css, var_name.trim()),
        None => color_value,
    };
    parse_hex_color(hex)
}

#[test]
fn status_going_fg_passes_wcag_aa() {
    let (r, g, b) = resolve_status_fg_hex(APP_CSS_SOURCE, ".cz-status-text--going");
    let ratio = contrast_on_white(r, g, b);
    assert!(
        ratio >= AA_MIN,
        "going fg (resolved from app.css): contrast {ratio:.2}:1 < AA {AA_MIN}:1"
    );
}

#[test]
fn status_not_going_fg_passes_wcag_aa() {
    let (r, g, b) = resolve_status_fg_hex(APP_CSS_SOURCE, ".cz-status-text--not-going");
    let ratio = contrast_on_white(r, g, b);
    assert!(
        ratio >= AA_MIN,
        "not-going fg (resolved from app.css): contrast {ratio:.2}:1 < AA {AA_MIN}:1"
    );
}

#[test]
fn status_attended_fg_passes_wcag_aa() {
    let (r, g, b) = resolve_status_fg_hex(APP_CSS_SOURCE, ".cz-status-text--attended");
    let ratio = contrast_on_white(r, g, b);
    assert!(
        ratio >= AA_MIN,
        "attended fg (resolved from app.css): contrast {ratio:.2}:1 < AA {AA_MIN}:1"
    );
}

#[test]
fn status_no_answer_fg_passes_wcag_aa() {
    let (r, g, b) = resolve_status_fg_hex(APP_CSS_SOURCE, ".cz-status-text--no-answer");
    let ratio = contrast_on_white(r, g, b);
    assert!(
        ratio >= AA_MIN,
        "no-answer fg (resolved from app.css): contrast {ratio:.2}:1 < AA {AA_MIN}:1"
    );
}

/// Old raw iOS colors that the RFC-020 v1.2 triplets replace must *not* pass AA —
/// confirming we actually needed the fix.
#[test]
fn old_ios_status_colors_fail_wcag_aa_on_text() {
    let ios_going = parse_hex_color("#007AFF"); // was used for status text
    let ios_attended = parse_hex_color("#34C759");
    assert!(
        contrast_on_white(ios_going.0, ios_going.1, ios_going.2) < AA_MIN,
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
        token_purpose::CREATE_TEMPLATE,
        token_purpose::DELETE_TEMPLATE,
        token_purpose::REMOVE_MEMBER,
        token_purpose::PROMOTE_MEMBER,
        token_purpose::DEMOTE_MEMBER,
        token_purpose::GENERATE_INVITE,
        token_purpose::REDEEM_INVITE,
        token_purpose::JOIN_PROFILE,
        token_purpose::LOGOUT,
    ];
    let set: HashSet<&str> = purposes.iter().copied().collect();
    assert_eq!(
        set.len(),
        purposes.len(),
        "duplicate token_purpose detected — each action must have a unique string"
    );
}
