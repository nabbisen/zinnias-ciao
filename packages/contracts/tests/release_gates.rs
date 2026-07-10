//! Release-gate checks (RFC-015).
//! Every item here maps to a row in the MVP release-gate matrix.

#![allow(clippy::assertions_on_constants)]

use zinnias_ciao_contracts::auth::token_purpose;
use zinnias_ciao_contracts::{
    AppError, FORM_TOKEN_TTL_SECONDS, RELINK_CODE_TTL_SECONDS, SESSION_TTL_SECONDS,
};

// ── Session / auth gates ──────────────────────────────────────────────────

#[test]
fn session_ttl_positive_and_bounded() {
    assert!(
        SESSION_TTL_SECONDS > 0,
        "session TTL must be positive (Max-Age=0 bug)"
    );
    assert!(SESSION_TTL_SECONDS >= 3600, "session TTL too short");
    assert!(
        SESSION_TTL_SECONDS <= 31 * 86400,
        "session TTL too long for invite-only MVP"
    );
}

#[test]
fn form_token_ttl_shorter_than_session() {
    assert!(
        FORM_TOKEN_TTL_SECONDS < SESSION_TTL_SECONDS,
        "form token must expire before the session"
    );
}

#[test]
fn session_ttl_never_derived_from_token_exp() {
    // Documents the regression: if someone naively computed TTL as
    // token_exp - now and the token was at the JWT leeway edge (~55s past exp),
    // Max-Age would be <= 0 and the browser would discard the cookie immediately.
    let token_exp: i64 = 1_000_000_000;
    let now_at_edge: i64 = 1_000_000_055; // 55 s past exp — within 60 s leeway
    let derived: i64 = token_exp - now_at_edge;
    assert!(
        derived <= 0,
        "derived TTL {} <= 0 demonstrates the bug",
        derived
    );
    // The correct value is always the constant:
    assert!(SESSION_TTL_SECONDS as i64 > 0);
}

// ── Error model gates ─────────────────────────────────────────────────────

#[test]
fn not_found_and_forbidden_same_message() {
    assert_eq!(
        AppError::not_found().user_message,
        AppError::forbidden().user_message
    );
}

#[test]
fn internal_error_message_generic() {
    let msg = AppError::internal().user_message;
    assert!(!msg.to_lowercase().contains("sql"));
    assert!(!msg.to_lowercase().contains("panic"));
    assert!(!msg.to_lowercase().contains("stack"));
}

#[test]
fn invite_error_message_generic() {
    let msg = AppError::invite_invalid().user_message;
    assert!(!msg.to_lowercase().contains("hmac"));
    assert!(!msg.to_lowercase().contains("hash"));
    assert!(!msg.to_lowercase().contains("database"));
}

#[test]
fn token_invalid_error_is_retryable() {
    assert!(AppError::token_invalid().retryable);
}

// ── Token purpose completeness gate ──────────────────────────────────────

#[test]
fn all_state_changing_routes_have_token_purpose() {
    // Every mutating route needs a purpose string so tokens can be scoped.
    let required = [
        token_purpose::SET_STATUS,
        token_purpose::SAVE_NOTE,
        token_purpose::DELETE_NOTE,
        token_purpose::CREATE_EVENT,
        token_purpose::EDIT_EVENT,
        token_purpose::CANCEL_EVENT,
        token_purpose::ATTENDANCE_OVERRIDE,
        token_purpose::ADMIN_HIDE_NOTE,
        token_purpose::REVOKE_INVITE,
        token_purpose::CALENDAR_REGENERATE,
        token_purpose::CALENDAR_REVOKE,
        token_purpose::CALENDAR_MATRIX_CSV_EXPORT,
        token_purpose::COMMUNITY_EXPORT,
        token_purpose::CREATE_TEMPLATE,
        token_purpose::DELETE_TEMPLATE,
        token_purpose::REMOVE_MEMBER,
        token_purpose::PROMOTE_MEMBER,
        token_purpose::DEMOTE_MEMBER,
        token_purpose::HELP_SIGNIN,
        token_purpose::REDEEM_RELINK,
        token_purpose::GENERATE_INVITE,
        token_purpose::REDEEM_INVITE,
        token_purpose::JOIN_PROFILE,
        token_purpose::LOGOUT,
        token_purpose::CREATE_COMMUNITY,
    ];
    for p in required {
        assert!(!p.is_empty(), "token purpose must not be empty: {p}");
        assert!(
            !p.contains(' '),
            "token purpose must not contain spaces: {p}"
        );
    }
}

// ── i18n parity gate ──────────────────────────────────────────────────────
// Every EN_* constant must have a non-empty JA_* counterpart.
// This test registers every member-facing string pair so a JA string going
// empty or missing causes `cargo test` to fail immediately.
// To add a new string: add EN_FOO and JA_FOO in i18n.rs, then add the pair below.

