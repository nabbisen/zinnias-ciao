//! Monthly attendance matrix rendering and view-model helpers (RFC-067).

use std::collections::HashMap;

use crate::db::{attendance, event as event_db, membership};
use crate::render;
use cells::cell_summary;
use detail::render_date_detail;
use zinnias_ciao_contracts::{Locale, i18n, tz};

mod cells;
mod detail;
#[cfg(test)]
mod tests;

pub(super) const MEMBER_ROW_CAP: usize = 100;
pub(super) const EVENT_DAY_ROW_CAP: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CalendarView {
    Month,
    List,
    Matrix,
}

impl CalendarView {
    pub(super) fn from_query(value: Option<&str>) -> Self {
        match value {
            Some("list") => Self::List,
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
        (Some(day), CalendarView::List) => {
            format!("communities:{year:04}-{month:02}:{day}:list")
        }
        (None, CalendarView::List) => format!("communities:{year:04}-{month:02}:list"),
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
    locale: Locale,
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
    let list_href = format!(
        "/c/{}/communities?month={}&amp;view=list",
        render::escape_html(community_id),
        render::escape_html(&month_key)
    );
    let matrix_href = format!(
        "/c/{}/communities?month={}{}&amp;view=matrix",
        render::escape_html(community_id),
        render::escape_html(&month_key),
        day_query
    );
    let tab = |href: &str, label: &str, selected: bool| {
        let (class, aria) = if selected {
            ("cz-tab cz-tab--active", " aria-current=\"page\"")
        } else {
            ("cz-tab", "")
        };
        format!("<a href=\"{href}\"{aria} class=\"{class}\">{label}</a>")
    };

    format!(
        "<nav aria-label=\"Calendar view\" class=\"cz-tabs\">{}{}{}\
         </nav>",
        tab(
            &month_href,
            i18n::t(locale, i18n::CALENDAR_VIEW_MONTH),
            current == CalendarView::Month
        ),
        tab(
            &list_href,
            i18n::t(locale, i18n::CALENDAR_VIEW_LIST),
            current == CalendarView::List
        ),
        tab(
            &matrix_href,
            i18n::t(locale, i18n::CALENDAR_VIEW_MATRIX),
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
    pub(super) can_export_csv: bool,
    pub(super) export_token: Option<&'a str>,
    pub(super) rows: &'a [event_db::HomeEventRow],
    pub(super) members: &'a [membership::MemberSummary],
    pub(super) attendances: &'a HashMap<String, Vec<attendance::AttendanceRow>>,
    pub(super) locale: Locale,
}

pub(super) fn render_matrix(input: MatrixRenderInput<'_>) -> String {
    let MatrixRenderInput {
        community_id,
        community_tz,
        year,
        month,
        selected_day,
        can_export_csv,
        export_token,
        rows,
        members,
        attendances,
        locale,
    } = input;

    if members.is_empty() {
        return format!(
            "<section class=\"cz-page-section\">\
             <h2 class=\"cz-section-title\">{}</h2>\
             <p class=\"cz-hint cz-hint--gap-top\">{}</p>\
             </section>",
            i18n::t(locale, i18n::CALENDAR_MATRIX_TITLE),
            i18n::t(locale, i18n::CALENDAR_MATRIX_NO_MEMBERS)
        );
    }

    if members.len() > MEMBER_ROW_CAP || rows.len() > EVENT_DAY_ROW_CAP {
        return render_too_large(community_id, year, month, locale);
    }

    let month_key = format!("{year:04}-{month:02}");
    let export_controls = render_export_controls(
        community_id,
        &month_key,
        can_export_csv.then_some(()).and(export_token),
        locale,
    );
    let export_table_attr = if can_export_csv && export_token.is_some() {
        " data-calendar-matrix-export=\"true\""
    } else {
        ""
    };
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
        let header_class = if selected {
            "cz-matrix-header-cell cz-matrix-header-cell--selected"
        } else {
            "cz-matrix-header-cell"
        };
        let aria_current = if selected {
            " aria-current=\"date\""
        } else {
            ""
        };
        header_cells.push_str(&format!(
            "<th scope=\"col\"{date_attr} class=\"{header_class}\">\
             <a href=\"{href}\"{aria_current} class=\"cz-matrix-header-link\">{day}</a></th>",
            date_attr = export_attr(
                "data-date",
                &day_date,
                can_export_csv && export_token.is_some()
            )
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
            let cell = cell_summary(&day_date, member, events, attendances, locale);
            cells.push_str(&format!(
                "<td aria-label=\"{label}\"{export_value_attr} class=\"cz-matrix-cell cz-matrix-cell--{state}\">\
                 {visual}</td>",
                label = render::escape_html(&cell.label),
                export_value_attr = export_attr(
                    "data-export-value",
                    &cell.export_value,
                    can_export_csv && export_token.is_some()
                ),
                state = cell.state,
                visual = cell.visual
            ));
        }
        body_rows.push_str(&format!(
            "<tr><th scope=\"row\"{member_attr} class=\"cz-matrix-member-header\">{name}</th>{cells}</tr>",
            member_attr = export_attr(
                "data-member-name",
                &member.display_name,
                can_export_csv && export_token.is_some()
            ),
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
        locale,
    );
    let (prev_year, prev_month) = super::calendar::add_months(year, month, -1);
    let (next_year, next_month) = super::calendar::add_months(year, month, 1);
    let month_url =
        |y: i32, m: i32| format!("/c/{community_id}/communities?month={y:04}-{m:02}&view=matrix");
    let current_url = format!("/c/{community_id}/communities?view=matrix");

    // Month header: same shape as calendar.rs's render_calendar_month
    // (RFC-072 Slice C date-format decision — full month name in English).
    let month_header = match locale {
        Locale::Ja => format!("{year}年{month}月"),
        Locale::En => format!("{} {year}", tz::month_name_en(month)),
    };
    format!(
        "<section aria-label=\"{title}\" class=\"cz-page-section cz-page-section--wide\">\
         <div class=\"cz-matrix-header\">\
         <h2 class=\"cz-section-title\">{title}</h2>\
         <p class=\"cz-month-subtitle\">{month_header}</p>{export_controls}</div>\
         <nav aria-label=\"Calendar month\" class=\"cz-calendar-nav\">\
         <a href=\"{prev_url}\" class=\"cz-link cz-link--nav\">{prev_label}</a>\
         <a href=\"{current_url}\" class=\"cz-link cz-link--nav\">{current_label}</a>\
         <a href=\"{next_url}\" class=\"cz-link cz-link--nav\">{next_label}</a>\
         </nav>\
         <div data-rfc067-matrix-scroller=\"true\" \
         class=\"cz-matrix-scroller\" tabindex=\"0\">\
         <table{export_table_attr} class=\"cz-matrix-table\">\
         <thead><tr><th scope=\"col\" class=\"cz-matrix-corner-cell\">\
         {member_col}</th>{header_cells}</tr></thead>\
         <tbody>{body_rows}</tbody></table></div>{detail}</section>",
        title = i18n::t(locale, i18n::CALENDAR_MATRIX_TITLE),
        month_header = month_header,
        prev_url = render::escape_html(&month_url(prev_year, prev_month)),
        next_url = render::escape_html(&month_url(next_year, next_month)),
        current_url = render::escape_html(&current_url),
        prev_label = i18n::t(locale, i18n::CALENDAR_PREV_MONTH),
        next_label = i18n::t(locale, i18n::CALENDAR_NEXT_MONTH),
        current_label = i18n::t(locale, i18n::CALENDAR_THIS_MONTH),
        member_col = i18n::t(locale, i18n::CALENDAR_MATRIX_MEMBER_COLUMN),
        export_controls = export_controls,
        export_table_attr = export_table_attr,
        header_cells = header_cells,
        body_rows = body_rows,
        detail = detail
    )
}

fn render_export_controls(
    community_id: &str,
    month_key: &str,
    token: Option<&str>,
    locale: Locale,
) -> String {
    let Some(token) = token else {
        return String::new();
    };
    format!(
        "<div class=\"cz-export-controls\">\
         <button type=\"button\" data-calendar-matrix-export-button=\"true\" \
         data-audit-url=\"/c/{cid}/admin/calendar/matrix-export/audit\" \
         data-month=\"{month}\" data-export-type=\"calendar_matrix_csv\" \
         data-token=\"{token}\" data-filename=\"ciao-attendance-{month}.csv\" \
         class=\"cz-export-button\">{label}</button>\
         <span data-calendar-matrix-export-status=\"true\" data-error-message=\"{error}\" aria-live=\"polite\" \
         class=\"cz-export-status\"></span></div>",
        cid = render::escape_html(community_id),
        month = render::escape_html(month_key),
        token = render::escape_html(token),
        label = i18n::t(locale, i18n::CALENDAR_MATRIX_CSV_EXPORT),
        error = render::escape_html(i18n::t(locale, i18n::CALENDAR_MATRIX_CSV_ERROR))
    )
}

fn export_attr(name: &str, value: &str, enabled: bool) -> String {
    if !enabled {
        return String::new();
    }
    format!(" {name}=\"{}\"", render::escape_html(value))
}

fn render_too_large(community_id: &str, year: i32, month: i32, locale: Locale) -> String {
    format!(
        "<section class=\"cz-page-section\">\
         <h2 class=\"cz-section-title\">{title}</h2>\
         <p role=\"status\" class=\"cz-notice cz-notice--inline\">{message}</p>\
         <p class=\"cz-notice-followup\"><a href=\"/c/{cid}/communities?month={year:04}-{month:02}\" \
         class=\"cz-link cz-link--strong\">\
         {calendar}</a></p></section>",
        title = i18n::t(locale, i18n::CALENDAR_MATRIX_TITLE),
        message = i18n::t(locale, i18n::CALENDAR_MATRIX_TOO_LARGE),
        cid = render::escape_html(community_id),
        year = year,
        month = month,
        calendar = i18n::t(locale, i18n::CALENDAR_VIEW_MONTH)
    )
}
