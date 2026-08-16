// RFC-072: the i18n accessor boundary. `t` must resolve each `Localized`
// pair to the correct language's string with no per-call-site match.
#[test]
fn t_resolves_locale_aware_pairs() {
    use super::{ME_SECTION_NAME, NAV_ME, ROLE_ADMIN, t};
    use crate::locale::Locale;

    assert_eq!(t(Locale::Ja, NAV_ME), super::JA_NAV_ME);
    assert_eq!(t(Locale::En, NAV_ME), super::EN_NAV_ME);
    assert_eq!(t(Locale::Ja, ME_SECTION_NAME), super::JA_ME_SECTION_NAME);
    assert_eq!(t(Locale::En, ME_SECTION_NAME), super::EN_ME_SECTION_NAME);
    assert_eq!(t(Locale::Ja, ROLE_ADMIN), super::JA_ROLE_ADMIN);
    assert_eq!(t(Locale::En, ROLE_ADMIN), super::EN_ROLE_ADMIN);
}

// RFC-072 Slice C: matrix/cells.rs substitutes these templates' `{}`
// placeholders positionally, not through `format!`. A mismatched
// placeholder count between `ja` and `en` would be a silent rendering bug
// (an English label missing a count, or a stray literal "{}" in output) —
// this test is the guard the handoff's §8 requires.
#[test]
fn cell_label_templates_have_matching_placeholder_counts() {
    use super::{
        CALENDAR_MATRIX_CELL_BREAKDOWN, CALENDAR_MATRIX_CELL_CANCELLED,
        CALENDAR_MATRIX_CELL_NO_EVENTS, CALENDAR_MATRIX_CELL_SINGLE_STATUS,
    };

    fn placeholder_count(s: &str) -> usize {
        s.matches("{}").count()
    }

    for (name, pair) in [
        (
            "CALENDAR_MATRIX_CELL_NO_EVENTS",
            CALENDAR_MATRIX_CELL_NO_EVENTS,
        ),
        (
            "CALENDAR_MATRIX_CELL_CANCELLED",
            CALENDAR_MATRIX_CELL_CANCELLED,
        ),
        (
            "CALENDAR_MATRIX_CELL_SINGLE_STATUS",
            CALENDAR_MATRIX_CELL_SINGLE_STATUS,
        ),
        (
            "CALENDAR_MATRIX_CELL_BREAKDOWN",
            CALENDAR_MATRIX_CELL_BREAKDOWN,
        ),
    ] {
        assert_eq!(
            placeholder_count(pair.ja),
            placeholder_count(pair.en),
            "{name}: ja and en must consume the same number of positional substitutions"
        );
    }
}

// Handoff 036 §4.2: JA_ADMIN_ATTEND_MEMBER_ARIA_LABEL is a bare constant (not
// a Localized pair — this admin page is Japanese-only by RFC-072 Slice D
// decision, so there is no `en` counterpart to compare against). The stable
// invariant to pin instead is its own placeholder count, matching
// `attendance.rs`'s `substitute_positional(..., &[&name])` call — one value,
// one placeholder — so a future edit can't silently change the arity out
// from under that call.
#[test]
fn admin_attend_member_aria_label_has_one_placeholder() {
    assert_eq!(
        super::JA_ADMIN_ATTEND_MEMBER_ARIA_LABEL
            .matches("{}")
            .count(),
        1,
        "JA_ADMIN_ATTEND_MEMBER_ARIA_LABEL must have exactly one {{}} placeholder, matching \
         attendance.rs's single-value substitute_positional call"
    );
}

// Handoff 036 §A: JA_ADMIN_EXPORT_SUMMARY_COUNTS is substituted by name
// (`.replace("{events}", ...)`/`.replace("{members}", ...)` in export.rs),
// not positionally — pin both placeholders present, in the reviewer-specified
// order (events, then members), so a future edit can't silently drop one or
// swap the order out from under export.rs's two `.replace()` calls.
#[test]
fn admin_export_summary_counts_has_both_named_placeholders_in_order() {
    let s = super::JA_ADMIN_EXPORT_SUMMARY_COUNTS;
    let events_pos = s
        .find("{events}")
        .expect("JA_ADMIN_EXPORT_SUMMARY_COUNTS must contain the {events} placeholder");
    let members_pos = s
        .find("{members}")
        .expect("JA_ADMIN_EXPORT_SUMMARY_COUNTS must contain the {members} placeholder");
    assert!(
        events_pos < members_pos,
        "JA_ADMIN_EXPORT_SUMMARY_COUNTS must keep {{events}} before {{members}}, matching the \
         original literal's order"
    );
}