#[test]
fn i18n_en_ja_parity_count() {
    use zinnias_ciao_contracts::i18n::*;
    let pairs = [
        (EN_JOIN_HEADING, JA_JOIN_HEADING),
        (EN_JOIN_SUBHEADING, JA_JOIN_SUBHEADING),
        (EN_JOIN_CODE_LABEL, JA_JOIN_CODE_LABEL),
        (EN_JOIN_CODE_HINT, JA_JOIN_CODE_HINT),
        (EN_JOIN_SUBMIT, JA_JOIN_SUBMIT),
        (EN_JOIN_PROFILE_HEADING, JA_JOIN_PROFILE_HEADING),
        (EN_JOIN_PROFILE_HINT, JA_JOIN_PROFILE_HINT),
        (EN_JOIN_PROFILE_LABEL, JA_JOIN_PROFILE_LABEL),
        (EN_JOIN_PROFILE_SUBMIT, JA_JOIN_PROFILE_SUBMIT),
        (EN_STATUS_GOING, JA_STATUS_GOING),
        (EN_STATUS_NOT_GOING, JA_STATUS_NOT_GOING),
        (EN_STATUS_ATTENDED, JA_STATUS_ATTENDED),
        (EN_STATUS_NO_ANSWER, JA_STATUS_NO_ANSWER),
        (EN_STATUS_ATTENDED_DISABLED, JA_STATUS_ATTENDED_DISABLED),
        (EN_NOTE_SAVE, JA_NOTE_SAVE),
        (EN_NOTE_DELETE, JA_NOTE_DELETE),
        (EN_NOTE_SAVED, JA_NOTE_SAVED),
        (EN_NOTE_TOO_LONG, JA_NOTE_TOO_LONG),
        (EN_SESSION_EXPIRED, JA_SESSION_EXPIRED),
        (EN_LOGOUT, JA_LOGOUT),
        (EN_LOGOUT_CONFIRM, JA_LOGOUT_CONFIRM),
        (EN_GENERAL_ERROR, JA_GENERAL_ERROR),
        (EN_OFFLINE_BANNER, JA_OFFLINE_BANNER),
        (EN_EMPTY_EVENTS, JA_EMPTY_EVENTS),
        (EN_EMPTY_EVENTS_HINT, JA_EMPTY_EVENTS_HINT),
        (EN_EMPTY_EVENTS_ADMIN, JA_EMPTY_EVENTS_ADMIN),
        (EN_NAV_HOME, JA_NAV_HOME),
        (EN_NAV_COMMUNITIES, JA_NAV_COMMUNITIES),
        (EN_NAV_ME, JA_NAV_ME),
        (EN_HOME_TODAY, JA_HOME_TODAY),
        (EN_HOME_THIS_WEEK, JA_HOME_THIS_WEEK),
        (EN_HOME_LATER, JA_HOME_LATER),
        (EN_HOME_CREATE_EVENT, JA_HOME_CREATE_EVENT),
        (EN_HOME_INVITE_MEMBERS, JA_HOME_INVITE_MEMBERS),
        (EN_HOME_MANAGE_MEMBERS, JA_HOME_MANAGE_MEMBERS),
        (EN_HOME_CALENDAR_TITLE, JA_HOME_CALENDAR_TITLE),
        (EN_HOME_CALENDAR_HELPER, JA_HOME_CALENDAR_HELPER),
        (EN_HOME_CALENDAR_EMPTY, JA_HOME_CALENDAR_EMPTY),
        (EN_HOME_CALENDAR_COUNT_SUFFIX, JA_HOME_CALENDAR_COUNT_SUFFIX),
        (EN_HOME_AGENDA_TITLE, JA_HOME_AGENDA_TITLE),
        (EN_CALENDAR_MONTH_TITLE, JA_CALENDAR_MONTH_TITLE),
        (EN_CALENDAR_PREV_MONTH, JA_CALENDAR_PREV_MONTH),
        (EN_CALENDAR_NEXT_MONTH, JA_CALENDAR_NEXT_MONTH),
        (EN_CALENDAR_THIS_MONTH, JA_CALENDAR_THIS_MONTH),
        (EN_CALENDAR_ALL_DAYS, JA_CALENDAR_ALL_DAYS),
        (EN_CALENDAR_EMPTY_MONTH, JA_CALENDAR_EMPTY_MONTH),
        (EN_CALENDAR_EMPTY_DAY, JA_CALENDAR_EMPTY_DAY),
        (EN_CALENDAR_CREATE_ON_DAY, JA_CALENDAR_CREATE_ON_DAY),
        (EN_CALENDAR_VIEW_MONTH, JA_CALENDAR_VIEW_MONTH),
        (EN_CALENDAR_VIEW_MATRIX, JA_CALENDAR_VIEW_MATRIX),
        (EN_CALENDAR_MATRIX_TITLE, JA_CALENDAR_MATRIX_TITLE),
        (EN_CALENDAR_MATRIX_TOO_LARGE, JA_CALENDAR_MATRIX_TOO_LARGE),
        (EN_CALENDAR_MATRIX_NO_MEMBERS, JA_CALENDAR_MATRIX_NO_MEMBERS),
        (EN_CALENDAR_MATRIX_CSV_EXPORT, JA_CALENDAR_MATRIX_CSV_EXPORT),
        (EN_CALENDAR_MATRIX_CSV_ERROR, JA_CALENDAR_MATRIX_CSV_ERROR),
        (EN_STATUS_CLEAR, JA_STATUS_CLEAR),
        (EN_STATUS_CLEAR_LABEL, JA_STATUS_CLEAR_LABEL),
        (EN_NOTE_SECTION_LABEL, JA_NOTE_SECTION_LABEL),
        (EN_NOTE_PLACEHOLDER_LABEL, JA_NOTE_PLACEHOLDER_LABEL),
        (EN_NOTE_CHAR_HINT, JA_NOTE_CHAR_HINT),
        (EN_NOTE_VISIBILITY, JA_NOTE_VISIBILITY),
        (EN_ME_SECTION_NAME, JA_ME_SECTION_NAME),
        (EN_ME_SECTION_COMMUNITY, JA_ME_SECTION_COMMUNITY),
        (EN_ME_SECTION_HELP, JA_ME_SECTION_HELP),
        (EN_ME_HELP_BODY, JA_ME_HELP_BODY),
        (EN_ADMIN_CREATE_EVENT_TITLE, JA_ADMIN_CREATE_EVENT_TITLE),
        (EN_ADMIN_CREATE_EVENT_SUBMIT, JA_ADMIN_CREATE_EVENT_SUBMIT),
        (EN_REPEAT_END_OPEN, JA_REPEAT_END_OPEN),
        (EN_REPEAT_END_UNTIL, JA_REPEAT_END_UNTIL),
        (EN_REPEAT_END_COUNT, JA_REPEAT_END_COUNT),
        (EN_REPEAT_COUNT_LABEL, JA_REPEAT_COUNT_LABEL),
        (EN_REPEAT_UNTIL_LABEL, JA_REPEAT_UNTIL_LABEL),
        (EN_OCCURRENCE_CANCEL_ACTION, JA_OCCURRENCE_CANCEL_ACTION),
        (EN_OCCURRENCE_CANCEL_TITLE, JA_OCCURRENCE_CANCEL_TITLE),
        (EN_OCCURRENCE_CANCEL_HELPER, JA_OCCURRENCE_CANCEL_HELPER),
        (EN_OCCURRENCE_CANCEL_SUBMIT, JA_OCCURRENCE_CANCEL_SUBMIT),
        (EN_OCCURRENCE_CANCELLED_BADGE, JA_OCCURRENCE_CANCELLED_BADGE),
        (EN_CALENDAR_OUT_OF_RANGE, JA_CALENDAR_OUT_OF_RANGE),
        (
            EN_CALENDAR_MATERIALIZATION_LIMIT,
            JA_CALENDAR_MATERIALIZATION_LIMIT,
        ),
        (
            EN_ADMIN_RECREATE_EVENT_ACTION,
            JA_ADMIN_RECREATE_EVENT_ACTION,
        ),
        (
            EN_ADMIN_RECREATE_EVENT_HELPER,
            JA_ADMIN_RECREATE_EVENT_HELPER,
        ),
        (EN_ADMIN_COPY_EVENT_ACTION, JA_ADMIN_COPY_EVENT_ACTION),
        (EN_ADMIN_COPY_EVENT_TITLE, JA_ADMIN_COPY_EVENT_TITLE),
        (EN_ADMIN_COPY_EVENT_HELPER, JA_ADMIN_COPY_EVENT_HELPER),
        (
            EN_ADMIN_COPY_EVENT_DATE_WARNING,
            JA_ADMIN_COPY_EVENT_DATE_WARNING,
        ),
        (
            EN_ADMIN_COPY_EVENT_MULTI_DAY_HELPER,
            JA_ADMIN_COPY_EVENT_MULTI_DAY_HELPER,
        ),
        (
            EN_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE,
            JA_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE,
        ),
        (
            EN_ADMIN_COPY_EVENT_RECURRING_PAST,
            JA_ADMIN_COPY_EVENT_RECURRING_PAST,
        ),
        (
            EN_ADMIN_COPY_EVENT_RECURRING_WINDOW,
            JA_ADMIN_COPY_EVENT_RECURRING_WINDOW,
        ),
        (EN_ADMIN_EDIT_EVENT_TITLE, JA_ADMIN_EDIT_EVENT_TITLE),
        (EN_ADMIN_EDIT_EVENT_SUBMIT, JA_ADMIN_EDIT_EVENT_SUBMIT),
        (EN_ADMIN_EDIT_EVENT_HINT, JA_ADMIN_EDIT_EVENT_HINT),
        (
            EN_ADMIN_EDIT_DETAILS_ONLY_HEADING,
            JA_ADMIN_EDIT_DETAILS_ONLY_HEADING,
        ),
        (
            EN_ADMIN_EDIT_SCHEDULE_HEADING,
            JA_ADMIN_EDIT_SCHEDULE_HEADING,
        ),
        (
            EN_ADMIN_EDIT_SCHEDULE_TOTAL_PREFIX,
            JA_ADMIN_EDIT_SCHEDULE_TOTAL_PREFIX,
        ),
        (
            EN_ADMIN_EDIT_SCHEDULE_TOTAL_SUFFIX,
            JA_ADMIN_EDIT_SCHEDULE_TOTAL_SUFFIX,
        ),
        (EN_ADMIN_EDIT_SCHEDULE_FIRST, JA_ADMIN_EDIT_SCHEDULE_FIRST),
        (EN_ADMIN_EDIT_SCHEDULE_LAST, JA_ADMIN_EDIT_SCHEDULE_LAST),
        (
            EN_ADMIN_EDIT_MULTI_DAY_HELPER,
            JA_ADMIN_EDIT_MULTI_DAY_HELPER,
        ),
        (
            EN_ADMIN_EDIT_RECURRING_HELPER,
            JA_ADMIN_EDIT_RECURRING_HELPER,
        ),
        (
            EN_ADMIN_EDIT_RESPONSES_PRESERVED,
            JA_ADMIN_EDIT_RESPONSES_PRESERVED,
        ),
        (
            EN_ADMIN_EDIT_SCHEDULE_NOT_EDITABLE,
            JA_ADMIN_EDIT_SCHEDULE_NOT_EDITABLE,
        ),
        (EN_ADMIN_CANCEL_EVENT_TITLE, JA_ADMIN_CANCEL_EVENT_TITLE),
        (EN_ADMIN_CANCEL_EVENT_BODY, JA_ADMIN_CANCEL_EVENT_BODY),
        (
            EN_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS,
            JA_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS,
        ),
        (EN_ADMIN_CANCEL_EVENT_KEEP, JA_ADMIN_CANCEL_EVENT_KEEP),
        (EN_ADMIN_CANCEL_EVENT_CONFIRM, JA_ADMIN_CANCEL_EVENT_CONFIRM),
        (
            EN_ADMIN_CANCEL_EVENT_CONFIRM_ALL_DAYS,
            JA_ADMIN_CANCEL_EVENT_CONFIRM_ALL_DAYS,
        ),
        (
            EN_ADMIN_CANNOT_EDIT_CANCELLED,
            JA_ADMIN_CANNOT_EDIT_CANCELLED,
        ),
        (EN_ADMIN_CANNOT_EDIT_STARTED, JA_ADMIN_CANNOT_EDIT_STARTED),
        (
            EN_ADMIN_CANNOT_ATTEND_CANCELLED,
            JA_ADMIN_CANNOT_ATTEND_CANCELLED,
        ),
        (EN_ADMIN_ATTEND_TITLE, JA_ADMIN_ATTEND_TITLE),
        (EN_ADMIN_ATTEND_SUBMIT, JA_ADMIN_ATTEND_SUBMIT),
        (EN_ADMIN_INVITES_TITLE, JA_ADMIN_INVITES_TITLE),
        (EN_ADMIN_INVITES_BODY, JA_ADMIN_INVITES_BODY),
        (EN_ADMIN_INVITES_GENERATE, JA_ADMIN_INVITES_GENERATE),
        (EN_ADMIN_INVITES_ACTIVE, JA_ADMIN_INVITES_ACTIVE),
        (EN_ADMIN_INVITES_NONE, JA_ADMIN_INVITES_NONE),
        (
            EN_ADMIN_INVITES_NEW_CODE_HINT,
            JA_ADMIN_INVITES_NEW_CODE_HINT,
        ),
        (EN_ADMIN_INVITES_REVOKE, JA_ADMIN_INVITES_REVOKE),
        (EN_ADMIN_INVITES_REVOKED, JA_ADMIN_INVITES_REVOKED),
        (
            EN_ADMIN_INVITES_BACK_TO_MEMBERS,
            JA_ADMIN_INVITES_BACK_TO_MEMBERS,
        ),
        (EN_ADMIN_MEMBERS_TITLE, JA_ADMIN_MEMBERS_TITLE),
        (
            EN_ADMIN_MEMBERS_GENERATE_INVITE,
            JA_ADMIN_MEMBERS_GENERATE_INVITE,
        ),
        (EN_ADMIN_MEMBERS_CURRENT_USER, JA_ADMIN_MEMBERS_CURRENT_USER),
        (EN_ADMIN_PROMOTE_ACTION, JA_ADMIN_PROMOTE_ACTION),
        (EN_ADMIN_DEMOTE_ACTION, JA_ADMIN_DEMOTE_ACTION),
        (EN_ADMIN_PROMOTE_TITLE, JA_ADMIN_PROMOTE_TITLE),
        (EN_ADMIN_PROMOTE_CONSEQUENCE, JA_ADMIN_PROMOTE_CONSEQUENCE),
        (EN_ADMIN_DEMOTE_TITLE, JA_ADMIN_DEMOTE_TITLE),
        (EN_ADMIN_DEMOTE_CONSEQUENCE, JA_ADMIN_DEMOTE_CONSEQUENCE),
        (EN_ADMIN_LAST_ADMIN_DEMOTE, JA_ADMIN_LAST_ADMIN_DEMOTE),
        (EN_ADMIN_REMOVE_TITLE, JA_ADMIN_REMOVE_TITLE),
        (EN_ADMIN_REMOVE_KEEP, JA_ADMIN_REMOVE_KEEP),
        (EN_ADMIN_REMOVE_CONFIRM, JA_ADMIN_REMOVE_CONFIRM),
        (EN_ADMIN_REMOVE_CONSEQUENCE, JA_ADMIN_REMOVE_CONSEQUENCE),
        (EN_ADMIN_LAST_ADMIN, JA_ADMIN_LAST_ADMIN),
        (EN_ADMIN_HELP_SIGNIN_ACTION, JA_ADMIN_HELP_SIGNIN_ACTION),
        (EN_ADMIN_HELP_SIGNIN_TITLE, JA_ADMIN_HELP_SIGNIN_TITLE),
        (
            EN_ADMIN_HELP_SIGNIN_CONSEQUENCE,
            JA_ADMIN_HELP_SIGNIN_CONSEQUENCE,
        ),
        (EN_ADMIN_HELP_SIGNIN_CREATE, JA_ADMIN_HELP_SIGNIN_CREATE),
        (
            EN_ADMIN_HELP_SIGNIN_CODE_HINT,
            JA_ADMIN_HELP_SIGNIN_CODE_HINT,
        ),
        (EN_RELINK_TITLE, JA_RELINK_TITLE),
        (EN_RELINK_BODY, JA_RELINK_BODY),
        (EN_RELINK_CODE_LABEL, JA_RELINK_CODE_LABEL),
        (EN_RELINK_SUBMIT, JA_RELINK_SUBMIT),
        (EN_RELINK_INVALID, JA_RELINK_INVALID),
        (EN_COMMUNITIES_JOIN_ANOTHER, JA_COMMUNITIES_JOIN_ANOTHER),
        (EN_COMMUNITY_CREATE_LINK, JA_COMMUNITY_CREATE_LINK),
        (EN_COMMUNITY_CREATE_TITLE, JA_COMMUNITY_CREATE_TITLE),
        (EN_COMMUNITY_CREATE_BODY, JA_COMMUNITY_CREATE_BODY),
        (
            EN_COMMUNITY_CREATE_NAME_LABEL,
            JA_COMMUNITY_CREATE_NAME_LABEL,
        ),
        (
            EN_COMMUNITY_CREATE_DISPLAY_NAME_LABEL,
            JA_COMMUNITY_CREATE_DISPLAY_NAME_LABEL,
        ),
        (
            EN_COMMUNITY_CREATE_TIMEZONE_LABEL,
            JA_COMMUNITY_CREATE_TIMEZONE_LABEL,
        ),
        (
            EN_COMMUNITY_CREATE_TIMEZONE_JAPAN,
            JA_COMMUNITY_CREATE_TIMEZONE_JAPAN,
        ),
        (EN_COMMUNITY_CREATE_SUBMIT, JA_COMMUNITY_CREATE_SUBMIT),
        (EN_COMMUNITY_CREATE_CANCEL, JA_COMMUNITY_CREATE_CANCEL),
        (EN_COMMUNITY_CREATE_DISABLED, JA_COMMUNITY_CREATE_DISABLED),
        (
            EN_COMMUNITY_CREATE_RATE_LIMITED,
            JA_COMMUNITY_CREATE_RATE_LIMITED,
        ),
        (
            EN_COMMUNITY_CREATE_NAME_ERROR,
            JA_COMMUNITY_CREATE_NAME_ERROR,
        ),
        (
            EN_COMMUNITY_CREATE_NAME_TOO_LONG,
            JA_COMMUNITY_CREATE_NAME_TOO_LONG,
        ),
        (
            EN_COMMUNITY_CREATE_NAME_INVALID,
            JA_COMMUNITY_CREATE_NAME_INVALID,
        ),
        (
            EN_COMMUNITY_CREATE_DISPLAY_NAME_ERROR,
            JA_COMMUNITY_CREATE_DISPLAY_NAME_ERROR,
        ),
        (
            EN_COMMUNITY_CREATE_TIMEZONE_ERROR,
            JA_COMMUNITY_CREATE_TIMEZONE_ERROR,
        ),
        (EN_ROLE_ADMIN, JA_ROLE_ADMIN),
        (EN_ROLE_MEMBER, JA_ROLE_MEMBER),
        (EN_HOME_FIRST_RUN_WELCOME, JA_HOME_FIRST_RUN_WELCOME),
        (EN_HOME_FIRST_RUN_NO_EVENTS, JA_HOME_FIRST_RUN_NO_EVENTS),
        (EN_HOME_FIRST_RUN_CREATE, JA_HOME_FIRST_RUN_CREATE),
        (EN_HOME_FIRST_RUN_INVITE_HINT, JA_HOME_FIRST_RUN_INVITE_HINT),
        (EN_REPEAT_LABEL, JA_REPEAT_LABEL),
        (EN_REPEAT_NONE, JA_REPEAT_NONE),
        (EN_REPEAT_WEEKLY, JA_REPEAT_WEEKLY),
        (EN_REPEAT_BIWEEKLY, JA_REPEAT_BIWEEKLY),
        (EN_REPEAT_MONTHLY, JA_REPEAT_MONTHLY),
        (EN_REPEAT_COUNT_UNIT, JA_REPEAT_COUNT_UNIT),
        (EN_REPEAT_COUNT_HINT, JA_REPEAT_COUNT_HINT),
        (EN_TEMPLATES_TITLE, JA_TEMPLATES_TITLE),
        (EN_TEMPLATES_DESCRIPTION, JA_TEMPLATES_DESCRIPTION),
        (EN_TEMPLATES_EMPTY, JA_TEMPLATES_EMPTY),
        (EN_TEMPLATES_SAVE_SECTION, JA_TEMPLATES_SAVE_SECTION),
        (EN_TEMPLATES_TITLE_LABEL, JA_TEMPLATES_TITLE_LABEL),
        (EN_TEMPLATES_LOC_LABEL, JA_TEMPLATES_LOC_LABEL),
        (EN_TEMPLATES_DUR_LABEL, JA_TEMPLATES_DUR_LABEL),
        (EN_TEMPLATES_SAVE_BTN, JA_TEMPLATES_SAVE_BTN),
        (EN_TEMPLATES_USE_BTN, JA_TEMPLATES_USE_BTN),
        (EN_TEMPLATES_DELETE_BTN, JA_TEMPLATES_DELETE_BTN),
        (EN_TEMPLATES_USE_LINK, JA_TEMPLATES_USE_LINK),
        (EN_EXPORT_TITLE, JA_EXPORT_TITLE),
        (EN_EXPORT_DESCRIPTION, JA_EXPORT_DESCRIPTION),
        (EN_EXPORT_PRIVACY_NOTE, JA_EXPORT_PRIVACY_NOTE),
        (EN_EXPORT_DOWNLOAD_BTN, JA_EXPORT_DOWNLOAD_BTN),
        (EN_EXPORT_SINGLE_USE, JA_EXPORT_SINGLE_USE),
        (EN_ME_SECTION_ABOUT, JA_ME_SECTION_ABOUT),
        (EN_ME_VERSION_LABEL, JA_ME_VERSION_LABEL),
        (EN_ME_REF_LABEL, JA_ME_REF_LABEL),
        (EN_ME_SECTION_DATA, JA_ME_SECTION_DATA),
        (EN_ME_EXPORT_LINK, JA_ME_EXPORT_LINK),
        (EN_ME_SECTION_ADMIN, JA_ME_SECTION_ADMIN),
        (EN_ME_MANAGE_MEMBERS, JA_ME_MANAGE_MEMBERS),
        (EN_CALENDAR_TITLE, JA_CALENDAR_TITLE),
        (EN_CALENDAR_DESCRIPTION, JA_CALENDAR_DESCRIPTION),
        (EN_CALENDAR_GENERATE, JA_CALENDAR_GENERATE),
        (EN_CALENDAR_DISABLE, JA_CALENDAR_DISABLE),
        (EN_CALENDAR_REGENERATE, JA_CALENDAR_REGENERATE),
        (EN_CALENDAR_PRIVACY_NOTE, JA_CALENDAR_PRIVACY_NOTE),
        (EN_CALENDAR_GENERATED_FLASH, JA_CALENDAR_GENERATED_FLASH),
        (EN_CALENDAR_REVOKED_FLASH, JA_CALENDAR_REVOKED_FLASH),
        (EN_EVENT_TITLE_HEADER, JA_EVENT_TITLE_HEADER),
        (EN_EVENT_ATTENDED_UNAVAILABLE, JA_EVENT_ATTENDED_UNAVAILABLE),
        (EN_EVENT_ATTENDED_ADMIN_ONLY, JA_EVENT_ATTENDED_ADMIN_ONLY),
        (EN_EVENT_MEMBER_FALLBACK, JA_EVENT_MEMBER_FALLBACK),
        (EN_JOIN_PAGE_TITLE, JA_JOIN_PAGE_TITLE),
        (EN_JOIN_PROFILE_PAGE_TITLE, JA_JOIN_PROFILE_PAGE_TITLE),
        // Added in v0.33.x — EN→JA inline string sweep
        (EN_NOT_FOUND, JA_NOT_FOUND),
        (EN_INTERNAL_ERROR, JA_INTERNAL_ERROR),
        (EN_ADMIN_ATTEND_CANCELLED, JA_ADMIN_ATTEND_CANCELLED),
        (EN_GENERAL_BACK, JA_GENERAL_BACK),
        (EN_ADMIN_EDIT_CANCELLED, JA_ADMIN_EDIT_CANCELLED),
        (EN_ADMIN_EDIT_STARTED, JA_ADMIN_EDIT_STARTED),
        (EN_NAV_BACK, JA_NAV_BACK),
        (EN_NAV_SWITCH_GO, JA_NAV_SWITCH_GO),
        (EN_NOTE_DELETE_BODY, JA_NOTE_DELETE_BODY),
        (EN_NOTE_KEEP_ACTION, JA_NOTE_KEEP_ACTION),
        (EN_FORM_FIELD_TITLE, JA_FORM_FIELD_TITLE),
        (EN_FORM_FIELD_DATE, JA_FORM_FIELD_DATE),
        (EN_FORM_FIELD_START, JA_FORM_FIELD_START),
        (EN_FORM_FIELD_END, JA_FORM_FIELD_END),
        (EN_FORM_FIELD_LOCATION, JA_FORM_FIELD_LOCATION),
        (EN_FORM_FIELD_DESC, JA_FORM_FIELD_DESC),
        (EN_EVENT_CANCELLED_BADGE, JA_EVENT_CANCELLED_BADGE),
        (EN_EVENT_WHOS_GOING, JA_EVENT_WHOS_GOING),
        (EN_EVENT_NOTES_SECTION, JA_EVENT_NOTES_SECTION),
        (EN_TZ_ERROR, JA_TZ_ERROR),
        (EN_CURRENT_BADGE, JA_CURRENT_BADGE),
        (EN_ME_CALENDAR_LABEL, JA_ME_CALENDAR_LABEL),
        (EN_ME_DATA_EXPORT, JA_ME_DATA_EXPORT),
    ];
    // Strings that are intentionally identical across languages (product name,
    // numeric units, etc.) are exempted from the identity check.
    const INTENTIONALLY_IDENTICAL: &[&str] = &["ciao.zinnias"];

    for (en, ja) in pairs {
        assert!(!en.is_empty(), "EN string empty");
        assert!(!ja.is_empty(), "JA string empty for EN: {en}");
        if !INTENTIONALLY_IDENTICAL.contains(&en) {
            assert_ne!(en, ja, "EN and JA are identical (likely copy-paste): {en}");
        }
    }
}

