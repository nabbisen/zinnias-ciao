use std::collections::HashMap;

use crate::db::{attendance, event as event_db};
use crate::render;
use zinnias_ciao_contracts::i18n;

use super::cells::{aggregate_counts, event_day_cancelled};

pub(super) fn render_date_detail(
    community_id: &str,
    community_tz: &str,
    detail_day: Option<&str>,
    rows_by_date: &HashMap<String, Vec<&event_db::HomeEventRow>>,
    member_count: usize,
    attendances: &HashMap<String, Vec<attendance::AttendanceRow>>,
) -> String {
    let Some(day) = detail_day else {
        return format!(
            "<section style=\"max-width:42rem;margin:1rem auto 0\">\
             <h3 style=\"font-size:1rem;font-weight:700;margin:0\">{}</h3>\
             <p style=\"font-size:.875rem;color:#6e6e73;margin:.5rem 0 0\">{}</p>\
             </section>",
            i18n::JA_HOME_AGENDA_TITLE,
            i18n::JA_CALENDAR_EMPTY_MONTH
        );
    };
    let events = rows_by_date.get(day).map(Vec::as_slice).unwrap_or(&[]);
    if events.is_empty() {
        return format!(
            "<section style=\"max-width:42rem;margin:1rem auto 0\">\
             <h3 style=\"font-size:1rem;font-weight:700;margin:0\">{day}</h3>\
             <p style=\"font-size:.875rem;color:#6e6e73;margin:.5rem 0 0\">{}</p>\
             </section>",
            i18n::JA_CALENDAR_EMPTY_DAY
        );
    }

    let mut items = String::new();
    for row in events {
        let date = render::format_day_time_tz(
            &render::CardDay {
                starts_at_utc: &row.starts_at_utc,
                ends_at_utc: &row.ends_at_utc,
                day_date: &row.day_date,
            },
            community_tz,
        );
        let counts = aggregate_counts(&row.day_id, member_count, attendances);
        let cancelled = if event_day_cancelled(row) {
            format!(
                "<span style=\"font-size:.75rem;color:#B42318;margin-left:.35rem\">{}</span>",
                if row.occurrence_status == "cancelled" {
                    i18n::JA_OCCURRENCE_CANCELLED_BADGE
                } else {
                    i18n::JA_EVENT_CANCELLED_BADGE
                }
            )
        } else {
            String::new()
        };
        items.push_str(&format!(
            "<li style=\"border-top:1px solid #F5F5F7;padding:.75rem 0\">\
             <a href=\"/c/{cid}/events/{eid}\" style=\"display:block;\
             text-decoration:none;color:inherit\">\
             <span style=\"display:block;font-size:.9375rem;font-weight:700;\
             line-height:1.35\">{title}{cancelled}</span>\
             <span style=\"display:block;font-size:.8125rem;color:#6e6e73;\
             margin-top:.25rem\">{date}</span></a>\
             <span style=\"display:block;font-size:.8125rem;color:#3A3A3C;\
             margin-top:.35rem\">{going} {going_count} · {not_going} {not_going_count} · \
             {attended} {attended_count} · {no_answer} {no_answer_count}</span></li>",
            cid = render::escape_html(community_id),
            eid = render::escape_html(&row.event_id),
            title = render::escape_html(&row.event_title),
            cancelled = cancelled,
            date = render::escape_html(&date),
            going = i18n::JA_STATUS_GOING,
            going_count = counts.going,
            not_going = i18n::JA_STATUS_NOT_GOING,
            not_going_count = counts.not_going,
            attended = i18n::JA_STATUS_ATTENDED,
            attended_count = counts.attended,
            no_answer = i18n::JA_STATUS_NO_ANSWER,
            no_answer_count = counts.no_answer
        ));
    }
    format!(
        "<section style=\"max-width:42rem;margin:1rem auto 0\">\
         <h3 style=\"font-size:1rem;font-weight:700;margin:0\">{day}</h3>\
         <ul style=\"list-style:none;margin:.5rem 0 0;padding:0\">{items}</ul>\
         </section>"
    )
}
