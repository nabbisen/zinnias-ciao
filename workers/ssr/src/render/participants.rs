use super::shell::escape_html;
use super::status::status_display;
use zinnias_ciao_contracts::Locale;
use zinnias_ciao_contracts::i18n;

pub struct ParticipantEntry<'a> {
    pub display_name: &'a str,
    pub status: Option<&'a str>,
}

pub fn participant_list(locale: Locale, participants: &[ParticipantEntry<'_>]) -> String {
    if participants.is_empty() {
        return format!(
            "<p class=\"cz-participant-empty\">{}</p>",
            i18n::t(locale, i18n::EVENT_MEMBER_FALLBACK)
        );
    }
    let rows: String = participants
        .iter()
        .map(|p| {
            let initials = initials(p.display_name);
            let (class, icon, label) = status_display(locale, p.status);
            format!(
                "<li class=\"cz-participant-row\">\
             <span class=\"cz-participant-avatar cz-participant-avatar--{class}\">{initials}</span>\
             <span class=\"cz-participant-name\">{name}</span>\
             <span class=\"cz-participant-status cz-status-text--{class}\">{icon} {label}</span>\
             </li>",
                initials = escape_html(&initials),
                name = escape_html(p.display_name),
            )
        })
        .collect();
    format!("<ul class=\"cz-participant-list\">{rows}</ul>")
}

pub(super) fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .map(|c| c.to_uppercase().to_string())
        .collect::<Vec<_>>()
        .join("")
}