#[test]
fn rfc054_member_facing_japanese_copy_avoids_technical_jargon() {
    use zinnias_ciao_contracts::i18n::*;

    let reviewed = [
        JA_SESSION_EXPIRED,
        JA_STATUS_GOING,
        JA_STATUS_NOT_GOING,
        JA_STATUS_ATTENDED,
        JA_STATUS_NO_ANSWER,
        JA_STATUS_CLEAR,
        JA_STATUS_CLEAR_LABEL,
        JA_CALENDAR_TITLE,
        JA_CALENDAR_DESCRIPTION,
        JA_CALENDAR_GENERATE,
        JA_CALENDAR_DISABLE,
        JA_CALENDAR_REGENERATE,
        JA_CALENDAR_PRIVACY_NOTE,
        JA_CALENDAR_GENERATED_FLASH,
        JA_CALENDAR_REVOKED_FLASH,
        JA_ME_CALENDAR_LABEL,
        JA_EXPORT_TITLE,
        JA_EXPORT_DESCRIPTION,
        JA_EXPORT_PRIVACY_NOTE,
        JA_EXPORT_DOWNLOAD_BTN,
        JA_ME_EXPORT_LINK,
        JA_ME_DATA_EXPORT,
    ];
    let forbidden = [
        "セッション",
        "トークン",
        "HMAC",
        "ICS",
        "iCS",
        "webcal",
        "JSON",
        "エクスポート",
    ];

    for text in reviewed {
        for term in forbidden {
            assert!(
                !text.contains(term),
                "RFC-054 Japanese member-facing copy contains technical jargon {term:?}: {text}"
            );
        }
    }
}

// ── D1 query budget documentation (RFC-029 / RFC-044) ────────────────────
//
// These constants document the approved D1 operation budget per route.
// The values are *code-level* counts (DB calls + form-token issues in the
// hot paths). They serve as a regression guard: if a future change inflates
// the count, the constant must be updated here with a deliberate review.
//
// All loop-based N+1s that existed before v0.24.0 are eliminated:
//   - Event Detail: list_for_day replaced with list_for_event_days (IN batch)
//   - Event Detail: per-note admin token loop replaced with a confirm-page link
//   - Export: per-event days+attendance+notes replaced with 3 IN queries
//
// The remaining per-day SET_STATUS token issue in Event Detail is bounded:
// single-day events = 1 token issue; recurring events bounded by
// After RFC-046 (event-bound SET_STATUS token), Event Detail issues exactly
// one token regardless of day count. The max-recurring budget collapses to
// the same value as single-day: 13 ops for any event.

/// Fixed D1 queries for Home (no loops above 1 per route):
/// memberships, events, member_count, my_statuses (IN), counts (IN),
/// communities_for_switcher + 2 spares = 8 total.
const QUERY_BUDGET_HOME: usize = 8;

/// Fixed D1 ops for Event Detail — any event, any recurrence count (RFC-046):
/// find_event, days, member_count, my_note, all_notes, all_members,
/// community, my_statuses (IN), counts (IN), all_day_attendances (IN),
/// 1 SET_STATUS token issue (event-bound, not per-day), 1 SAVE_NOTE token issue,
/// communities_for_switcher = 13 total.
/// Before RFC-046, max-recurring was 65 (52 per-day token writes). Now flat.
const QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY: usize = 13;
const QUERY_BUDGET_EVENT_DETAIL_MAX_RECURRING: usize = 13; // same: 1 token regardless of days

/// D1 queries for Export (any community size): 5 fixed + 3 IN batches = 8.
/// Was O(events * days) before v0.25.0; now a flat 8 regardless of size.
const QUERY_BUDGET_EXPORT: usize = 8;

#[test]
fn query_budgets_are_positive_and_ordered() {
    assert!(QUERY_BUDGET_HOME > 0);
    assert!(QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY > QUERY_BUDGET_HOME);
    // After RFC-046 both single-day and max-recurring are identical (13).
    assert_eq!(
        QUERY_BUDGET_EVENT_DETAIL_MAX_RECURRING, QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY,
        "RFC-046: event-bound token makes recurring cost identical to single-day"
    );
    assert!(QUERY_BUDGET_EXPORT > 0);
    // Export must be flat (well under the old per-event worst case):
    assert!(
        QUERY_BUDGET_EXPORT < 20,
        "Export budget {QUERY_BUDGET_EXPORT} exceeds expected flat upper bound"
    );
    // Event detail must be well under the old per-day worst case of 65:
    assert!(
        QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY < 20,
        "Event detail budget {QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY} suggests an N+1 regression"
    );
    assert!(
        QUERY_BUDGET_EVENT_DETAIL_MAX_RECURRING < 20,
        "Event detail recurring budget suggests an N+1 regression"
    );
}

// ── Static source query-count gates (RFC-044 §6.1) ───────────────────────
//
// Count `.await` calls on DB functions in the key handler source files and
// assert they don't regress above their declared budgets. Uses include_str! so
// the check fires on every `cargo test` run without a live database.
//
// The counting heuristic: lines containing `.await` in a handler are almost
// always D1 operations; non-DB awaits (form_data(), etc.) are few and counted
// conservatively. The gate fires if the count exceeds 2× the budget — tight
// enough to catch a major N+1 regression but loose enough to survive minor
// refactors without constant adjustment. A count approaching the 2× ceiling
// should trigger manual budget review.

