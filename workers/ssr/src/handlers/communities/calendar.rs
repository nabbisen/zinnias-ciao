use crate::db::event as event_db;
use crate::render;
use zinnias_ciao_contracts::{Locale, i18n, tz};

mod events;

pub(super) fn month_bounds(year: i32, month: i32) -> (String, String) {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    (
        format!("{year:04}-{month:02}-01"),
        format!("{next_year:04}-{next_month:02}-01"),
    )
}

/// Calendar tab's day-detail section (RFC-073). Always present in the DOM —
/// `#calendar-day-detail` is a link target for the grid's date-cell
/// fragments, so it must exist whether or not a day is currently selected.
/// With a day selected, it reuses the existing month/day event-list helper
/// (event links only, no attendance counts, per the RFC's Calendar Tab
/// decision). With no day selected, it shows a short prompt instead of the
/// full month list — the full month list is the Events list tab's job.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_calendar_day_detail(
    community_id: &str,
    community_tz: &str,
    rows: &[event_db::HomeEventRow],
    selected_day: Option<&str>,
    year: i32,
    month: i32,
    can_create_event: bool,
    locale: Locale,
) -> String {
    let inner = match selected_day {
        Some(day) => events::render_calendar_events(
            community_id,
            community_tz,
            rows,
            Some(day),
            year,
            month,
            can_create_event,
            locale,
        ),
        None => format!(
            "<section class=\"cz-page-section\">\
             <p class=\"cz-hint\">{}</p></section>",
            i18n::t(locale, i18n::CALENDAR_DAY_DETAIL_PROMPT)
        ),
    };
    format!("<div id=\"calendar-day-detail\">{inner}</div>")
}

