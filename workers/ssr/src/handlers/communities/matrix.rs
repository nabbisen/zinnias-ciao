//! Monthly attendance matrix rendering and view-model helpers (RFC-067).

use std::collections::HashMap;

use crate::db::{attendance, event as event_db, membership};
use crate::render;
use cells::cell_summary;
use detail::render_date_detail;
use zinnias_ciao_contracts::{i18n, tz};

mod cells;
mod detail;

pub(super) const MEMBER_ROW_CAP: usize = 100;
pub(super) const EVENT_DAY_ROW_CAP: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CalendarView {
    Month,
    Matrix,
}

impl CalendarView {
    pub(super) fn from_query(value: Option<&str>) -> Self {
        match value {
            Some("matrix") => Self::Matrix,
            _ => Self::Month,
        }
    }
}

pub(super) fn switcher_next(
    year: i32,
    month: i32,
    selected_day: Option<&str>,
    view: CalendarView,
) -> String {
    match (selected_day, view) {
        (Some(day), CalendarView::Matrix) => {
            format!("communities:{year:04}-{month:02}:{day}:matrix")
        }
        (None, CalendarView::Matrix) => format!("communities:{year:04}-{month:02}:matrix"),
        (Some(day), CalendarView::Month) => format!("communities:{year:04}-{month:02}:{day}"),
        (None, CalendarView::Month) => format!("communities:{year:04}-{month:02}"),
    }
}

pub(super) fn render_mode_tabs(
    community_id: &str,
    year: i32,
    month: i32,
    selected_day: Option<&str>,
    current: CalendarView,
) -> String {
    let month_key = format!("{year:04}-{month:02}");
    let day_query = selected_day
        .map(|day| format!("&amp;day={}", render::escape_html(day)))
        .unwrap_or_default();
    let month_href = format!(
        "/c/{}/communities?month={}{}",
        render::escape_html(community_id),
        render::escape_html(&month_key),
        day_query
    );
    let matrix_href = format!(
        "/c/{}/communities?month={}{}&amp;view=matrix",
        render::escape_html(community_id),
        render::escape_html(&month_key),
        day_query
    );
    let tab = |href: &str, label: &str, selected: bool| {
        let (bg, color, border, aria) = if selected {
            ("#1D1D1F", "#FFFFFF", "#1D1D1F", " aria-current=\"page\"")
        } else {
            ("#FFFFFF", "#1D1D1F", "#D1D1D6", "")
        };
        format!(
            "<a href=\"{href}\"{aria} style=\"min-height:40px;display:inline-flex;\
             align-items:center;justify-content:center;padding:.35rem .75rem;\
             border:1px solid {border};border-radius:8px;background:{bg};color:{color};\
             text-decoration:none;font-size:.875rem;font-weight:700;white-space:nowrap\">\
             {label}</a>"
        )
    };

    format!(
        "<nav aria-label=\"Calendar view\" style=\"display:flex;gap:.5rem;\
         margin:0 auto 1rem;max-width:42rem;flex-wrap:wrap\">{}{}\
         </nav>",
        tab(
            &month_href,
            i18n::JA_CALENDAR_VIEW_MONTH,
            current == CalendarView::Month
        ),
        tab(
            &matrix_href,
            i18n::JA_CALENDAR_VIEW_MATRIX,
            current == CalendarView::Matrix
        )
    )
}

pub(super) struct MatrixRenderInput<'a> {
    pub(super) community_id: &'a str,
    pub(super) community_tz: &'a str,
    pub(super) year: i32,
    pub(super) month: i32,
    pub(super) selected_day: Option<&'a str>,
    pub(super) rows: &'a [event_db::HomeEventRow],
    pub(super) members: &'a [membership::MemberSummary],
    pub(super) attendances: &'a HashMap<String, Vec<attendance::AttendanceRow>>,
}