const HOME_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/home.rs");
const EVENT_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/event.rs");
const EXPORT_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/export.rs");
const CALENDAR_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/calendar.rs");
const COMMUNITY_CREATE_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/community_create.rs");
const ME_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/me.rs");
const LIB_SRC: &str = include_str!("../../../workers/ssr/src/lib.rs");
const AUTHZ_SRC: &str = include_str!("../../../workers/ssr/src/authz.rs");
const RATE_LIMIT_SRC: &str = include_str!("../../../workers/ssr/src/rate_limit.rs");
const CALENDAR_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/calendar.rs");
const COMMUNITY_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/community.rs");
const EVENT_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/event.rs");
const ICS_SRC: &str = include_str!("../../../packages/contracts/src/ics.rs");
const WRANGLER_TOML_SRC: &str = include_str!("../../../wrangler.toml");
const GITIGNORE_SRC: &str = include_str!("../../../.gitignore");
const MIGRATION_0009_SRC: &str = include_str!("../../../migrations/0009_recurrence_v2.sql");
const EVENT_SERIES_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/event_series.rs");
const EVENT_ADMIN_DOMAIN_SRC: &str = include_str!("../../../packages/domain/src/event_admin.rs");

#[test]
fn tracked_wrangler_template_contains_only_placeholder_resource_ids() {
    let mut checked = 0usize;

    for (idx, line) in WRANGLER_TOML_SRC.lines().enumerate() {
        let content = line.split('#').next().unwrap_or("").trim();
        let key = if content.starts_with("database_id") {
            Some("D1 database_id")
        } else if content.starts_with("id") && content.contains('=') {
            Some("KV namespace id")
        } else {
            None
        };

        let Some(key) = key else {
            continue;
        };
        let value = content
            .split_once('=')
            .map(|(_, value)| value.trim().trim_matches('"'))
            .expect("wrangler resource id line must use key = value syntax");
        checked += 1;

        assert!(
            value == "local" || value.starts_with("REPLACE_WITH_"),
            "tracked wrangler.toml line {} contains a real {key} value ({value:?}); \
             keep real hosted D1/KV IDs in ignored wrangler*.local.toml files",
            idx + 1
        );
    }

    assert!(
        checked >= 6,
        "release gate expected to inspect top-level, dev, staging, and production D1/KV ids"
    );
}

#[test]
fn local_wrangler_configs_remain_ignored() {
    let required_patterns = ["wrangler.*.local.toml", "wrangler.local.toml"];

    for pattern in required_patterns {
        assert!(
            GITIGNORE_SRC.lines().any(|line| line.trim() == pattern),
            ".gitignore must keep {pattern:?} so real hosted D1/KV IDs stay out of Git"
        );
    }
}

#[test]
fn rfc065_legacy_migration_does_not_treat_utc_clock_as_local_time() {
    assert!(
        !MIGRATION_0009_SRC.contains("substr(first_day.starts_at_utc")
            && !MIGRATION_0009_SRC.contains("substr(first_day.ends_at_utc"),
        "RFC-065 migration must not backfill local recurrence times from UTC clock text"
    );
    assert!(
        MIGRATION_0009_SRC.contains("future materialization is disabled")
            && MIGRATION_0009_SRC.contains("NULL,\n    NULL,"),
        "RFC-065 legacy series must use explicit null local times when safe local clocks are unavailable"
    );
}

#[test]
fn rfc065_exception_shape_is_checked_by_database() {
    assert!(
        MIGRATION_0009_SRC.contains("action = 'skip' AND event_day_id IS NULL")
            && MIGRATION_0009_SRC.contains("action = 'cancel' AND event_day_id IS NOT NULL"),
        "RFC-065 exception table must enforce skip/cancel event_day_id shape"
    );
}

#[test]
fn rfc065_materialization_uses_after_date_and_shared_request_budget() {
    assert!(
        EVENT_ADMIN_DOMAIN_SRC.contains("generate_recurrence_occurrences_after")
            && EVENT_ADMIN_DOMAIN_SRC.contains("after_day_date"),
        "RFC-065 domain generator must support rolling materialization after existing dates"
    );
    assert!(
        EVENT_SERIES_DB_SRC.contains("let mut remaining = RECURRENCE_MATERIALIZATION_INSERT_CAP")
            && EVENT_SERIES_DB_SRC
                .contains("materialize_series(db, &row, through_day_date, remaining)")
            && EVENT_SERIES_DB_SRC
                .contains("remaining = remaining.saturating_sub(report.inserted)"),
        "RFC-065 community materialization must enforce one shared insert budget per request"
    );
    assert!(
        EVENT_SERIES_DB_SRC.contains("generate_recurrence_occurrences_after")
            && EVENT_SERIES_DB_SRC.contains("previous_materialized"),
        "RFC-065 materializer must generate after materialized_through instead of replaying the first capped batch"
    );
}

/// Count non-comment lines containing `.await` in a source string.
fn count_awaits(src: &str) -> usize {
    src.lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("//") && t.contains(".await")
        })
        .count()
}

#[test]
fn home_handler_await_count_within_budget() {
    // Home handler awaits: require_auth (session), list_active_for_user (community
    // switcher route), require_membership, home_upcoming, list_active_for_user
    // (switcher), count_active, find_active (community), list_mine_for_days,
    // counts_for_days, list_communities_for_user.  Total ≈ 10-11 DB awaits.
    // Gate: must not exceed 2 × budget.
    let awaits = count_awaits(HOME_HANDLER_SRC);
    assert!(
        awaits <= QUERY_BUDGET_HOME * 2,
        "home.rs has {awaits} .await calls, exceeds 2× budget ({}).\
         Investigate for N+1 regression.",
        QUERY_BUDGET_HOME * 2
    );
}

#[test]
fn event_detail_handler_await_count_within_budget() {
    // Event detail GET awaits: require_auth, require_membership, find_for_community,
    // days_for_event, count_active, find_mine (note), list_for_event (notes),
    // list_all_active (members), find_active (community), list_mine_for_days,
    // counts_for_days (IN), list_for_event_days (IN), issue token (×2 SET_STATUS +
    // SAVE_NOTE), list_communities_for_user.  ~13 DB awaits for the GET handler.
    // The full file also contains POST handlers; total awaits will be higher.
    // Gate: file total must not regress into obviously N+1 territory (> 50).
    let awaits = count_awaits(EVENT_HANDLER_SRC);
    assert!(
        awaits <= 50,
        "event.rs has {awaits} .await calls total across all handlers.\
         Investigate if event detail GET alone exceeds {QUERY_BUDGET_EVENT_DETAIL_SINGLE_DAY}."
    );
}

#[test]
fn export_handler_await_count_within_budget() {
    // export.rs contains three handlers (page, JSON download, token/revoke) plus
    // the build_export helper. The per-route budget is 8 flat IN-batched queries.
    // With ~3 handlers + helper, the file-level ceiling is 30 to catch a
    // clear N+1 regression while allowing normal multi-handler structure.
    // The important invariant (batched IN queries, no per-row fetch) is documented
    // in QUERY_BUDGET_EXPORT and enforced via code review; a live harness (RFC-044)
    // will provide the precise per-route assertion when staging is available.
    let awaits = count_awaits(EXPORT_HANDLER_SRC);
    assert!(
        awaits <= 30,
        "export.rs has {awaits} .await calls across all handlers, exceeds ceiling (30).\
         Investigate for N+1 regression — the export route must use batched IN queries."
    );
}

// ── Service worker version gate (RFC-044 §11 step 1) ─────────────────────
//
// sw.js CACHE_VERSION must equal the package version at every release.
// A mismatch means the service worker will not invalidate old caches on deploy.
//
// This test reads both files at test time using include_str! so it fires on
// every `cargo test` run without any external tooling.

const SW_JS_SOURCE: &str = include_str!("../../../workers/ssr/static/sw.js");
const APP_JS_SOURCE: &str = include_str!("../../../workers/ssr/static/app.js");
const WORKSPACE_CARGO_TOML: &str = include_str!("../../../Cargo.toml");

#[test]
fn sw_cache_version_matches_workspace_version() {
    // Extract CACHE_VERSION from sw.js:  const CACHE_VERSION = 'vX.Y.Z';
    let cache_ver = SW_JS_SOURCE
        .lines()
        .find(|l| l.trim_start().starts_with("const CACHE_VERSION"))
        .and_then(|l| {
            // e.g.  const CACHE_VERSION = 'v0.25.0';
            let after_eq = l.split_once('=')?.1;
            let inner = after_eq
                .trim()
                .trim_start_matches('\'')
                .trim_end_matches(';')
                .trim_end_matches('\'');
            // Strip the leading 'v'
            inner.strip_prefix('v')
        })
        .expect("CACHE_VERSION not found in sw.js");

    // Extract version from [workspace.package] block in Cargo.toml.
    // Find the version line that follows the [workspace.package] header.
    let workspace_ver = {
        let mut in_workspace_pkg = false;
        let mut found = None;
        for line in WORKSPACE_CARGO_TOML.lines() {
            let trimmed = line.trim();
            if trimmed == "[workspace.package]" {
                in_workspace_pkg = true;
                continue;
            }
            if in_workspace_pkg {
                if trimmed.starts_with('[') {
                    break; // left the [workspace.package] section
                }
                if trimmed.starts_with("version") {
                    // version     = "0.25.0"
                    found = trimmed
                        .split_once('=')
                        .map(|(_, v)| v.trim().trim_matches('"').to_owned());
                    break;
                }
            }
        }
        found.expect("workspace version not found in Cargo.toml")
    };

    assert_eq!(
        cache_ver, workspace_ver,
        "sw.js CACHE_VERSION 'v{cache_ver}' does not match workspace version '{workspace_ver}'. \
         Update sw.js CACHE_VERSION when bumping the version."
    );
}

// ── Japanese-only rendered-text gate (RFC-049) ───────────────────────────
//
// The pilot ships Japanese UI only. English words leaked into rendered link
// and button text twice in v0.35.x (event-detail "← Home", communities
// "Invite members" / "Manage members"). These were inline literals, not i18n
// constants, so the i18n parity gate did not catch them.
//
// This gate scans the handler/render sources for the specific regressions that
// occurred and a few obvious English UI words appearing as element text. It is
// deliberately narrow: it matches ">Word</a>" or ">Word</button>" shapes with a
// known English UI vocabulary, not arbitrary English (comments, code, ARIA
// values, and HTTP header literals must remain unflagged).

const COMMUNITIES_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/communities.rs");
const COMMUNITIES_MATRIX_SRC: &str = concat!(
    include_str!("../../../workers/ssr/src/handlers/communities/matrix.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix/cells.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix/detail.rs")
);
const COMMUNITIES_SRC: &str = concat!(
    include_str!("../../../workers/ssr/src/handlers/communities.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/calendar.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/calendar/events.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix/cells.rs"),
    include_str!("../../../workers/ssr/src/handlers/communities/matrix/detail.rs")
);
const COMMUNITY_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/community.rs");
const ADMIN_EVENTS_SRC: &str = concat!(
    include_str!("../../../workers/ssr/src/handlers/admin/events.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/attendance.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/cancel.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/copy.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/create.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/edit.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/forms.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/policy.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/recreate.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/summary.rs"),
    include_str!("../../../workers/ssr/src/handlers/admin/events/support.rs"),
);
const ADMIN_EVENTS_COPY_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/events/copy.rs");
const ROLE_TRANSFER_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/role_transfer.rs");
const MEMBER_REMOVE_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/member_remove.rs");
const HELP_SIGNIN_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/help_signin.rs");
const RELINK_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/relink.rs");
const MEMBERSHIP_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/membership.rs");
const RELINK_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/relink.rs");
const SESSION_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/session.rs");
const APP_JS_SRC: &str = include_str!("../../../workers/ssr/static/app.js");
const RENDER_SRC: &str = concat!(
    include_str!("../../../workers/ssr/src/render.rs"),
    include_str!("../../../workers/ssr/src/render/errors.rs"),
    include_str!("../../../workers/ssr/src/render/event_card.rs"),
    include_str!("../../../workers/ssr/src/render/nav.rs"),
    include_str!("../../../workers/ssr/src/render/notes.rs"),
    include_str!("../../../workers/ssr/src/render/participants.rs"),
    include_str!("../../../workers/ssr/src/render/shell.rs"),
    include_str!("../../../workers/ssr/src/render/status.rs"),
    include_str!("../../../workers/ssr/src/render/time.rs"),
);
const STATIC_FILES_SRC: &str = include_str!("../../../workers/ssr/src/handlers/static_files.rs");

