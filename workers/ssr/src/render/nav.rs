use super::shell::escape_html;
use zinnias_ciao_contracts::Locale;
use zinnias_ciao_contracts::i18n;

/// Bottom tab navigation (Home | Communities | Me). Non-migrated pages:
/// always Japanese labels. Behavior unchanged by RFC-072.
pub fn bottom_nav(community_id: &str, active: &str) -> String {
    bottom_nav_localized(community_id, active, Locale::Ja)
}

/// Bottom tab navigation with locale-selected labels (RFC-072).
pub fn bottom_nav_localized(community_id: &str, active: &str, locale: Locale) -> String {
    let tab = |label: &str, href: &str, id: &str| -> String {
        let aria = if id == active {
            " aria-current=\"page\""
        } else {
            ""
        };
        let active_class = if id == active {
            " cz-bottom-nav-tab--active"
        } else {
            ""
        };
        format!(
            "<a href=\"{href}\" class=\"cz-bottom-nav-tab{active_class}\"{aria}>{label}</a>",
            href = escape_html(href),
        )
    };
    format!(
        "<nav role=\"navigation\" aria-label=\"Main\" class=\"cz-bottom-nav\">\
         {home}{communities}{me}\
         </nav>",
        home = tab(
            i18n::t(locale, i18n::NAV_HOME),
            &format!("/c/{community_id}/home"),
            "home"
        ),
        communities = tab(
            i18n::t(locale, i18n::NAV_COMMUNITIES),
            &format!("/c/{community_id}/communities"),
            "communities"
        ),
        me = tab(
            i18n::t(locale, i18n::NAV_ME),
            &format!("/c/{community_id}/me"),
            "me"
        ),
    )
}

/// Simple header for pages that don't need a community switcher.
pub fn header(title: &str, community_name: &str) -> String {
    format!(
        "<header class=\"cz-header\">\
         <span class=\"cz-header-title\">{title}</span>\
         <span class=\"cz-header-community\">{community}</span>\
         </header>",
        title = escape_html(title),
        community = escape_html(community_name),
    )
}

/// Header with a community switcher `<select>` in place of the static name.
pub fn header_with_switcher(
    title: &str,
    current_community_id: &str,
    communities: &[(impl AsRef<str>, impl AsRef<str>)],
) -> String {
    header_with_switcher_next(title, current_community_id, communities, "home")
}

/// Header with a community switcher, falling back to target Home on switch
/// (RFC-074) with the switch button's own label locale-selected (RFC-072).
/// Used by pages that have no cross-community equivalent to preserve (Event
/// Detail, note-delete confirmation): a bare `next="home"` call, not
/// [`header_with_switcher_next_localized`] with an explicit token.
pub fn header_with_switcher_localized(
    title: &str,
    current_community_id: &str,
    communities: &[(impl AsRef<str>, impl AsRef<str>)],
    locale: Locale,
) -> String {
    header_with_switcher_next_localized(title, current_community_id, communities, "home", locale)
}

/// Non-migrated pages: always the Japanese "Switch" button label. Behavior
/// unchanged by RFC-072; migrated pages call
/// [`header_with_switcher_next_localized`] instead.
pub fn header_with_switcher_next(
    title: &str,
    current_community_id: &str,
    communities: &[(impl AsRef<str>, impl AsRef<str>)],
    next: &str,
) -> String {
    header_with_switcher_next_localized(title, current_community_id, communities, next, Locale::Ja)
}

/// Header with a community switcher, with the switch button's own label
/// locale-selected (RFC-072). `title` must already be the string resolved
/// for `locale`; community names in `communities` are user data, not app
/// copy, and are never localized.
pub fn header_with_switcher_next_localized(
    title: &str,
    current_community_id: &str,
    communities: &[(impl AsRef<str>, impl AsRef<str>)],
    next: &str,
    locale: Locale,
) -> String {
    let title_s = escape_html(title);

    let options: String = communities
        .iter()
        .map(|(id, name)| {
            let id_s = escape_html(id.as_ref());
            let name_s = escape_html(name.as_ref());
            let sel = if id.as_ref() == current_community_id {
                " selected"
            } else {
                ""
            };
            format!("<option value='{id_s}'{sel}>{name_s}</option>")
        })
        .collect();

    let mut h = String::new();
    h.push_str("<header class='cz-header-switcher'>");
    h.push_str("<span class='cz-header-switcher-title'>");
    h.push_str(&title_s);
    h.push_str("</span>");
    h.push_str("<form method='get' action='/switch' class='cz-header-switcher-form'>");
    h.push_str("<input type='hidden' name='next' value='");
    h.push_str(&escape_html(next));
    h.push_str("'>");
    h.push_str("<select name='community' aria-label='");
    h.push_str(&escape_html(i18n::t(locale, i18n::NAV_SWITCH_ARIA_LABEL)));
    h.push_str("' ");
    h.push_str("class='cz-header-switcher-select'>");
    h.push_str(&options);
    h.push_str("</select>");
    h.push_str("<button type='submit' class='cz-header-switcher-button'>");
    h.push_str(i18n::t(locale, i18n::NAV_SWITCH_GO));
    h.push_str("</button>");
    h.push_str("</form>");
    h.push_str("</header>");
    h
}