/// Events list tab (RFC-073): the full month-scoped event list, regardless
/// of any selected day, with its own month navigation using `view=list`
/// hrefs. Reuses the same event-list helper as day detail, always passing
/// `selected_day = None` so `day` never filters this tab's contents.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_calendar_list(
    community_id: &str,
    community_tz: &str,
    rows: &[event_db::HomeEventRow],
    year: i32,
    month: i32,
    can_create_event: bool,
    locale: Locale,
) -> String {
    let (prev_year, prev_month) = add_months(year, month, -1);
    let (next_year, next_month) = add_months(year, month, 1);
    let month_url =
        |y: i32, m: i32| format!("/c/{community_id}/communities?month={y:04}-{m:02}&view=list");
    let current_url = format!("/c/{community_id}/communities?view=list");
    let nav = format!(
        "<nav aria-label=\"{nav_label}\" class=\"cz-calendar-nav\">\
         <a href=\"{prev_url}\" class=\"cz-link cz-link--nav\">{prev_label}</a>\
         <a href=\"{current_url}\" class=\"cz-link cz-link--nav\">{current_label}</a>\
         <a href=\"{next_url}\" class=\"cz-link cz-link--nav\">{next_label}</a>\
         </nav>",
        nav_label = i18n::t(locale, i18n::CALENDAR_MONTH_NAV_ARIA_LABEL),
        prev_url = render::escape_html(&month_url(prev_year, prev_month)),
        next_url = render::escape_html(&month_url(next_year, next_month)),
        current_url = render::escape_html(&current_url),
        prev_label = i18n::t(locale, i18n::CALENDAR_PREV_MONTH),
        next_label = i18n::t(locale, i18n::CALENDAR_NEXT_MONTH),
        current_label = i18n::t(locale, i18n::CALENDAR_THIS_MONTH),
    );
    let list = events::render_calendar_events(
        community_id,
        community_tz,
        rows,
        None,
        year,
        month,
        can_create_event,
        locale,
    );
    format!("{nav}{list}")
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_calendar_month(
    community_id: &str,
    year: i32,
    month: i32,
    today_day: Option<i32>,
    selected_day: Option<&str>,
    rows: &[event_db::HomeEventRow],
    locale: Locale,
) -> String {
    use std::collections::BTreeMap;

    let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
    for row in rows {
        let Some((row_year, row_month, row_day)) = parse_ymd(&row.day_date) else {
            continue;
        };
        if row_year == year && row_month == month {
            *counts.entry(row_day).or_default() += 1;
        }
    }

    let weekdays = ["日", "月", "火", "水", "木", "金", "土"];
    let mut cells = String::new();
    for label in weekdays {
        cells.push_str(&format!("<div class=\"cz-calendar-weekday\">{label}</div>"));
    }

    for _ in 0..weekday_sunday_zero(year, month, 1) {
        cells.push_str("<div aria-hidden=\"true\" class=\"cz-calendar-day-empty\"></div>");
    }

    let month_key = format!("{year:04}-{month:02}");
    let days_in_month = tz::days_in_month(year, month);
    for day in 1..=days_in_month {
        let count = counts.get(&day).copied().unwrap_or_default();
        let is_today = today_day == Some(day);
        let day_date = format!("{year:04}-{month:02}-{day:02}");
        let is_selected = selected_day == Some(day_date.as_str());
        let has_events = count > 0;
        // RFC-075: state is expressed through classes, not an inline colour —
        // declaration order in app.css reproduces the original if/else-if
        // precedence (ordinary < has-events < today < selected) for every
        // property including border-width. See
        // `calendar_overview_contract_is_explicit` in release_gates.rs.
        let mut day_class = String::from("cz-calendar-day");
        if has_events {
            day_class.push_str(" cz-calendar-day--has-events");
        }
        if is_today {
            day_class.push_str(" cz-calendar-day--today");
        }
        if is_selected {
            day_class.push_str(" cz-calendar-day--selected");
        }
        let today_label = i18n::t(locale, i18n::TODAY);
        let marker_html = match (is_today, has_events, is_selected) {
            (true, true, true) => format!(
                "<span class=\"cz-calendar-day-badge cz-calendar-day-badge--selected\">\
                 <span>{today_label}</span><span aria-hidden=\"true\">●</span></span>"
            ),
            (true, true, false) => format!(
                "<span class=\"cz-calendar-day-badge\">\
                 <span>{today_label}</span><span aria-hidden=\"true\">●</span></span>"
            ),
            (true, false, true) => format!(
                "<span class=\"cz-calendar-day-label cz-calendar-day-label--selected\">{today_label}</span>"
            ),
            (true, false, false) => {
                format!("<span class=\"cz-calendar-day-label\">{today_label}</span>")
            }
            (false, true, _) => {
                "<span aria-hidden=\"true\" class=\"cz-calendar-day-dot\">●</span>".to_string()
            }
            (false, false, _) => {
                "<span aria-hidden=\"true\" class=\"cz-calendar-day-empty-marker\">&nbsp;</span>"
                    .to_string()
            }
        };
        // RFC-072 Slice C: the day-cell aria-label is a composed sentence,
        // not a single swappable string, so it is built per locale rather
        // than through a single `i18n::t` call. The Japanese date segment
        // keeps its established "{year}年{month}月{day}日" form unchanged
        // (repeats year/month per cell, as before); the English segment
        // uses the day-label convention used everywhere else in the app
        // (`date_label_en`, which omits the year — the month header
        // immediately above the grid already carries it) rather than
        // inventing a third shape for this one sentence.
        let date_segment = match locale {
            Locale::Ja => format!("{year}年{month}月{day}日"),
            Locale::En => tz::date_label_en(&day_date),
        };
        let today_suffix = if is_today {
            match locale {
                Locale::Ja => format!("、{today_label}"),
                Locale::En => format!(", {today_label}"),
            }
        } else {
            String::new()
        };
        let events_suffix = if has_events {
            // Label-value form, not "{count} events" — avoids pluralization
            // entirely (RFC-072 Slice C, same technique as matrix/cells.rs).
            let events_label = i18n::t(locale, i18n::CALENDAR_DAY_EVENTS_COUNT);
            match locale {
                Locale::Ja => format!("、{events_label}{count}件"),
                Locale::En => format!(", {events_label}{count}"),
            }
        } else {
            String::new()
        };
        let aria_label = format!("{date_segment}{today_suffix}{events_suffix}");
        let aria_current = if is_selected {
            " aria-current=\"date\""
        } else {
            ""
        };
        cells.push_str(&format!(
            "<a href=\"/c/{cid}/communities?month={month_key}&amp;day={day_date}#calendar-day-detail\" \
             aria-label=\"{aria}\"{aria_current} class=\"{day_class}\">\
             <span class=\"cz-calendar-day-number\">{day}</span>{marker_html}</a>",
            cid = render::escape_html(community_id),
            month_key = render::escape_html(&month_key),
            day_date = render::escape_html(&day_date),
            aria = render::escape_html(&aria_label),
            aria_current = aria_current,
            day_class = day_class,
            day = day,
            marker_html = marker_html
        ));
    }

    let (prev_year, prev_month) = add_months(year, month, -1);
    let (next_year, next_month) = add_months(year, month, 1);
    let month_url = |y: i32, m: i32| format!("/c/{community_id}/communities?month={y:04}-{m:02}");
    let current_url = format!("/c/{community_id}/communities");
    let clear_filter = if selected_day.is_some() {
        format!(
            "<a href=\"/c/{cid}/communities?month={month_key}\" \
             class=\"cz-link cz-link--nav\">{label}</a>",
            cid = render::escape_html(community_id),
            month_key = render::escape_html(&month_key),
            label = i18n::t(locale, i18n::CALENDAR_ALL_DAYS),
        )
    } else {
        String::new()
    };

    let empty = if counts.is_empty() {
        format!(
            "<p class=\"cz-hint cz-hint--gap-top cz-hint--center\">{}</p>",
            i18n::t(locale, i18n::CALENDAR_EMPTY_MONTH)
        )
    } else {
        String::new()
    };

    // Month header: full spelled-out month name in English (RFC-072 Slice C
    // date-format decision — "August 2026", a page title with room), the
    // established "{year}年{month}月" form unchanged in Japanese.
    let month_header = match locale {
        Locale::Ja => format!("{year}年{month}月"),
        Locale::En => format!("{} {year}", tz::month_name_en(month)),
    };
    format!(
        "<section aria-label=\"{title}\" class=\"cz-page-section\">\
         <div class=\"cz-section-header\">\
         <h2 class=\"cz-section-title cz-section-title--lg\">{title}</h2>\
         <p class=\"cz-month-subtitle\">{month_header}</p>\
         </div>\
         <nav aria-label=\"{nav_label}\" class=\"cz-calendar-nav\">\
         <a href=\"{prev_url}\" class=\"cz-link cz-link--nav\">{prev_label}</a>\
         <a href=\"{current_url}\" class=\"cz-link cz-link--nav\">{current_label}</a>\
         <a href=\"{next_url}\" class=\"cz-link cz-link--nav\">{next_label}</a>\
         </nav>\
         <p class=\"cz-hint cz-hint--lined\">{helper}</p>\
         <div class=\"cz-calendar-card\">\
         <div class=\"cz-calendar-grid\">{cells}</div>{empty}</div>\
         <div class=\"cz-calendar-clear-filter-row\">{clear_filter}</div>\
         </section>",
        title = i18n::t(locale, i18n::CALENDAR_MONTH_TITLE),
        nav_label = i18n::t(locale, i18n::CALENDAR_MONTH_NAV_ARIA_LABEL),
        helper = i18n::t(locale, i18n::HOME_CALENDAR_HELPER),
        month_header = month_header,
        prev_url = render::escape_html(&month_url(prev_year, prev_month)),
        next_url = render::escape_html(&month_url(next_year, next_month)),
        current_url = render::escape_html(&current_url),
        prev_label = i18n::t(locale, i18n::CALENDAR_PREV_MONTH),
        next_label = i18n::t(locale, i18n::CALENDAR_NEXT_MONTH),
        current_label = i18n::t(locale, i18n::CALENDAR_THIS_MONTH),
        cells = cells,
        empty = empty,
        clear_filter = clear_filter
    )
}

pub(super) fn parse_month(month: &str) -> Option<(i32, i32)> {
    if month.len() != 7 || month.get(4..5)? != "-" {
        return None;
    }
    let year = month.get(..4)?.parse::<i32>().ok()?;
    let month = month.get(5..7)?.parse::<i32>().ok()?;
    if !(1..=12).contains(&month) {
        return None;
    }
    Some((year, month))
}

pub(super) fn parse_ymd(date: &str) -> Option<(i32, i32, i32)> {
    if date.len() != 10 || date.get(4..5)? != "-" || date.get(7..8)? != "-" {
        return None;
    }
    let year = date.get(..4)?.parse().ok()?;
    let month = date.get(5..7)?.parse().ok()?;
    let day = date.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=tz::days_in_month(year, month)).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

pub(super) fn add_months(year: i32, month: i32, delta: i32) -> (i32, i32) {
    let zero_based = year * 12 + (month - 1) + delta;
    (zero_based.div_euclid(12), zero_based.rem_euclid(12) + 1)
}

fn weekday_sunday_zero(year: i32, month: i32, day: i32) -> i32 {
    if !(1..=12).contains(&month) {
        return 0;
    }
    let offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year;
    if month < 3 {
        y -= 1;
    }
    (y + y / 4 - y / 100 + y / 400 + offsets[(month - 1) as usize] + day) % 7
}
