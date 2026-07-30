use crate::db::event as event_db;
use crate::render;
use zinnias_ciao_contracts::{Locale, i18n};

#[allow(clippy::too_many_arguments)]
pub(super) fn render_calendar_events(
    community_id: &str,
    community_tz: &str,
    rows: &[event_db::HomeEventRow],
    selected_day: Option<&str>,
    year: i32,
    month: i32,
    can_create_event: bool,
    locale: Locale,
) -> String {
    let items: String = rows
        .iter()
        .filter(|row| {
            selected_day
                .map(|day| row.day_date.as_str() == day)
                .unwrap_or(true)
        })
        .map(|row| {
            let date = render::format_day_time_tz_localized(
                &render::CardDay {
                    starts_at_utc: &row.starts_at_utc,
                    ends_at_utc: &row.ends_at_utc,
                    day_date: &row.day_date,
                },
                community_tz,
                locale,
            );
            let cancelled =
                if row.event_status == "cancelled" || row.occurrence_status == "cancelled" {
                    format!(
                        "<span class=\"cz-event-cancelled-badge\">{}</span>",
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
            let location = row.event_location.as_deref().unwrap_or("");
            let location_html = if location.is_empty() {
                String::new()
            } else {
                format!(
                    "<span class=\"cz-event-location\"> · {}</span>",
                    render::escape_html(location)
                )
            };
            format!(
                "<li class=\"cz-event-list-item\">\
                 <a href=\"/c/{cid}/events/{eid}\" class=\"cz-event-link\">\
                 <span class=\"cz-event-title\">{title}{cancelled}</span>\
                 <span class=\"cz-event-meta\">{date}{location}</span>\
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

    let empty_copy = i18n::t(
        locale,
        if selected_day.is_some() {
            i18n::CALENDAR_EMPTY_DAY
        } else {
            i18n::CALENDAR_EMPTY_MONTH
        },
    );
    let content = if items.is_empty() {
        format!("<p class=\"cz-hint cz-hint--gap-top\">{}</p>", empty_copy)
    } else {
        format!("<ul class=\"cz-event-list\">{items}</ul>")
    };
    let create_on_day = match (selected_day, can_create_event) {
        (Some(day), true) => format!(
            "<a href=\"/c/{cid}/admin/events/new?day={day}\" \
             class=\"cz-link cz-link--action\">{label}</a>",
            cid = render::escape_html(community_id),
            day = render::escape_html(day),
            label = i18n::t(locale, i18n::CALENDAR_CREATE_ON_DAY)
        ),
        _ => String::new(),
    };

    format!(
        "<section class=\"cz-page-section\">\
         <h2 class=\"cz-section-title\">{title}</h2>\
         <p class=\"cz-agenda-scope\">{scope}</p>\
         {create_on_day}{content}</section>",
        title = i18n::t(locale, i18n::HOME_AGENDA_TITLE),
        scope = selected_day
            .map(render::escape_html)
            .unwrap_or_else(|| format!("{year:04}-{month:02}")),
        create_on_day = create_on_day,
        content = content
    )
}
