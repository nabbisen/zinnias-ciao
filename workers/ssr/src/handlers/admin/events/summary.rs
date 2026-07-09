use crate::db::event as event_db;
use crate::render;
use zinnias_ciao_contracts::i18n;

pub(super) fn render_schedule_summary(
    days: &[event_db::EventDayRow],
    community_tz: &str,
) -> String {
    if days.is_empty() {
        return String::new();
    }
    let labels: Vec<String> = days
        .iter()
        .map(|day| {
            render::format_day_time_tz(
                &render::CardDay {
                    starts_at_utc: &day.starts_at_utc,
                    ends_at_utc: &day.ends_at_utc,
                    day_date: &day.day_date,
                },
                community_tz,
            )
        })
        .collect();

    let content = if labels.len() <= 3 {
        let items: String = labels
            .iter()
            .map(|label| {
                format!(
                    "<li style=\"font-size:.875rem;color:#1D1D1F;margin:.25rem 0\">{}</li>",
                    render::escape_html(label)
                )
            })
            .collect();
        format!("<ul style=\"margin:.5rem 0 0;padding-left:1.25rem\">{items}</ul>")
    } else {
        let first = labels.first().map(String::as_str).unwrap_or("");
        let last = labels.last().map(String::as_str).unwrap_or("");
        format!(
            "<p style=\"font-size:.875rem;color:#1D1D1F;margin:.5rem 0 .25rem\">\
             {total_prefix}{count}{total_suffix}</p>\
             <p style=\"font-size:.875rem;color:#1D1D1F;margin:.25rem 0\">\
             {first_label}: {first}</p>\
             <p style=\"font-size:.875rem;color:#1D1D1F;margin:.25rem 0\">\
             {last_label}: {last}</p>",
            total_prefix = i18n::JA_ADMIN_EDIT_SCHEDULE_TOTAL_PREFIX,
            count = labels.len(),
            total_suffix = i18n::JA_ADMIN_EDIT_SCHEDULE_TOTAL_SUFFIX,
            first_label = i18n::JA_ADMIN_EDIT_SCHEDULE_FIRST,
            first = render::escape_html(first),
            last_label = i18n::JA_ADMIN_EDIT_SCHEDULE_LAST,
            last = render::escape_html(last),
        )
    };

    format!(
        "<section aria-label=\"{heading}\" style=\"margin:0 0 1.25rem;padding:1rem;\
         border:1px solid #E5E5EA;border-radius:12px;background:#FAFAFB\">\
         <h2 style=\"font-size:1rem;font-weight:700;margin:0\">{heading}</h2>\
         {content}</section>",
        heading = i18n::JA_ADMIN_EDIT_SCHEDULE_HEADING,
        content = content,
    )
}