#[test]
fn no_known_english_ui_leaks_in_rendered_text() {
    // Exact regressions that previously shipped — keep them from returning.
    let forbidden: &[&str] = &[
        ">Invite members<",
        ">Manage members<",
        "\u{2190} Home<", // "← Home" — must be "← ホーム"
        ">Home</a>",
        ">Members</a>",
        ">Go</button>", // bare English fallback button (use JA)
    ];
    for src in [
        EVENT_HANDLER_SRC,
        COMMUNITIES_SRC,
        RENDER_SRC,
        HOME_HANDLER_SRC,
        COMMUNITY_CREATE_HANDLER_SRC,
        MEMBERS_HANDLER_SRC,
        ROLE_TRANSFER_HANDLER_SRC,
        MEMBER_REMOVE_HANDLER_SRC,
    ] {
        for needle in forbidden {
            assert!(
                !src.contains(needle),
                "English UI text leaked into rendered output: {needle:?}. \
                 Pilot is Japanese-only (RFC-049) — use a JA_* i18n constant."
            );
        }
    }
}

#[test]
fn rfc061_member_management_is_discoverable_from_admin_workflows() {
    assert!(
        HOME_HANDLER_SRC.contains("/c/{cid}/admin/members")
            && HOME_HANDLER_SRC.contains("JA_HOME_MANAGE_MEMBERS")
            && !HOME_HANDLER_SRC.contains("invite_label = i18n::JA_HOME_INVITE_MEMBERS"),
        "RFC-061 Home admin shortcut must lead to member management, not directly to invite codes"
    );
    assert!(
        ME_HANDLER_SRC.contains("JA_ME_SECTION_ADMIN")
            && ME_HANDLER_SRC.contains("JA_ME_MANAGE_MEMBERS")
            && ME_HANDLER_SRC.contains("/c/{cid}/admin/members")
            && ME_HANDLER_SRC.contains("/c/{cid}/admin/export"),
        "RFC-061 Me page must expose admin tools with member management and export"
    );
    assert!(
        MEMBERS_HANDLER_SRC.contains("JA_ADMIN_INVITES_BACK_TO_MEMBERS")
            && MEMBERS_HANDLER_SRC.contains("JA_ADMIN_MEMBERS_GENERATE_INVITE")
            && MEMBERS_HANDLER_SRC.contains("JA_ADMIN_MEMBERS_CURRENT_USER")
            && !MEMBERS_HANDLER_SRC.contains("Generate invite code</a>"),
        "RFC-061 members/invites pages must use reviewed JA copy and link invites back to members"
    );
}

#[test]
fn rfc061_admin_switch_targets_require_admin_role() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("fn is_admin_target")
            && COMMUNITY_HANDLER_SRC.contains("m.role == \"admin\"")
            && COMMUNITY_HANDLER_SRC.contains("Some(\"admin_members\") if is_admin_target")
            && COMMUNITY_HANDLER_SRC.contains("Some(\"admin_invites\") if is_admin_target")
            && COMMUNITY_HANDLER_SRC.contains("Some(\"admin_events_new\") if is_admin_target")
            && MEMBERS_HANDLER_SRC.contains("\"admin_members\"")
            && MEMBERS_HANDLER_SRC.contains("\"admin_invites\""),
        "RFC-061 admin switch targets must preserve admin pages only for target communities where the user is admin"
    );
}

#[test]
fn rfc062_role_transfer_uses_guarded_member_management_flow() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("\"promote\"")
            && COMMUNITY_HANDLER_SRC.contains("get_promote_member")
            && COMMUNITY_HANDLER_SRC.contains("post_promote_member")
            && COMMUNITY_HANDLER_SRC.contains("\"demote\"")
            && COMMUNITY_HANDLER_SRC.contains("get_demote_member")
            && COMMUNITY_HANDLER_SRC.contains("post_demote_member"),
        "RFC-062 promote/demote routes must be registered explicitly"
    );
    assert!(
        ROLE_TRANSFER_HANDLER_SRC.contains("token_purpose::PROMOTE_MEMBER")
            && ROLE_TRANSFER_HANDLER_SRC.contains("token_purpose::DEMOTE_MEMBER")
            && ROLE_TRANSFER_HANDLER_SRC.contains("JA_ADMIN_PROMOTE_ACTION")
            && ROLE_TRANSFER_HANDLER_SRC.contains("JA_ADMIN_DEMOTE_ACTION")
            && ROLE_TRANSFER_HANDLER_SRC
                .contains("target_membership_id == membership.membership_id"),
        "RFC-062 handlers must use dedicated token purposes, reviewed copy, and server-side self-target denial"
    );
    assert!(
        ROLE_TRANSFER_HANDLER_SRC.contains("membership.promoted_to_admin")
            && ROLE_TRANSFER_HANDLER_SRC.contains("membership.demoted_to_member")
            && ROLE_TRANSFER_HANDLER_SRC.contains("None,"),
        "RFC-062 role changes must audit direction by action name without extra metadata"
    );
}

#[test]
fn rfc062_role_transfer_writes_are_scoped_and_guarded() {
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn promote_to_admin")
            && MEMBERSHIP_DB_SRC.contains("SET role = 'admin'")
            && MEMBERSHIP_DB_SRC.contains("id = ?1")
            && MEMBERSHIP_DB_SRC.contains("community_id = ?2")
            && MEMBERSHIP_DB_SRC.contains("removed_at IS NULL")
            && MEMBERSHIP_DB_SRC.contains("role = 'member'"),
        "RFC-062 promote update must be scoped by membership id, community id, active membership, and current role"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn demote_to_member")
            && MEMBERSHIP_DB_SRC.contains("SET role = 'member'")
            && MEMBERSHIP_DB_SRC.contains("role = 'admin'")
            && MEMBERSHIP_DB_SRC.contains("SELECT COUNT(*) FROM community_memberships")
            && MEMBERSHIP_DB_SRC.contains("> 1"),
        "RFC-062 demote update must re-check active admin count in the conditional write"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn soft_remove_guarded")
            && MEMBERSHIP_DB_SRC.contains("role != 'admin'")
            && MEMBERSHIP_DB_SRC.contains("SELECT COUNT(*) FROM community_memberships")
            && MEMBER_REMOVE_HANDLER_SRC.contains("soft_remove_guarded")
            && !MEMBER_REMOVE_HANDLER_SRC.contains("soft_remove(&db, target_membership_id"),
        "RFC-062 must retrofit member removal to use the guarded admin-count-preserving update"
    );
}

#[test]
fn rfc062_admin_invites_remain_member_role_only() {
    let insert_start = MEMBERS_HANDLER_SRC
        .find("invite_db::insert(")
        .expect("invite insert call should exist");
    let insert_end = MEMBERS_HANDLER_SRC[insert_start..]
        .find(".await?;")
        .map(|offset| insert_start + offset)
        .expect("invite insert await should exist");
    let invite_insert = &MEMBERS_HANDLER_SRC[insert_start..insert_end];
    assert!(
        invite_insert.contains("\"member\",") && !invite_insert.contains("\"admin\","),
        "RFC-062 keeps admin-granting invite codes out of the UI; generated invites must grant member role"
    );
}

#[test]
fn rfc063_removal_only_policy_is_locked() {
    use zinnias_ciao_contracts::i18n::*;

    assert_eq!(JA_ADMIN_REMOVE_CONFIRM, "メンバーから外す");
    assert!(
        JA_ADMIN_REMOVE_CONSEQUENCE.contains("残ります")
            && EN_ADMIN_REMOVE_CONSEQUENCE
                .to_ascii_lowercase()
                .contains("remain"),
        "RFC-063 removal copy must say access ends and past records remain in both locales"
    );

    for (label, src) in [
        ("members handler", MEMBERS_HANDLER_SRC),
        ("member remove handler", MEMBER_REMOVE_HANDLER_SRC),
        ("role transfer handler", ROLE_TRANSFER_HANDLER_SRC),
        ("community router", COMMUNITY_HANDLER_SRC),
    ] {
        let lowered = src.to_ascii_lowercase();
        for forbidden in ["reactivate", "suspend", "restore"] {
            assert!(
                !lowered.contains(forbidden),
                "RFC-063 Option A must not expose {forbidden:?} in {label}"
            );
        }
    }
}

#[test]
fn rfc063_readd_uses_new_identity_without_display_name_merge() {
    assert!(
        JOIN_HANDLER_SRC.contains("let user_id = crate::crypto::random_token();")
            && JOIN_HANDLER_SRC.contains("let membership_id = crate::crypto::random_token();")
            && JOIN_HANDLER_SRC.contains("membership_db::insert_user(&db, &user_id)")
            && JOIN_HANDLER_SRC.contains("membership_db::insert_membership("),
        "RFC-063 Option A requires invite redemption to create a fresh user and membership"
    );
    assert!(
        !JOIN_HANDLER_SRC.contains("WHERE display_name")
            && !JOIN_HANDLER_SRC.contains("display_name = ?")
            && !JOIN_HANDLER_SRC.contains("find_by_display_name"),
        "RFC-063 must not re-add or merge memberships by display name"
    );
}

#[test]
fn rfc063_active_member_queries_exclude_removed_members() {
    let list_start = MEMBERSHIP_DB_SRC
        .find("pub async fn list_all_active")
        .expect("list_all_active should exist");
    let list_end = MEMBERSHIP_DB_SRC[list_start..]
        .find("pub async fn find_active_summary")
        .map(|offset| list_start + offset)
        .expect("find_active_summary should follow list_all_active");
    let list_all_active = &MEMBERSHIP_DB_SRC[list_start..list_end];
    assert!(
        list_all_active.contains("removed_at IS NULL"),
        "RFC-063 active member list must exclude removed memberships"
    );

    let find_start = MEMBERSHIP_DB_SRC
        .find("pub async fn find_active(")
        .expect("find_active should exist");
    let find_end = MEMBERSHIP_DB_SRC[find_start..]
        .find("pub async fn find_active_by_id")
        .map(|offset| find_start + offset)
        .expect("find_active_by_id should follow find_active");
    let find_active = &MEMBERSHIP_DB_SRC[find_start..find_end];
    assert!(
        find_active.contains("removed_at IS NULL"),
        "RFC-063 active authorization lookup must exclude removed memberships"
    );
}

#[test]
fn rfc024_help_signin_copy_and_ttl_are_locked() {
    use zinnias_ciao_contracts::i18n::*;

    assert_eq!(RELINK_CODE_TTL_SECONDS, 15 * 60);
    assert_eq!(JA_ADMIN_HELP_SIGNIN_ACTION, "サインインを手伝う");
    assert_eq!(EN_ADMIN_HELP_SIGNIN_ACTION, "Help sign in again");
    assert_eq!(
        JA_RELINK_INVALID,
        "このコードは無効か、有効期限が切れています。"
    );
    assert_eq!(EN_RELINK_INVALID, "This code is invalid or has expired.");

    for (label, src) in [
        ("help-signin handler", HELP_SIGNIN_HANDLER_SRC),
        ("relink handler", RELINK_HANDLER_SRC),
        ("community router", COMMUNITY_HANDLER_SRC),
    ] {
        let lowered = src.to_ascii_lowercase();
        for forbidden in ["reactivate", "suspend", "restore"] {
            assert!(
                !lowered.contains(forbidden),
                "RFC-024 help-signin surface must not expose {forbidden:?} in {label}"
            );
        }
    }
}

#[test]
fn rfc024_relink_codes_are_membership_scoped_hmacs() {
    assert!(
        RELINK_DB_SRC.contains("membership_relink_codes")
            && RELINK_DB_SRC.contains("code_hmac")
            && RELINK_DB_SRC.contains("community_id")
            && RELINK_DB_SRC.contains("membership_id")
            && RELINK_DB_SRC.contains("created_by_membership_id"),
        "RFC-024 relink code table access must keep HMAC code, community, target membership, and creator membership fields"
    );
    assert!(
        RELINK_DB_SRC.contains("HMAC")
            || HELP_SIGNIN_HANDLER_SRC.contains("hmac_hex(&crate::crypto::pepper(env)")
            || HELP_SIGNIN_HANDLER_SRC.contains("hmac_hex(&pepper"),
        "RFC-024 codes must be HMAC hashed before storage"
    );
    assert!(
        RELINK_DB_SRC.contains("revoke_unused_for_membership")
            && RELINK_DB_SRC.contains("revoked_at = ?1")
            && HELP_SIGNIN_HANDLER_SRC.contains("revoke_unused_for_membership"),
        "RFC-024 must revoke prior unused codes when creating a new code for the same membership"
    );
}

