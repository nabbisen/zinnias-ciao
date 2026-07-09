use super::participants::initials;
use super::shell::escape_html;
use super::status::status_display;
use super::time::{parse_utc_display, parse_utc_time};

#[test]
fn escape_script_tag() {
    let out = escape_html("<script>alert(\"xss\")</script>");
    assert!(!out.contains('<') && !out.contains('>'));
    assert!(out.contains("&lt;script&gt;"));
}

#[test]
fn escape_ampersand() {
    assert_eq!(escape_html("a&b"), "a&amp;b");
}

#[test]
fn escape_clean_string() {
    assert_eq!(escape_html("hello world"), "hello world");
}

#[test]
fn title_escaped_in_shell() {
    // Verify the title is properly escaped when inserted into the page shell.
    // We test escape_html directly here because page() wraps a worker::Response
    // and cannot be constructed in a native test environment.
    let escaped = escape_html("<bad>&title");
    assert!(escaped.contains("&lt;bad&gt;"));
    assert!(escaped.contains("&amp;"));
    assert!(!escaped.contains('<'));
    assert!(!escaped.contains('>'));
}

#[test]
fn initials_two_words() {
    assert_eq!(initials("Aya Tanaka"), "AT");
}

#[test]
fn initials_one_word() {
    assert_eq!(initials("Aya"), "A");
}

#[test]
fn initials_japanese_name() {
    // Each kanji is one Unicode char; we take the first two.
    assert_eq!(initials("田中 花子"), "田花");
}

#[test]
fn parse_utc_time_basic() {
    assert_eq!(parse_utc_time("2026-06-14T10:30:00.000Z"), "10:30");
}

#[test]
fn parse_utc_display_uses_ja_format() {
    // Home card date display must use Japanese convention, not "Jun 14".
    let out = parse_utc_display("2026-06-14T09:00:00.000Z");
    assert!(
        !out.contains("Jun"),
        "must not contain English month: {out}"
    );
    assert!(out.contains("月"), "must contain 月: {out}");
    assert!(out.contains("日"), "must contain 日: {out}");
    assert!(out.contains("09:00"), "must contain time: {out}");
}

#[test]
fn status_display_going() {
    let (_, _, label) = status_display(Some("going"));
    assert!(!label.is_empty());
    assert!(
        !label.contains("Going"),
        "label must be Japanese, got: {label}"
    );
}

#[test]
fn status_display_not_going() {
    let (_, _, label) = status_display(Some("not_going"));
    assert!(!label.is_empty());
    assert!(
        !label.contains("No Go"),
        "label must be Japanese, got: {label}"
    );
}

#[test]
fn status_display_no_answer_is_default() {
    let (_, _, label_none) = status_display(None);
    let (_, _, label_unknown) = status_display(Some("unknown_value"));
    assert_eq!(
        label_none, label_unknown,
        "unknown status must use same label as None"
    );
    assert!(!label_none.is_empty());
}
