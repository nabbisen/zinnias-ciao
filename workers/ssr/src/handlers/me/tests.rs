use super::*;

#[test]
fn me_flash_message_uses_fixed_success_code() {
    use zinnias_ciao_contracts::Locale;

    assert_eq!(
        me_flash_message(Locale::Ja, Some("display_name_updated")),
        Some(i18n::JA_ME_DISPLAY_NAME_UPDATED)
    );
    assert_eq!(
        me_flash_message(Locale::En, Some("display_name_updated")),
        Some(i18n::EN_ME_DISPLAY_NAME_UPDATED)
    );
    assert_eq!(
        me_flash_message(Locale::Ja, Some("名前を変更しました")),
        None
    );
    assert_eq!(
        me_flash_message(Locale::Ja, Some("<script>alert(1)</script>")),
        None
    );
    assert_eq!(me_flash_message(Locale::Ja, None), None);
}

#[test]
fn display_name_validation_errors_share_member_facing_copy() {
    assert_eq!(
        display_name_error(DisplayNameError::Empty),
        i18n::JA_ME_DISPLAY_NAME_ERROR
    );
    assert_eq!(
        display_name_error(DisplayNameError::TooLong),
        i18n::JA_ME_DISPLAY_NAME_ERROR
    );
    assert_eq!(
        display_name_error(DisplayNameError::InvalidChars),
        i18n::JA_ME_DISPLAY_NAME_ERROR
    );
}

#[test]
fn display_name_form_escapes_hostile_display_value() {
    let membership = test_membership();
    let html = display_name_form_body(
        &membership,
        "tok_form",
        r#"" autofocus onfocus="alert(1)" <script>alert(2)</script>"#,
        None,
    );

    assert!(html.contains("&quot; autofocus onfocus=&quot;alert(1)&quot;"));
    assert!(html.contains("&lt;script&gt;alert(2)&lt;/script&gt;"));
    assert!(!html.contains(r#"" autofocus onfocus="alert(1)""#));
    assert!(!html.contains("<script>"));
}

#[test]
fn display_name_form_escapes_error_and_token_values() {
    let membership = test_membership();
    let html = display_name_form_body(
        &membership,
        r#"" onmouseover="alert(1)"#,
        "Safe Name",
        Some(r#"<img src=x onerror=alert(1)>"#),
    );

    assert!(html.contains("value=\"&quot; onmouseover=&quot;alert(1)\""));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(!html.contains("<img"));
    assert!(!html.contains(r#"" onmouseover="alert(1)""#));
}

fn test_membership() -> crate::authz::MembershipContext {
    crate::authz::MembershipContext {
        membership_id: "mem_test".to_owned(),
        community_id: "com_test".to_owned(),
        user_id: "usr_test".to_owned(),
        role: "member".to_owned(),
        display_name: "Member".to_owned(),
        locale: zinnias_ciao_contracts::Locale::Ja,
    }
}
