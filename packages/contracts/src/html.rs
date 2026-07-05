//! HTML escaping utilities (XSS prevention, RFC-013 §8).
//!
//! This module is the single authoritative implementation of HTML escaping.
//! `workers/ssr/src/render.rs` delegates to this function, ensuring every
//! user-generated string inserted into a page goes through a tested escape
//! path. The five characters `& < > " '` cover all HTML injection vectors
//! for content placed in element text or attribute values.

/// Escape a string for safe insertion into HTML text or attribute values.
///
/// Replaces `& < > " '` with their HTML entity equivalents.
/// The output is safe to insert between tags or inside `"…"` attributes.
///
/// This function does **not** sanitise URLs or CSS. Never interpolate
/// untrusted values into `href`/`src`/`style` attributes without additional
/// validation.
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
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

#[cfg(test)]
mod tests;
