//! HTML render helpers — shared shell, escape, and UI components.
//
// Design-vocabulary items (constants, helper fns) are declared here for future
// use and kept for reference; suppress dead_code warnings at module level.
#![allow(dead_code)]

use worker::{Response, Result};
use zinnias_ciao_contracts::i18n;

// ── CSS design tokens (RFC-011 §5 / RFC-020 v1.2 §E) ─────────────────────
// Must stay in sync with workers/ssr/static/app.css --cz-* custom properties.
//
// Status foregrounds use the AA-passing triplet values (≥4.5:1 on white).
// Use CZ_COLOR_* only for decorative fills (avatar rings, tints).
const CZ_COLOR_BG:             &str = "#FFFFFF";
const CZ_COLOR_SURFACE:        &str = "#F5F5F7";
const CZ_COLOR_SURFACE_STRONG: &str = "#E5E5EA";
const CZ_COLOR_TEXT_PRIMARY:   &str = "#1D1D1F";
const CZ_COLOR_TEXT_SECONDARY: &str = "#6E6E73";
const CZ_COLOR_DANGER:         &str = "#FF3B30";
const CZ_BORDER:               &str = "#E5E5EA"; // --cz-color-surface-strong
const CZ_BORDER_LIGHT:         &str = "#F5F5F7"; // --cz-color-surface

// Status triplets — fg passes WCAG AA (≥4.5:1) on white and on its own bg.
const CZ_STATUS_GOING_FG:        &str = "#005BBB"; // 5.0:1 on white
const CZ_STATUS_GOING_BG:        &str = "#EAF3FF";
const CZ_STATUS_GOING_BORDER:    &str = "#007AFF";
const CZ_STATUS_NOT_GOING_FG:    &str = "#B42318"; // 5.9:1 on white
const CZ_STATUS_NOT_GOING_BG:    &str = "#FFF0EF";
const CZ_STATUS_NOT_GOING_BORDER:&str = "#FF3B30";
const CZ_STATUS_ATTENDED_FG:     &str = "#167A34"; // 4.7:1 on white
const CZ_STATUS_ATTENDED_BG:     &str = "#EDFAF0";
const CZ_STATUS_ATTENDED_BORDER: &str = "#34C759";
const CZ_STATUS_NO_ANSWER_FG:    &str = "#6E6E73"; // 4.5:1 on white
const CZ_STATUS_NO_ANSWER_BG:    &str = "#F5F5F7";
const CZ_STATUS_NO_ANSWER_BORDER:&str = "#D1D1D6";

// Raw status colors — decorative use only (avatar rings, tints).
const CZ_COLOR_GOING:     &str = "#007AFF";
const CZ_COLOR_NOT_GOING: &str = "#FF3B30";
const CZ_COLOR_ATTENDED:  &str = "#34C759";
const CZ_COLOR_NO_ANSWER: &str = "#8E8E93";

// ── Status icons (RFC-011 §4) ─────────────────────────────────────────────
// Inline SVG — each icon is 1em × 1em, aria-hidden (label carries meaning).
const ICON_GOING: &str =
    "<svg aria-hidden='true' width='1em' height='1em' viewBox='0 0 16 16' fill='currentColor'>     <path d='M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28              a.75.75 0 0 1 1.06-1.06L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0z'/></svg>";
const ICON_NOT_GOING: &str =
    "<svg aria-hidden='true' width='1em' height='1em' viewBox='0 0 16 16' fill='currentColor'>     <path d='M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.75.75 0 1 1 1.06 1.06              L9.06 8l3.22 3.22a.75.75 0 1 1-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 0 1              -1.06-1.06L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06z'/></svg>";
const ICON_ATTENDED: &str =
    "<svg aria-hidden='true' width='1em' height='1em' viewBox='0 0 16 16' fill='currentColor'>     <path d='M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0zm3.78 5.22a.75.75 0 0 0-1.06 0L7 8.94              5.28 7.22a.75.75 0 0 0-1.06 1.06l2.25 2.25a.75.75 0 0 0 1.06 0l4.25-4.25              a.75.75 0 0 0 0-1.06z'/></svg>";
