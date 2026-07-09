use worker::{Response, Result};

// Static asset paths.
const MANIFEST: &str = "/manifest.webmanifest";
const CSS: &str = "/static/app.css";
const JS: &str = "/static/app.js?v=0.53.0-render-split";
const THEME: &str = "#007AFF";

/// Full HTML document shell.
pub(super) fn shell(title: &str, body: &str) -> String {
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
        t = escape_html(title),
        body = body,
    )
}

/// Render a full page. Used by all handlers.
pub fn page(title: &str, body: &str) -> Result<Response> {
    Response::from_html(shell(title, body))
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
