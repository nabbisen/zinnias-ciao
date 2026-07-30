use std::collections::HashMap;

use crate::db::{attendance, event as event_db, membership};
use zinnias_ciao_contracts::{Locale, i18n};

pub(super) struct CellSummary {
    pub(super) visual: String,
    pub(super) export_value: String,
    pub(super) label: String,
    pub(super) color: &'static str,
    pub(super) background: &'static str,
}

/// Substitute a template's `{}` placeholders positionally, in order. Not
/// `format!` (the template is a runtime `&str`, not a literal) — every
/// `Localized` template consumed here must have the same placeholder count
/// on both `ja` and `en`, checked by
/// `cell_label_templates_have_matching_placeholder_counts` (RFC-072 Slice C).
/// Excess values are ignored; a template asking for more values than given
/// leaves that placeholder's `{}` in the output rather than panicking.
pub(super) fn substitute_positional(template: &str, values: &[&str]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    let mut next = values.iter();
    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'}') {
            chars.next();
            if let Some(value) = next.next() {
                result.push_str(value);
            } else {
                result.push_str("{}");
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub(super) fn cell_summary(
    day_date: &str,
    member: &membership::MemberSummary,
    events: &[&event_db::HomeEventRow],
    attendances: &HashMap<String, Vec<attendance::AttendanceRow>>,
    locale: Locale,
) -> CellSummary {
    if events.is_empty() {
        return CellSummary {
            visual: "&nbsp;".to_string(),
            export_value: String::new(),
            label: substitute_positional(
                i18n::t(locale, i18n::CALENDAR_MATRIX_CELL_NO_EVENTS),
                &[day_date, &member.display_name],
            ),
            color: "#8E8E93",
            background: "#FAFAFB",
        };
    }

    if events.len() == 1 {
        let row = events[0];
        if event_day_cancelled(row) {
            return CellSummary {
                visual: "中".to_string(),
                export_value: "中".to_string(),
                label: substitute_positional(
                    i18n::t(locale, i18n::CALENDAR_MATRIX_CELL_CANCELLED),
                    &[day_date, &member.display_name],
                ),
                color: "#6E6E73",
                background: "#F5F5F7",
            };
        }
        let status = status_for_member(&row.day_id, &member.id, attendances);
        let (visual, label_status, color, bg) = single_status_display(locale, status);
        return CellSummary {
            visual: visual.to_string(),
            export_value: visual.to_string(),
            label: substitute_positional(
                i18n::t(locale, i18n::CALENDAR_MATRIX_CELL_SINGLE_STATUS),
                &[day_date, &member.display_name, label_status],
            ),
            color,
            background: bg,
        };
    }

    let mut going = 0usize;
    let mut not_going = 0usize;
    let mut attended = 0usize;
    let mut cancelled = 0usize;
    let mut total = 0usize;
    for row in events {
        if event_day_cancelled(row) {
            cancelled += 1;
            continue;
        }
        total += 1;
        match status_for_member(&row.day_id, &member.id, attendances) {
            Some("going") => going += 1,
            Some("not_going") => not_going += 1,
            Some("attended") => attended += 1,
            _ => {}
        }
    }
    if total == 0 {
        return CellSummary {
            visual: "中".to_string(),
            export_value: "中".to_string(),
            label: substitute_positional(
                i18n::t(locale, i18n::CALENDAR_MATRIX_CELL_BREAKDOWN),
                &[
                    day_date,
                    &member.display_name,
                    &events.len().to_string(),
                    &cancelled.to_string(),
                    "0",
                    "0",
                    "0",
                    "0",
                ],
            ),
            color: "#6E6E73",
            background: "#F5F5F7",
        };
    }
    let answered = going + not_going + attended;
    let no_reply = total.saturating_sub(answered);
    CellSummary {
        visual: format!("{answered}/{total}"),
        export_value: format!("{answered}/{total}"),
        label: substitute_positional(
            i18n::t(locale, i18n::CALENDAR_MATRIX_CELL_BREAKDOWN),
            &[
                day_date,
                &member.display_name,
                &events.len().to_string(),
                &cancelled.to_string(),
                &going.to_string(),
                &not_going.to_string(),
                &attended.to_string(),
                &no_reply.to_string(),
            ],
        ),
        color: if no_reply == 0 { "#0A7F43" } else { "#3A3A3C" },
        background: "#FFFFFF",
    }
}

fn single_status_display(
    locale: Locale,
    status: Option<&str>,
) -> (&'static str, &'static str, &'static str, &'static str) {
    match status {
        Some("going") => (
            "○",
            i18n::t(locale, i18n::STATUS_GOING),
            "#0A7F43",
            "#F0FFF6",
        ),
        Some("not_going") => (
            "×",
            i18n::t(locale, i18n::STATUS_NOT_GOING),
            "#B42318",
            "#FFF5F3",
        ),
        Some("attended") => (
            "済",
            i18n::t(locale, i18n::STATUS_ATTENDED),
            "#0057B8",
            "#F0F7FF",
        ),
        _ => (
            "?",
            i18n::t(locale, i18n::STATUS_NO_ANSWER),
            "#6E6E73",
            "#FFFFFF",
        ),
    }
}

#[derive(Default)]
pub(super) struct AggregateCounts {
    pub(super) going: usize,
    pub(super) not_going: usize,
    pub(super) attended: usize,
    pub(super) no_answer: usize,
}

pub(super) fn aggregate_counts(
    day_id: &str,
    member_count: usize,
    attendances: &HashMap<String, Vec<attendance::AttendanceRow>>,
) -> AggregateCounts {
    let mut counts = AggregateCounts::default();
    let rows = attendances.get(day_id).map(Vec::as_slice).unwrap_or(&[]);
    for row in rows {
        match row.status.as_deref() {
            Some("going") => counts.going += 1,
            Some("not_going") => counts.not_going += 1,
            Some("attended") => counts.attended += 1,
            _ => {}
        }
    }
    let answered = counts.going + counts.not_going + counts.attended;
    counts.no_answer = member_count.saturating_sub(answered);
    counts
}

fn status_for_member<'a>(
    day_id: &str,
    member_id: &str,
    attendances: &'a HashMap<String, Vec<attendance::AttendanceRow>>,
) -> Option<&'a str> {
    attendances
        .get(day_id)?
        .iter()
        .find(|row| row.membership_id == member_id)
        .and_then(|row| row.status.as_deref())
}

pub(super) fn event_day_cancelled(row: &event_db::HomeEventRow) -> bool {
    row.event_status == "cancelled" || row.occurrence_status == "cancelled"
}