const ICON_NO_ANSWER: &str =
    "<svg aria-hidden='true' width='1em' height='1em' viewBox='0 0 16 16' fill='currentColor'>     <path d='M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0zM8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0              0-13zM7.25 10.5h1.5v1.5h-1.5zm0-7h1.5v5.5h-1.5z'/></svg>";


// ── Static asset paths ────────────────────────────────────────────────────
const MANIFEST: &str = "/manifest.webmanifest";
const CSS: &str      = "/static/app.css";
const JS: &str       = "/static/app.js";
const THEME: &str    = "#007AFF";

// ── Shell ─────────────────────────────────────────────────────────────────

/// Full HTML document shell.
fn shell(title: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
<html lang=\"ja\">\n\
<head>\n\
  <meta charset=\"utf-8\">\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
  <meta name=\"theme-color\" content=\"{THEME}\">\n\
  <title>{t} \u{2014} ciao.zinnias</title>\n\
  <link rel=\"manifest\" href=\"{MANIFEST}\">\n\
  <link rel=\"stylesheet\" href=\"{CSS}\">\n\
</head>\n\
<body>\n\
{body}\n\
<script src=\"{JS}\" defer></script>\n\
</body>\n\
</html>",
        t    = escape_html(title),
        body = body,
    )
}

/// Render a full page. Used by all handlers.
pub fn page(title: &str, body: &str) -> Result<Response> {
    Response::from_html(shell(title, body))
}

/// Escape a string for safe HTML text node insertion (RFC-012 / RFC-007).
/// This is the single escape function used everywhere — never emit raw user text.
/// Escape a string for safe insertion into HTML text or attribute values.
///
/// This is the single authoritative HTML escape path (XSS prevention, RFC-013 §8).
/// The implementation lives in `zinnias_ciao_contracts::html::escape_html` where
/// it can be unit-tested natively. Every user-generated string on a page must
/// pass through this function.
pub fn escape_html(s: &str) -> String {
    zinnias_ciao_contracts::html::escape_html(s)
}

// ── Navigation shell ──────────────────────────────────────────────────────

/// Bottom tab navigation (Home | Communities | Me).
pub fn bottom_nav(community_id: &str, active: &str) -> String {
    let tab = |label: &str, href: &str, id: &str| -> String {
        let aria = if id == active { " aria-current=\"page\"" } else { "" };
        let style = if id == active {
            "color:#007AFF;font-weight:600"
        } else {
            "color:#6E6E73"
        };
        format!(
            "<a href=\"{href}\" style=\"flex:1;text-align:center;padding:.75rem 0;\
             text-decoration:none;font-size:.8125rem;{style}\"{aria}>{label}</a>",
            href = escape_html(href),
        )
    };
    format!(
        "<nav role=\"navigation\" aria-label=\"Main\" \
         style=\"position:fixed;bottom:0;left:0;right:0;display:flex;\
         background:#FFFFFF;border-top:1px solid #E5E5EA;\
         padding-bottom:env(safe-area-inset-bottom)\">\
         {home}{communities}{me}\
         </nav>",
        home        = tab(i18n::JA_NAV_HOME, &format!("/c/{community_id}/home"), "home"),
        communities = tab(i18n::JA_NAV_COMMUNITIES, &format!("/c/{community_id}/communities"), "communities"),
        me          = tab(i18n::JA_NAV_ME, &format!("/c/{community_id}/me"), "me"),
    )
}

/// App header bar.
/// Simple header — for pages that don't need a community switcher (join, errors).
pub fn header(title: &str, community_name: &str) -> String {
    format!(
        "<header style=\"position:sticky;top:0;background:#FFFFFF;border-bottom:1px solid #E5E5EA;\
         padding:.875rem 1rem;display:flex;justify-content:space-between;align-items:center;z-index:10\">\
         <span style=\"font-size:1.25rem;font-weight:600\">{title}</span>\
         <span style=\"font-size:.8125rem;color:#6E6E73\">{community}</span>\
         </header>",
        title     = escape_html(title),
        community = escape_html(community_name),
    )
}

