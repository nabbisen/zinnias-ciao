use super::*;
use zinnias_ciao_contracts::Locale;

#[test]
fn community_name_error_follows_locale() {
    assert_eq!(
        community_name_error(Locale::Ja, CommunityNameError::Empty),
        i18n::JA_COMMUNITY_CREATE_NAME_ERROR
    );
    assert_eq!(
        community_name_error(Locale::En, CommunityNameError::Empty),
        i18n::EN_COMMUNITY_CREATE_NAME_ERROR
    );
    assert_eq!(
        community_name_error(Locale::Ja, CommunityNameError::TooLong),
        i18n::JA_COMMUNITY_CREATE_NAME_TOO_LONG
    );
    assert_eq!(
        community_name_error(Locale::En, CommunityNameError::TooLong),
        i18n::EN_COMMUNITY_CREATE_NAME_TOO_LONG
    );
    assert_eq!(
        community_name_error(Locale::Ja, CommunityNameError::InvalidCharacter),
        i18n::JA_COMMUNITY_CREATE_NAME_INVALID
    );
    assert_eq!(
        community_name_error(Locale::En, CommunityNameError::InvalidCharacter),
        i18n::EN_COMMUNITY_CREATE_NAME_INVALID
    );
}

#[test]
fn display_name_error_follows_locale() {
    assert_eq!(
        display_name_error(Locale::Ja, DisplayNameError::Empty),
        i18n::JA_COMMUNITY_CREATE_DISPLAY_NAME_ERROR
    );
    assert_eq!(
        display_name_error(Locale::En, DisplayNameError::Empty),
        i18n::EN_COMMUNITY_CREATE_DISPLAY_NAME_ERROR
    );
    assert_eq!(
        display_name_error(Locale::Ja, DisplayNameError::TooLong),
        i18n::JA_COMMUNITY_CREATE_DISPLAY_NAME_ERROR
    );
    assert_eq!(
        display_name_error(Locale::En, DisplayNameError::InvalidChars),
        i18n::EN_COMMUNITY_CREATE_DISPLAY_NAME_ERROR
    );
}
