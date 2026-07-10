use super::{calendar, matrix};
use crate::db::{attendance, event as event_db, membership};
use std::collections::HashMap;

#[test]
fn month_parse_rejects_bad_values() {
    assert_eq!(calendar::parse_month("2026-07"), Some((2026, 7)));
    assert_eq!(calendar::parse_month("2026-13"), None);
    assert_eq!(calendar::parse_month("202607"), None);
    assert_eq!(calendar::parse_month("2026/07"), None);
}

#[test]
fn ymd_parse_rejects_bad_values() {
    assert_eq!(calendar::parse_ymd("2026-07-05"), Some((2026, 7, 5)));
    assert_eq!(calendar::parse_ymd("2026-07-05x"), None);
    assert_eq!(calendar::parse_ymd("2026-07-32"), None);
    assert_eq!(calendar::parse_ymd("2026/07/05"), None);
}

#[test]
fn add_months_crosses_years() {
    assert_eq!(calendar::add_months(2026, 1, -1), (2025, 12));
    assert_eq!(calendar::add_months(2026, 12, 1), (2027, 1));
}

#[test]
fn switcher_next_preserves_month_and_day() {
    assert_eq!(
        matrix::switcher_next(2026, 7, None, matrix::CalendarView::Month),
        "communities:2026-07"
    );
    assert_eq!(
        matrix::switcher_next(2026, 7, Some("2026-07-05"), matrix::CalendarView::Month),
        "communities:2026-07:2026-07-05"
    );
}

#[test]
fn switcher_next_preserves_matrix_mode() {
    assert_eq!(
        matrix::switcher_next(2026, 7, None, matrix::CalendarView::Matrix),
        "communities:2026-07:matrix"
    );
    assert_eq!(
        matrix::switcher_next(2026, 7, Some("2026-07-05"), matrix::CalendarView::Matrix),
        "communities:2026-07:2026-07-05:matrix"
    );
}

#[test]
fn matrix_render_uses_contract_symbols_and_multi_event_summary() {
    let members = vec![membership::MemberSummary {
        id: "mem_a".to_string(),
        display_name: "Alice".to_string(),
        role: "member".to_string(),
    }];
    let rows = vec![
        event_row("day_1", "event_1", "2026-07-05", "Morning", "scheduled"),
        event_row("day_2", "event_2", "2026-07-06", "Lunch", "scheduled"),
        event_row("day_3", "event_3", "2026-07-06", "Dinner", "scheduled"),
        event_row("day_4", "event_4", "2026-07-07", "Cancelled", "cancelled"),
    ];
    let attendances = attendance_map(vec![
        attendance_row("day_1", "mem_a", Some("going")),
        attendance_row("day_2", "mem_a", Some("not_going")),
    ]);

    let html = matrix::render_matrix(matrix::MatrixRenderInput {
        community_id: "community-a",
        community_tz: "Asia/Tokyo",
        year: 2026,
        month: 7,
        selected_day: Some("2026-07-06"),
        rows: &rows,
        members: &members,
        attendances: &attendances,
    });

    assert!(html.contains(">○</td>"));
    assert!(html.contains(">1/2</td>"));
    assert!(html.contains(">中</td>"));
    assert!(html.contains("予定2件"));
    assert!(html.contains("不参加1件"));
    assert!(html.contains("未回答1件"));
    assert!(html.contains("/c/community-a/events/event_2"));
    assert!(!html.to_ascii_lowercase().contains("csv"));
}

#[test]
fn matrix_render_shows_cap_fallback() {
    let members = (0..=matrix::MEMBER_ROW_CAP)
        .map(|idx| membership::MemberSummary {
            id: format!("mem_{idx}"),
            display_name: format!("Member {idx}"),
            role: "member".to_string(),
        })
        .collect::<Vec<_>>();
    let html = matrix::render_matrix(matrix::MatrixRenderInput {
        community_id: "community-a",
        community_tz: "Asia/Tokyo",
        year: 2026,
        month: 7,
        selected_day: None,
        rows: &[],
        members: &members,
        attendances: &HashMap::new(),
    });

    assert!(html.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_MATRIX_TOO_LARGE));
    assert!(html.contains("/c/community-a/communities?month=2026-07"));
}

#[test]
fn matrix_render_shows_event_day_over_cap_fallback() {
    let members = vec![membership::MemberSummary {
        id: "mem_1".to_string(),
        display_name: "Member 1".to_string(),
        role: "member".to_string(),
    }];
    let rows = (0..=matrix::EVENT_DAY_ROW_CAP)
        .map(|idx| {
            event_row(
                &format!("day_{idx}"),
                &format!("event_{idx}"),
                "2026-07-15",
                "Event",
                "scheduled",
            )
        })
        .collect::<Vec<_>>();
    let html = matrix::render_matrix(matrix::MatrixRenderInput {
        community_id: "community-a",
        community_tz: "Asia/Tokyo",
        year: 2026,
        month: 7,
        selected_day: None,
        rows: &rows,
        members: &members,
        attendances: &HashMap::new(),
    });

    assert!(html.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_MATRIX_TOO_LARGE));
    assert!(html.contains("/c/community-a/communities?month=2026-07"));
}

#[test]
fn matrix_render_allows_event_day_cap_boundary() {
    let members = vec![membership::MemberSummary {
        id: "mem_1".to_string(),
        display_name: "Member 1".to_string(),
        role: "member".to_string(),
    }];
    let rows = (0..matrix::EVENT_DAY_ROW_CAP)
        .map(|idx| {
            event_row(
                &format!("day_{idx}"),
                &format!("event_{idx}"),
                "2026-07-15",
                "Event",
                "scheduled",
            )
        })
        .collect::<Vec<_>>();
    let html = matrix::render_matrix(matrix::MatrixRenderInput {
        community_id: "community-a",
        community_tz: "Asia/Tokyo",
        year: 2026,
        month: 7,
        selected_day: None,
        rows: &rows,
        members: &members,
        attendances: &HashMap::new(),
    });

    assert!(!html.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_MATRIX_TOO_LARGE));
    assert!(html.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_MATRIX_TITLE));
}

fn event_row(
    day_id: &str,
    event_id: &str,
    day_date: &str,
    title: &str,
    occurrence_status: &str,
) -> event_db::HomeEventRow {
    event_db::HomeEventRow {
        community_id: "community-a".to_string(),
        event_id: event_id.to_string(),
        event_title: title.to_string(),
        event_location: None,
        event_status: "scheduled".to_string(),
        day_id: day_id.to_string(),
        day_date: day_date.to_string(),
        starts_at_utc: format!("{day_date}T00:00:00Z"),
        ends_at_utc: format!("{day_date}T01:00:00Z"),
        occurrence_status: occurrence_status.to_string(),
        series_id: None,
        seq: 1,
        total_days: 1,
    }
}

fn attendance_row(
    day_id: &str,
    member_id: &str,
    status: Option<&str>,
) -> attendance::AttendanceRow {
    attendance::AttendanceRow {
        event_day_id: day_id.to_string(),
        membership_id: member_id.to_string(),
        status: status.map(str::to_string),
        status_updated_at: Some("2026-07-01T00:00:00Z".to_string()),
    }
}

fn attendance_map(
    rows: Vec<attendance::AttendanceRow>,
) -> HashMap<String, Vec<attendance::AttendanceRow>> {
    let mut map: HashMap<String, Vec<attendance::AttendanceRow>> = HashMap::new();
    for row in rows {
        map.entry(row.event_day_id.clone()).or_default().push(row);
    }
    map
}
