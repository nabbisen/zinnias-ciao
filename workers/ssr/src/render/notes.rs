use super::shell::escape_html;
use super::status::{
    CZ_BORDER, CZ_COLOR_DANGER, CZ_COLOR_TEXT_SECONDARY, CZ_STATUS_ATTENDED_FG,
    CZ_STATUS_GOING_BORDER,
};

/// Note textarea form for Event Detail (RFC-007).
pub fn note_form(
    community_id: &str,
    event_id: &str,
    save_token: &str,
    existing_note: Option<&str>,
    flash: Option<&str>,
) -> String {
    let flash_html = flash
        .map(|f| {
            format!(
                "<p role=\"status\" style=\"font-size:.875rem;color:{};margin:.5rem 0\">{}</p>",
                CZ_STATUS_ATTENDED_FG,
                escape_html(f)
            )
        })
        .unwrap_or_default();

    let delete_btn = if existing_note.is_some() {
        format!(
            "<a href=\"/c/{cid}/events/{eid}/my-note/delete\" \
             style=\"display:inline-block;font-size:.875rem;color:{danger};padding:.25rem;\
             min-height:44px;line-height:44px;text-decoration:none\">{del}</a>",
            del = zinnias_ciao_contracts::i18n::JA_NOTE_DELETE,
            cid = escape_html(community_id),
            eid = escape_html(event_id),
            danger = CZ_COLOR_DANGER,
        )
    } else {
        String::new()
    };

    format!(
        "<section aria-label=\"{note_section_label}\" style=\"margin:1.5rem 0\">\
         <h2 style=\"font-size:1.0625rem;font-weight:600;margin-bottom:.75rem\">{note_section_label}</h2>\
         {flash}\
         <p style=\"font-size:.75rem;color:{muted};margin-bottom:.5rem\" aria-live=\"polite\">\
         {note_visibility}</p>\
         <form method=\"post\" action=\"/c/{cid}/events/{eid}/my-note\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           <textarea name=\"note\" rows=\"3\" maxlength=\"200\" \
             style=\"width:100%;padding:.75rem;border:1px solid {border};\
             border-radius:12px;font-size:1rem;resize:vertical;box-sizing:border-box\" \
             aria-label=\"{note_placeholder_label}\">{existing}</textarea>\
           <div style=\"display:flex;justify-content:space-between;align-items:center;margin-top:.5rem\">\
             <span class=\"note-counter\" style=\"font-size:.75rem;color:{muted}\" aria-live=\"polite\">{note_char_hint}</span>\
             <button type=\"submit\" \
               style=\"padding:.625rem 1.25rem;background:{going_border};color:#FFFFFF;\
               border:none;border-radius:14px;font-size:.9375rem;font-weight:600;\
               min-height:44px;cursor:pointer\">{note_save}</button>\
           </div>\
         </form>\
         {delete}\
         </section>",
        cid = escape_html(community_id),
        eid = escape_html(event_id),
        tok = escape_html(save_token),
        existing = escape_html(existing_note.unwrap_or("")),
        flash = flash_html,
        delete = delete_btn,
        muted = CZ_COLOR_TEXT_SECONDARY,
        border = CZ_BORDER,
        going_border = CZ_STATUS_GOING_BORDER,
        note_section_label = zinnias_ciao_contracts::i18n::JA_NOTE_SECTION_LABEL,
        note_placeholder_label = zinnias_ciao_contracts::i18n::JA_NOTE_PLACEHOLDER_LABEL,
        note_char_hint = zinnias_ciao_contracts::i18n::JA_NOTE_CHAR_HINT,
        note_visibility = zinnias_ciao_contracts::i18n::JA_NOTE_VISIBILITY,
        note_save = zinnias_ciao_contracts::i18n::JA_NOTE_SAVE,
    )
}

/// Admin "Remove note" button for a specific member's note on an event.
pub fn admin_note_hide_form(
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
    _token: &str,
) -> String {
    let label = zinnias_ciao_contracts::i18n::JA_NOTE_DELETE;
    format!(
        "<a href=\"/c/{cid}/admin/events/{eid}/notes/{mid}/hide\" \
         style=\"font-size:.75rem;color:{danger};padding:.25rem .375rem;\
         min-height:44px;line-height:44px;display:inline-block;text-decoration:none\" \
         aria-label=\"{lbl}\">{lbl}</a>",
        cid = escape_html(community_id),
        eid = escape_html(event_id),
        mid = escape_html(target_membership_id),
        danger = CZ_COLOR_DANGER,
        lbl = label,
    )
}
