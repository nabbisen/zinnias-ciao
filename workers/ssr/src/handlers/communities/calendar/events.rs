use crate::db::event as event_db;
use crate::render;
use zinnias_ciao_contracts::i18n;

pub(super) fn render_calendar_events(
    community_id: &str,
    community_tz: &str,
    rows: &[event_db::HomeEventRow],
    selected_day: Option<&str>,
    year: i32,
    month: i32,
    can_create_event: bool,
) -> String {
    let items: String = rows
        .iter()
        .filter(|row| {
            selected_day
                .map(|day| row.day_date.as_str() == day)
                .unwrap_or(true)
        })
        .map(|row| {
            let date = render::format_day_time_tz(
                &render::CardDay {
                    starts_at_utc: &row.starts_at_utc,
                    ends_at_utc: &row.ends_at_utc,
                    day_date: &row.day_date,
                },
                community_tz,
            );
            let cancelled = if row.event_status == "cancelled"
                || row.occurrence_status == "cancelled"
            {
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
            let location = row.event_location.as_deref().unwrap_or("");
            let location_html = if location.is_empty() {
                String::new()
            } else {
                format!(
                    "<span style=\"color:#6e6e73\"> · {}</span>",
                    render::escape_html(location)
                )
            };
            format!(
                "<li style=\"border-top:1px solid #F5F5F7\">\
                 <a href=\"/c/{cid}/events/{eid}\" style=\"display:block;\
                 padding:.875rem 0;text-decoration:none;color:inherit\">\
                 <span style=\"display:block;font-size:1rem;font-weight:600;\
                 line-height:1.35\">{title}{cancelled}</span>\
                 <span style=\"display:block;font-size:.8125rem;color:#6e6e73;\
                 margin-top:.25rem\">{date}{location}</span>\
                 </a></li>",
                cid = render::escape_html(community_id),
                eid = render::escape_html(&row.event_id),
                title = render::escape_html(&row.event_title),
                cancelled = cancelled,
                date = render::escape_html(&date),
                location = location_html,
            )
        })
        .collect();

    let empty_copy = if selected_day.is_some() {
        i18n::JA_CALENDAR_EMPTY_DAY
    } else {
        i18n::JA_CALENDAR_EMPTY_MONTH
    };
    let content = if items.is_empty() {
        format!(
            "<p style=\"font-size:.875rem;color:#6e6e73;margin:.75rem 0 0\">{}</p>",
            empty_copy
        )
    } else {
        format!("<ul style=\"list-style:none;margin:.5rem 0 0;padding:0\">{items}</ul>")
    };
    let create_on_day = match (selected_day, can_create_event) {
        (Some(day), true) => format!(
            "<a href=\"/c/{cid}/admin/events/new?day={day}\" \
             style=\"display:inline-flex;align-items:center;justify-content:center;\
             min-height:44px;margin:.75rem 0 0;color:#007AFF;text-decoration:none;\
             font-size:.875rem;font-weight:600\">{label}</a>",
            cid = render::escape_html(community_id),
            day = render::escape_html(day),
            label = i18n::JA_CALENDAR_CREATE_ON_DAY
        ),
        _ => String::new(),
    };

    format!(
        "<section style=\"margin:0 auto 1.5rem;max-width:42rem\">\
         <h2 style=\"font-size:1.125rem;font-weight:700;margin:0\">{title}</h2>\
         <p style=\"font-size:.8125rem;color:#6e6e73;margin:.25rem 0 0\">{scope}</p>\
         {create_on_day}{content}</section>",
        title = i18n::JA_HOME_AGENDA_TITLE,
        scope = selected_day
            .map(render::escape_html)
            .unwrap_or_else(|| format!("{year:04}-{month:02}")),
        create_on_day = create_on_day,
        content = content
    )
}