#[test]
fn rfc024_redemption_rechecks_active_membership_and_community() {
    assert!(
        RELINK_DB_SRC.contains("JOIN community_memberships m ON m.id = r.membership_id")
            && RELINK_DB_SRC.contains("m.removed_at IS NULL")
            && RELINK_DB_SRC.contains("m.community_id = r.community_id")
            && RELINK_DB_SRC.contains("m.user_id"),
        "RFC-024 redemption must resolve membership_id to user_id and re-check active community membership"
    );
    assert!(
        !RELINK_DB_SRC.contains("display_name")
            && !RELINK_HANDLER_SRC.contains("display_name")
            && !HELP_SIGNIN_HANDLER_SRC.contains("WHERE display_name"),
        "RFC-024 must not recover or merge by display name"
    );
    assert!(
        JOIN_HANDLER_SRC.contains("let user_id = crate::crypto::random_token();")
            && JOIN_HANDLER_SRC.contains("let membership_id = crate::crypto::random_token();")
            && JOIN_HANDLER_SRC.contains("membership_db::insert_membership("),
        "RFC-024 invite-era help-signin relies on join minting a fresh user_id and membership per invite redemption"
    );
}

#[test]
fn rfc024_redemption_is_single_use_generic_and_revokes_old_sessions() {
    assert!(
        RELINK_DB_SRC.contains("pub async fn mark_used")
            && RELINK_DB_SRC.contains("used_at IS NULL")
            && RELINK_HANDLER_SRC.contains("mark_used"),
        "RFC-024 redemption must mark codes used with a conditional single-use update"
    );
    assert!(
        RELINK_HANDLER_SRC.contains("JA_RELINK_INVALID")
            && !RELINK_HANDLER_SRC.contains("already used")
            && !RELINK_HANDLER_SRC.contains("wrong community"),
        "RFC-024 redemption failures must use one generic error"
    );
    assert!(
        SESSION_DB_SRC.contains("pub async fn revoke_others_for_user")
            && RELINK_HANDLER_SRC.contains("revoke_others_for_user")
            && SESSION_DB_SRC.contains("id != ?3"),
        "RFC-024 redemption must revoke other active sessions for the target user after inserting the new session"
    );
    let audit_write_count = RELINK_HANDLER_SRC.matches("audit::write(").count();
    assert!(
        RELINK_HANDLER_SRC.contains("rate_limit::is_relink_rate_limited")
            && RELINK_HANDLER_SRC.contains("record_relink_failure")
            && audit_write_count == 1
            && RELINK_HANDLER_SRC.contains("\"membership.relink_redeemed\""),
        "RFC-024 failed redemption should be rate-limited, not audited as a membership event"
    );
}

#[test]
fn rfc057_community_creation_is_guarded_active_admin_only() {
    assert!(
        LIB_SRC.contains("(Method::Get, \"/communities/new\")")
            && LIB_SRC.contains("(Method::Post, \"/communities/new\")"),
        "RFC-057 route must be top-level /communities/new, not scoped under /c/:id"
    );
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("require_auth")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("require_active_admin_somewhere"),
        "Community creation must require an authenticated active admin somewhere"
    );
    assert!(
        AUTHZ_SRC.contains("find_first_admin_for_user"),
        "Active-admin-somewhere eligibility must be enforced through authz"
    );
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("COMMUNITY_CREATION_ENABLED"),
        "Community creation must be guarded by an operator feature flag"
    );
}

#[test]
fn rfc057_token_idempotency_rate_limit_and_timezone_are_fixed() {
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("token_purpose::CREATE_COMMUNITY")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("set_result")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("if let Some(community_id) = replay"),
        "Community creation must use scoped form tokens and replay to the created community"
    );
    assert!(
        RATE_LIMIT_SRC.contains("community_create_user")
            && RATE_LIMIT_SRC.contains("community_create_session")
            && RATE_LIMIT_SRC.contains("community_create_ip")
            && RATE_LIMIT_SRC.contains("COMMUNITY_CREATION_MAX_PER_WINDOW"),
        "Community creation must be rate-limited by user, session, and IP"
    );
    assert!(
        COMMUNITY_CREATE_HANDLER_SRC.contains("SUPPORTED_TIMEZONE: &str = \"Asia/Tokyo\"")
            && COMMUNITY_CREATE_HANDLER_SRC.contains("timezone != SUPPORTED_TIMEZONE"),
        "v0.41.0 must expose only the reviewed Japan-time selection"
    );
}

#[test]
fn rfc057_creation_writes_only_community_membership_and_audit() {
    assert!(
        COMMUNITY_DB_SRC.contains("INSERT INTO communities")
            && COMMUNITY_DB_SRC.contains("INSERT INTO community_memberships")
            && COMMUNITY_DB_SRC.contains("INSERT INTO audit_log")
            && COMMUNITY_DB_SRC.contains("db.batch"),
        "Community creation must batch community, first-admin membership, and audit writes"
    );
    assert!(
        COMMUNITY_DB_SRC.contains("community.created")
            && COMMUNITY_DB_SRC.contains("membership.created_first_admin"),
        "Community creation must emit the reviewed audit events"
    );
    assert!(
        COMMUNITY_DB_SRC.contains("metadata_json")
            && !COMMUNITY_DB_SRC.contains("action, metadata, created_at"),
        "Community creation audit insert must match the D1 schema column metadata_json"
    );
    for forbidden in [
        "event_days",
        "events",
        "attendance",
        "notes",
        "invite_codes",
        "event_templates",
    ] {
        assert!(
            !COMMUNITY_DB_SRC.contains(forbidden),
            "Community creation DB helper must not copy or generate {forbidden}"
        );
    }
    assert!(
        !COMMUNITY_CREATE_HANDLER_SRC.contains("GENERATE_INVITE")
            && !COMMUNITY_CREATE_HANDLER_SRC.contains("insert_invite")
            && !COMMUNITY_CREATE_HANDLER_SRC.contains("invite_code"),
        "Community creation must not auto-generate an invite code"
    );
}

#[test]
fn rfc057_me_entry_and_feature_flag_defaults_are_reviewed() {
    assert!(
        ME_HANDLER_SRC.contains("JA_COMMUNITY_CREATE_LINK")
            && ME_HANDLER_SRC.contains("/communities/new")
            && ME_HANDLER_SRC.contains("find_first_admin_for_user")
            && ME_HANDLER_SRC.contains("community_creation_enabled"),
        "Me page must show the quiet create-community entry only for eligible admins"
    );
    assert!(
        WRANGLER_TOML_SRC.contains("[env.dev.vars]")
            && WRANGLER_TOML_SRC.contains("COMMUNITY_CREATION_ENABLED = \"true\"")
            && WRANGLER_TOML_SRC.contains("[env.production.vars]")
            && WRANGLER_TOML_SRC.contains("COMMUNITY_CREATION_ENABLED = \"false\""),
        "Community creation flag should be enabled for local/staging review and off in production by default"
    );
}

#[test]
fn rfc056_home_lists_communities_without_switcher() {
    assert!(
        HOME_HANDLER_SRC.contains("home_upcoming_for_communities"),
        "Home must batch nearby events across all user communities"
    );
    assert!(
        HOME_HANDLER_SRC.contains("render_home_communities"),
        "Home must render communities one by one"
    );
    assert!(
        HOME_HANDLER_SRC.contains("render::header(i18n::JA_NAV_HOME"),
        "Home must use a simple header without the community switcher"
    );
    assert!(
        !HOME_HANDLER_SRC.contains("header_with_switcher(i18n::JA_NAV_HOME"),
        "Home must not render the community switcher"
    );
}

#[test]
fn rfc056_calendar_page_owns_calendar_and_switcher() {
    assert!(
        COMMUNITIES_SRC.contains("render_calendar_month"),
        "The former Communities tab must render the active community calendar"
    );
    assert!(
        COMMUNITIES_SRC.contains("render_calendar_events"),
        "Calendar page must render the active community event list below the month grid"
    );
    assert!(
        COMMUNITIES_SRC.contains("event_db::calendar_month_for_community")
            && COMMUNITIES_SRC.contains("community_id")
            && COMMUNITIES_SRC.contains("month_start")
            && COMMUNITIES_SRC.contains("next_month_start"),
        "Calendar page events must be scoped to the selected active community and visible month"
    );
    assert!(
        !COMMUNITIES_SRC.contains("home_upcoming(&db, community_id"),
        "Calendar page must not use the Home next-30-days query for its month grid"
    );
    assert!(
        COMMUNITIES_SRC.contains("href=\\\"/c/{cid}/events/{eid}\\\""),
        "Calendar page event list must link into the selected community's Event Detail"
    );
    assert!(
        COMMUNITIES_SRC.contains("header_with_switcher_next"),
        "Calendar page must keep the community switcher"
    );
    assert!(
        COMMUNITIES_SRC.contains("switcher_next")
            && COMMUNITY_HANDLER_SRC.contains("communities:")
            && COMMUNITY_HANDLER_SRC.contains("calendar_next_destination"),
        "Calendar switcher must preserve the Calendar page, selected month, and selected day after switching communities"
    );
    assert!(
        COMMUNITIES_SRC.contains("query_pairs()")
            && COMMUNITIES_SRC.contains("\"month\"")
            && COMMUNITIES_SRC.contains("\"day\"")
            && COMMUNITIES_SRC.contains("JA_CALENDAR_PREV_MONTH")
            && COMMUNITIES_SRC.contains("JA_CALENDAR_NEXT_MONTH")
            && COMMUNITIES_SRC.contains("JA_CALENDAR_THIS_MONTH")
            && COMMUNITIES_SRC.contains("JA_CALENDAR_ALL_DAYS"),
        "Calendar page must support month navigation and a clearable selected-day agenda"
    );
    assert!(
        COMMUNITIES_SRC.contains("?month={month_key}&amp;day={day_date}")
            && COMMUNITIES_SRC.contains("aria-current=\\\"date\\\""),
        "Calendar day cells must link to a day-filtered agenda with accessible current-day state"
    );
    assert!(
        !RENDER_SRC.contains("onchange='this.form.submit()'"),
        "Community switcher must not rely on inline onchange handlers because CSP blocks them"
    );
    assert!(
        RENDER_SRC.contains("/static/app.js?v=0.57.0-rfc056-rfc065-rfc066-rfc067-rfc068")
            && STATIC_FILES_SRC
                .contains("/static/app.js?v=0.57.0-rfc056-rfc065-rfc066-rfc067-rfc068"),
        "HTML shell must cache-bust app.js so same-version switcher fixes are not hidden by the service worker"
    );
    assert!(
        RENDER_SRC.contains("<button type='submit'")
            && RENDER_SRC.contains("JA_NAV_SWITCH_GO")
            && !RENDER_SRC.contains("<noscript><button type='submit'"),
        "Community switcher must have a visible submit fallback, not only a noscript-only button"
    );
    assert!(
        APP_JS_SRC.contains("form[action=\"/switch\"]")
            && APP_JS_SRC.contains("select[name=\"community\"]")
            && APP_JS_SRC.contains("button.hidden = true")
            && APP_JS_SRC.contains("form.submit()"),
        "External app.js must auto-submit the community switcher under CSP"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("admin_events_new"),
        "Admin event creation switcher must keep users on the create-event page for the selected community"
    );
    assert!(
        COMMUNITIES_SRC.contains("grid-template-columns:repeat(7,minmax(0,1fr))"),
        "Calendar overview must keep a stable seven-column grid"
    );
}

