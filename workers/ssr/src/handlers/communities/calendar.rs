use crate::db::event as event_db;
use crate::render;
use zinnias_ciao_contracts::{i18n, tz};

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

pub(super) fn render_calendar_events(
    community_id: &str,
    community_tz: &str,
    rows: &[event_db::HomeEventRow],
    selected_day: Option<&str>,
    year: i32,
    month: i32,
    can_create_event: bool,
) -> String {
    events::render_calendar_events(
        community_id,
        community_tz,
        rows,
        selected_day,
        year,
        month,
        can_create_event,
    )
}

pub(super) fn render_calendar_month(
    community_id: &str,
    year: i32,
    month: i32,
    today_day: Option<i32>,
    selected_day: Option<&str>,
    rows: &[event_db::HomeEventRow],
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
        cells.push_str(&format!(
            "<div style=\"min-height:28px;display:flex;align-items:center;\
             justify-content:center;font-size:.75rem;font-weight:700;color:#6e6e73\">\
             {label}</div>"
        ));
    }

    for _ in 0..weekday_sunday_zero(year, month, 1) {
        cells.push_str(
            "<div aria-hidden=\"true\" style=\"min-height:54px;border-radius:10px\"></div>",
        );
    }

    let month_key = format!("{year:04}-{month:02}");
    let days_in_month = tz::days_in_month(year, month);
    for day in 1..=days_in_month {
        let count = counts.get(&day).copied().unwrap_or_default();
        let is_today = today_day == Some(day);
        let day_date = format!("{year:04}-{month:02}-{day:02}");
        let is_selected = selected_day == Some(day_date.as_str());
        let has_events = count > 0;
        let bg = if is_selected {
            "#EAF3FF"
        } else if is_today {
            "#FAFAFB"
        } else if has_events {
            "#FFFFFF"
        } else {
            "#F5F5F7"
        };
        let border = if is_selected {
            "#007AFF"
        } else if is_today || has_events {
            "#D1D1D6"
        } else {
            "#F5F5F7"
        };
        let border_width = if is_today && !is_selected {
            "2px"
        } else {
            "1px"
        };
        let day_color = if is_selected {
            "#0057B8"
        } else if is_today {
            "#3A3A3C"
        } else {
            "#1D1D1F"
        };
        let marker_html = match (is_today, has_events, is_selected) {
            (true, true, true) => "<span style=\"display:flex;gap:.125rem;align-items:center;\
                 justify-content:center;font-size:.6875rem;font-weight:700;\
                 color:#0057B8;line-height:1.2\">\
                 <span>今日</span><span aria-hidden=\"true\">●</span></span>"
                .to_string(),
            (true, true, false) => "<span style=\"display:flex;gap:.125rem;align-items:center;\
                 justify-content:center;font-size:.6875rem;font-weight:600;\
                 color:#6E6E73;line-height:1.2\">\
                 <span>今日</span><span aria-hidden=\"true\">●</span></span>"
                .to_string(),
            (true, false, true) => {
                "<span style=\"font-size:.6875rem;font-weight:700;color:#0057B8;\
                 line-height:1.2\">今日</span>"
                    .to_string()
            }
            (true, false, false) => {
                "<span style=\"font-size:.6875rem;font-weight:600;color:#6E6E73;\
                 line-height:1.2\">今日</span>"
                    .to_string()
            }
            (false, true, _) => {
                "<span aria-hidden=\"true\" style=\"font-size:.8125rem;font-weight:700;\
                 color:#007AFF;line-height:1.2\">●</span>"
                    .to_string()
            }
            (false, false, _) => {
                "<span aria-hidden=\"true\" style=\"font-size:.6875rem;line-height:1.2\">\
                 &nbsp;</span>"
                    .to_string()
            }
        };
        let aria_label = if has_events {
            let today_suffix = if is_today { "、今日" } else { "" };
            format!(
                "{year}年{month}月{day}日{today_suffix}、予定{count}{}",
                i18n::JA_HOME_CALENDAR_COUNT_SUFFIX
            )
        } else if is_today {
            format!("{year}年{month}月{day}日、今日")
        } else {
            format!("{year}年{month}月{day}日")
        };
        let aria_current = if is_selected {
            " aria-current=\"date\""
        } else {
            ""
        };
        cells.push_str(&format!(
            "<a href=\"/c/{cid}/communities?month={month_key}&amp;day={day_date}\" \
             aria-label=\"{aria}\"{aria_current} style=\"min-height:60px;border:{border_width} solid {border};\
             border-radius:10px;background:{bg};padding:.375rem .25rem;display:flex;\
             flex-direction:column;align-items:center;justify-content:space-between;\
             text-align:center;box-sizing:border-box;text-decoration:none;color:inherit\">\
             <span style=\"font-size:.9375rem;font-weight:700;color:{day_color};\
             line-height:1.1\">{day}</span>{marker_html}</a>",
            cid = render::escape_html(community_id),
            month_key = render::escape_html(&month_key),
            day_date = render::escape_html(&day_date),
            aria = render::escape_html(&aria_label),
            aria_current = aria_current,
            border = border,
            border_width = border_width,
            bg = bg,
            day_color = day_color,
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
             style=\"font-size:.875rem;color:#007AFF;text-decoration:none;min-height:44px;\
             display:inline-flex;align-items:center\">{label}</a>",
            cid = render::escape_html(community_id),
            month_key = render::escape_html(&month_key),
            label = i18n::JA_CALENDAR_ALL_DAYS,
        )
    } else {
        String::new()
    };

    let empty = if counts.is_empty() {
        format!(
            "<p style=\"font-size:.875rem;color:#6e6e73;margin:.75rem 0 0;\
             text-align:center\">{}</p>",
            i18n::JA_CALENDAR_EMPTY_MONTH
        )
    } else {
        String::new()
    };

    format!(
        "<section aria-label=\"{title}\" style=\"margin:0 auto 1.5rem;\
         max-width:42rem\">\
         <div style=\"display:flex;align-items:flex-end;justify-content:space-between;\
         gap:.75rem;margin-bottom:.75rem;flex-wrap:wrap\">\
         <h2 style=\"font-size:1.25rem;font-weight:700;margin:0\">{title}</h2>\
         <p style=\"font-size:.9375rem;font-weight:700;color:#6e6e73;margin:0\">\
         {year}年{month}月</p>\
         </div>\
         <nav aria-label=\"Calendar month\" style=\"display:flex;gap:.5rem;\
         align-items:center;justify-content:space-between;margin:0 0 .75rem\">\
         <a href=\"{prev_url}\" style=\"min-height:44px;display:inline-flex;align-items:center;\
         color:#007AFF;text-decoration:none;font-size:.875rem\">{prev_label}</a>\
         <a href=\"{current_url}\" style=\"min-height:44px;display:inline-flex;align-items:center;\
         color:#007AFF;text-decoration:none;font-size:.875rem\">{current_label}</a>\
         <a href=\"{next_url}\" style=\"min-height:44px;display:inline-flex;align-items:center;\
         color:#007AFF;text-decoration:none;font-size:.875rem\">{next_label}</a>\
         </nav>\
         <p style=\"font-size:.875rem;color:#6e6e73;line-height:1.5;margin:0 0 .75rem\">\
         {helper}</p>\
         <div style=\"background:#FFFFFF;border:1px solid #E5E5EA;border-radius:16px;\
         padding:.75rem;box-shadow:0 1px 2px rgba(0,0,0,.04)\">\
         <div style=\"display:grid;grid-template-columns:repeat(7,minmax(0,1fr));\
         gap:.25rem\">{cells}</div>{empty}</div>\
         <div style=\"margin-top:.5rem\">{clear_filter}</div>\
         </section>",
        title = i18n::JA_CALENDAR_MONTH_TITLE,
        helper = i18n::JA_HOME_CALENDAR_HELPER,
        year = year,
        month = month,
        prev_url = render::escape_html(&month_url(prev_year, prev_month)),
        next_url = render::escape_html(&month_url(next_year, next_month)),
        current_url = render::escape_html(&current_url),
        prev_label = i18n::JA_CALENDAR_PREV_MONTH,
        next_label = i18n::JA_CALENDAR_NEXT_MONTH,
        current_label = i18n::JA_CALENDAR_THIS_MONTH,
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