/// Header with a community switcher `<select>` in place of the static name.
///
/// `communities` is a slice of `(community_id, community_name)` pairs for the
/// current user. When the user has only one community the select is still shown
/// (consistent UI) but there is nothing else to switch to.
/// Submits via a tiny inline form (no JS needed; works with JS off via POST,
/// enhanced by an onchange JS redirect when JS is available).
pub fn header_with_switcher(
    title: &str,
    current_community_id: &str,
    communities: &[(impl AsRef<str>, impl AsRef<str>)],
) -> String {
    let title_s = escape_html(title);

    // <option> elements — use single-quoted HTML attributes to avoid \" in Rust strings.
    let options: String = communities.iter().map(|(id, name)| {
        let id_s   = escape_html(id.as_ref());
        let name_s = escape_html(name.as_ref());
        let sel    = if id.as_ref() == current_community_id { " selected" } else { "" };
        format!("<option value='{id_s}'{sel}>{name_s}</option>")
    }).collect();

    // onchange navigates immediately with JS.
    // Without JS the select still shows the current community visually.
    let mut h = String::new();
    h.push_str("<header style='position:sticky;top:0;background:#FFFFFF;");
    h.push_str("border-bottom:1px solid #E5E5EA;");
    h.push_str("padding:.875rem 1rem;display:flex;justify-content:space-between;");
    h.push_str("align-items:center;gap:.5rem;z-index:10'>");
    h.push_str("<span style='font-size:1.25rem;font-weight:600;white-space:nowrap'>");
    h.push_str(&title_s);
    // No-JS: a GET form to /switch with a submit button works without scripts.
    // Progressive enhancement: onchange auto-submits so JS users skip the button.
    h.push_str("</span>");
    h.push_str("<form method='get' action='/switch' style='margin:0'>");
    h.push_str("<select name='community' aria-label='Switch community' ");
    h.push_str("onchange='this.form.submit()' ");
    h.push_str("style='font-size:.8125rem;color:#6E6E73;background:none;border:none;");
    h.push_str("border-bottom:1px solid #E5E5EA;padding:.125rem .25rem;");
    h.push_str("max-width:160px;cursor:pointer'>");
    h.push_str(&options);
    h.push_str("</select>");
    h.push_str("<noscript><button type='submit' style='font-size:.8125rem;");
    h.push_str("margin-left:.25rem;min-height:44px;cursor:pointer'>Go</button></noscript>");
    h.push_str("</form>");
    h.push_str("</header>");
    h
}// ── Status chip / buttons ─────────────────────────────────────────────────

/// Colour, icon, and label for a status value — text/icon use (AA-passing fg).
pub fn status_display(status: Option<&str>) -> (&'static str, &'static str, &'static str) {
    // returns (fg_color, icon, label)
    match status {
        Some("going")     => (CZ_STATUS_GOING_FG,    ICON_GOING,     i18n::JA_STATUS_GOING),
        Some("not_going") => (CZ_STATUS_NOT_GOING_FG, ICON_NOT_GOING, i18n::JA_STATUS_NOT_GOING),
        Some("attended")  => (CZ_STATUS_ATTENDED_FG,  ICON_ATTENDED,  i18n::JA_STATUS_ATTENDED),
        _                 => (CZ_STATUS_NO_ANSWER_FG, ICON_NO_ANSWER, i18n::JA_STATUS_NO_ANSWER),
    }
}