#[test]
fn rfc053_calendar_feed_privacy_and_revocation_ux_is_guarded() {
    assert!(
        CALENDAR_HANDLER_SRC.contains("JA_CALENDAR_PRIVACY_NOTE")
            && CALENDAR_HANDLER_SRC.contains("JA_CALENDAR_GENERATED_FLASH")
            && CALENDAR_HANDLER_SRC.contains("JA_CALENDAR_REVOKED_FLASH")
            && CALENDAR_HANDLER_SRC.contains("calendar_flash_message")
            && CALENDAR_HANDLER_SRC.contains("?flash=generated")
            && CALENDAR_HANDLER_SRC.contains("?flash=disabled")
            && CALENDAR_HANDLER_SRC.contains("url.port()"),
        "RFC-053 calendar feed page must use reviewed fixed copy and fixed flash codes"
    );
    assert!(
        !CALENDAR_HANDLER_SRC.contains("Feed+URL+generated")
            && !CALENDAR_HANDLER_SRC.contains("Feed+disabled")
            && !CALENDAR_HANDLER_SRC.contains("render::escape_html(&f)"),
        "Calendar feed actions must not surface raw or English flash query text"
    );
    let calendar_audit_helper_src = CALENDAR_HANDLER_SRC
        .split("async fn write_calendar_token_audit")
        .nth(1)
        .and_then(|s| s.split("fn redirect").next())
        .expect("Calendar token audit helper must exist");
    assert!(
        CALENDAR_HANDLER_SRC.contains("\"calendar_token_generated\"")
            && CALENDAR_HANDLER_SRC.contains("\"calendar_token_revoked\"")
            && calendar_audit_helper_src.contains("crate::audit::write")
            && calendar_audit_helper_src.contains("\"calendar_feed\"")
            && calendar_audit_helper_src.contains("let target_id: Option<&str> = None;")
            && calendar_audit_helper_src
                .contains("let metadata: Option<serde_json::Value> = None;"),
        "Calendar token generation/revocation must be audited without token-bearing target_id or metadata"
    );
    assert!(
        CALENDAR_HANDLER_SRC.contains("Cache-Control")
            && CALENDAR_HANDLER_SRC.contains("no-store, private")
            && CALENDAR_HANDLER_SRC.contains("Referrer-Policy")
            && CALENDAR_HANDLER_SRC.contains("no-referrer")
            && CALENDAR_HANDLER_SRC.contains("X-Content-Type-Options")
            && CALENDAR_HANDLER_SRC.contains("nosniff")
            && LIB_SRC.contains("h.get(\"Referrer-Policy\")")
            && LIB_SRC.contains("Handlers may set a stricter policy"),
        "Bearer ICS responses must avoid caching, referrer leakage, and content sniffing"
    );

    assert!(
        CALENDAR_DB_SRC.contains("pub async fn events_for_feed")
            && CALENDAR_DB_SRC.contains("e.title")
            && CALENDAR_DB_SRC.contains("e.location")
            && CALENDAR_DB_SRC.contains("e.status")
            && CALENDAR_DB_SRC.contains("ed.starts_at_utc")
            && CALENDAR_DB_SRC.contains("ed.ends_at_utc")
            && CALENDAR_DB_SRC.contains("WHERE ed.community_id = ?1"),
        "ICS feed query must stay community-scoped and limited to event title/time/location/status"
    );
    let feed_query_src = CALENDAR_DB_SRC
        .split("pub async fn events_for_feed")
        .nth(1)
        .expect("events_for_feed must exist");
    for forbidden in [
        "attendance",
        "event_notes",
        "invite_codes",
        "community_memberships",
        "display_name",
        "description",
    ] {
        assert!(
            !feed_query_src.contains(forbidden),
            "ICS feed query must not expose {forbidden}"
        );
    }

    assert!(
        ICS_SRC.contains("SUMMARY:")
            && ICS_SRC.contains("DTSTART:")
            && ICS_SRC.contains("DTEND:")
            && ICS_SRC.contains("LOCATION:")
            && ICS_SRC.contains("STATUS:"),
        "ICS builder must keep the reviewed title/time/location/status output"
    );
    for forbidden in ["ATTENDEE", "DESCRIPTION", "COMMENT", "ORGANIZER"] {
        assert!(
            !ICS_SRC.contains(forbidden),
            "ICS output must not include participant, note, or admin fields: {forbidden}"
        );
    }
}

#[test]
fn calendar_overview_contract_is_explicit() {
    let calendar_src = COMMUNITIES_SRC
        .split("fn render_calendar_month")
        .nth(1)
        .expect("Calendar page must keep a dedicated calendar renderer");

    assert!(
        calendar_src.contains("JA_HOME_CALENDAR_HELPER"),
        "Calendar overview must include helper copy explaining that details are in the list below"
    );
    assert!(
        calendar_src.contains("今日"),
        "Today must be identified by visible text, not color alone"
    );
    assert!(
        calendar_src.contains('●'),
        "Event presence must use a visible marker, not color alone"
    );
    assert!(
        calendar_src.contains("<a href=")
            && calendar_src.contains("aria-current=\\\"date\\\"")
            && calendar_src.contains("JA_CALENDAR_ALL_DAYS"),
        "Calendar day cells are interactive in v0.42.0 and must expose selected-day state plus a clear filter"
    );
    assert!(
        !calendar_src.contains("is_selected || is_today")
            && calendar_src.contains("#FAFAFB")
            && calendar_src.contains("let border_width = if is_today && !is_selected")
            && calendar_src.contains("border:{border_width} solid {border}")
            && calendar_src.contains("#6E6E73"),
        "Today styling must stay calmer than selected-day styling and distinct from ordinary event days"
    );
}

#[test]
fn rfc067_monthly_attendance_matrix_contract_is_guarded() {
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("mod calendar;")
            && COMMUNITIES_HANDLER_SRC.contains("mod matrix;")
            && COMMUNITIES_HANDLER_SRC.contains("matrix::render_matrix")
            && COMMUNITIES_HANDLER_SRC.contains("attendance_db::list_for_event_days"),
        "RFC-067 must keep Calendar route orchestration split from matrix rendering and use one batched attendance query"
    );
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("calendar_month_for_community_limited")
            && COMMUNITIES_HANDLER_SRC.contains("matrix::EVENT_DAY_ROW_CAP + 1")
            && EVENT_DB_SRC.contains("pub async fn calendar_month_for_community_limited")
            && EVENT_DB_SRC.contains("LIMIT ?4"),
        "RFC-067 matrix mode must fetch one row past the event-day cap so over-cap months cannot render truncated matrices"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("pub(super) const MEMBER_ROW_CAP: usize = 100")
            && COMMUNITIES_MATRIX_SRC.contains("pub(super) const EVENT_DAY_ROW_CAP: usize = 300")
            && COMMUNITIES_MATRIX_SRC.contains("JA_CALENDAR_MATRIX_TOO_LARGE"),
        "RFC-067 matrix caps and too-large fallback must stay fixed"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("ORDER BY display_name ASC, id ASC"),
        "RFC-067 matrix member ordering must be stable for duplicate display names"
    );
    assert!(
        COMMUNITY_HANDLER_SRC.contains("[\"communities\", month, \"matrix\"]")
            && COMMUNITY_HANDLER_SRC.contains("[\"communities\", month, day, \"matrix\"]")
            && COMMUNITY_HANDLER_SRC.contains("&view=matrix"),
        "RFC-067 community switcher grammar must preserve exact matrix mode shapes"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("CalendarView::Matrix")
            && COMMUNITIES_MATRIX_SRC.contains("view=matrix")
            && COMMUNITIES_MATRIX_SRC.contains("JA_CALENDAR_VIEW_MATRIX")
            && COMMUNITIES_MATRIX_SRC.contains("JA_CALENDAR_MATRIX_TITLE"),
        "RFC-067 matrix mode must be route-backed and visibly switchable"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("\"○\"")
            && COMMUNITIES_MATRIX_SRC.contains("\"×\"")
            && COMMUNITIES_MATRIX_SRC.contains("\"済\"")
            && COMMUNITIES_MATRIX_SRC.contains("\"?\"")
            && COMMUNITIES_MATRIX_SRC.contains("\"中\"")
            && COMMUNITIES_MATRIX_SRC.contains("format!(\"{answered}/{total}\")")
            && COMMUNITIES_MATRIX_SRC.contains("未回答{}件"),
        "RFC-067 matrix cells must keep the reviewed single-event symbols and multi-event answered/total accessible breakdown"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("data-export-value")
            && COMMUNITIES_MATRIX_SRC.contains("can_export_csv")
            && COMMUNITIES_MATRIX_SRC.contains("export_token")
            && COMMUNITIES_MATRIX_SRC.contains("render_too_large"),
        "RFC-067/RFC-068 matrix export metadata must stay renderer-owned, admin-gated, and absent from too-large fallback"
    );
}

#[test]
fn rfc068_calendar_matrix_csv_export_contract_is_guarded() {
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("token_purpose::CALENDAR_MATRIX_CSV_EXPORT")
            && COMMUNITIES_HANDLER_SRC.contains("calendar_matrix_csv_bound_resource")
            && COMMUNITIES_HANDLER_SRC.contains("post_matrix_export_audit")
            && COMMUNITIES_HANDLER_SRC.contains("form_token::set_result")
            && COMMUNITY_HANDLER_SRC.contains("\"calendar/matrix-export/audit\""),
        "RFC-068 matrix CSV export must use a dedicated month-bound single-use token and audited admin POST route"
    );
    assert!(
        COMMUNITIES_HANDLER_SRC.contains("\"calendar_matrix_csv.export_requested\"")
            && !COMMUNITIES_HANDLER_SRC.contains("\"calendar_matrix_csv.exported\"")
            && COMMUNITIES_HANDLER_SRC.contains("\"month\"")
            && COMMUNITIES_HANDLER_SRC.contains("\"export_type\""),
        "RFC-068 audit action must be metadata-only export_requested, not exported"
    );
    assert!(
        COMMUNITIES_MATRIX_SRC.contains("data-calendar-matrix-export-button")
            && COMMUNITIES_MATRIX_SRC.contains("data-calendar-matrix-export=\\\"true\\\"")
            && COMMUNITIES_MATRIX_SRC.contains("data-export-value")
            && COMMUNITIES_MATRIX_SRC.contains("data-member-name")
            && COMMUNITIES_MATRIX_SRC.contains("data-date")
            && COMMUNITIES_MATRIX_SRC.contains("JA_CALENDAR_MATRIX_CSV_EXPORT"),
        "RFC-068 admin matrix markup must carry the reviewed export controls and explicit cell values"
    );
    assert!(
        APP_JS_SOURCE.contains("matrixCsvFromTable")
            && APP_JS_SOURCE.contains("requestMatrixCsvAudit")
            && APP_JS_SOURCE.contains("downloadMatrixCsv")
            && APP_JS_SOURCE.contains("URL.createObjectURL")
            && APP_JS_SOURCE.contains("new Blob")
            && APP_JS_SOURCE.contains("fetch(button.dataset.auditUrl")
            && APP_JS_SOURCE.contains("/^[\\s]*[=+\\-@]/"),
        "RFC-068 CSV must be generated client-side from rendered matrix after audit request, with formula hardening"
    );
    assert!(
        !APP_JS_SOURCE.contains("/export/csv") && !COMMUNITY_HANDLER_SRC.contains("export/csv"),
        "RFC-068 must not add a server CSV/data export endpoint"
    );
}

#[test]
fn rfc059_calendar_create_from_day_is_route_backed() {
    assert!(
        COMMUNITIES_SRC.contains("membership_db::find_active")
            && COMMUNITIES_SRC.contains("membership.role == \"admin\"")
            && COMMUNITIES_SRC.contains("can_create_event"),
        "Calendar create-from-day action must be rendered only for active admins"
    );
    assert!(
        COMMUNITIES_SRC.contains("/admin/events/new?day={day}")
            && COMMUNITIES_SRC.contains("JA_CALENDAR_CREATE_ON_DAY"),
        "Selected Calendar days must expose a route-backed create-event link"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("valid_prefill_day")
            && ADMIN_EVENTS_SRC.contains("query_pairs()")
            && ADMIN_EVENTS_SRC.contains("\"day\"")
            && ADMIN_EVENTS_SRC.contains("prefill_day.as_deref()"),
        "Create Event must validate and prefill the Calendar-selected day"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("admin_events_new_next")
            && COMMUNITY_HANDLER_SRC.contains("admin_events_new:")
            && COMMUNITY_HANDLER_SRC.contains("admin_events_new_destination"),
        "Create Event switcher must preserve a Calendar-selected day when switching communities"
    );
}

