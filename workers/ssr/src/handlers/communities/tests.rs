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
fn switcher_next_preserves_list_mode() {
    assert_eq!(
        matrix::switcher_next(2026, 7, None, matrix::CalendarView::List),
        "communities:2026-07:list"
    );
    assert_eq!(
        matrix::switcher_next(2026, 7, Some("2026-07-05"), matrix::CalendarView::List),
        "communities:2026-07:2026-07-05:list"
    );
}

#[test]
fn calendar_view_from_query_parses_and_falls_back() {
    assert_eq!(
        matrix::CalendarView::from_query(Some("list")),
        matrix::CalendarView::List
    );
    assert_eq!(
        matrix::CalendarView::from_query(Some("matrix")),
        matrix::CalendarView::Matrix
    );
    assert_eq!(
        matrix::CalendarView::from_query(None),
        matrix::CalendarView::Month
    );
    assert_eq!(
        matrix::CalendarView::from_query(Some("garbage")),
        matrix::CalendarView::Month
    );
    assert_eq!(
        matrix::CalendarView::from_query(Some("")),
        matrix::CalendarView::Month
    );
}

#[test]
fn render_mode_tabs_shows_three_tabs_and_list_href_omits_day() {
    let html = matrix::render_mode_tabs(
        "community-a",
        2026,
        7,
        Some("2026-07-05"),
        matrix::CalendarView::Month,
    );
    assert!(html.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_VIEW_MONTH));
    assert!(html.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_VIEW_LIST));
    assert!(html.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_VIEW_MATRIX));
    assert!(
        html.contains("/c/community-a/communities?month=2026-07&amp;view=list\""),
        "the Events list tab href must omit `day` even when a day is selected: {html}"
    );
    assert!(
        html.contains(
            "/c/community-a/communities?month=2026-07&amp;day=2026-07-05&amp;view=matrix\""
        ),
        "the matrix tab href must still preserve the selected day: {html}"
    );
}

#[test]
fn day_detail_always_renders_and_is_day_scoped_not_full_month() {
    let rows = vec![
        event_row("day_1", "event_1", "2026-07-05", "Morning", "scheduled"),
        event_row("day_2", "event_2", "2026-07-06", "Lunch", "scheduled"),
    ];

    let no_day_selected = calendar::render_calendar_day_detail(
        "community-a",
        "Asia/Tokyo",
        &rows,
        None,
        2026,
        7,
        false,
    );
    assert!(no_day_selected.contains("id=\"calendar-day-detail\""));
    assert!(no_day_selected.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_DAY_DETAIL_PROMPT));
    assert!(
        !no_day_selected.contains("/c/community-a/events/event_1")
            && !no_day_selected.contains("/c/community-a/events/event_2"),
        "with no day selected, day detail must not leak the full month list: {no_day_selected}"
    );

    let day_selected = calendar::render_calendar_day_detail(
        "community-a",
        "Asia/Tokyo",
        &rows,
        Some("2026-07-05"),
        2026,
        7,
        false,
    );
    assert!(day_selected.contains("id=\"calendar-day-detail\""));
    assert!(day_selected.contains("/c/community-a/events/event_1"));
    assert!(
        !day_selected.contains("/c/community-a/events/event_2"),
        "day detail must be scoped to the selected day only, not the full month: {day_selected}"
    );
}

#[test]
fn events_list_tab_ignores_selected_day_and_shows_full_month() {
    let rows = vec![
        event_row("day_1", "event_1", "2026-07-05", "Morning", "scheduled"),
        event_row("day_2", "event_2", "2026-07-06", "Lunch", "scheduled"),
    ];

    let html = calendar::render_calendar_list("community-a", "Asia/Tokyo", &rows, 2026, 7, false);
    assert!(html.contains("/c/community-a/events/event_1"));
    assert!(html.contains("/c/community-a/events/event_2"));
    assert!(html.contains("view=list"));
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
        can_export_csv: false,
        export_token: None,
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
    assert!(!html.contains("data-export-value"));
}

#[test]
fn matrix_render_adds_csv_export_contract_for_admin_only() {
    let members = vec![membership::MemberSummary {
        id: "mem_a".to_string(),
        display_name: "Alice, Example".to_string(),
        role: "admin".to_string(),
    }];
    let rows = vec![
        event_row("day_1", "event_1", "2026-07-05", "Morning", "scheduled"),
        event_row("day_2", "event_2", "2026-07-06", "Cancelled", "cancelled"),
    ];
    let attendances = attendance_map(vec![attendance_row("day_1", "mem_a", Some("going"))]);

    let html = matrix::render_matrix(matrix::MatrixRenderInput {
        community_id: "community-a",
        community_tz: "Asia/Tokyo",
        year: 2026,
        month: 7,
        selected_day: None,
        can_export_csv: true,
        export_token: Some("tok_admin"),
        rows: &rows,
        members: &members,
        attendances: &attendances,
    });

    assert!(html.contains("data-calendar-matrix-export=\"true\""));
    assert!(html.contains("data-calendar-matrix-export-button=\"true\""));
    assert!(html.contains("data-audit-url=\"/c/community-a/admin/calendar/matrix-export/audit\""));
    assert!(html.contains("data-month=\"2026-07\""));
    assert!(html.contains("data-export-type=\"calendar_matrix_csv\""));
    assert!(html.contains("data-token=\"tok_admin\""));
    assert!(html.contains("data-date=\"2026-07-05\""));
    assert!(html.contains("data-member-name=\"Alice, Example\""));
    assert!(html.contains("data-export-value=\"○\""));
    assert!(html.contains("data-export-value=\"中\""));
    assert!(html.contains("data-export-value=\"\""));
}

#[test]
fn matrix_render_omits_csv_export_contract_for_non_admin() {
    let members = vec![membership::MemberSummary {
        id: "mem_a".to_string(),
        display_name: "Alice".to_string(),
        role: "member".to_string(),
    }];
    let rows = vec![event_row(
        "day_1",
        "event_1",
        "2026-07-05",
        "Morning",
        "scheduled",
    )];
    let attendances = attendance_map(vec![attendance_row("day_1", "mem_a", Some("going"))]);

    let html = matrix::render_matrix(matrix::MatrixRenderInput {
        community_id: "community-a",
        community_tz: "Asia/Tokyo",
        year: 2026,
        month: 7,
        selected_day: None,
        can_export_csv: false,
        export_token: None,
        rows: &rows,
        members: &members,
        attendances: &attendances,
    });

    assert!(!html.contains("data-calendar-matrix-export"));
    assert!(!html.contains("data-calendar-matrix-export-button"));
    assert!(!html.contains("data-export-value"));
    assert!(!html.contains("data-member-name"));
    assert!(!html.contains("data-date=\"2026-07-05\""));
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
        can_export_csv: true,
        export_token: Some("tok_admin"),
        rows: &[],
        members: &members,
        attendances: &HashMap::new(),
    });

    assert!(html.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_MATRIX_TOO_LARGE));
    assert!(html.contains("/c/community-a/communities?month=2026-07"));
    assert!(!html.contains("data-calendar-matrix-export"));
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
        can_export_csv: true,
        export_token: Some("tok_admin"),
        rows: &rows,
        members: &members,
        attendances: &HashMap::new(),
    });

    assert!(html.contains(zinnias_ciao_contracts::i18n::JA_CALENDAR_MATRIX_TOO_LARGE));
    assert!(html.contains("/c/community-a/communities?month=2026-07"));
    assert!(!html.contains("data-calendar-matrix-export"));
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
        can_export_csv: false,
        export_token: None,
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
