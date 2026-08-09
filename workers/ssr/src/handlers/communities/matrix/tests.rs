use super::cells::{cell_summary, substitute_positional};
use super::detail::render_date_detail;
use crate::db::attendance;
use crate::db::event as event_db;
use crate::db::membership;
use std::collections::HashMap;
use zinnias_ciao_contracts::{Locale, i18n};

#[test]
fn date_detail_no_day_selected_follows_locale() {
    let rows_by_date: HashMap<String, Vec<&event_db::HomeEventRow>> = HashMap::new();
    let attendances: HashMap<String, Vec<attendance::AttendanceRow>> = HashMap::new();

    let ja = render_date_detail(
        "community-a",
        "Asia/Tokyo",
        None,
        &rows_by_date,
        3,
        &attendances,
        Locale::Ja,
    );
    assert!(ja.contains(&format!(">{}</h3>", i18n::JA_HOME_AGENDA_TITLE)));
    assert!(ja.contains(i18n::JA_CALENDAR_EMPTY_MONTH));
    assert!(!ja.contains(&format!(">{}</h3>", i18n::EN_HOME_AGENDA_TITLE)));
    assert!(!ja.contains(i18n::EN_CALENDAR_EMPTY_MONTH));

    let en = render_date_detail(
        "community-a",
        "Asia/Tokyo",
        None,
        &rows_by_date,
        3,
        &attendances,
        Locale::En,
    );
    assert!(en.contains(&format!(">{}</h3>", i18n::EN_HOME_AGENDA_TITLE)));
    assert!(en.contains(i18n::EN_CALENDAR_EMPTY_MONTH));
    assert!(!en.contains(&format!(">{}</h3>", i18n::JA_HOME_AGENDA_TITLE)));
    assert!(!en.contains(i18n::JA_CALENDAR_EMPTY_MONTH));
}

#[test]
fn substitute_positional_fills_placeholders_in_order() {
    assert_eq!(
        substitute_positional("{}, {}, going: {}", &["Mon, 3 Aug", "Alice", "2"]),
        "Mon, 3 Aug, Alice, going: 2"
    );
}

#[test]
fn substitute_positional_leaves_unfilled_placeholder_rather_than_panicking() {
    assert_eq!(
        substitute_positional("{}, {}", &["only-one"]),
        "only-one, {}"
    );
}

#[test]
fn cell_summary_labels_follow_locale() {
    let member = membership::MemberSummary {
        id: "mem_a".to_string(),
        display_name: "Alice".to_string(),
        role: "member".to_string(),
    };
    let attendances: HashMap<String, Vec<attendance::AttendanceRow>> = HashMap::new();

    let ja = cell_summary("2026-07-05", &member, &[], &attendances, Locale::Ja);
    assert!(ja.label.contains("予定なし"));
    assert!(!ja.label.contains("no events"));

    let en = cell_summary("2026-07-05", &member, &[], &attendances, Locale::En);
    assert!(en.label.contains("no events"));
    assert!(!en.label.contains("予定なし"));
}

#[test]
fn cell_summary_breakdown_avoids_pluralization_in_english() {
    let member = membership::MemberSummary {
        id: "mem_a".to_string(),
        display_name: "Alice".to_string(),
        role: "member".to_string(),
    };
    let row_a = event_db::HomeEventRow {
        community_id: "community-a".to_string(),
        event_id: "event_1".to_string(),
        event_title: "Morning".to_string(),
        event_location: None,
        event_status: "scheduled".to_string(),
        day_id: "day_1".to_string(),
        day_date: "2026-07-05".to_string(),
        starts_at_utc: "2026-07-05T00:00:00Z".to_string(),
        ends_at_utc: "2026-07-05T01:00:00Z".to_string(),
        occurrence_status: "scheduled".to_string(),
    };
    let row_b = event_db::HomeEventRow {
        community_id: "community-a".to_string(),
        event_id: "event_2".to_string(),
        event_title: "Lunch".to_string(),
        event_location: None,
        event_status: "scheduled".to_string(),
        day_id: "day_2".to_string(),
        day_date: "2026-07-05".to_string(),
        starts_at_utc: "2026-07-05T02:00:00Z".to_string(),
        ends_at_utc: "2026-07-05T03:00:00Z".to_string(),
        occurrence_status: "scheduled".to_string(),
    };
    let events: Vec<&event_db::HomeEventRow> = vec![&row_a, &row_b];
    let attendances: HashMap<String, Vec<attendance::AttendanceRow>> = HashMap::from([(
        "day_1".to_string(),
        vec![attendance::AttendanceRow {
            event_day_id: "day_1".to_string(),
            membership_id: "mem_a".to_string(),
            status: Some("going".to_string()),
        }],
    )]);

    let en = cell_summary("2026-07-05", &member, &events, &attendances, Locale::En);
    assert!(
        en.label.contains("going: 1"),
        "must use label-value form, not pluralized count: {}",
        en.label
    );
    assert!(!en.label.contains("1 events"));
    assert!(!en.label.contains("1 going"));
}
