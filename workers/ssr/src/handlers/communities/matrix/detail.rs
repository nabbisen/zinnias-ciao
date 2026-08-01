use std::collections::HashMap;

use crate::db::{attendance, event as event_db};
use crate::render;
use zinnias_ciao_contracts::{Locale, i18n};

use super::cells::{aggregate_counts, event_day_cancelled};

pub(super) fn render_date_detail(
    community_id: &str,
    community_tz: &str,
    detail_day: Option<&str>,
    rows_by_date: &HashMap<String, Vec<&event_db::HomeEventRow>>,
    member_count: usize,
    attendances: &HashMap<String, Vec<attendance::AttendanceRow>>,
    locale: Locale,
) -> String {
    let Some(day) = detail_day else {
        return format!(
            "<section class=\"cz-matrix-detail-section\">\
             <h3 class=\"cz-matrix-detail-heading\">{}</h3>\
             <p class=\"cz-matrix-detail-empty-text\">{}</p>\
             </section>",
            i18n::t(locale, i18n::HOME_AGENDA_TITLE),
            i18n::t(locale, i18n::CALENDAR_EMPTY_MONTH)
        );
    };
    let events = rows_by_date.get(day).map(Vec::as_slice).unwrap_or(&[]);
    if events.is_empty() {
        return format!(
            "<section class=\"cz-matrix-detail-section\">\
             <h3 class=\"cz-matrix-detail-heading\">{day}</h3>\
             <p class=\"cz-matrix-detail-empty-text\">{}</p>\
             </section>",
            i18n::t(locale, i18n::CALENDAR_EMPTY_DAY)
        );
    }

    let mut items = String::new();
    for row in events {
        let date = render::format_day_time_tz_localized(
            &render::CardDay {
                starts_at_utc: &row.starts_at_utc,
                ends_at_utc: &row.ends_at_utc,
                day_date: &row.day_date,
            },
            community_tz,
            locale,
        );
        let counts = aggregate_counts(&row.day_id, member_count, attendances);
        let cancelled = if event_day_cancelled(row) {
            format!(
                "<span class=\"cz-matrix-detail-cancelled-badge\">{}</span>",
                i18n::t(
                    locale,
                    if row.occurrence_status == "cancelled" {
                        i18n::OCCURRENCE_CANCELLED_BADGE
                    } else {
                        i18n::EVENT_CANCELLED_BADGE
                    }
                )
            )
        } else {
            String::new()
        };
        items.push_str(&format!(
            "<li class=\"cz-matrix-detail-item\">\
             <a href=\"/c/{cid}/events/{eid}\" class=\"cz-matrix-detail-item-link\">\
             <span class=\"cz-matrix-detail-item-title\">{title}{cancelled}</span>\
             <span class=\"cz-matrix-detail-item-date\">{date}</span></a>\
             <span class=\"cz-matrix-detail-item-counts\">{going} {going_count} · {not_going} {not_going_count} · \
             {attended} {attended_count} · {no_answer} {no_answer_count}</span></li>",
            cid = render::escape_html(community_id),
            eid = render::escape_html(&row.event_id),
            title = render::escape_html(&row.event_title),
            cancelled = cancelled,
            date = render::escape_html(&date),
            going = i18n::t(locale, i18n::STATUS_GOING),
            going_count = counts.going,
            not_going = i18n::t(locale, i18n::STATUS_NOT_GOING),
            not_going_count = counts.not_going,
            attended = i18n::t(locale, i18n::STATUS_ATTENDED),
            attended_count = counts.attended,
            no_answer = i18n::t(locale, i18n::STATUS_NO_ANSWER),
            no_answer_count = counts.no_answer
        ));
    }
    format!(
        "<section class=\"cz-matrix-detail-section\">\
         <h3 class=\"cz-matrix-detail-heading\">{day}</h3>\
         <ul class=\"cz-matrix-detail-list\">{items}</ul>\
         </section>"
    )
}
