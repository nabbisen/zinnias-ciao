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
mod tests {
    use super::*;

    #[test]
    fn plain_text_unchanged() {
        assert_eq!(escape_html("hello world"), "hello world");
        assert_eq!(escape_html(""), "");
    }

    #[test]
    fn ampersand_escaped() {
        assert_eq!(escape_html("A & B"), "A &amp; B");
    }

    #[test]
    fn angle_brackets_escaped() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("</script>"), "&lt;/script&gt;");
    }

    #[test]
    fn double_quote_escaped() {
        assert_eq!(escape_html("say \"hello\""), "say &quot;hello&quot;");
    }

    #[test]
    fn single_quote_escaped() {
        assert_eq!(escape_html("it's fine"), "it&#x27;s fine");
    }

    #[test]
    fn xss_vector_fully_escaped() {
        // Classic script injection attempt.
        let malicious = "<script>alert('xss')</script>";
        let escaped = escape_html(malicious);
        assert!(!escaped.contains('<'), "< must be escaped");
        assert!(!escaped.contains('>'), "> must be escaped");
        assert!(!escaped.contains('\''), "' must be escaped");
        assert_eq!(
            escaped,
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn attribute_injection_escaped() {
        // Value injected into href="…" with a quote to break out of attribute.
        let input = r#"foo" onclick="evil()"#;
        let escaped = escape_html(input);
        assert!(
            !escaped.contains('"'),
            "\" must be escaped in attribute context"
        );
    }

    #[test]
    fn all_five_entities() {
        assert_eq!(escape_html("&<>\"'"), "&amp;&lt;&gt;&quot;&#x27;");
    }

    #[test]
    fn japanese_text_preserved() {
        let ja = "6月14日（土）イベント";
        assert_eq!(escape_html(ja), ja);
    }

    #[test]
    fn multi_byte_and_special_mix() {
        assert_eq!(escape_html("田中さん <Admin>"), "田中さん &lt;Admin&gt;");
    }
}