/// Full triplet (fg, bg, border) for a status — used by buttons and surface fills.
pub fn status_triplet(status: Option<&str>) -> (&'static str, &'static str, &'static str) {
    match status {
        Some("going")     => (CZ_STATUS_GOING_FG,    CZ_STATUS_GOING_BG,    CZ_STATUS_GOING_BORDER),
        Some("not_going") => (CZ_STATUS_NOT_GOING_FG, CZ_STATUS_NOT_GOING_BG, CZ_STATUS_NOT_GOING_BORDER),
        Some("attended")  => (CZ_STATUS_ATTENDED_FG,  CZ_STATUS_ATTENDED_BG,  CZ_STATUS_ATTENDED_BORDER),
        _                 => (CZ_STATUS_NO_ANSWER_FG, CZ_STATUS_NO_ANSWER_BG, CZ_STATUS_NO_ANSWER_BORDER),
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
/// `day_id`, `event_id`, `community_id` scope the POST.
/// `token` is the server-issued form token (AD-4).
/// `current` is the member's current status (None = No answer).
/// `can_set_attended` controls whether Attended is enabled.
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
        let bg    = if is_current { bg_sel } else { CZ_COLOR_SURFACE };
        let val_str = value.unwrap_or("clear");
        let disabled_attr = if disabled { " disabled" } else { "" };
        let title_attr = if disabled && !reason.is_empty() {
            format!(" title=\"{}\"", escape_html(reason))
        } else { String::new() };
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

    let going_btn    = btn(Some("going"),     i18n::JA_STATUS_GOING,     ICON_GOING,     false, "");
    let notgoing_btn = btn(Some("not_going"), i18n::JA_STATUS_NOT_GOING, ICON_NOT_GOING, false, "");
    let attended_btn = btn(
        Some("attended"), i18n::JA_STATUS_ATTENDED, ICON_ATTENDED,
        !can_set_attended, attended_disabled_reason,
    );

    // Show a "Clear" link only when the member has an explicit status
    let clear_btn = if current.is_some() {
        format!(
            "<button type=\"submit\" name=\"status\" value=\"clear\" \
             style=\"font-size:.75rem;color:#6E6E73;background:none;border:none;\
             padding:.25rem;cursor:pointer\" aria-label=\"{clear_label}\">{clear}</button>",
                clear_label = i18n::JA_STATUS_CLEAR_LABEL,
                clear       = i18n::JA_STATUS_CLEAR,
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
        cid  = escape_html(community_id),
        eid  = escape_html(event_id),
        did  = escape_html(day_id),
        tok  = escape_html(token),
        going    = going_btn,
        notgoing = notgoing_btn,
        attended = attended_btn,
        clear    = clear_btn,
    )
}

// ── Note editor ───────────────────────────────────────────────────────────

/// Note textarea form for Event Detail (RFC-007).
pub fn note_form(
    community_id: &str,
    event_id: &str,
    save_token: &str,
    existing_note: Option<&str>,
    flash: Option<&str>,
) -> String {
    let flash_html = flash
        .map(|f| format!(
            "<p role=\"status\" style=\"font-size:.875rem;color:{};margin:.5rem 0\">{}</p>",
            CZ_STATUS_ATTENDED_FG, // AA-passing green for success text
            escape_html(f)
        ))
        .unwrap_or_default();

    // The delete button navigates to a route-backed confirmation page (RFC-043).
    // No form token needed here — the confirmation page issues its own token.
    let delete_btn = if existing_note.is_some() {
        format!(
            "<a href=\"/c/{cid}/events/{eid}/my-note/delete\" \
             style=\"display:inline-block;font-size:.875rem;color:{danger};padding:.25rem;\
             min-height:44px;line-height:44px;text-decoration:none\">{del}</a>",
            del    = zinnias_ciao_contracts::i18n::JA_NOTE_DELETE,
            cid    = escape_html(community_id),
            eid    = escape_html(event_id),
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
             <span style=\"font-size:.75rem;color:{muted}\">{note_char_hint}</span>\
             <button type=\"submit\" \
               style=\"padding:.625rem 1.25rem;background:{going_border};color:#FFFFFF;\
               border:none;border-radius:14px;font-size:.9375rem;font-weight:600;\
               min-height:44px;cursor:pointer\">{note_save}</button>\
           </div>\
         </form>\
         {delete}\
         </section>",
        cid          = escape_html(community_id),
        eid          = escape_html(event_id),
        tok          = escape_html(save_token),
        existing     = escape_html(existing_note.unwrap_or("")),
        flash        = flash_html,
        delete       = delete_btn,
        muted             = CZ_COLOR_TEXT_SECONDARY,
        border            = CZ_BORDER,
        going_border      = CZ_STATUS_GOING_BORDER,
        note_section_label    = i18n::JA_NOTE_SECTION_LABEL,
        note_placeholder_label = i18n::JA_NOTE_PLACEHOLDER_LABEL,
        note_char_hint        = i18n::JA_NOTE_CHAR_HINT,
        note_visibility       = i18n::JA_NOTE_VISIBILITY,
        note_save             = i18n::JA_NOTE_SAVE,
    )
}

/// Admin "Remove note" button for a specific member's note on an event (RFC-007/010).
/// Shown only to admins in the notes list section of Event Detail.
pub fn admin_note_hide_form(
    community_id: &str,
    event_id: &str,
    target_membership_id: &str,
    _token: &str, // token now issued on the confirmation page (RFC-043)
) -> String {
    // Navigate to the route-backed confirmation page; no JS confirm() needed.
    let label = zinnias_ciao_contracts::i18n::JA_NOTE_DELETE;
    format!(
        "<a href=\"/c/{cid}/admin/events/{eid}/notes/{mid}/hide\" \
         style=\"font-size:.75rem;color:{danger};padding:.25rem .375rem;\
         min-height:44px;line-height:44px;display:inline-block;text-decoration:none\" \
         aria-label=\"{lbl}\">{lbl}</a>",
        cid    = escape_html(community_id),
        eid    = escape_html(event_id),
        mid    = escape_html(target_membership_id),
        danger = CZ_COLOR_DANGER,
        lbl    = label,
    )
}

// ── Event card ────────────────────────────────────────────────────────────

pub struct CardDay<'a> {
    pub starts_at_utc: &'a str,
    pub ends_at_utc: &'a str,
    pub day_date: &'a str,
}

/// One event card for the Home list.
pub fn event_card(
    community_id: &str,
    event_id: &str,
    title: &str,
    location: Option<&str>,
    is_cancelled: bool,
    nearest_day: &CardDay<'_>,
    total_days: u32,
    my_status: Option<&str>,
    going: u32, not_going: u32, no_answer: u32,
    tz: &str,
) -> String {
    let (_, icon, label) = status_display(my_status);
    let (sc, _, _) = status_display(my_status);
    let cancelled_badge = if is_cancelled {
        format!("<span style=\"font-size:.75rem;background:#FF3B30;color:#FFFFFF;\
         border-radius:99px;padding:.125rem .5rem;margin-left:.5rem\">{}</span>",
         zinnias_ciao_contracts::i18n::JA_ADMIN_CANCEL_EVENT_CONFIRM)
    } else { String::new() };
    let multi_badge = if total_days > 1 {
        format!("<span style=\"font-size:.75rem;color:#6E6E73\"> · {total_days} 日間</span>")
    } else { String::new() };
    let loc_html = location.map(|l| format!(
        "<span style=\"color:#6E6E73;font-size:.875rem\"> · {}</span>",
        escape_html(l)
    )).unwrap_or_default();
    let counts_line = format!(
        "{} {} · {} {} · {} {}",
        zinnias_ciao_contracts::i18n::JA_STATUS_GOING, going,
        zinnias_ciao_contracts::i18n::JA_STATUS_NOT_GOING, not_going,
        zinnias_ciao_contracts::i18n::JA_STATUS_NO_ANSWER, no_answer,
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
        cid       = escape_html(community_id),
        eid       = escape_html(event_id),
        title     = escape_html(title),
        cancelled = cancelled_badge,
        multi     = multi_badge,
        time      = format_day_time_tz(nearest_day, tz),
        loc       = loc_html,
        counts    = counts_line,
    )
}

/// Format a day's time range for display, adjusted for the community timezone.
/// `tz` is an IANA timezone name (e.g. "Asia/Tokyo"). Falls back to UTC display.
pub fn format_day_time_tz(day: &CardDay<'_>, tz: &str) -> String {
    let offset_mins = tz_offset_minutes(tz);
    let starts = apply_offset_display(day.starts_at_utc, offset_mins);
    let ends   = apply_offset_time(day.ends_at_utc,   offset_mins);
    format!("{starts}–{ends}")
}

/// Format a day's time range for display (UTC — used when timezone is unknown).
fn format_day_time(day: &CardDay<'_>) -> String {
    let starts = parse_utc_display(day.starts_at_utc);
    let ends   = parse_utc_time(day.ends_at_utc);
    format!("{starts}–{ends}")
}

/// Fixed UTC-offset map for IANA names — delegates to contracts::tz (RFC-018).
fn tz_offset_minutes(tz: &str) -> i32 {
    zinnias_ciao_contracts::tz::offset_minutes_or_utc(tz)
}

/// Apply a UTC offset and return ("YYYY-MM-DD", "HH:MM") in local time.
fn utc_to_local_parts(utc: &str, offset_mins: i32) -> (String, String) {
    zinnias_ciao_contracts::tz::to_local_parts(utc, offset_mins)
}

/// Apply a UTC offset and return "Mon D, HH:MM" in local time.
fn apply_offset_display(utc: &str, offset_mins: i32) -> String {
    let (local_date, time_hm) = utc_to_local_parts(utc, offset_mins);
    if local_date.is_empty() { return parse_utc_display(utc); }
    // Japan-first pilot: use JA date label (e.g. "6月14日 09:00")
    let date_label = zinnias_ciao_contracts::tz::date_label_ja(&local_date);
    format!("{date_label} {time_hm}")
}

/// Apply a UTC offset and return only "HH:MM" in local time.
fn apply_offset_time(utc: &str, offset_mins: i32) -> String {
    utc_to_local_parts(utc, offset_mins).1
}

fn parse_utc_display(utc: &str) -> String {
    // Japan-first pilot: "2026-06-14T09:00:00.000Z" -> "6月14日（日） 09:00"
    let parts: Vec<&str> = utc.splitn(2, 'T').collect();
    if parts.len() < 2 { return utc.to_owned(); }
    let date = parts[0];
    let time = parts[1].get(..5).unwrap_or("");
    let date_label = zinnias_ciao_contracts::tz::date_label_ja(date);
    format!("{date_label} {time}")
}

fn parse_utc_time(utc: &str) -> String {
    utc.splitn(2, 'T')
        .nth(1)
        .and_then(|t| t.get(..5))
        .unwrap_or("")
        .to_owned()
}

/// Public re-export for handlers that need offset arithmetic (e.g. event.rs).
pub fn tz_offset_minutes_pub(tz: &str) -> i32 { tz_offset_minutes(tz) }
pub fn utc_to_local_parts_pub(utc: &str, offset: i32) -> (String, String) { utc_to_local_parts(utc, offset) }
pub fn apply_offset_time_pub(utc: &str, offset: i32) -> String { apply_offset_time(utc, offset) }

// ── Participant list ──────────────────────────────────────────────────────

pub struct ParticipantEntry<'a> {
    pub display_name: &'a str,
    pub status: Option<&'a str>,
}

pub fn participant_list(participants: &[ParticipantEntry<'_>]) -> String {
    if participants.is_empty() {
        return format!("<p style=\"color:#6E6E73;font-size:.875rem\">{}</p>", zinnias_ciao_contracts::i18n::JA_EVENT_MEMBER_FALLBACK);
    }
    let rows: String = participants.iter().map(|p| {
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
            name     = escape_html(p.display_name),
        )
    }).collect();
    format!("<ul style=\"list-style:none;padding:0;margin:0\">{rows}</ul>")
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .map(|c| c.to_uppercase().to_string())
        .collect::<Vec<_>>()
        .join("")
}

// ── Common pages ─────────────────────────────────────────────────────────

pub fn placeholder() -> Result<Response> {
    let body = format!("<main style=\"padding:2rem;font-family:system-ui,sans-serif;max-width:480px;margin:auto\">\
  <h1 style=\"font-size:1.25rem;font-weight:600\">{}</h1>\
  <p style=\"color:#6E6E73;font-size:.875rem\">{}</p>\
</main>",
        zinnias_ciao_contracts::i18n::JA_JOIN_HEADING,
        zinnias_ciao_contracts::i18n::JA_GENERAL_ERROR,
    );
    Response::from_html(shell(i18n::JA_JOIN_HEADING, &body))
}

pub fn not_found() -> Result<Response> {
    let body = format!("<main style=\"padding:2rem\"><p>{}</p></main>",
        zinnias_ciao_contracts::i18n::JA_NOT_FOUND);
    Ok(Response::from_html(shell(zinnias_ciao_contracts::i18n::JA_NOT_FOUND, &body))?.with_status(404))
}

pub fn internal_error() -> Result<Response> {
    let body = format!("<main style=\"padding:2rem\"><p>{}</p></main>",
        zinnias_ciao_contracts::i18n::JA_INTERNAL_ERROR);
    Ok(Response::from_html(shell(zinnias_ciao_contracts::i18n::JA_GENERAL_ERROR, &body))?.with_status(500))
}

pub fn session_expired() -> Result<Response> {
    let body = format!(
        "<main style=\"padding:2rem;font-family:system-ui,sans-serif;max-width:480px;margin:auto\">\
         <p style=\"color:#FF3B30\">{msg}</p>\
         <a href=\"/join\" style=\"display:inline-block;margin-top:1rem;color:#007AFF\">{join}</a></main>",
        msg  = zinnias_ciao_contracts::i18n::JA_SESSION_EXPIRED,
        join = zinnias_ciao_contracts::i18n::JA_JOIN_SUBMIT,
    );
    Ok(Response::from_html(shell(zinnias_ciao_contracts::i18n::JA_GENERAL_ERROR, &body))?.with_status(401))
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_script_tag() {
        let out = escape_html("<script>alert(\"xss\")</script>");
        assert!(!out.contains('<') && !out.contains('>'));
        assert!(out.contains("&lt;script&gt;"));
    }

    #[test]
    fn escape_ampersand() {
        assert_eq!(escape_html("a&b"), "a&amp;b");
    }

    #[test]
    fn escape_clean_string() {
        assert_eq!(escape_html("hello world"), "hello world");
    }

    #[test]
    fn title_escaped_in_shell() {
        // Verify the title is properly escaped when inserted into the page shell.
        // We test escape_html directly here because page() wraps a worker::Response
        // and cannot be constructed in a native test environment.
        let escaped = escape_html("<bad>&title");
        assert!(escaped.contains("&lt;bad&gt;"));
        assert!(escaped.contains("&amp;"));
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
    }

    #[test]
    fn initials_two_words() {
        assert_eq!(initials("Aya Tanaka"), "AT");
    }

    #[test]
    fn initials_one_word() {
        assert_eq!(initials("Aya"), "A");
    }

    #[test]
    fn initials_japanese_name() {
        // Each kanji is one Unicode char; we take the first two.
        assert_eq!(initials("田中 花子"), "田花");
    }

    #[test]
    fn parse_utc_time_basic() {
        assert_eq!(parse_utc_time("2026-06-14T10:30:00.000Z"), "10:30");
    }

    #[test]
    fn parse_utc_display_uses_ja_format() {
        // Home card date display must use Japanese convention, not "Jun 14".
        let out = parse_utc_display("2026-06-14T09:00:00.000Z");
        assert!(!out.contains("Jun"),   "must not contain English month: {out}");
        assert!(out.contains("月"),     "must contain 月: {out}");
        assert!(out.contains("日"),     "must contain 日: {out}");
        assert!(out.contains("09:00"),  "must contain time: {out}");
    }

    #[test]
    fn status_display_going() {
        let (_, _, label) = status_display(Some("going"));
        assert!(!label.is_empty());
        assert!(!label.contains("Going"), "label must be Japanese, got: {label}");
    }

    #[test]
    fn status_display_not_going() {
        let (_, _, label) = status_display(Some("not_going"));
        assert!(!label.is_empty());
        assert!(!label.contains("No Go"), "label must be Japanese, got: {label}");
    }

    #[test]
    fn status_display_no_answer_is_default() {
        let (_, _, label_none) = status_display(None);
        let (_, _, label_unknown) = status_display(Some("unknown_value"));
        assert_eq!(label_none, label_unknown, "unknown status must use same label as None");
        assert!(!label_none.is_empty());
    }
}