pub(super) fn render_matrix(input: MatrixRenderInput<'_>) -> String {
    let MatrixRenderInput {
        community_id,
        community_tz,
        year,
        month,
        selected_day,
        rows,
        members,
        attendances,
    } = input;

    if members.is_empty() {
        return format!(
            "<section style=\"margin:0 auto 1.5rem;max-width:42rem\">\
             <h2 style=\"font-size:1.125rem;font-weight:700;margin:0\">{}</h2>\
             <p style=\"font-size:.875rem;color:#6e6e73;margin:.75rem 0 0\">{}</p>\
             </section>",
            i18n::JA_CALENDAR_MATRIX_TITLE,
            i18n::JA_CALENDAR_MATRIX_NO_MEMBERS
        );
    }

    if members.len() > MEMBER_ROW_CAP || rows.len() > EVENT_DAY_ROW_CAP {
        return render_too_large(community_id, year, month);
    }

    let month_key = format!("{year:04}-{month:02}");
    let days_in_month = tz::days_in_month(year, month);
    let mut rows_by_date: HashMap<String, Vec<&event_db::HomeEventRow>> = HashMap::new();
    for row in rows {
        rows_by_date
            .entry(row.day_date.clone())
            .or_default()
            .push(row);
    }
    for date_rows in rows_by_date.values_mut() {
        date_rows.sort_by(|a, b| {
            a.starts_at_utc
                .cmp(&b.starts_at_utc)
                .then_with(|| a.event_title.cmp(&b.event_title))
        });
    }

    let detail_day = selected_day
        .map(str::to_owned)
        .or_else(|| rows.first().map(|row| row.day_date.clone()));

    let mut header_cells = String::new();
    for day in 1..=days_in_month {
        let day_date = format!("{year:04}-{month:02}-{day:02}");
        let selected = detail_day.as_deref() == Some(day_date.as_str());
        let href = format!(
            "/c/{}/communities?month={}&amp;day={}&amp;view=matrix",
            render::escape_html(community_id),
            render::escape_html(&month_key),
            render::escape_html(&day_date)
        );
        let bg = if selected { "#EAF3FF" } else { "#F5F5F7" };
        let border = if selected { "#007AFF" } else { "#E5E5EA" };
        let aria_current = if selected {
            " aria-current=\"date\""
        } else {
            ""
        };
        header_cells.push_str(&format!(
            "<th scope=\"col\" style=\"position:sticky;top:0;z-index:2;\
             background:{bg};border:1px solid {border};padding:0;min-width:3.25rem;\
             text-align:center\">\
             <a href=\"{href}\"{aria_current} style=\"display:flex;min-height:44px;\
             align-items:center;justify-content:center;color:#1D1D1F;text-decoration:none;\
             font-size:.8125rem;font-weight:700\">{day}</a></th>"
        ));
    }

    let mut body_rows = String::new();
    for member in members {
        let mut cells = String::new();
        for day in 1..=days_in_month {
            let day_date = format!("{year:04}-{month:02}-{day:02}");
            let events = rows_by_date
                .get(&day_date)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let cell = cell_summary(&day_date, member, events, attendances);
            cells.push_str(&format!(
                "<td aria-label=\"{label}\" style=\"border:1px solid #E5E5EA;\
                 min-width:3.25rem;height:2.75rem;text-align:center;vertical-align:middle;\
                 font-size:.875rem;font-weight:700;color:{color};background:{bg}\">\
                 {visual}</td>",
                label = render::escape_html(&cell.label),
                color = cell.color,
                bg = cell.background,
                visual = cell.visual
            ));
        }
        body_rows.push_str(&format!(
            "<tr><th scope=\"row\" style=\"position:sticky;left:0;z-index:1;\
             background:#FFFFFF;border:1px solid #E5E5EA;text-align:left;\
             min-width:8rem;max-width:10rem;padding:.5rem;font-size:.875rem;\
             line-height:1.3;white-space:normal\">{name}</th>{cells}</tr>",
            name = render::escape_html(&member.display_name),
            cells = cells
        ));
    }

    let detail = render_date_detail(
        community_id,
        community_tz,
        detail_day.as_deref(),
        &rows_by_date,
        members.len(),
        attendances,
    );
    let (prev_year, prev_month) = super::calendar::add_months(year, month, -1);
    let (next_year, next_month) = super::calendar::add_months(year, month, 1);
    let month_url =
        |y: i32, m: i32| format!("/c/{community_id}/communities?month={y:04}-{m:02}&view=matrix");
    let current_url = format!("/c/{community_id}/communities?view=matrix");

    format!(
        "<section aria-label=\"{title}\" style=\"margin:0 auto 1.5rem;\
         max-width:100%\">\
         <div style=\"max-width:42rem;margin:0 auto .75rem;display:flex;\
         align-items:flex-end;justify-content:space-between;gap:.75rem;flex-wrap:wrap\">\
         <h2 style=\"font-size:1.125rem;font-weight:700;margin:0\">{title}</h2>\
         <p style=\"font-size:.9375rem;font-weight:700;color:#6e6e73;margin:0\">\
         {year}年{month}月</p></div>\
         <nav aria-label=\"Calendar month\" style=\"display:flex;gap:.5rem;\
         align-items:center;justify-content:space-between;margin:0 auto .75rem;\
         max-width:42rem\">\
         <a href=\"{prev_url}\" style=\"min-height:44px;display:inline-flex;align-items:center;\
         color:#007AFF;text-decoration:none;font-size:.875rem\">{prev_label}</a>\
         <a href=\"{current_url}\" style=\"min-height:44px;display:inline-flex;align-items:center;\
         color:#007AFF;text-decoration:none;font-size:.875rem\">{current_label}</a>\
         <a href=\"{next_url}\" style=\"min-height:44px;display:inline-flex;align-items:center;\
         color:#007AFF;text-decoration:none;font-size:.875rem\">{next_label}</a>\
         </nav>\
         <div data-rfc067-matrix-scroller=\"true\" \
         style=\"overflow-x:auto;max-width:100%;border:1px solid #E5E5EA;\
         border-radius:8px;background:#FFFFFF\" tabindex=\"0\">\
         <table style=\"border-collapse:separate;border-spacing:0;min-width:72rem;\
         width:max-content\">\
         <thead><tr><th scope=\"col\" style=\"position:sticky;left:0;top:0;\
         z-index:3;background:#FFFFFF;border:1px solid #E5E5EA;text-align:left;\
         min-width:8rem;padding:.5rem;font-size:.8125rem;color:#6e6e73\">\
         メンバー</th>{header_cells}</tr></thead>\
         <tbody>{body_rows}</tbody></table></div>{detail}</section>",
        title = i18n::JA_CALENDAR_MATRIX_TITLE,
        year = year,
        month = month,
        prev_url = render::escape_html(&month_url(prev_year, prev_month)),
        next_url = render::escape_html(&month_url(next_year, next_month)),
        current_url = render::escape_html(&current_url),
        prev_label = i18n::JA_CALENDAR_PREV_MONTH,
        next_label = i18n::JA_CALENDAR_NEXT_MONTH,
        current_label = i18n::JA_CALENDAR_THIS_MONTH,
        header_cells = header_cells,
        body_rows = body_rows,
        detail = detail
    )
}

fn render_too_large(community_id: &str, year: i32, month: i32) -> String {
    format!(
        "<section style=\"margin:0 auto 1.5rem;max-width:42rem\">\
         <h2 style=\"font-size:1.125rem;font-weight:700;margin:0\">{title}</h2>\
         <p role=\"status\" style=\"font-size:.875rem;color:#6e6e73;\
         background:#F5F5F7;border-radius:8px;padding:.75rem;margin:.75rem 0 0;\
         line-height:1.5\">{message}</p>\
         <p style=\"margin:.75rem 0 0\"><a href=\"/c/{cid}/communities?month={year:04}-{month:02}\" \
         style=\"color:#007AFF;text-decoration:none;font-size:.875rem;font-weight:600\">\
         {calendar}</a></p></section>",
        title = i18n::JA_CALENDAR_MATRIX_TITLE,
        message = i18n::JA_CALENDAR_MATRIX_TOO_LARGE,
        cid = render::escape_html(community_id),
        year = year,
        month = month,
        calendar = i18n::JA_CALENDAR_VIEW_MONTH
    )
}
