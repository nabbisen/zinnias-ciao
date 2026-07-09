use super::shell::escape_html;
use zinnias_ciao_contracts::i18n;

// CSS design tokens (RFC-011 §5 / RFC-020 v1.2 §E).
// Must stay in sync with workers/ssr/static/app.css --cz-* custom properties.
const CZ_COLOR_BG: &str = "#FFFFFF";
pub(super) const CZ_COLOR_SURFACE: &str = "#F5F5F7";
const CZ_COLOR_SURFACE_STRONG: &str = "#E5E5EA";
const CZ_COLOR_TEXT_PRIMARY: &str = "#1D1D1F";
pub(super) const CZ_COLOR_TEXT_SECONDARY: &str = "#6E6E73";
pub(super) const CZ_COLOR_DANGER: &str = "#FF3B30";
pub(super) const CZ_BORDER: &str = "#E5E5EA";
const CZ_BORDER_LIGHT: &str = "#F5F5F7";

// Status triplets — fg passes WCAG AA (>=4.5:1) on white and on its own bg.
const CZ_STATUS_GOING_FG: &str = "#005BBB";
const CZ_STATUS_GOING_BG: &str = "#EAF3FF";
pub(super) const CZ_STATUS_GOING_BORDER: &str = "#007AFF";
const CZ_STATUS_NOT_GOING_FG: &str = "#B42318";
const CZ_STATUS_NOT_GOING_BG: &str = "#FFF0EF";
const CZ_STATUS_NOT_GOING_BORDER: &str = "#FF3B30";
pub(super) const CZ_STATUS_ATTENDED_FG: &str = "#167A34";
const CZ_STATUS_ATTENDED_BG: &str = "#EDFAF0";
const CZ_STATUS_ATTENDED_BORDER: &str = "#34C759";
const CZ_STATUS_NO_ANSWER_FG: &str = "#6E6E73";
const CZ_STATUS_NO_ANSWER_BG: &str = "#F5F5F7";
const CZ_STATUS_NO_ANSWER_BORDER: &str = "#D1D1D6";

// Raw status colors — decorative use only (avatar rings, tints).
const CZ_COLOR_GOING: &str = "#007AFF";
const CZ_COLOR_NOT_GOING: &str = "#FF3B30";
const CZ_COLOR_ATTENDED: &str = "#34C759";
const CZ_COLOR_NO_ANSWER: &str = "#8E8E93";

// Status icons (RFC-011 §4).
const ICON_GOING: &str = "<svg aria-hidden='true' width='1em' height='1em' viewBox='0 0 16 16' fill='currentColor'>     <path d='M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28              a.75.75 0 0 1 1.06-1.06L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0z'/></svg>";
const ICON_NOT_GOING: &str = "<svg aria-hidden='true' width='1em' height='1em' viewBox='0 0 16 16' fill='currentColor'>     <path d='M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.75.75 0 1 1 1.06 1.06              L9.06 8l3.22 3.22a.75.75 0 1 1-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 0 1              -1.06-1.06L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06z'/></svg>";
const ICON_ATTENDED: &str = "<svg aria-hidden='true' width='1em' height='1em' viewBox='0 0 16 16' fill='currentColor'>     <path d='M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0zm3.78 5.22a.75.75 0 0 0-1.06 0L7 8.94              5.28 7.22a.75.75 0 0 0-1.06 1.06l2.25 2.25a.75.75 0 0 0 1.06 0l4.25-4.25              a.75.75 0 0 0 0-1.06z'/></svg>";
const ICON_NO_ANSWER: &str = "<svg aria-hidden='true' width='1em' height='1em' viewBox='0 0 16 16' fill='currentColor'>     <path d='M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0zM8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0              0-13zM7.25 10.5h1.5v1.5h-1.5zm0-7h1.5v5.5h-1.5z'/></svg>";

/// Colour, icon, and label for a status value — text/icon use (AA-passing fg).
pub fn status_display(status: Option<&str>) -> (&'static str, &'static str, &'static str) {
    match status {
        Some("going") => (CZ_STATUS_GOING_FG, ICON_GOING, i18n::JA_STATUS_GOING),
        Some("not_going") => (
            CZ_STATUS_NOT_GOING_FG,
            ICON_NOT_GOING,
            i18n::JA_STATUS_NOT_GOING,
        ),
        Some("attended") => (
            CZ_STATUS_ATTENDED_FG,
            ICON_ATTENDED,
            i18n::JA_STATUS_ATTENDED,
        ),
        _ => (
            CZ_STATUS_NO_ANSWER_FG,
            ICON_NO_ANSWER,
            i18n::JA_STATUS_NO_ANSWER,
        ),
    }
}

