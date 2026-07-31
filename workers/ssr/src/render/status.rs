use super::shell::escape_html;
use zinnias_ciao_contracts::Locale;
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

/// The `cz-status-*--{suffix}` class suffix for a status value — the single
/// source of truth other status-to-class mappings must go through, so the
/// display path and the form path can never assign different classes to the
/// same status (RFC-075 Slice 2).
fn status_class(status: Option<&str>) -> &'static str {
    match status {
        Some("going") => "going",
        Some("not_going") => "not-going",
        Some("attended") => "attended",
        _ => "no-answer",
    }
}

/// Status class suffix, icon, and label for a status value — text/icon use.
/// `locale` selects the label only (RFC-072); the class and icon are
/// locale-independent. **RFC-075 Slice 2 reshape:** the first element used to
/// be an AA-passing fg hex colour: `status_display` computed it and the
/// caller wrote `color:{fg}` inline. Colour is now a class (`cz-status-text--
/// {suffix}` etc., app.css), so the first element is the class suffix
/// instead. Every caller was updated with this migration; the rendered
/// colour is unchanged — same tokens, referenced from CSS instead of Rust.
pub fn status_display(
    locale: Locale,
    status: Option<&str>,
) -> (&'static str, &'static str, &'static str) {
    let class = status_class(status);
    match status {
        Some("going") => (class, ICON_GOING, i18n::t(locale, i18n::STATUS_GOING)),
        Some("not_going") => (
            class,
            ICON_NOT_GOING,
            i18n::t(locale, i18n::STATUS_NOT_GOING),
        ),
        Some("attended") => (class, ICON_ATTENDED, i18n::t(locale, i18n::STATUS_ATTENDED)),
        _ => (
            class,
            ICON_NO_ANSWER,
            i18n::t(locale, i18n::STATUS_NO_ANSWER),
        ),
    }
}

/// Full triplet (fg, bg, border) for a status.
///
/// **RFC-075 Slice 2 note:** this was `status_form`'s only caller, computing
/// the inline `border`/`background`/`color` values its buttons used to carry.
/// Those are now `cz-status-btn--{suffix}` classes plus a `--current`
/// modifier (app.css) instead, so this function has no caller left anywhere
/// in the tree as of this slice. Left in place rather than deleted — the
/// same reasoning §7.4 applies to `event_card.rs`: removing a now-unused
/// function is a distinct decision from a presentation migration and
/// deserves its own small, reviewed step, not a silent deletion bundled in
/// here. Flagged in the review request.
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
pub fn status_chip(locale: Locale, status: Option<&str>) -> String {
    let (class, icon, label) = status_display(locale, status);
    format!("<span class=\"cz-status-chip cz-status-text--{class}\">{icon} {label}</span>")
}

/// Three-button status form for Event Detail (RFC-006).
#[allow(clippy::too_many_arguments)]
pub fn status_form(
    locale: Locale,
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
        let class = status_class(value);
        let current_class = if is_current {
            " cz-status-btn--current"
        } else {
            ""
        };
        let val_str = value.unwrap_or("clear");
        let disabled_attr = if disabled { " disabled" } else { "" };
        let title_attr = if disabled && !reason.is_empty() {
            format!(" title=\"{}\"", escape_html(reason))
        } else {
            String::new()
        };
        format!(
            "<button type=\"submit\" name=\"status\" value=\"{val}\" \
             class=\"cz-status-btn cz-status-btn--{class}{current_class}\"\
             {disabled_attr}{title_attr} aria-label=\"{label}\">\
             {icon} {label}</button>",
            val = escape_html(val_str),
        )
    };

    let going_btn = btn(
        Some("going"),
        i18n::t(locale, i18n::STATUS_GOING),
        ICON_GOING,
        false,
        "",
    );
    let notgoing_btn = btn(
        Some("not_going"),
        i18n::t(locale, i18n::STATUS_NOT_GOING),
        ICON_NOT_GOING,
        false,
        "",
    );
    let attended_btn = btn(
        Some("attended"),
        i18n::t(locale, i18n::STATUS_ATTENDED),
        ICON_ATTENDED,
        !can_set_attended,
        attended_disabled_reason,
    );

    let clear_btn = if current.is_some() {
        format!(
            "<button type=\"submit\" name=\"status\" value=\"clear\" \
             class=\"cz-status-clear-btn\" aria-label=\"{clear_label}\">{clear}</button>",
            clear_label = i18n::t(locale, i18n::STATUS_CLEAR_LABEL),
            clear = i18n::t(locale, i18n::STATUS_CLEAR),
        )
    } else {
        String::new()
    };

    format!(
        "<form method=\"post\" \
         action=\"/c/{cid}/events/{eid}/days/{did}/my-status\" \
         class=\"cz-status-form\">\
         <input type=\"hidden\" name=\"_token\" value=\"{tok}\">\
         <div class=\"cz-status-form-buttons\">{going}{notgoing}{attended}</div>\
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
