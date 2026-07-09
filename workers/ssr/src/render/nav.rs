use super::shell::escape_html;
use zinnias_ciao_contracts::i18n;

/// Bottom tab navigation (Home | Communities | Me).
pub fn bottom_nav(community_id: &str, active: &str) -> String {
    let tab = |label: &str, href: &str, id: &str| -> String {
        let aria = if id == active {
            " aria-current=\"page\""
        } else {
            ""
        };
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
        home = tab(
            i18n::JA_NAV_HOME,
            &format!("/c/{community_id}/home"),
            "home"
        ),
        communities = tab(
            i18n::JA_NAV_COMMUNITIES,
            &format!("/c/{community_id}/communities"),
            "communities"
        ),
        me = tab(i18n::JA_NAV_ME, &format!("/c/{community_id}/me"), "me"),
    )
}

/// Simple header for pages that don't need a community switcher.
pub fn header(title: &str, community_name: &str) -> String {
    format!(
        "<header style=\"position:sticky;top:0;background:#FFFFFF;border-bottom:1px solid #E5E5EA;\
         padding:.875rem 1rem;display:flex;justify-content:space-between;align-items:center;z-index:10\">\
         <span style=\"font-size:1.25rem;font-weight:600\">{title}</span>\
         <span style=\"font-size:.8125rem;color:#6E6E73\">{community}</span>\
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

pub fn header_with_switcher_next(
    title: &str,
    current_community_id: &str,
    communities: &[(impl AsRef<str>, impl AsRef<str>)],
    next: &str,
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
    h.push_str("<header style='position:sticky;top:0;background:#FFFFFF;");
    h.push_str("border-bottom:1px solid #E5E5EA;");
    h.push_str("padding:.875rem 1rem;display:flex;justify-content:space-between;");
    h.push_str("align-items:center;gap:.5rem;flex-wrap:wrap;z-index:10'>");
    h.push_str("<span style='font-size:1.25rem;font-weight:600;flex:1 1 12rem;");
    h.push_str("min-width:0;white-space:normal;overflow-wrap:anywhere'>");
    h.push_str(&title_s);
    h.push_str("</span>");
    h.push_str("<form method='get' action='/switch' style='margin:0;min-width:0;max-width:100%'>");
    h.push_str("<input type='hidden' name='next' value='");
    h.push_str(&escape_html(next));
    h.push_str("'>");
    h.push_str("<select name='community' aria-label='Switch community' ");
    h.push_str("style='font-size:.8125rem;color:#6E6E73;background:none;border:none;");
    h.push_str("border-bottom:1px solid #E5E5EA;padding:.125rem .25rem;");
    h.push_str("max-width:100%;box-sizing:border-box;cursor:pointer'>");
    h.push_str(&options);
    h.push_str("</select>");
    h.push_str("<button type='submit' style='font-size:.8125rem;");
    h.push_str("margin-left:.25rem;min-height:44px;cursor:pointer;");
    h.push_str("background:#F5F5F7;color:#1D1D1F;border:1px solid #D1D1D6;");
    h.push_str("border-radius:8px;padding:.25rem .5rem'>");
    h.push_str(i18n::JA_NAV_SWITCH_GO);
    h.push_str("</button>");
    h.push_str("</form>");
    h.push_str("</header>");
    h
}
