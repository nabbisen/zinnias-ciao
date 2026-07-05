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
