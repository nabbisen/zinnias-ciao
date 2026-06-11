//! HTML render helpers.
//!
//! M0: shell + placeholder only.
//! Later milestones add per-page render functions here.

use worker::{Response, Result};

const MANIFEST: &str = "/manifest.webmanifest";
const CSS: &str = "/static/app.css";
const JS: &str = "/static/app.js";
const THEME: &str = "#007AFF";

/// Shared HTML shell — wraps every page in the design-system scaffold.
fn shell(title: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
<html lang=\"en\">\n\
<head>\n\
  <meta charset=\"utf-8\">\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
  <meta name=\"theme-color\" content=\"{THEME}\">\n\
  <title>{title} \u{2014} ciao.zinnias</title>\n\
  <link rel=\"manifest\" href=\"{MANIFEST}\">\n\
  <link rel=\"stylesheet\" href=\"{CSS}\">\n\
</head>\n\
<body>\n\
{body}\n\
<script src=\"{JS}\" defer></script>\n\
</body>\n\
</html>",
        title = escape_html(title),
        body = body,
    )
}

/// Escape a string for safe HTML text node insertion (RFC-012 / RFC-007).
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            other => out.push(other),
        }
    }
    out
}

/// M0 placeholder page.
pub fn placeholder() -> Result<Response> {
    let body = "<main style=\"padding:2rem;font-family:system-ui,sans-serif;max-width:480px;margin:auto\">\n\
  <h1 style=\"font-size:1.25rem;font-weight:600\">ciao.zinnias</h1>\n\
  <p>Private community schedule sharing.</p>\n\
  <p style=\"color:#6e6e73;font-size:.875rem\">This environment is not ready for members yet.</p>\n\
</main>";
    Response::from_html(shell("ciao.zinnias", body))
}

/// Generic 404 — deliberately does not reveal resource existence (RFC-004).
pub fn not_found() -> Result<Response> {
    let body = "<main style=\"padding:2rem\"><p>Not found.</p></main>";
    Ok(Response::from_html(shell("Not found", body))?.with_status(404))
}

/// Generic 500 — no internal detail exposed (RFC-012).
pub fn internal_error() -> Result<Response> {
    let body = "<main style=\"padding:2rem\"><p>Something went wrong. Please try again.</p></main>";
    Ok(Response::from_html(shell("Error", body))?.with_status(500))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_script_tag() {
        let input = "<script>alert(\"xss\")</script>";
        let out = escape_html(input);
        assert!(!out.contains('<'));
        assert!(!out.contains('>'));
        assert!(out.contains("&lt;script&gt;"));
    }

    #[test]
    fn escape_html_ampersand() {
        assert_eq!(escape_html("a&b"), "a&amp;b");
    }

    #[test]
    fn escape_html_clean_string() {
        assert_eq!(escape_html("hello world"), "hello world");
    }

    #[test]
    fn escape_html_in_title() {
        // A title with special chars must not break the shell HTML
        let html = shell("<bad>", "");
        assert!(html.contains("&lt;bad&gt;"));
        assert!(!html.contains("<bad>"));
    }
}

/// Render a full page using the shared shell. Used by handlers.
pub fn page(title: &str, body: &str) -> worker::Result<worker::Response> {
    worker::Response::from_html(shell(title, body))
}
