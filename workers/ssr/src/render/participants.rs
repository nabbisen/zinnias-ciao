use super::shell::escape_html;
use super::status::status_display;

pub struct ParticipantEntry<'a> {
    pub display_name: &'a str,
    pub status: Option<&'a str>,
}

pub fn participant_list(participants: &[ParticipantEntry<'_>]) -> String {
    if participants.is_empty() {
        return format!(
            "<p style=\"color:#6E6E73;font-size:.875rem\">{}</p>",
            zinnias_ciao_contracts::i18n::JA_EVENT_MEMBER_FALLBACK
        );
    }
    let rows: String = participants
        .iter()
        .map(|p| {
            let initials = initials(p.display_name);
            let (color, icon, label) = status_display(p.status);
            format!(
                "<li style=\"display:flex;align-items:center;gap:.75rem;padding:.5rem 0;\
             border-bottom:1px solid #F5F5F7\">\
             <span style=\"width:2rem;height:2rem;border-radius:50%;background:{color}22;\
             color:{color};display:flex;align-items:center;justify-content:center;\
             font-size:.75rem;font-weight:700;flex-shrink:0\">{initials}</span>\
             <span style=\"flex:1;font-size:.9375rem\">{name}</span>\
             <span style=\"font-size:.8125rem;color:{color}\">{icon} {label}</span>\
             </li>",
                initials = escape_html(&initials),
                name = escape_html(p.display_name),
            )
        })
        .collect();
    format!("<ul style=\"list-style:none;padding:0;margin:0\">{rows}</ul>")
}

pub(super) fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .map(|c| c.to_uppercase().to_string())
        .collect::<Vec<_>>()
        .join("")
}
