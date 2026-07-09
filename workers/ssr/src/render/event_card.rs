use super::shell::escape_html;
use super::status::status_display;
use super::time::format_day_time_tz;

pub struct CardDay<'a> {
    pub starts_at_utc: &'a str,
    pub ends_at_utc: &'a str,
    pub day_date: &'a str,
}

/// One event card for the Home list.
#[allow(clippy::too_many_arguments)]
pub fn event_card(
    community_id: &str,
    event_id: &str,
    title: &str,
    location: Option<&str>,
    is_cancelled: bool,
    nearest_day: &CardDay<'_>,
    total_days: u32,
    my_status: Option<&str>,
    going: u32,
    not_going: u32,
    no_answer: u32,
    tz: &str,
) -> String {
    let (_, icon, label) = status_display(my_status);
    let (sc, _, _) = status_display(my_status);
    let cancelled_badge = if is_cancelled {
        format!(
            "<span style=\"font-size:.75rem;background:#FF3B30;color:#FFFFFF;\
         border-radius:99px;padding:.125rem .5rem;margin-left:.5rem\">{}</span>",
            zinnias_ciao_contracts::i18n::JA_ADMIN_CANCEL_EVENT_CONFIRM
        )
    } else {
        String::new()
    };
    let multi_badge = if total_days > 1 {
        format!("<span style=\"font-size:.75rem;color:#6E6E73\"> · {total_days} 日間</span>")
    } else {
        String::new()
    };
    let loc_html = location
        .map(|l| {
            format!(
                "<span style=\"color:#6E6E73;font-size:.875rem\"> · {}</span>",
                escape_html(l)
            )
        })
        .unwrap_or_default();
    let counts_line = format!(
        "{} {} · {} {} · {} {}",
        zinnias_ciao_contracts::i18n::JA_STATUS_GOING,
        going,
        zinnias_ciao_contracts::i18n::JA_STATUS_NOT_GOING,
        not_going,
        zinnias_ciao_contracts::i18n::JA_STATUS_NO_ANSWER,
        no_answer,
    );
    let muted = if is_cancelled { "opacity:.5;" } else { "" };

    format!(
        "<a href=\"/c/{cid}/events/{eid}\" style=\"display:block;text-decoration:none;color:inherit\">\
         <article style=\"background:#FFFFFF;border-radius:16px;padding:1rem;\
         box-shadow:0 1px 3px rgba(0,0,0,.08);margin-bottom:.75rem;{muted}\">\
           <div style=\"display:flex;align-items:center;gap:.5rem;margin-bottom:.375rem\">\
             <span style=\"color:{sc};font-weight:600;font-size:.875rem\">{icon} {label}</span>\
             {cancelled}\
           </div>\
           <div style=\"font-size:1rem;font-weight:600\">{title}{multi}</div>\
           <div style=\"font-size:.875rem;color:#3c3c3e;margin-top:.25rem\">\
             {time}{loc}\
           </div>\
           <div style=\"font-size:.8125rem;color:#6E6E73;margin-top:.375rem\">\
             {counts}\
           </div>\
         </article></a>",
        cid = escape_html(community_id),
        eid = escape_html(event_id),
        title = escape_html(title),
        cancelled = cancelled_badge,
        multi = multi_badge,
        time = format_day_time_tz(nearest_day, tz),
        loc = loc_html,
        counts = counts_line,
    )
}