/// Full triplet (fg, bg, border) for a status — used by buttons and surface fills.
pub fn status_triplet(status: Option<&str>) -> (&'static str, &'static str, &'static str) {
    match status {
        Some("going") => (
            CZ_STATUS_GOING_FG,
            CZ_STATUS_GOING_BG,
            CZ_STATUS_GOING_BORDER,
        ),
        Some("not_going") => (
            CZ_STATUS_NOT_GOING_FG,
            CZ_STATUS_NOT_GOING_BG,
            CZ_STATUS_NOT_GOING_BORDER,
        ),
        Some("attended") => (
            CZ_STATUS_ATTENDED_FG,
            CZ_STATUS_ATTENDED_BG,
            CZ_STATUS_ATTENDED_BORDER,
        ),
        _ => (
            CZ_STATUS_NO_ANSWER_FG,
            CZ_STATUS_NO_ANSWER_BG,
            CZ_STATUS_NO_ANSWER_BORDER,
        ),
    }
}

/// Status chip for event cards (read-only).
pub fn status_chip(status: Option<&str>) -> String {
    let (color, icon, label) = status_display(status);
    format!(
        "<span style=\"display:inline-flex;align-items:center;gap:.25rem;\
         color:{color};font-size:.8125rem;font-weight:600\">\
         {icon} {label}</span>"
    )
}

/// Three-button status form for Event Detail (RFC-006).
#[allow(clippy::too_many_arguments)]
pub fn status_form(
    community_id: &str,
    event_id: &str,
    day_id: &str,
    token: &str,
    current: Option<&str>,
    can_set_attended: bool,
    attended_disabled_reason: &str,
) -> String {
    let btn = |value: Option<&str>, label: &str, icon: &str, disabled: bool, reason: &str| {
        let is_current = current == value;
        let (fg, bg_sel, border) = status_triplet(value);
        let bg = if is_current { bg_sel } else { CZ_COLOR_SURFACE };
        let val_str = value.unwrap_or("clear");
        let disabled_attr = if disabled { " disabled" } else { "" };
        let title_attr = if disabled && !reason.is_empty() {
            format!(" title=\"{}\"", escape_html(reason))
        } else {
            String::new()
        };
        format!(
            "<button type=\"submit\" name=\"status\" value=\"{val}\" \
             style=\"flex:1;padding:.75rem .5rem;border:2px solid {border};\
             border-radius:14px;background:{bg};color:{fg};\
             font-size:.875rem;font-weight:600;min-height:44px;cursor:pointer;\
             display:flex;align-items:center;justify-content:center;gap:.25rem\"\
             {disabled_attr}{title_attr} aria-label=\"{label}\">\
             {icon} {label}</button>",
            val = escape_html(val_str),
        )
    };

    let going_btn = btn(Some("going"), i18n::JA_STATUS_GOING, ICON_GOING, false, "");
    let notgoing_btn = btn(
        Some("not_going"),
        i18n::JA_STATUS_NOT_GOING,
        ICON_NOT_GOING,
        false,
        "",
    );
    let attended_btn = btn(
        Some("attended"),
        i18n::JA_STATUS_ATTENDED,
        ICON_ATTENDED,
        !can_set_attended,
        attended_disabled_reason,
    );

    let clear_btn = if current.is_some() {
        format!(
            "<button type=\"submit\" name=\"status\" value=\"clear\" \
             style=\"font-size:.75rem;color:#6E6E73;background:none;border:none;\
             padding:.25rem;cursor:pointer\" aria-label=\"{clear_label}\">{clear}</button>",
            clear_label = i18n::JA_STATUS_CLEAR_LABEL,
            clear = i18n::JA_STATUS_CLEAR,
        )
    } else {
        String::new()
    };

    format!(
        "<form method=\"post\" \
         action=\"/c/{cid}/events/{eid}/days/{did}/my-status\" \
         style=\"margin:1rem 0\">\
         <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
         <div style=\"display:flex;gap:.5rem\">{going}{notgoing}{attended}</div>\
         {clear}\
         </form>",
        cid = escape_html(community_id),
        eid = escape_html(event_id),
        did = escape_html(day_id),
        tok = escape_html(token),
        going = going_btn,
        notgoing = notgoing_btn,
        attended = attended_btn,
        clear = clear_btn,
    )
}
