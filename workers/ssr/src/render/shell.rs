use worker::{Response, Result};
use zinnias_ciao_contracts::Locale;

// Static asset paths.
const MANIFEST: &str = "/manifest.webmanifest";
const CSS: &str = "/static/app.css";
const JS: &str = "/static/app.js?v=0.62.0";
const THEME: &str = "#007AFF";

/// Full HTML document shell for the given `lang` attribute value.
pub(super) fn shell_with_lang(lang: &str, title: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
<html lang=\"{lang}\">\n\
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
        lang = lang,
        t = escape_html(title),
        body = body,
    )
}

/// Full HTML document shell. Non-migrated pages: always Japanese.
///
/// RFC-084 (Handoff 084) converted the last caller of [`page`] below,
/// leaving this and `page` itself with zero callers anywhere in the
/// codebase — kept, not removed, per that package's explicit "no helper is
/// removed by this package" scope; whether to delete them is a decision
/// for whoever picks up next, not assumed here.
#[allow(dead_code)]
pub(super) fn shell(title: &str, body: &str) -> String {
    shell_with_lang("ja", title, body)
}

/// Render a full page. Was used by every non-migrated handler; always
/// renders `lang="ja"` regardless of any membership preference — this
/// function's behavior is unchanged by RFC-072. Migrated pages call
/// [`page_localized`] instead.
///
/// RFC-083 Slice D's last package (RFC-084, Handoff 084) converted this
/// function's own last remaining caller (`handlers/account/*.rs`) —
/// confirmed by a repo-wide grep for `render::page(` finding none. Kept
/// rather than removed: that handoff's own scope said "no helper is
/// removed by this package," so deleting it here would have exceeded
/// authorization even though it is now provably dead. Left for an
/// explicit decision, not silently cleaned up.
#[allow(dead_code)]
pub fn page(title: &str, body: &str) -> Result<Response> {
    Response::from_html(shell(title, body))
}

/// Render a full page for a page migrated to locale-aware rendering
/// (RFC-072). `title` must already be the string resolved for `locale` —
/// this function only threads `locale` into `html lang`; it does not
/// resolve any string itself.
pub fn page_localized(locale: Locale, title: &str, body: &str) -> Result<Response> {
    Response::from_html(shell_with_lang(locale.code(), title, body))
}

/// Escape a string for safe insertion into HTML text or attribute values.
///
/// This is the single authoritative HTML escape path (XSS prevention,
/// RFC-013 §8). The implementation lives in
/// `zinnias_ciao_contracts::html::escape_html` where it can be unit-tested
/// natively. Every user-generated string on a page must pass through this
/// function.
pub fn escape_html(s: &str) -> String {
    zinnias_ciao_contracts::html::escape_html(s)
}