#[test]
fn rfc051_event_edit_semantics_are_details_only_for_multi_day() {
    assert!(
        ADMIN_EVENTS_SRC.contains("fn event_schedule_editable")
            && ADMIN_EVENTS_SRC.contains("days.len() == 1 && !event_is_recurring(event)")
            && ADMIN_EVENTS_SRC.contains("repeat_rule != \"none\"")
            && ADMIN_EVENTS_SRC.contains("repeat_count.is_some()"),
        "RFC-051 schedule editing must be limited to one-day non-recurring events"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("render_single_day_edit_fields")
            && ADMIN_EVENTS_SRC.contains("render_details_only_event_edit_fields")
            && ADMIN_EVENTS_SRC.contains("render_schedule_summary"),
        "RFC-051 edit UI must split single-day edit from details-only edit with a schedule summary"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("JA_ADMIN_EDIT_MULTI_DAY_HELPER")
            && ADMIN_EVENTS_SRC.contains("JA_ADMIN_EDIT_RECURRING_HELPER")
            && ADMIN_EVENTS_SRC.contains("JA_ADMIN_EDIT_RESPONSES_PRESERVED"),
        "Details-only edit must explain what can be changed and that schedule/attendance stay unchanged"
    );
    let details_only_src = ADMIN_EVENTS_SRC
        .split("fn render_details_only_event_edit_fields")
        .nth(1)
        .and_then(|s| s.split("fn render_error_html").next())
        .expect("details-only edit renderer must exist");
    for forbidden in [
        "name=\"day_date\"",
        "name=\"starts_at\"",
        "name=\"ends_at\"",
        "name=\"repeat_rule\"",
        "name=\"repeat_count\"",
    ] {
        assert!(
            !details_only_src.contains(forbidden),
            "Details-only edit form must not render schedule or recurrence control {forbidden}"
        );
    }
    assert!(
        ADMIN_EVENTS_SRC.contains("edit_post_contains_schedule_fields")
            && ADMIN_EVENTS_SRC.contains("JA_ADMIN_EDIT_SCHEDULE_NOT_EDITABLE")
            && ADMIN_EVENTS_SRC.contains("validate_event_details")
            && ADMIN_EVENTS_SRC.contains("edit_scope"),
        "Details-only POST must reject direct schedule fields, validate only details, and audit the edit scope"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("JA_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS")
            && ADMIN_EVENTS_SRC.contains("JA_ADMIN_CANCEL_EVENT_CONFIRM_ALL_DAYS"),
        "Cancellation confirmation must state whole-event scope for multi-day/recurring events"
    );
}

#[test]
fn rfc060_cancelled_event_recreate_is_admin_only_and_details_only() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("\"recreate\"")
            && COMMUNITY_HANDLER_SRC.contains("get_recreate_event"),
        "RFC-060 must route GET /c/:cid/admin/events/:eid/recreate"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("pub async fn get_recreate_event")
            && ADMIN_EVENTS_SRC.contains("require_admin")
            && ADMIN_EVENTS_SRC.contains("event_can_seed_recreate(&event)")
            && ADMIN_EVENTS_SRC.contains("token_purpose::CREATE_EVENT"),
        "Recreate GET must require an active same-community admin, a cancelled source, and a create token"
    );
    assert!(
        EVENT_HANDLER_SRC.contains("membership.is_admin() && event.status == \"cancelled\"")
            && EVENT_HANDLER_SRC.contains("JA_ADMIN_RECREATE_EVENT_ACTION")
            && EVENT_HANDLER_SRC.contains("/admin/events/{eid}/recreate"),
        "Event Detail must show the recreate action only to admins on cancelled events"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("copy_source_event_id")
            && ADMIN_EVENTS_SRC
                .contains("event_db::find_for_community(&db, &source_id, community_id)")
            && ADMIN_EVENTS_SRC.contains("return render::not_found()")
            && ADMIN_EVENTS_SRC.contains("created_from_cancelled_event_id"),
        "Create POST must re-check source event community/status and record safe provenance"
    );
    let recreate_fields_src = ADMIN_EVENTS_SRC
        .split("fn render_recreate_event_create_fields")
        .nth(1)
        .and_then(|s| s.split("fn render_single_day_edit_fields").next())
        .expect("recreate form renderer must exist");
    assert!(
        recreate_fields_src.contains("JA_ADMIN_RECREATE_EVENT_HELPER")
            && recreate_fields_src.contains("event.location.as_deref()")
            && recreate_fields_src.contains("event.description.as_deref()"),
        "Recreate form must explain the boundary and prefill only title/location/description"
    );
    for copied_schedule in [
        "event.repeat_rule",
        "event.repeat_count",
        "day_date: Some",
        "starts_at: Some",
        "ends_at: Some",
    ] {
        assert!(
            !recreate_fields_src.contains(copied_schedule),
            "Recreate form must not copy schedule/recurrence field {copied_schedule}"
        );
    }
}

#[test]
fn rfc066_event_copy_is_admin_reviewed_prefill_not_clone() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("\"copy\"")
            && COMMUNITY_HANDLER_SRC.contains("get_copy_event"),
        "RFC-066 must route GET /c/:cid/admin/events/:eid/copy"
    );
    assert!(
        ADMIN_EVENTS_COPY_SRC.contains("pub async fn get_copy_event")
            && ADMIN_EVENTS_COPY_SRC.contains("require_admin")
            && ADMIN_EVENTS_COPY_SRC.contains("event_db::find_for_community")
            && ADMIN_EVENTS_COPY_SRC.contains("event_db::days_for_event")
            && ADMIN_EVENTS_COPY_SRC.contains("series_db::find_for_event")
            && ADMIN_EVENTS_COPY_SRC.contains("token_purpose::CREATE_EVENT"),
        "Copy GET must require an active same-community admin and load only scoped event/day/series source data"
    );
    for forbidden_source in [
        "attendance",
        "event_note",
        "invite",
        "audit::",
        "form_token",
    ] {
        assert!(
            !ADMIN_EVENTS_COPY_SRC.contains(forbidden_source),
            "Copy source prefill must not load {forbidden_source}"
        );
    }
    assert!(
        EVENT_HANDLER_SRC.contains("membership.is_admin()")
            && EVENT_HANDLER_SRC.contains("JA_ADMIN_COPY_EVENT_ACTION")
            && EVENT_HANDLER_SRC.contains("/admin/events/{eid}/copy"),
        "Event Detail must expose the copy action to active admins"
    );
    assert!(
        ADMIN_EVENTS_SRC.contains("copy_mode")
            && ADMIN_EVENTS_SRC.contains("\"event_copy\"")
            && ADMIN_EVENTS_SRC.contains("event_can_seed_copy")
            && ADMIN_EVENTS_SRC.contains("event_can_seed_recreate")
            && ADMIN_EVENTS_SRC.contains("created_from_cancelled_event_id")
            && ADMIN_EVENTS_SRC.contains("\"copy_source_event_id\""),
        "Create POST must separate RFC-066 event-copy provenance from RFC-060 cancelled-event recreate"
    );
    assert!(
        ADMIN_EVENTS_COPY_SRC.contains("JA_ADMIN_COPY_EVENT_RECURRING_PAST")
            && ADMIN_EVENTS_COPY_SRC.contains("JA_ADMIN_COPY_EVENT_RECURRING_WINDOW")
            && ADMIN_EVENTS_COPY_SRC.contains("normal_create_default")
            && ADMIN_EVENTS_COPY_SRC.contains("until >= series.start_day_date.as_str()"),
        "Copy prefill must implement reviewed recurring normalization rules"
    );
}

#[test]
fn note_form_has_counter_element_for_js() {
    // The app.js memo counter targets `.note-counter`. If the rendered form
    // omits that class, the live N/200 counter silently never updates (the
    // button-disable still works, but the visible count does not). This
    // regression shipped in v0.35.x.
    assert!(
        RENDER_SRC.contains("note-counter"),
        "note_form must render an element with class \"note-counter\" so the \
         app.js character counter has a target. Without it the live count is dead."
    );
}

// ── Invite code generation security gates ────────────────────────────────
//
// §7.1: fail-closed randomness. The generator must not silently fall back to
// deterministic output if the OS RNG is unavailable. The previous implementation
// used `.unwrap_or_default()` on `getrandom`, which on failure left the byte
// buffer zeroed, producing the code "AAAAAA". The fix uses `?` propagation.
//
// §7.2: rejection sampling. The alphabet has 31 characters; 256 % 31 = 8.
// The previous implementation used `b % 31`, which over-represents the first
// 8 characters by one count out of every 256 draws. The fix discards bytes
// >= 248 and redraws.

const MEMBERS_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/members.rs");
const JOIN_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/join.rs");
const INVITE_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/invite.rs");

#[test]
fn invite_code_generator_does_not_use_unwrap_or_default_on_getrandom() {
    // If this fails, the generator has regressed to fail-open: randomness
    // failure would silently produce a deterministic invite code.
    //
    // getrandom 0.4 renamed the entry point from `getrandom::getrandom` to
    // `getrandom::fill`. The source must use `?` or `.expect()` after the
    // call, not `.unwrap_or_default()` or `.ok()`.
    // Invite generation must propagate getrandom errors instead of silently
    // falling back to deterministic bytes.
    let lines: Vec<&str> = MEMBERS_HANDLER_SRC
        .lines()
        .filter(|l| l.contains("getrandom::fill") || l.contains("getrandom::getrandom"))
        .collect();
    for l in &lines {
        assert!(
            !l.contains("unwrap_or_default") && !l.contains(".ok()"),
            "getrandom call uses fail-open error handling: {l:?}\n\
             Must use `?` or `.expect()` — silence on RNG failure produces \
             a deterministic invite code."
        );
    }
}

#[test]
fn invite_code_generator_uses_rejection_sampling() {
    // The unbiased ceiling must appear in the source to confirm rejection
    // sampling is in use. 248 = 256 - (256 % 31) is the exact value.
    assert!(
        MEMBERS_HANDLER_SRC.contains("248")
            || MEMBERS_HANDLER_SRC.contains("unbiased_ceiling")
            || MEMBERS_HANDLER_SRC.contains("256 - (256 % alpha_len)"),
        "generate_invite_code no longer references the rejection-sampling ceiling (248 or \
         unbiased_ceiling or the expression). Verify the modulo-bias fix is still in place."
    );
    // The old biased pattern must not be present.
    assert!(
        !MEMBERS_HANDLER_SRC.contains("unwrap_or_default();\n    bytes.iter()"),
        "generate_invite_code appears to have reverted to the biased modulo pattern."
    );
}

#[test]
fn join_profile_backfills_invite_membership_after_membership_exists() {
    let mark_used = JOIN_HANDLER_SRC
        .find("crate::db::invite::mark_used(&db, &invite_id)")
        .expect("join profile must atomically mark invite used");
    let insert_user = JOIN_HANDLER_SRC
        .find("membership_db::insert_user(&db, &user_id)")
        .expect("join profile must insert user");
    let insert_membership = JOIN_HANDLER_SRC
        .find("membership_db::insert_membership(")
        .expect("join profile must insert membership");
    let assign_used_membership = JOIN_HANDLER_SRC
        .find("crate::db::invite::assign_used_membership(&db, &invite_id, &membership_id)")
        .expect("join profile must backfill invite used_by_membership_id");

    assert!(
        mark_used < insert_user && mark_used < insert_membership,
        "invite must be claimed before user/session side effects so races create one winner"
    );
    assert!(
        insert_membership < assign_used_membership,
        "used_by_membership_id references community_memberships(id); backfill it only after \
         insert_membership succeeds"
    );
}

#[test]
fn invite_mark_used_does_not_write_membership_fk() {
    let mark_start = INVITE_DB_SRC
        .find("pub async fn mark_used(")
        .expect("invite::mark_used must exist");
    let assign_start = INVITE_DB_SRC
        .find("pub async fn assign_used_membership(")
        .expect("invite::assign_used_membership must exist");
    let mark_body = &INVITE_DB_SRC[mark_start..assign_start];
    let assign_body = &INVITE_DB_SRC[assign_start..];

    assert!(
        mark_body.contains("SET used_at = ?1"),
        "mark_used should perform the atomic one-winner claim"
    );
    assert!(
        !mark_body.contains("used_by_membership_id"),
        "mark_used must not write used_by_membership_id before the membership FK target exists"
    );
    assert!(
        assign_body.contains("SET used_by_membership_id = ?1"),
        "assign_used_membership should perform the post-membership FK backfill"
    );
}
