use super::shell::escape_html;
use zinnias_ciao_contracts::Locale;
use zinnias_ciao_contracts::i18n;

/// Note textarea form for Event Detail (RFC-007).
pub fn note_form(
    locale: Locale,
    community_id: &str,
    event_id: &str,
    save_token: &str,
    existing_note: Option<&str>,
    flash: Option<&str>,
) -> String {
    let flash_html = flash
        .map(|f| {
            format!(
                "<p role=\"status\" class=\"cz-note-flash\">{}</p>",
                escape_html(f)
            )
        })
        .unwrap_or_default();

    let delete_btn = if existing_note.is_some() {
        format!(
            "<a href=\"/c/{cid}/events/{eid}/my-note/delete\" class=\"cz-note-delete-link\">{del}</a>",
            del = i18n::t(locale, i18n::NOTE_DELETE),
            cid = escape_html(community_id),
            eid = escape_html(event_id),
        )
    } else {
        String::new()
    };

    format!(
        "<section aria-label=\"{note_section_label}\" class=\"cz-note-section\">\
         <h2 class=\"cz-note-heading\">{note_section_label}</h2>\
         {flash}\
         <p class=\"cz-note-visibility\" aria-live=\"polite\">\
         {note_visibility}</p>\
         <form method=\"post\" action=\"/c/{cid}/events/{eid}/my-note\">\
           <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
           <textarea name=\"note\" rows=\"3\" maxlength=\"200\" \
             class=\"cz-note-textarea\" \
             aria-label=\"{note_placeholder_label}\">{existing}</textarea>\
           <div class=\"cz-note-footer\">\
             <span class=\"note-counter cz-note-counter\" aria-live=\"polite\">{note_char_hint}</span>\
             <button type=\"submit\" \
               class=\"cz-note-save-button\">{note_save}</button>\
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
        note_section_label = i18n::t(locale, i18n::NOTE_SECTION_LABEL),
        note_placeholder_label = i18n::t(locale, i18n::NOTE_PLACEHOLDER_LABEL),
        note_char_hint = i18n::t(locale, i18n::NOTE_CHAR_HINT),
        note_visibility = i18n::t(locale, i18n::NOTE_VISIBILITY),
        note_save = i18n::t(locale, i18n::NOTE_SAVE),
    )
}

/// Admin "Remove note" button for a specific member's note on an event.
pub fn admin_note_hide_form(
    locale: Locale,
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
    _token: &str,
) -> String {
    let label = i18n::t(locale, i18n::NOTE_DELETE);
    format!(
        "<a href=\"/c/{cid}/admin/events/{eid}/notes/{mid}/hide\" \
         class=\"cz-note-admin-hide-link\" \
         aria-label=\"{lbl}\">{lbl}</a>",
        cid = escape_html(community_id),
        eid = escape_html(event_id),
        mid = escape_html(target_membership_id),
        lbl = label,
    )
}
