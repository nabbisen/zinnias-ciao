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
    assert!(
        RENDER_SRC.contains("fn recovery_links()")
            && RENDER_SRC.contains("href=\\\"/\\\"")
            && RENDER_SRC.contains("href=\\\"/join\\\"")
            && LIB_SRC.contains("is_not_found_error(&error)")
            && LIB_SRC.contains("render::not_found()"),
        "Generic not-found/forbidden and error pages must be recoverable and expected authorization denial must not render as internal error"
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
        token_purpose::CHANGE_DISPLAY_NAME,
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
// To add a new string: add EN_FOO and JA_FOO in the relevant i18n child module,
// then add the pair below.

#[test]
fn i18n_en_ja_parity_count() {
    use zinnias_ciao_contracts::i18n::*;
    let pairs = [
        (EN_JOIN_HEADING, JA_JOIN_HEADING),
        (EN_JOIN_SUBHEADING, JA_JOIN_SUBHEADING),
        (EN_JOIN_CODE_LABEL, JA_JOIN_CODE_LABEL),
        (EN_JOIN_CODE_HINT, JA_JOIN_CODE_HINT),
        (EN_JOIN_RELINK_HINT, JA_JOIN_RELINK_HINT),
        (EN_JOIN_RELINK_LINK, JA_JOIN_RELINK_LINK),
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
        (EN_CONFIGURATION_UNAVAILABLE, JA_CONFIGURATION_UNAVAILABLE),
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
        (EN_ME_CHANGE_DISPLAY_NAME, JA_ME_CHANGE_DISPLAY_NAME),
        (EN_ME_DISPLAY_NAME_EDIT_TITLE, JA_ME_DISPLAY_NAME_EDIT_TITLE),
        (
            EN_ME_DISPLAY_NAME_EDIT_SUBMIT,
            JA_ME_DISPLAY_NAME_EDIT_SUBMIT,
        ),
        (
            EN_ME_DISPLAY_NAME_EDIT_CANCEL,
            JA_ME_DISPLAY_NAME_EDIT_CANCEL,
        ),
        (EN_ME_DISPLAY_NAME_UPDATED, JA_ME_DISPLAY_NAME_UPDATED),
        (EN_ME_DISPLAY_NAME_ERROR, JA_ME_DISPLAY_NAME_ERROR),
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
        (
            EN_ADMIN_INVITES_REVEAL_WARNING,
            JA_ADMIN_INVITES_REVEAL_WARNING,
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
        (
            EN_ADMIN_HELP_SIGNIN_RELINK_HINT,
            JA_ADMIN_HELP_SIGNIN_RELINK_HINT,
        ),
        (
            EN_ADMIN_HELP_SIGNIN_RELINK_LINK,
            JA_ADMIN_HELP_SIGNIN_RELINK_LINK,
        ),
        (
            EN_ADMIN_HELP_SIGNIN_COPY_CODE,
            JA_ADMIN_HELP_SIGNIN_COPY_CODE,
        ),
        (
            EN_ADMIN_HELP_SIGNIN_COPY_DONE,
            JA_ADMIN_HELP_SIGNIN_COPY_DONE,
        ),
        (
            EN_ADMIN_HELP_SIGNIN_COPY_FAILED,
            JA_ADMIN_HELP_SIGNIN_COPY_FAILED,
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
const AUTH_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/auth.rs");
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
const CRYPTO_SRC: &str = include_str!("../../../workers/ssr/src/crypto.rs");
const CRYPTO_TESTS_SRC: &str = include_str!("../../../workers/ssr/src/crypto/tests.rs");
const CODLET_SRC: &str = include_str!("../../../workers/ssr/src/codlet.rs");
const SESSION_SRC: &str = include_str!("../../../workers/ssr/src/session.rs");
const HEALTH_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/health.rs");
const SETUP_SCRIPT_SRC: &str = include_str!("../../../scripts/setup.mjs");
const SETUP_CORE_SRC: &str = include_str!("../../../scripts/lib/setup-core.mjs");
const LOCAL_SECRET_FILE_SRC: &str = include_str!("../../../scripts/lib/local-secret-file.mjs");
const LOCAL_SECRET_TEST_SRC: &str = include_str!("../../../scripts/test-local-secret-setup.mjs");
const ISOLATED_WORKER_TEST_SRC: &str =
    include_str!("../../../scripts/lib/isolated-worker-test.mjs");
const BOOTSTRAP_SCRIPT_SRC: &str = include_str!("../../../scripts/bootstrap-cloudflare.mjs");
const BOOTSTRAP_CORE_SRC: &str = include_str!("../../../scripts/lib/bootstrap-cloudflare-core.mjs");
const BOOTSTRAP_TEST_SRC: &str = include_str!("../../../scripts/test-bootstrap-cloudflare.mjs");
const PEPPER_CONFIGURATION_TEST_SRC: &str =
    include_str!("../../../scripts/test-hmac-pepper-configuration.mjs");
const RFC077_DIRECT_PEPPER_CALLERS: &[(&str, &str, usize)] = &[
    ("codlet", CODLET_SRC, 2),
    ("session", SESSION_SRC, 1),
    ("health", HEALTH_HANDLER_SRC, 1),
    (
        "join",
        include_str!("../../../workers/ssr/src/handlers/join.rs"),
        3,
    ),
    (
        "relink",
        include_str!("../../../workers/ssr/src/handlers/relink.rs"),
        2,
    ),
    ("calendar", CALENDAR_HANDLER_SRC, 2),
    (
        "communities",
        include_str!("../../../workers/ssr/src/handlers/communities.rs"),
        1,
    ),
    ("community-create", COMMUNITY_CREATE_HANDLER_SRC, 1),
    ("me", ME_HANDLER_SRC, 1),
    (
        "operator",
        include_str!("../../../workers/ssr/src/handlers/operator.rs"),
        1,
    ),
    (
        "admin-help-signin",
        include_str!("../../../workers/ssr/src/handlers/admin/help_signin.rs"),
        1,
    ),
    (
        "admin-members",
        include_str!("../../../workers/ssr/src/handlers/admin/members.rs"),
        1,
    ),
];
const RFC077_CODLET_ISSUE_CALLERS: &[(&str, &str, usize)] = &[
    (
        "templates",
        include_str!("../../../workers/ssr/src/handlers/templates.rs"),
        2,
    ),
    ("me", ME_HANDLER_SRC, 3),
    ("export", EXPORT_HANDLER_SRC, 1),
    ("event", EVENT_HANDLER_SRC, 3),
    ("community-create", COMMUNITY_CREATE_HANDLER_SRC, 2),
    (
        "communities",
        include_str!("../../../workers/ssr/src/handlers/communities.rs"),
        1,
    ),
    ("calendar", CALENDAR_HANDLER_SRC, 2),
    (
        "admin-help-signin",
        include_str!("../../../workers/ssr/src/handlers/admin/help_signin.rs"),
        1,
    ),
    (
        "admin-role-transfer",
        include_str!("../../../workers/ssr/src/handlers/admin/role_transfer.rs"),
        1,
    ),
    (
        "admin-member-remove",
        include_str!("../../../workers/ssr/src/handlers/admin/member_remove.rs"),
        1,
    ),
    (
        "admin-members",
        include_str!("../../../workers/ssr/src/handlers/admin/members.rs"),
        2,
    ),
    (
        "event-attendance",
        include_str!("../../../workers/ssr/src/handlers/admin/events/attendance.rs"),
        1,
    ),
    (
        "event-cancel",
        include_str!("../../../workers/ssr/src/handlers/admin/events/cancel.rs"),
        1,
    ),
    (
        "event-copy",
        include_str!("../../../workers/ssr/src/handlers/admin/events/copy.rs"),
        1,
    ),
    (
        "event-create",
        include_str!("../../../workers/ssr/src/handlers/admin/events/create.rs"),
        1,
    ),
    (
        "event-edit",
        include_str!("../../../workers/ssr/src/handlers/admin/events/edit.rs"),
        1,
    ),
    (
        "event-notes",
        include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs"),
        1,
    ),
    (
        "event-occurrence",
        include_str!("../../../workers/ssr/src/handlers/admin/events/occurrence.rs"),
        1,
    ),
    (
        "event-recreate",
        include_str!("../../../workers/ssr/src/handlers/admin/events/recreate.rs"),
        1,
    ),
];
const RFC077_AUTH_HANDLER_SOURCES: &[(&str, &str)] = &[
    (
        "templates",
        include_str!("../../../workers/ssr/src/handlers/templates.rs"),
    ),
    ("me", ME_HANDLER_SRC),
    ("export", EXPORT_HANDLER_SRC),
    ("event", EVENT_HANDLER_SRC),
    ("community-create", COMMUNITY_CREATE_HANDLER_SRC),
    (
        "communities",
        include_str!("../../../workers/ssr/src/handlers/communities.rs"),
    ),
    ("calendar", CALENDAR_HANDLER_SRC),
    (
        "community",
        include_str!("../../../workers/ssr/src/handlers/community.rs"),
    ),
    (
        "join",
        include_str!("../../../workers/ssr/src/handlers/join.rs"),
    ),
    (
        "relink",
        include_str!("../../../workers/ssr/src/handlers/relink.rs"),
    ),
    ("auth", AUTH_HANDLER_SRC),
    ("home", HOME_HANDLER_SRC),
    (
        "admin-help-signin",
        include_str!("../../../workers/ssr/src/handlers/admin/help_signin.rs"),
    ),
    (
        "admin-role-transfer",
        include_str!("../../../workers/ssr/src/handlers/admin/role_transfer.rs"),
    ),
    (
        "admin-member-remove",
        include_str!("../../../workers/ssr/src/handlers/admin/member_remove.rs"),
    ),
    (
        "admin-members",
        include_str!("../../../workers/ssr/src/handlers/admin/members.rs"),
    ),
    (
        "event-attendance",
        include_str!("../../../workers/ssr/src/handlers/admin/events/attendance.rs"),
    ),
    (
        "event-cancel",
        include_str!("../../../workers/ssr/src/handlers/admin/events/cancel.rs"),
    ),
    (
        "event-copy",
        include_str!("../../../workers/ssr/src/handlers/admin/events/copy.rs"),
    ),
    (
        "event-create",
        include_str!("../../../workers/ssr/src/handlers/admin/events/create.rs"),
    ),
    (
        "event-edit",
        include_str!("../../../workers/ssr/src/handlers/admin/events/edit.rs"),
    ),
    (
        "event-notes",
        include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs"),
    ),
    (
        "event-occurrence",
        include_str!("../../../workers/ssr/src/handlers/admin/events/occurrence.rs"),
    ),
    (
        "event-recreate",
        include_str!("../../../workers/ssr/src/handlers/admin/events/recreate.rs"),
    ),
];
const RFC077_EXECUTABLE_GATE_SOURCES: &[(&str, &str)] = &[
    (
        "admin-role-transfer",
        include_str!("../../../scripts/smoke/admin-role-transfer.mjs"),
    ),
    (
        "calendar-matrix-csv-export",
        include_str!("../../../scripts/smoke/calendar-matrix-csv-export.mjs"),
    ),
    (
        "event-copy",
        include_str!("../../../scripts/smoke/event-copy.mjs"),
    ),
    (
        "help-signin",
        include_str!("../../../scripts/smoke/help-signin.mjs"),
    ),
    (
        "invite-redemption",
        include_str!("../../../scripts/smoke/invite-redemption.mjs"),
    ),
    (
        "member-management",
        include_str!("../../../scripts/smoke/member-management.mjs"),
    ),
    (
        "monthly-attendance-matrix",
        include_str!("../../../scripts/smoke/monthly-attendance-matrix.mjs"),
    ),
    (
        "recurrence-v2",
        include_str!("../../../scripts/smoke/recurrence-v2.mjs"),
    ),
    (
        "self-display-name-editing",
        include_str!("../../../scripts/smoke/self-display-name-editing.mjs"),
    ),
    (
        "audit-class-a-failures",
        include_str!("../../../scripts/test-audit-class-a-failures.mjs"),
    ),
];

#[test]
fn rfc077_has_one_secret_only_pepper_resolver() {
    assert_eq!(
        CRYPTO_SRC.matches(".secret(\"HMAC_PEPPER\")").count(),
        1,
        "RFC-077 requires exactly one HMAC_PEPPER secret-binding read"
    );
    assert!(
        !CRYPTO_SRC.contains(".var(\"HMAC_PEPPER\")")
            && !LIB_SRC.contains(".secret(\"HMAC_PEPPER\")")
            && !LIB_SRC.contains(".var(\"HMAC_PEPPER\")"),
        "RFC-077 forbids a plain-var fallback and binding reads outside crypto.rs"
    );
    assert!(
        CRYPTO_SRC.contains("pub struct HmacPepper(String)")
            && CRYPTO_SRC.contains("pub(crate) fn as_str(&self) -> &str")
            && !CRYPTO_SRC.contains("impl fmt::Debug for HmacPepper")
            && !CRYPTO_SRC.contains("impl fmt::Display for HmacPepper"),
        "validated pepper material must retain its narrow opaque interface"
    );
    assert!(
        CRYPTO_TESTS_SRC.contains("pepper_validation_uses_utf8_byte_length_and_preserves_input")
            && CRYPTO_TESTS_SRC.contains("repeat(4097)")
            && CRYPTO_TESTS_SRC.contains("for sentinel in LEGACY_SENTINELS"),
        "pepper validation boundary cases must remain covered"
    );
}

#[test]
fn rfc077_preflight_and_health_remain_fail_closed() {
    let classify = LIB_SRC
        .find("let security_class = request_security_class")
        .expect("request classification must exist");
    let preflight = LIB_SRC
        .find("crypto::pepper(&env)")
        .expect("protected request preflight must exist");
    let dispatch = LIB_SRC
        .find("|| dispatch_request(req")
        .expect("route dispatch must exist");
    assert!(classify < preflight && preflight < dispatch);
    for path in [
        "/manifest.webmanifest",
        "/sw.js",
        "/static/app.css",
        "/static/app.js",
        "/offline",
        "/version",
    ] {
        assert_eq!(
            LIB_SRC.matches(&format!("\"{path}\"")).count(),
            3,
            "{path} must occur only in dispatch, classifier, and classifier tests"
        );
    }
    assert!(
        HEALTH_HANDLER_SRC.contains("crate::crypto::pepper(env).is_ok()")
            && HEALTH_HANDLER_SRC.contains("\"ready\": true")
            && HEALTH_HANDLER_SRC.contains("\"ready\": false")
            && HEALTH_HANDLER_SRC.contains(".with_status(503)"),
        "health must report readiness from the same validated pepper resolver"
    );
    assert!(
        CODLET_SRC.contains("pub async fn issue_token(")
            && CODLET_SRC.contains(") -> Result<String>")
            && !CODLET_SRC.contains("unwrap_or_default")
            && CODLET_SRC.find("crate::crypto::pepper(env)") < CODLET_SRC.find("env.d1(\"DB\")"),
        "codlet issuance must propagate pepper failures without empty substitution"
    );
    assert!(
        LIB_SRC.contains("rejected_configuration_never_invokes_binding_continuation")
            && LIB_SRC.contains("D1 continuation was invoked")
            && LIB_SRC.contains("KV continuation was invoked"),
        "the native pre-binding spy must pin zero D1 and KV continuation access"
    );
}

#[test]
fn rfc077_pepper_codlet_and_auth_caller_inventories_are_closed() {
    assert_eq!(
        LIB_SRC.matches("crypto::pepper(&env)").count(),
        1,
        "main must have exactly one direct protected-request preflight"
    );
    let direct_count: usize = RFC077_DIRECT_PEPPER_CALLERS
        .iter()
        .map(|(name, source, expected)| {
            let actual = source.matches("crate::crypto::pepper(").count();
            assert_eq!(
                actual, *expected,
                "direct pepper caller inventory drifted: {name}"
            );
            actual
        })
        .sum();
    assert_eq!(direct_count, 17, "direct pepper caller total drifted");

    let issue_count: usize = RFC077_CODLET_ISSUE_CALLERS
        .iter()
        .map(|(name, source, expected)| {
            let actual = source.matches("crate::codlet::issue_token(").count();
            assert_eq!(
                actual, *expected,
                "codlet issuance caller inventory drifted: {name}"
            );
            actual
        })
        .sum();
    assert_eq!(issue_count, 27, "codlet issuance caller total drifted");

    assert!(
        SESSION_SRC.contains("pub enum AuthError")
            && SESSION_SRC.contains("AuthError::Unauthenticated")
            && SESSION_SRC.contains("Err(error) => return Err(error.into_worker_error())"),
        "configuration and runtime errors must remain distinguishable from unauthenticated"
    );
    for (name, source) in RFC077_AUTH_HANDLER_SOURCES {
        if source.contains("require_auth_or!") {
            assert!(
                !source.contains("match crate::session::require_auth"),
                "macro-managed handler retained an untyped auth match: {name}"
            );
        } else {
            assert!(
                source.contains("AuthError::Unauthenticated")
                    && source.contains("Err(error) => return Err(error.into_worker_error())")
                    && !source.contains("Err(_) =>"),
                "explicit auth handler is outside the typed propagation inventory: {name}"
            );
        }
    }
}

#[test]
fn rfc077_configuration_and_local_harness_are_pinned() {
    assert_eq!(
        WRANGLER_TOML_SRC
            .matches("required = [\"HMAC_PEPPER\"]")
            .count(),
        4,
        "root, dev, staging, and production must each require HMAC_PEPPER"
    );
    for pattern in [".dev.vars*", ".env*"] {
        assert!(
            GITIGNORE_SRC.lines().any(|line| line.trim() == pattern),
            "local secret files must remain ignored: {pattern}"
        );
    }
    assert!(
        SETUP_SCRIPT_SRC.contains("loadOrCreateLocalPepper")
            && SETUP_SCRIPT_SRC.contains("runDeveloperSetup")
            && SETUP_CORE_SRC.contains("const pepper = await adapter.loadPepper(projectRoot)")
            && LOCAL_SECRET_FILE_SRC.contains("fs.open(path, 'wx', 0o600)")
            && LOCAL_SECRET_FILE_SRC.contains("await handle.chmod(0o600)")
            && !LOCAL_SECRET_FILE_SRC.contains("fs.chmod(path")
            && !LOCAL_SECRET_FILE_SRC.contains("fs.unlink(path")
            && LOCAL_SECRET_FILE_SRC.contains("O_NOFOLLOW")
            && LOCAL_SECRET_FILE_SRC.contains("DOTENV_LINE")
            && LOCAL_SECRET_TEST_SRC.contains("exclusive-create collision was not injected")
            && LOCAL_SECRET_TEST_SRC.contains("replacement path was modified")
            && LOCAL_SECRET_TEST_SRC.contains("injected_close_failure")
            && LOCAL_SECRET_TEST_SRC.contains("injected_close_after_release_failure")
            && LOCAL_SECRET_TEST_SRC.contains("closeAfterReleaseSanitization")
            && LOCAL_SECRET_TEST_SRC.contains("seed HMAC disagreed"),
        "local setup must use handle-safe permissions/cleanup, exact dotenv parsing, and the adversarial matrix"
    );
    assert!(
        LOCAL_SECRET_FILE_SRC.find("if (originalHandle)")
            < LOCAL_SECRET_FILE_SRC.find("reopened = await fs.open")
            && LOCAL_SECRET_FILE_SRC.contains("if (await sanitizeHandle(originalHandle)) return")
            && LOCAL_SECRET_FILE_SRC.contains("await sanitizeHandle(reopened)"),
        "close failure cleanup must fall back from an unusable original handle to an identity-checked non-following reopen"
    );
    assert!(
        ISOLATED_WORKER_TEST_SRC.contains("CLOUDFLARE_LOAD_DEV_VARS_FROM_DOT_ENV: 'false'")
            && ISOLATED_WORKER_TEST_SRC.contains("CLOUDFLARE_INCLUDE_PROCESS_ENV: 'false'")
            && ISOLATED_WORKER_TEST_SRC.contains("const environment = {")
            && ISOLATED_WORKER_TEST_SRC.contains("mkdtemp")
            && ISOLATED_WORKER_TEST_SRC.contains("randomBytes(32)")
            && ISOLATED_WORKER_TEST_SRC
                .contains("cp(join(repositoryRoot, 'workers', 'ssr', 'build')")
            && ISOLATED_WORKER_TEST_SRC
                .contains("const canaryPath = join(container, '.dev.vars.dev')")
            && ISOLATED_WORKER_TEST_SRC.contains("wrangler-child-wrapper.mjs")
            && ISOLATED_WORKER_TEST_SRC.contains("child-environment-keys.json")
            && ISOLATED_WORKER_TEST_SRC.contains("rm(container, { recursive: true"),
        "runtime tests must copy artifacts and use a sanitized, canaried, child-audited fixture"
    );
    assert!(
        PEPPER_CONFIGURATION_TEST_SRC.contains("assert.equal(result.text, unavailableBody)")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("optionalRecoverySecret")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("COMMUNITY_RECOVERY_TOKEN")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("assertChildEnvironmentAudit"),
        "runtime evidence must pin the exact body, optional recovery, and child environment audit"
    );
}

#[test]
fn rfc077_bootstrap_and_executable_gates_remain_safe() {
    assert!(
        BOOTSTRAP_SCRIPT_SRC.contains("runBootstrap")
            && BOOTSTRAP_CORE_SRC.contains("Missing --target; choose staging or production explicitly.")
            && BOOTSTRAP_CORE_SRC.contains("parsed.length !== 1")
            && BOOTSTRAP_CORE_SRC.contains("rejectDuplicateJsonKeys(stdout)")
            && BOOTSTRAP_CORE_SRC.contains("if (keys.has(key)) throw new DuplicateJsonKeyError")
            && BOOTSTRAP_CORE_SRC.contains("--rotate-hmac-pepper")
            && BOOTSTRAP_CORE_SRC.contains("--confirm-rotation")
            && BOOTSTRAP_CORE_SRC.contains("ROTATE ${targetName}")
            && BOOTSTRAP_CORE_SRC.contains("provisioned-not-ready")
            && BOOTSTRAP_CORE_SRC.contains("Keep this Worker dark")
            && BOOTSTRAP_CORE_SRC.contains("sessions, invites, relink/help-signin codes, form tokens, calendar tokens, and recovery codes"),
        "bootstrap must distinguish fresh provisioning from explicit destructive rotation"
    );
    assert!(
        BOOTSTRAP_TEST_SRC.contains("missingTargetStops")
            && BOOTSTRAP_TEST_SRC.contains("strictSingleResultEnvelope")
            && BOOTSTRAP_TEST_SRC.contains("duplicateResultKeysStop")
            && BOOTSTRAP_TEST_SRC.contains("escaped-duplicate-second")
            && BOOTSTRAP_TEST_SRC.contains("escaped-duplicate-first")
            && BOOTSTRAP_TEST_SRC.contains("exactCommandAdapter")
            && BOOTSTRAP_TEST_SRC.contains("fake adapter rejected unexpected SQL"),
        "bootstrap fake adapter must reject unexpected commands and ambiguous result forms"
    );
    assert_eq!(
        BOOTSTRAP_CORE_SRC.matches("'d1_migrations'").count(),
        1,
        "only the D1 migration ledger may be exempt from application freshness checks"
    );
    assert!(
        PEPPER_CONFIGURATION_TEST_SRC.contains("invalidPhase('missing'")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("rotationInvalidates")
            && PEPPER_CONFIGURATION_TEST_SRC.contains("nonMutation"),
        "the isolated runtime gate must cover missing/invalid secrets, mutation safety, and key-change semantics"
    );
    for (name, source) in RFC077_EXECUTABLE_GATE_SOURCES {
        assert!(
            source.contains("prepareIsolatedWorkerTest"),
            "{name} must use the shared isolated Worker fixture"
        );
        assert!(
            !source.contains("dev-pepper-change-in-production") && !source.contains("dev-pepper"),
            "{name} must not embed a legacy pepper sentinel"
        );
    }
    assert!(
        !PACKAGE_JSON_SRC.contains("rfc077") && !PACKAGE_JSON_SRC.contains("rfc-077"),
        "development commands must use domain/role/function names, not RFC numbers"
    );
}

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
const OPERATOR_HANDLER_SRC: &str = include_str!("../../../workers/ssr/src/handlers/operator.rs");
const MEMBERSHIP_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/membership.rs");
const RELINK_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/relink.rs");
const RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC: &str =
    include_str!("../../../scripts/recover-community-access.mjs");
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
        MEMBERSHIP_DB_SRC.contains("AuditAction::MembershipPromotedToAdmin")
            && MEMBERSHIP_DB_SRC.contains("AuditAction::MembershipDemotedToMember")
            && MEMBERSHIP_DB_SRC.matches("AuditMetadata::None").count() >= 2,
        "RFC-062 role changes must audit direction by action name without extra metadata"
    );
}

#[test]
fn rfc062_role_transfer_writes_are_scoped_and_guarded() {
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn promote_to_admin_required")
            && MEMBERSHIP_DB_SRC.contains("SET role = 'admin'")
            && MEMBERSHIP_DB_SRC.contains("id = ?1")
            && MEMBERSHIP_DB_SRC.contains("community_id = ?2")
            && MEMBERSHIP_DB_SRC.contains("removed_at IS NULL")
            && MEMBERSHIP_DB_SRC.contains("role = 'member'"),
        "RFC-062 promote update must be scoped by membership id, community id, active membership, and current role"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn demote_to_member_required")
            && MEMBERSHIP_DB_SRC.contains("SET role = 'member'")
            && MEMBERSHIP_DB_SRC.contains("role = 'admin'")
            && MEMBERSHIP_DB_SRC.contains("SELECT COUNT(*) FROM community_memberships")
            && MEMBERSHIP_DB_SRC.contains("> 1"),
        "RFC-062 demote update must re-check active admin count in the conditional write"
    );
    assert!(
        MEMBERSHIP_DB_SRC.contains("pub async fn soft_remove_guarded_required")
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
        .find("invite_db::insert_required(")
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
            && JOIN_HANDLER_SRC.contains("crate::db::invite::redeem_required(")
            && INVITE_DB_SRC.contains("INSERT INTO users")
            && INVITE_DB_SRC.contains("INSERT INTO community_memberships"),
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
    assert!(
        JA_ADMIN_HELP_SIGNIN_RELINK_HINT.contains("招待コード欄では使えません。")
            && JA_ADMIN_HELP_SIGNIN_RELINK_LINK == "サインインし直す画面を開く"
            && HELP_SIGNIN_HANDLER_SRC.contains("href=\\\"/relink\\\"")
            && JOIN_HANDLER_SRC.contains("href=\\\"/relink\\\"")
            && RENDER_SRC.contains("href=\\\"/relink\\\"")
            && HELP_SIGNIN_HANDLER_SRC.contains("data-copy-code-button")
            && HELP_SIGNIN_HANDLER_SRC.contains("data-copy-code-value")
            && APP_JS_SRC.contains("navigator.clipboard.writeText")
            && JA_JOIN_RELINK_LINK == JA_ADMIN_HELP_SIGNIN_RELINK_LINK
            && HELP_SIGNIN_HANDLER_SRC.contains("rel=\\\"noopener\\\""),
        "RFC-024 help-signin result must direct members to /relink and distinguish it from invite-code entry"
    );
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
            || HELP_SIGNIN_HANDLER_SRC.contains("hmac_hex(&pepper")
            || HELP_SIGNIN_HANDLER_SRC.contains("hmac_hex(pepper.as_str()"),
        "RFC-024 codes must be HMAC hashed before storage"
    );
    assert!(
        RELINK_DB_SRC.contains("pub async fn issue_required")
            && RELINK_DB_SRC.contains("revoked_at=?1")
            && HELP_SIGNIN_HANDLER_SRC.contains("relink_db::issue_required"),
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
            && INVITE_DB_SRC.contains("INSERT INTO community_memberships"),
        "RFC-024 invite-era help-signin relies on join minting a fresh user_id and membership per invite redemption"
    );
}

#[test]
fn rfc024_redemption_is_single_use_generic_and_revokes_old_sessions() {
    assert!(
        RELINK_DB_SRC.contains("pub async fn redeem_required")
            && RELINK_DB_SRC.contains("used_at IS NULL")
            && RELINK_HANDLER_SRC.contains("relink_db::redeem_required"),
        "RFC-024 redemption must mark codes used with a conditional single-use update"
    );
    assert!(
        RELINK_HANDLER_SRC.contains("JA_RELINK_INVALID")
            && !RELINK_HANDLER_SRC.contains("already used")
            && !RELINK_HANDLER_SRC.contains("wrong community"),
        "RFC-024 redemption failures must use one generic error"
    );
    assert!(
        RELINK_DB_SRC.contains("UPDATE sessions SET revoked_at=?1")
            && RELINK_DB_SRC.contains("id!=?3")
            && RELINK_DB_SRC.contains("EXISTS (SELECT 1 FROM sessions keep"),
        "RFC-024 redemption must revoke other active sessions for the target user after inserting the new session"
    );
    assert!(
        RELINK_HANDLER_SRC.contains("rate_limit::is_relink_rate_limited")
            && RELINK_HANDLER_SRC.contains("record_relink_failure")
            && !RELINK_HANDLER_SRC.contains("write_legacy")
            && RELINK_DB_SRC.contains("AuditAction::MembershipRelinkRedeemed"),
        "RFC-024 failed redemption should be rate-limited, not audited as a membership event"
    );
}

#[test]
fn rfc069_operator_recovery_is_disabled_and_secret_authorized_by_default() {
    assert!(
        LIB_SRC.contains("(Method::Post, \"/operator/recovery/community-access\")"),
        "RFC-069 operator recovery must use the reviewed POST-only route"
    );
    assert!(
        WRANGLER_TOML_SRC.contains("COMMUNITY_RECOVERY_ENABLED = \"false\"")
            && !WRANGLER_TOML_SRC.contains("COMMUNITY_RECOVERY_ENABLED = \"true\""),
        "Tracked wrangler.toml must keep RFC-069 recovery disabled by default"
    );
    assert!(
        !WRANGLER_TOML_SRC.contains("COMMUNITY_RECOVERY_TOKEN ="),
        "Tracked wrangler.toml must not contain the operator recovery bearer token"
    );
    assert!(
        OPERATOR_HANDLER_SRC.contains("env.secret(\"COMMUNITY_RECOVERY_TOKEN\")")
            && !OPERATOR_HANDLER_SRC.contains("env.var(\"COMMUNITY_RECOVERY_TOKEN\")")
            && OPERATOR_HANDLER_SRC.contains("constant_time_eq")
            && OPERATOR_HANDLER_SRC.contains("Authorization")
            && OPERATOR_HANDLER_SRC.contains("\"Bearer \""),
        "RFC-069 endpoint must read the token only as a secret and compare the bearer in constant time"
    );
}

#[test]
fn rfc069_operator_recovery_targets_existing_active_admins_only() {
    assert!(
        OPERATOR_HANDLER_SRC.contains("community_db::find_active")
            && OPERATOR_HANDLER_SRC.contains("membership_db::find_active_by_id")
            && OPERATOR_HANDLER_SRC.contains("target.role == \"admin\"")
            && OPERATOR_HANDLER_SRC.contains("render::not_found()"),
        "RFC-069 endpoint must converge invalid community/membership/non-admin cases on generic not-found"
    );
    assert!(
        OPERATOR_HANDLER_SRC.contains("relink_db::issue_required")
            && RELINK_DB_SRC.contains("INSERT INTO membership_relink_codes")
            && RELINK_DB_SRC.contains("audit::required_record")
            && RELINK_DB_SRC.contains("audit::execute_required_tail")
            && !OPERATOR_HANDLER_SRC.contains("let _ = audit::write")
            && !OPERATOR_HANDLER_SRC.contains("audit::write("),
        "RFC-069 endpoint must batch relink-code mutation with required audit evidence and must not discard audit errors"
    );
}

#[test]
fn rfc069_operator_recovery_audit_correlates_creation_and_redemption() {
    assert!(
        RELINK_DB_SRC.contains("AuditAction::OperatorRecoveryAdminRelinkCreated")
            && RELINK_DB_SRC.contains("AuditMetadata::OperatorRecovery")
            && OPERATOR_HANDLER_SRC.contains("Some(body.operator_label)")
            && RELINK_DB_SRC.contains("relink_code_id: id.to_owned()")
            && !RELINK_DB_SRC.contains("let metadata = serde_json::json!"),
        "RFC-069 creation audit must include bounded operator label and relink correlation metadata"
    );
    assert!(
        RELINK_DB_SRC.contains("AuditAction::MembershipRelinkRedeemed")
            && RELINK_DB_SRC.contains("relink_code_id: target.id.clone()"),
        "RFC-069 redemption audit must include the relink_code_id created by the operator endpoint"
    );
}

#[test]
fn rfc069_operator_tool_requires_explicit_target_and_production_confirmation() {
    assert!(
        RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("COMMUNITY_RECOVERY_TOKEN")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("process.env.COMMUNITY_RECOVERY_TOKEN")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--target")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--url")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--community-id")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--admin-membership-id")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--operator-label"),
        "RFC-069 operator tool must require explicit environment, URL, target IDs, label, and env token"
    );
    assert!(
        RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("--confirm-production")
            && RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("Type \"production\"")
            && !RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("writeFile")
            && !RECOVER_COMMUNITY_ACCESS_SCRIPT_SRC.contains("appendFile"),
        "RFC-069 operator tool must require production confirmation and avoid plaintext-code evidence files"
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
            && COMMUNITY_DB_SRC.contains("execute_required_batch")
            && COMMUNITY_DB_SRC.contains("AuditAction::CommunityCreated")
            && COMMUNITY_DB_SRC.contains("AuditAction::MembershipCreatedFirstAdmin"),
        "Community creation must batch community, first-admin membership, and audit writes"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("community.created")
            && RFC079_AUDIT_CORE_SRC.contains("membership.created_first_admin"),
        "Community creation must emit the reviewed audit events"
    );
    assert!(
        !COMMUNITY_DB_SRC.contains("INSERT INTO audit_log")
            && COMMUNITY_DB_SRC.matches("AuditMetadata::None").count() == 2,
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
fn rfc070_self_display_name_editing_routes_are_member_scoped() {
    assert!(
        COMMUNITY_HANDLER_SRC.contains("\"me/display-name\"")
            && COMMUNITY_HANDLER_SRC.contains("get_display_name")
            && COMMUNITY_HANDLER_SRC.contains("post_display_name"),
        "RFC-070 must expose GET/POST /c/:cid/me/display-name through the community router"
    );
    assert!(
        ME_HANDLER_SRC.contains("require_membership(env, &auth, community_id)")
            && !ME_HANDLER_SRC.contains("require_admin(env, &auth, community_id)"),
        "RFC-070 display-name editing must require active membership, not admin role"
    );
    assert!(
        ME_HANDLER_SRC.contains("JA_ME_CHANGE_DISPLAY_NAME")
            && ME_HANDLER_SRC.contains("JA_ME_DISPLAY_NAME_UPDATED")
            && ME_HANDLER_SRC.contains("?flash={DISPLAY_NAME_UPDATED_REF}"),
        "RFC-070 Me page must expose the edit link and fixed-code success feedback"
    );
}

#[test]
fn rfc070_display_name_token_replay_and_validation_are_guarded() {
    assert!(
        ME_HANDLER_SRC.contains("token_purpose::CHANGE_DISPLAY_NAME")
            && ME_HANDLER_SRC.contains("validate_display_name(&raw_display_name)")
            && ME_HANDLER_SRC.contains("consume_detailed")
            && ME_HANDLER_SRC.contains("ConsumeResult::Replay(Some(result_ref))")
            && ME_HANDLER_SRC.contains("DISPLAY_NAME_UPDATED_REF")
            && ME_HANDLER_SRC.contains("DISPLAY_NAME_UNCHANGED_REF"),
        "RFC-070 must validate before detailed token consume and branch replays by stored result_ref"
    );
    assert!(
        ME_HANDLER_SRC.contains("form_token::set_result")
            && ME_HANDLER_SRC.contains("DISPLAY_NAME_UNCHANGED_REF")
            && ME_HANDLER_SRC.contains("return redirect(&format!(\"/c/{community_id}/me\"))"),
        "RFC-070 same-value no-op must store display_name_unchanged before redirecting"
    );
    assert!(
        ME_HANDLER_SRC.contains("ConsumeResult::Replay(_)")
            && !ME_HANDLER_SRC.contains("if replay.is_some()"),
        "RFC-070 must not use the older replay-is-some pattern that misses consumed tokens with no result_ref"
    );
}

#[test]
fn rfc070_display_name_update_audit_and_result_are_batched() {
    assert!(
        ME_HANDLER_SRC.contains("db.batch(vec![update_stmt, audit_stmt, result_stmt])")
            && ME_HANDLER_SRC.contains("UPDATE community_memberships")
            && ME_HANDLER_SRC.contains("SET display_name = ?1")
            && ME_HANDLER_SRC.contains("AND community_id = ?3")
            && ME_HANDLER_SRC.contains("AND user_id = ?4")
            && ME_HANDLER_SRC.contains("AND removed_at IS NULL")
            && ME_HANDLER_SRC.contains("AND display_name != ?1"),
        "RFC-070 display-name update must be scoped to active membership, community, and authenticated user"
    );
    assert!(
        ME_HANDLER_SRC.contains("AuditAction::MembershipDisplayNameUpdated")
            && ME_HANDLER_SRC.contains("AuditMetadata::DisplayNameChanged")
            && ME_HANDLER_SRC.contains("statement_after_one_change")
            && ME_HANDLER_SRC.contains("UPDATE form_tokens")
            && ME_HANDLER_SRC.contains("result_ref = ?1")
            && ME_HANDLER_SRC.contains("require_changed(&results, 0")
            && ME_HANDLER_SRC.contains("require_changed(&results, 1")
            && ME_HANDLER_SRC.contains("require_changed(&results, 2"),
        "RFC-070 actual changes must batch update, audit insert, replay-result storage, and check each write"
    );
    assert!(
        !ME_HANDLER_SRC.contains("let _ = audit::write")
            && !ME_HANDLER_SRC.contains("audit::write("),
        "RFC-070 display-name updates must not use best-effort audit::write"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC
            .contains("DisplayNameChanged => json!({ \"changed_fields\": [\"display_name\"] })")
            && !ME_HANDLER_SRC.contains("serde_json::json!")
            && !ME_HANDLER_SRC.contains("INSERT INTO audit_log"),
        "RFC-070 metadata_json must keep IDs in audit columns and store only the changed field list"
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
        RENDER_SRC
            .contains("/static/app.js?v=0.59.0-rfc056-rfc065-rfc066-rfc067-rfc068-rfc064-rfc069")
            && STATIC_FILES_SRC.contains(
                "/static/app.js?v=0.59.0-rfc056-rfc065-rfc066-rfc067-rfc068-rfc064-rfc069"
            ),
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
    assert!(
        CALENDAR_HANDLER_SRC.contains("cal_db::rotate_required")
            && CALENDAR_HANDLER_SRC.contains("cal_db::revoke_required")
            && CALENDAR_DB_SRC.contains("AuditAction::CalendarFeedTokenGenerated")
            && CALENDAR_DB_SRC.contains("AuditAction::CalendarFeedTokenRevoked")
            && CALENDAR_DB_SRC.contains("AuditMetadata::None")
            && !CALENDAR_DB_SRC.contains("LegacyAuditAction"),
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
            && RFC079_EVENT_WRITE_DB_SRC.contains("edit_scope"),
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
            && ADMIN_EVENTS_SRC.contains("EventCreationMode::CancelledRecreate")
            && ADMIN_EVENTS_SRC.contains("source_event_id: Some(source_id.clone())"),
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
            && ADMIN_EVENTS_SRC.contains("EventCreationMode::CancelledRecreate")
            && ADMIN_EVENTS_SRC.contains("EventCreationMode::EventCopy")
            && ADMIN_EVENTS_SRC.contains("source_event_id: Some(source_id.clone())"),
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
const INVITE_SMOKE_SRC: &str = include_str!("../../../scripts/smoke/invite-redemption.mjs");

#[test]
fn rfc076_one_time_invite_response_isolation_is_pinned() {
    let production = MEMBERS_HANDLER_SRC
        .split("#[cfg(test)]\nmod tests")
        .next()
        .expect("members handler production source must precede tests");
    assert!(
        !production.contains("invites?code=")
            && !production.contains("query_pairs().find(|(k, _)| k == \"code\")"),
        "production must never create or render a plaintext invite-code query handoff"
    );

    let get = compact_brace_block(production, "pub async fn get_invites");
    assert!(
        get.contains("leturl=req.url()?")
            && get.contains("run_invite_get_preflight(invite_get_preflight(")
            && get.contains("ControlFlow::Break(location)=>legacy_query_redirect(&location)")
            && get.contains("get_invites_authenticated(req,env,community_id,flash)")
            && !get.contains("require_auth")
            && !get.contains("require_admin")
            && !get.contains("env.d1")
            && !get.contains("issue_token")
            && !get.contains("render_invites_page"),
        "legacy code-query containment must run before every authenticated/application continuation"
    );
    let authenticated = compact_brace_block(production, "async fn get_invites_authenticated");
    assert!(
        authenticated.contains("require_auth")
            && authenticated.contains("require_admin")
            && authenticated.contains("render_invites_page"),
        "clean invite GET must delegate all authenticated work to the private continuation"
    );
    let legacy_redirect = compact_brace_block(production, "fn legacy_query_redirect");
    assert!(
        legacy_redirect.contains("with_status(303)")
            && legacy_redirect.contains("set(\"Location\",location)")
            && legacy_redirect.contains("set(\"Referrer-Policy\",\"no-referrer\")"),
        "legacy query containment must emit a clean 303 with no-referrer"
    );

    let canonical = compact_brace_block(production, "fn canonical_invites_path");
    assert!(
        canonical.contains("byte.is_ascii_alphanumeric()")
            && canonical.contains("matches!(byte,b'_'|b'-')")
            && canonical.contains("\"%{byte:02X}\"")
            && canonical.contains("\"/c/{encoded}/admin/invites\""),
        "canonical invite path must use the reviewed byte allowlist and uppercase percent encoding"
    );
    let runner = compact_brace_block(production, "fn run_invite_get_preflight");
    assert!(
        runner.contains("InviteGetPreflight::Continue=>ControlFlow::Continue(continuation())")
            && runner.contains(
                "InviteGetPreflight::CanonicalRedirect(location)=>ControlFlow::Break(location)"
            ),
        "preflight runner must make the early-return continuation boundary executable"
    );

    assert!(
        production.contains("struct InviteCodeReveal(String);")
            && !production.contains("derive(Debug")
            && !production.contains("impl std::fmt::Display for InviteCodeReveal")
            && !production.contains("Serialize for InviteCodeReveal"),
        "plaintext reveal must remain a narrow private non-formatting/non-serializing type"
    );
    let reveal = compact_brace_block(production, "fn invite_reveal_html");
    assert!(
        reveal.contains("render::escape_html(reveal.as_str())")
            && reveal.contains("JA_ADMIN_INVITES_REVEAL_WARNING")
            && !reveal.contains("data-")
            && !reveal.contains("<script"),
        "reveal renderer must place escaped plaintext only in the dedicated text panel"
    );

    let post = compact_brace_block(production, "pub async fn post_generate_invite");
    assert!(
        post.contains("returnredirect(&canonical_invites_path(community_id))")
            && post.contains("letcreated=matchinvite_db::insert_required(")
            && post.contains("Err(_)=>returnrender::service_unavailable()")
            && post.contains("consume_detailed(")
            && post.contains("matches!(consume,ConsumeResult::Replay(_))")
            && post.contains("InviteCodeReveal::new(code)")
            && post.contains("letmutresponse=matchrender_invites_page(")
            && post
                .matches("Err(_)=>returnrender::service_unavailable()")
                .count()
                == 2
            && post.contains("set(\"Cache-Control\",\"no-store,private\")")
            && post.contains("set(\"Referrer-Policy\",\"no-referrer\")")
            && !post.contains("Location")
            && !post.contains("?code="),
        "first generation must render directly with strict headers; replay must use a clean canonical 303"
    );
    assert!(
        INVITE_DB_SRC.contains("AuditAction::InviteCodeGenerated")
            && INVITE_DB_SRC.contains("audit::execute_required(db, mutation, &record)")
            && post.contains("\"member\""),
        "RFC-076 must preserve the central RFC-079 Class A batch and member-only grant"
    );
    assert!(
        MEMBERS_HANDLER_SRC
            .contains("legacy_query_preflight_never_invokes_authenticated_continuation")
            && MEMBERS_HANDLER_SRC
                .contains("code_query_preflight_matches_empty_repeated_and_encoded_keys")
            && MEMBERS_HANDLER_SRC
                .contains("canonical_invite_path_encodes_every_non_allowlisted_byte")
            && MEMBERS_HANDLER_SRC.contains("reveal_html_contains_code_once_and_only_as_text"),
        "focused native tests must pin early ordering, query decoding, path containment, and reveal placement"
    );

    for required in [
        "legacy-code-query-is-contained-before-authentication",
        "redirectRequestHasNoReferrer",
        "admin-generated-invite-is-stored-and-shown-once",
        "directStatus",
        "noStorePrivate",
        "codeFreeBrowserUrl",
        "codeAppearsOnce",
        "consumed-generation-token-replay-is-clean-and-non-mutating",
        "required-audit-failure-rolls-back-and-discloses-nothing",
        "proof_fail_invite_generation_audit",
        "exactlyOneCentralEvent",
        "audit.required_batch_failed",
        "failure_category=storage route_class=class_a",
        "generation-recovers-after-local-trigger-cleanup",
        "DROP TRIGGER IF EXISTS proof_fail_invite_generation_audit",
    ] {
        assert!(
            INVITE_SMOKE_SRC.contains(required),
            "RFC-076 smoke must retain required assertion/evidence marker: {required}"
        );
    }
    for forbidden in [
        "inviteCodeFromLocation",
        "screenshot(adminPage, 'admin-generated-invite-is-stored-and-shown-once')",
        "observed:",
        "inviteRows:",
        "values:",
    ] {
        assert!(
            !INVITE_SMOKE_SRC.contains(forbidden),
            "RFC-076 smoke/report must not retain plaintext-sensitive marker: {forbidden}"
        );
    }
}

// ── RFC-079 Package 0A audit inventory gate ─────────────────────────────

const RFC079_ASSERTION_FIXTURE_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit_change_assertion.sql");
const RFC079_ASSERTION_WORKER_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit-assertion-worker.mjs");
const RFC079_ASSERTION_RUNNER_SRC: &str = include_str!("../../../scripts/test-audit-assertion.mjs");
const RFC079_AUDIT_CORE_SRC: &str = include_str!("../../../workers/ssr/src/audit.rs");
const RFC079_MIGRATION_SRC: &str = include_str!("../../../migrations/0010_audit_integrity.sql");
const RFC079_MIGRATION_RUNNER_SRC: &str = include_str!("../../../scripts/test-audit-migration.mjs");
const RFC079_AUDIT_POLICY_SRC: &str = include_str!("../../../docs/src/maintainer/audit-policy.md");
const RFC079_BACKUP_RECOVERY_SRC: &str =
    include_str!("../../../docs/src/maintainer/backup-recovery.md");
const RFC079_DEPLOYMENT_SRC: &str = include_str!("../../../docs/src/shared/deployment.md");
const RFC079_OPERATIONS_SRC: &str = include_str!("../../../docs/src/maintainer/operations.md");
const RFC079_RFC014_SRC: &str =
    include_str!("../../../rfcs/done/014-observability-audit-and-privacy-logging.md");
const RFC079_RFC052_SRC: &str =
    include_str!("../../../rfcs/done/052-audit-retention-and-operator-access-policy.md");
const RFC079_RFC071_SRC: &str = include_str!(
    "../../../rfcs/proposed/071-application-threat-model-and-form-security-baseline.md"
);
const RFC079_RFC050_SRC: &str =
    include_str!("../../../rfcs/proposed/050-staging-runtime-verification-evidence-pack.md");
const RFC079_THREAT_MODEL_SRC: &str =
    include_str!("../../../docs/src/developer/security-threat-model.md");
const RFC079_ARCHITECTURE_SRC: &str = include_str!("../../../docs/src/developer/architecture.md");
const RFC079_RELEASE_CHECKLIST_SRC: &str =
    include_str!("../../../docs/src/tester/release-checklist.md");
const RFC079_ATTENDANCE_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/attendance.rs");
const RFC079_EVENT_WRITE_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/event_write.rs");
const RFC079_EVENT_NOTE_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/event_note.rs");
const RFC079_EVENT_TEMPLATE_DB_SRC: &str =
    include_str!("../../../workers/ssr/src/db/event_template.rs");
const RFC079_TEMPLATES_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/templates.rs");
const RFC079_NOTE_HANDLER_SRC: &str =
    include_str!("../../../workers/ssr/src/handlers/admin/events/notes.rs");
const RFC079_ATOMICITY_RUNNER_SRC: &str = include_str!("../../../scripts/test-audit-atomicity.mjs");
const RFC079_ATOMICITY_WORKER_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit-atomicity-worker.mjs");
const RFC079_ATOMICITY_FIXTURE_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit_atomicity.sql");
const RFC079_BOUNDARY_RUNNER_SRC: &str = include_str!("../../../scripts/test-audit-boundaries.mjs");
const RFC079_CLASS_A_FAILURE_RUNNER_SRC: &str =
    include_str!("../../../scripts/test-audit-class-a-failures.mjs");
const RFC079_BOUNDARY_WORKER_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit-boundaries-worker.mjs");
const RFC079_BOUNDARY_FIXTURE_SRC: &str =
    include_str!("../../../workers/ssr/tests/fixtures/audit_response_boundaries.sql");
const RFC079_COMMUNITY_DB_SRC: &str = include_str!("../../../workers/ssr/src/db/community.rs");
const PACKAGE_JSON_SRC: &str = include_str!("../../../package.json");

#[derive(Default)]
struct AuditSourceScan {
    direct_inserts: std::collections::BTreeMap<String, usize>,
    assertion_table_refs: Vec<String>,
    operation_id_refs: Vec<String>,
    compatibility_refs: Vec<String>,
    ignored_audit_results: Vec<String>,
    background_audit_refs: Vec<String>,
}

fn compact_source(value: &str) -> String {
    value.split_whitespace().collect()
}

fn compact_brace_block(source: &str, marker: &str) -> String {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing source marker {marker:?}"));
    let open = source[start..]
        .find('{')
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("source marker {marker:?} has no opening brace"));
    let mut depth = 0usize;
    let mut end = None;
    for (offset, ch) in source[open..].char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth
                .checked_sub(1)
                .expect("brace parser encountered an unmatched closing brace");
            if depth == 0 {
                end = Some(open + offset + ch.len_utf8());
                break;
            }
        }
    }
    let end = end.unwrap_or_else(|| panic!("source marker {marker:?} has unbalanced braces"));
    compact_source(&source[start..end])
}

fn collect_rust_sources(
    root: &std::path::Path,
    directory: &std::path::Path,
    output: &mut Vec<(String, String)>,
) {
    let mut entries = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to scan {}: {error}", directory.display()))
        .map(|entry| entry.expect("Rust source directory entry must be readable"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .expect("Rust source entry type must be readable");
        if file_type.is_dir() {
            collect_rust_sources(root, &path, output);
        } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            let relative = path
                .strip_prefix(root)
                .expect("scanned Rust source must stay under workers/ssr/src")
                .to_string_lossy()
                .replace('\\', "/");
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
            output.push((relative, source));
        }
    }
}

fn scan_audit_sources(sources: &[(String, String)]) -> AuditSourceScan {
    let mut scan = AuditSourceScan::default();
    for (path, source) in sources {
        let direct_count = source.matches("INSERT INTO audit_log").count();
        if direct_count > 0 {
            scan.direct_inserts.insert(path.clone(), direct_count);
        }
        if source.contains("audit_change_assertions") {
            scan.assertion_table_refs.push(path.clone());
        }
        if source.contains("operation_id") {
            scan.operation_id_refs.push(path.clone());
        }
        if source.contains("LegacyAuditAction")
            || source.contains("LegacyAuditMetadata")
            || source.contains("write_legacy")
        {
            scan.compatibility_refs.push(path.clone());
        }
        if source.contains("let _ = audit::") || source.contains("let _ = crate::audit::") {
            scan.ignored_audit_results.push(path.clone());
        }
        let compact = compact_source(source);
        if (compact.contains("waitUntil")
            || compact.contains("wait_until")
            || compact.contains("spawn_local"))
            && compact.contains("audit")
        {
            scan.background_audit_refs.push(path.clone());
        }
    }
    scan
}

fn audit_action_owners<'a>(sources: &'a [(String, String)], variant: &str) -> Vec<&'a str> {
    let needle = format!("AuditAction::{variant}");
    sources
        .iter()
        .filter(|(path, source)| {
            path != "audit.rs" && !source.contains("LegacyAuditAction") && source.contains(&needle)
        })
        .map(|(path, _)| path.as_str())
        .collect()
}

#[test]
fn rfc079_class_a_failure_telemetry_is_centrally_and_exhaustively_owned() {
    let production_core = RFC079_AUDIT_CORE_SRC
        .split("#[cfg(test)]\nmod tests")
        .next()
        .expect("audit.rs production source must precede its test module");
    assert_eq!(
        production_core
            .matches("audit.required_batch_failed")
            .count(),
        1,
        "audit.rs must own exactly one production Class A event literal"
    );
    for required in [
        "RequiredBatch",
        "\"class_a\"",
        "INVALID_REQUEST_ID",
        "\"invalid_request_id\"",
        "safe_event_request_id",
        "required_record_with_sink",
        "owned_record_with_sink",
        "emit_failure_with",
        "log_class_a_failure",
    ] {
        assert!(
            production_core.contains(required),
            "Class A failure owner is missing {required}"
        );
    }

    let classification = compact_brace_block(production_core, "pub(crate) const fn is_class_a");
    let class_a_actions = [
        "CommunityCreated",
        "MembershipCreatedFirstAdmin",
        "MembershipDisplayNameUpdated",
        "InviteCodeGenerated",
        "InviteCodeRevoked",
        "InviteCodeRedeemed",
        "MembershipRelinkCodeCreated",
        "MembershipRelinkRedeemed",
        "OperatorRecoveryAdminRelinkCreated",
        "MembershipRemoved",
        "MembershipPromotedToAdmin",
        "MembershipDemotedToMember",
        "EventCreated",
        "EventEdited",
        "EventCancelled",
        "EventOccurrenceCancelled",
        "AttendanceAdminOverride",
        "AttendanceAdminSetAttended",
        "EventNoteAdminHidden",
        "CalendarFeedTokenGenerated",
        "CalendarFeedTokenRevoked",
        "EventTemplateCreated",
        "EventTemplateDeleted",
    ];
    for action in class_a_actions {
        assert!(
            classification.contains(&format!("Self::{action}")),
            "Class A classification lost {action}"
        );
    }
    for action in [
        "CommunityExportAuthorized",
        "CalendarMatrixCsvExportRequested",
        "SessionLogout",
    ] {
        assert!(
            !classification.contains(&format!("Self::{action}")),
            "Class B/C action {action} must not classify as Class A"
        );
    }

    let executors = [
        "execute_required(",
        "execute_required_bounded(",
        "execute_required_attendance_override(",
        "execute_required_batch(",
        "execute_required_tail(",
        "execute_asserted_required(",
    ];
    assert_eq!(
        production_core
            .matches("pub(crate) async fn execute_")
            .count(),
        executors.len(),
        "a new central audit executor must be added to the exhaustive failure-ownership gate"
    );
    for executor in executors {
        let block = compact_brace_block(production_core, executor);
        assert!(
            block.contains("log_class_a_failure"),
            "{executor} must route failures through the central Class A owner"
        );
        assert!(
            block.contains("AuditFailureCategory::Construction")
                && block.contains("AuditFailureCategory::Storage")
                && block.contains("db.batch"),
            "{executor} must cover setup, D1, and post-D1 failure phases"
        );
        assert!(
            !block.contains("?;"),
            "{executor} contains a bare fallible escape that can bypass telemetry"
        );
    }

    let class_b = compact_brace_block(production_core, "pub(crate) async fn write_pre_disclosure");
    let class_c = compact_brace_block(
        production_core,
        "pub(crate) async fn write_logout_secondary",
    );
    assert!(
        class_b.contains("owned_record_with_sink")
            && class_b.contains("AuditFailureEvent::PreDisclosure")
            && !class_b.contains("RequiredBatch"),
        "Class B must retain exclusive PreDisclosure failure ownership"
    );
    assert!(
        class_c.contains("owned_record_with_sink")
            && class_c.contains("AuditFailureEvent::SecondaryWrite")
            && !class_c.contains("RequiredBatch"),
        "Class C must retain exclusive SecondaryWrite failure ownership"
    );

    let formatter = compact_brace_block(production_core, "fn format_failure_event");
    assert!(
        formatter.contains("safe_event_request_id(request_id)")
            && !formatter.contains("metadata")
            && !formatter.contains("community_id")
            && !formatter.contains("actor_membership_id")
            && !formatter.contains("target_id")
            && !formatter.contains("error")
            && !formatter.contains("sql")
            && !formatter.contains("bind"),
        "failure formatter must use the safe request-ID boundary and only bounded fields"
    );

    let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../workers/ssr/src")
        .canonicalize()
        .expect("workers/ssr/src must exist for the Class A ownership scan");
    let mut sources = Vec::new();
    collect_rust_sources(&source_root, &source_root, &mut sources);
    let outside_owners = sources
        .iter()
        .filter(|(path, source)| {
            path != "audit.rs"
                && (source.contains("audit.required_batch_failed")
                    || source.contains("AuditFailureEvent::RequiredBatch")
                    || source.contains("log_class_a_failure"))
        })
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    assert!(
        outside_owners.is_empty(),
        "Class A failure telemetry escaped audit.rs: {outside_owners:?}"
    );

    let community = compact_source(RFC079_COMMUNITY_DB_SRC);
    assert!(
        community.contains("letprimary_audit=audit::required_record(")
            && community.contains("AuditAction::CommunityCreated")
            && community.contains("letmembership_audit=audit::required_record(")
            && community.contains("AuditAction::MembershipCreatedFirstAdmin")
            && community.contains("&primary_audit,&[membership_audit]"),
        "community creation must pin community.created as primary and membership.created_first_admin as additional"
    );

    let runner = compact_source(RFC079_CLASS_A_FAILURE_RUNNER_SRC);
    for required in [
        "prepareIsolatedWorkerTest(",
        "isolated.spawnDev(",
        "/c/${communityId}/admin/invites",
        "/communities/new",
        "/join/profile",
        "audit.required_batch_failed",
        "DROPTRIGGERIFEXISTS",
        "awaitisolated.cleanup()",
    ] {
        assert!(
            runner.contains(required),
            "compiled Class A proof runner lost containment or route evidence: {required}"
        );
    }
    let isolated = compact_source(ISOLATED_WORKER_TEST_SRC);
    for required in [
        "mkdtemp(",
        "--persist-to",
        "wrangler.toml",
        "'dev'",
        "'--local'",
        "'127.0.0.1'",
        "awaitrm(container,{recursive:true,force:true",
    ] {
        assert!(
            isolated.contains(required),
            "shared compiled-Worker fixture lost containment evidence: {required}"
        );
    }
    for forbidden in [
        "--remote",
        "audit-atomicity-worker.mjs",
        "audit-assertion-worker.mjs",
        "writeFile(",
        "copyFile(",
    ] {
        if forbidden == "--remote" {
            assert_eq!(
                RFC079_CLASS_A_FAILURE_RUNNER_SRC.matches(forbidden).count(),
                1,
                "the only --remote occurrence must remain the refusal guard"
            );
        } else {
            assert!(
                !RFC079_CLASS_A_FAILURE_RUNNER_SRC.contains(forbidden),
                "compiled Class A proof must not create or reuse a synthetic Worker: {forbidden}"
            );
        }
    }
    assert!(
        PACKAGE_JSON_SRC.contains(
            "\"test:audit-class-a-failures\": \"node scripts/test-audit-class-a-failures.mjs\""
        ),
        "package.json must expose the domain-named Class A proof command"
    );
}

#[test]
fn rfc079_package0a_current_audit_inventory_is_pinned() {
    let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../workers/ssr/src")
        .canonicalize()
        .expect("workers/ssr/src must exist for the RFC-079 repository-wide source gate");
    let mut sources = Vec::new();
    collect_rust_sources(&source_root, &source_root, &mut sources);
    assert!(
        sources.len() >= 78,
        "RFC-079 source gate scanned too few Rust files; repository root may be wrong"
    );
    let scan = scan_audit_sources(&sources);

    let expected_direct_inserts =
        std::collections::BTreeMap::from([("audit.rs".to_owned(), 1usize)]);
    assert_eq!(
        scan.direct_inserts, expected_direct_inserts,
        "repository-wide audit INSERT distribution changed; audit.rs must remain the sole production audit INSERT owner"
    );

    assert!(
        scan.compatibility_refs.is_empty()
            && scan.ignored_audit_results.is_empty()
            && scan.background_audit_refs.is_empty(),
        "Package 7 removal boundary rejects compatibility, ignored-result, and background audit surfaces: compatibility={:?}, ignored={:?}, background={:?}",
        scan.compatibility_refs,
        scan.ignored_audit_results,
        scan.background_audit_refs,
    );
    assert!(
        scan.assertion_table_refs == vec!["audit.rs".to_owned()]
            && scan.operation_id_refs == vec!["audit.rs".to_owned()],
        "Package 7 assertion table/operation IDs must be owned only by audit.rs: table={:?}, operation_id={:?}",
        scan.assertion_table_refs,
        scan.operation_id_refs
    );

    let source_by_path = sources
        .iter()
        .map(|(path, source)| (path.as_str(), compact_source(source)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let role_source = sources
        .iter()
        .find(|(path, _)| path == "handlers/admin/role_transfer.rs")
        .map(|(_, source)| source.as_str())
        .expect("role-transfer audit source must exist");
    assert_eq!(
        compact_brace_block(role_source, "enum RoleMutation"),
        "enumRoleMutation{Promote,Demote,}",
        "RoleMutation variants changed; reconcile the accepted dynamic audit inventory"
    );
    for required in [
        "AuditAction::MembershipPromotedToAdmin",
        "AuditAction::MembershipDemotedToMember",
    ] {
        assert!(
            source_by_path["db/membership.rs"].contains(&compact_source(required)),
            "role audit-action mapping is missing {required}"
        );
    }
    let class_a = [
        "community.created",
        "membership.created_first_admin",
        "membership.display_name_updated",
        "invite_code.generated",
        "invite_code.revoked",
        "invite_code.redeemed",
        "membership.relink_code_created",
        "membership.relink_redeemed",
        "operator_recovery.admin_relink_created",
        "membership.removed",
        "membership.promoted_to_admin",
        "membership.demoted_to_member",
        "event.created",
        "event.edited",
        "event.cancelled",
        "event.occurrence_cancelled",
        "attendance.admin_override",
        "attendance.admin_set_attended",
        "event_note.admin_hidden",
        "calendar_feed.token_generated",
        "calendar_feed.token_revoked",
        "event_template.created",
        "event_template.deleted",
    ];
    let class_b = [
        "community.export_authorized",
        "calendar_matrix_csv.export_requested",
    ];
    let class_c = ["session.logout"];
    let mut canonical_actions = std::collections::BTreeSet::new();
    for canonical in class_a.into_iter().chain(class_b).chain(class_c) {
        assert!(
            canonical_actions.insert(canonical),
            "duplicate canonical RFC-079 action in inventory: {canonical}"
        );
    }
    assert_eq!(
        canonical_actions.len(),
        26,
        "RFC-079 inventory must remain 23 Class A + 2 Class B + 1 Class C"
    );
    for (variant, expected_owners) in [
        ("CommunityCreated", &["db/community.rs"][..]),
        ("MembershipCreatedFirstAdmin", &["db/community.rs"][..]),
        ("MembershipDisplayNameUpdated", &["handlers/me.rs"][..]),
        ("InviteCodeGenerated", &["db/invite.rs"][..]),
        ("InviteCodeRevoked", &["db/invite.rs"][..]),
        ("InviteCodeRedeemed", &["db/invite.rs"][..]),
        ("MembershipRelinkCodeCreated", &["db/relink.rs"][..]),
        ("MembershipRelinkRedeemed", &["db/relink.rs"][..]),
        ("OperatorRecoveryAdminRelinkCreated", &["db/relink.rs"][..]),
        ("MembershipRemoved", &["db/membership.rs"][..]),
        ("MembershipPromotedToAdmin", &["db/membership.rs"][..]),
        ("MembershipDemotedToMember", &["db/membership.rs"][..]),
        ("EventCreated", &["db/event_write.rs"][..]),
        ("EventEdited", &["db/event_write.rs"][..]),
        ("EventCancelled", &["db/event_write.rs"][..]),
        ("EventOccurrenceCancelled", &["db/event_write.rs"][..]),
        ("AttendanceAdminOverride", &["db/attendance.rs"][..]),
        ("AttendanceAdminSetAttended", &["db/attendance.rs"][..]),
        ("EventNoteAdminHidden", &["db/event_note.rs"][..]),
        ("CalendarFeedTokenGenerated", &["db/calendar.rs"][..]),
        ("CalendarFeedTokenRevoked", &["db/calendar.rs"][..]),
        ("EventTemplateCreated", &["db/event_template.rs"][..]),
        ("EventTemplateDeleted", &["db/event_template.rs"][..]),
        ("CommunityExportAuthorized", &["handlers/export.rs"][..]),
        (
            "CalendarMatrixCsvExportRequested",
            &["handlers/communities.rs"][..],
        ),
        ("SessionLogout", &[][..]),
    ] {
        assert_eq!(
            audit_action_owners(&sources, variant),
            expected_owners,
            "typed audit action ownership changed for {variant}; reconcile the RFC-079 inventory"
        );
    }
}

#[test]
fn rfc079_closed_core_and_package7_removal_boundary_are_pinned() {
    for required in [
        "pub(crate) enum AuditAction",
        "pub(crate) enum AuditMetadata",
        "pub(crate) struct AuditRecord",
        "fn sanitize_and_serialize",
        "fn format_event",
        "const MAX_METADATA_DEPTH: usize = 8",
        "const MAX_METADATA_NODES: usize = 128",
        "const MAX_METADATA_BYTES: usize = 2_048",
    ] {
        assert!(
            RFC079_AUDIT_CORE_SRC.contains(required),
            "RFC-079 closed-core contract is missing {required:?}"
        );
    }
    assert!(
        !RFC079_AUDIT_CORE_SRC.contains("pub async fn write(")
            && !RFC079_AUDIT_CORE_SRC.contains("pub(crate) async fn write(")
            && !RFC079_AUDIT_CORE_SRC.contains("action: &str")
            && !RFC079_AUDIT_CORE_SRC.contains("metadata: Option<serde_json::Value>"),
        "Package 7 must not restore an arbitrary string/Value audit writer"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("event=audit.write request_id={}")
            && !RFC079_AUDIT_CORE_SRC.contains("target={}:{}")
            && !RFC079_AUDIT_CORE_SRC.contains("actor={} community={}"),
        "Package 7 structured audit events must exclude raw actor/community/target identifiers"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("earliest deployable code")
            && !RFC079_AUDIT_CORE_SRC.contains("LegacyAuditAction")
            && !RFC079_AUDIT_CORE_SRC.contains("LegacyAuditMetadata")
            && !RFC079_AUDIT_CORE_SRC.contains("write_legacy"),
        "Package 7 must remove the compatibility adapter while preserving the separately reviewed release boundary"
    );
}

#[test]
fn rfc079_package2_migration_and_operator_policy_are_pinned() {
    for required in [
        "CREATE TABLE audit_log_v2",
        "request_id           TEXT NOT NULL",
        "json_valid(metadata_json)",
        "json_type(metadata_json) = 'object'",
        "length(CAST(metadata_json AS BLOB)) <= 2048",
        "CHECK (length(request_id) BETWEEN 1 AND 96)",
        "CREATE TABLE audit_change_assertions",
        "length(operation_id) = 26",
        "changed_count INTEGER NOT NULL CHECK (changed_count = 1)",
        "CREATE INDEX idx_audit_log_community_created_at",
        "CREATE INDEX idx_audit_log_action_created_at",
    ] {
        assert!(
            RFC079_MIGRATION_SRC.contains(required),
            "RFC-079 migration 0010 is missing {required:?}"
        );
    }
    assert!(
        RFC079_MIGRATION_SRC.contains("'legacy'")
            && RFC079_MIGRATION_SRC.contains("'{}'")
            && !RFC079_MIGRATION_SRC.contains("SELECT metadata_json")
            && !RFC079_MIGRATION_SRC.contains("metadata_json FROM audit_log"),
        "migration 0010 must assign legacy request IDs and empty metadata without reading legacy metadata"
    );
    assert_eq!(
        RFC079_MIGRATION_SRC.matches("EXCEPT").count(),
        2,
        "migration 0010 must compare preserved core rows in both directions"
    );
    let verify_position = RFC079_MIGRATION_SRC
        .find("'core_rows_reverse'")
        .expect("migration must contain reverse core-row verification");
    let swap_position = RFC079_MIGRATION_SRC
        .find("ALTER TABLE audit_log RENAME TO audit_log_legacy_0010")
        .expect("migration must swap the old audit table only after verification");
    let drop_position = RFC079_MIGRATION_SRC
        .find("DROP TABLE audit_log_legacy_0010")
        .expect("migration must remove the old audit table after the swap");
    assert!(
        verify_position < swap_position && swap_position < drop_position,
        "migration verification, swap, and drop order changed"
    );
    assert!(
        RFC079_MIGRATION_SRC.contains("CHECK (passed = 1)")
            && RFC079_MIGRATION_SRC.contains("DROP TABLE audit_migration_0010_guard"),
        "migration mismatch checks must fail closed and leave no guard table"
    );
    let central_statement_builder = RFC079_AUDIT_CORE_SRC
        .split("fn statement_with_suffix")
        .nth(1)
        .and_then(|source| source.split("fn success_event").next())
        .expect("central audit statement builder must exist");
    assert!(
        RFC079_AUDIT_CORE_SRC.contains(
            "(id, request_id, community_id, actor_membership_id, target_kind, target_id, action, metadata_json, created_at)"
        ) && central_statement_builder.contains("D1Type::Text(self.request_id.as_str())")
            && central_statement_builder.matches("unwrap_or(D1Type::Null)").count() == 3,
        "the central statement builder must bind the validated request ID required by migration 0010"
    );

    assert!(
        RFC079_MIGRATION_RUNNER_SRC.contains("'migrations', 'apply'")
            && RFC079_MIGRATION_RUNNER_SRC.contains("'--local'")
            && RFC079_MIGRATION_RUNNER_SRC.contains("'--persist-to'")
            && !RFC079_MIGRATION_RUNNER_SRC.contains("'--remote'")
            && RFC079_MIGRATION_RUNNER_SRC.contains("legacyMetadataSelected: false")
            && RFC079_MIGRATION_RUNNER_SRC.contains("legacyMetadataPrinted: false")
            && RFC079_MIGRATION_RUNNER_SRC.contains("inheritNonAuthorityEnvironment")
            && RFC079_MIGRATION_RUNNER_SRC.contains("sentinel authority value was read"),
        "Package 2 rehearsal must apply the real ledger locally, isolate authority, and never select/print legacy metadata"
    );

    let alias_occurrence = RFC079_AUDIT_POLICY_SRC
        .find("target_kind = 'event_day' AND action = 'occurrence_cancelled'")
        .expect("audit policy must define the event-day alias");
    let alias_generated = RFC079_AUDIT_POLICY_SRC
        .find("target_kind = 'calendar_feed' AND action = 'calendar_token_generated'")
        .expect("audit policy must define the calendar-generation alias");
    let alias_revoked = RFC079_AUDIT_POLICY_SRC
        .find("target_kind = 'calendar_feed' AND action = 'calendar_token_revoked'")
        .expect("audit policy must define the calendar-revocation alias");
    let generic_rule = RFC079_AUDIT_POLICY_SRC
        .find("WHEN instr(action, '.') > 0 THEN action")
        .expect("audit policy must define the namespaced-action fallback");
    assert!(
        alias_occurrence < generic_rule
            && alias_generated < generic_rule
            && alias_revoked < generic_rule
            && RFC079_AUDIT_POLICY_SRC.contains("target_kind AS raw_target_kind")
            && RFC079_AUDIT_POLICY_SRC.contains("action AS raw_action")
            && RFC079_AUDIT_POLICY_SRC.contains("community.exported")
            && RFC079_AUDIT_POLICY_SRC.contains("community.export_authorized"),
        "raw-history compatibility policy must preserve raw values and apply explicit aliases before generic rules"
    );
    assert!(
        RFC079_BACKUP_RECOVERY_SRC.contains("pre-0010")
            && RFC079_BACKUP_RECOVERY_SRC.contains("potentially sensitive")
            && RFC079_BACKUP_RECOVERY_SRC.contains("Roll-forward recovery")
            && RFC079_DEPLOYMENT_SRC.contains("earliest deployable code boundary")
            && RFC079_DEPLOYMENT_SRC.contains("Package 8")
            && RFC079_DEPLOYMENT_SRC.contains("roll-forward only")
            && RFC079_OPERATIONS_SRC.contains("Mixed legacy and canonical audit actions")
            && RFC079_OPERATIONS_SRC.contains("Do not select `metadata_json`"),
        "Package 2 operator policy must cover sensitive backups, non-deployment, compatibility queries, and roll-forward recovery"
    );
}

#[test]
fn rfc079_package3_simple_required_batches_are_pinned() {
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("pub(crate) fn statement_after_one_change")
            && RFC079_AUDIT_CORE_SRC.contains("WHERE changes() = 1")
            && RFC079_AUDIT_CORE_SRC.contains("pub(crate) async fn execute_required")
            && RFC079_AUDIT_CORE_SRC
                .contains("let audit_statement = match audit.statement_after_one_change(db)")
            && RFC079_AUDIT_CORE_SRC.contains("db.batch(vec![mutation, audit_statement])")
            && RFC079_AUDIT_CORE_SRC
                .contains("log_class_a_failure(audit, AuditFailureCategory::Construction)")
            && RFC079_AUDIT_CORE_SRC.contains("(0, 0) => Ok(false)")
            && RFC079_AUDIT_CORE_SRC.contains("(1, 1)"),
        "Package 3 conditional required-audit primitive must keep mutation/audit adjacency and zero-or-one cardinality"
    );

    for (name, source, expected_batches) in [
        ("templates", RFC079_EVENT_TEMPLATE_DB_SRC, 2usize),
        ("invites", INVITE_DB_SRC, 2),
        ("membership", MEMBERSHIP_DB_SRC, 3),
        ("note moderation", RFC079_EVENT_NOTE_DB_SRC, 1),
        ("single attendance", RFC079_ATTENDANCE_DB_SRC, 1),
    ] {
        assert_eq!(
            source
                .matches("audit::execute_required(db, mutation, &record)")
                .count(),
            expected_batches,
            "Package 3 {name} helper count changed"
        );
        assert!(
            source.contains("role = 'admin'") && source.contains("removed_at IS NULL"),
            "Package 3 {name} mutation must repeat active-admin authorization in SQL"
        );
    }

    for (name, source) in [
        ("community", COMMUNITY_DB_SRC),
        ("display name", ME_HANDLER_SRC),
        ("operator recovery", OPERATOR_HANDLER_SRC),
        ("templates", RFC079_TEMPLATES_HANDLER_SRC),
        ("note moderation", RFC079_NOTE_HANDLER_SRC),
        ("single attendance", EVENT_HANDLER_SRC),
        ("invites", MEMBERS_HANDLER_SRC),
        ("member removal", MEMBER_REMOVE_HANDLER_SRC),
        ("role transfer", ROLE_TRANSFER_HANDLER_SRC),
    ] {
        assert!(
            !source.contains("INSERT INTO audit_log")
                && !source.contains("let _ = audit::write_legacy")
                && !source.contains("let _ = crate::audit::write_legacy"),
            "Package 3 {name} surface must use only the central typed audit builder"
        );
    }

    assert!(
        COMMUNITY_DB_SRC.matches("audit::required_record").count() == 2
            && COMMUNITY_DB_SRC.contains("audit::execute_required_batch")
            && ME_HANDLER_SRC.contains("AuditMetadata::DisplayNameChanged")
            && ME_HANDLER_SRC.contains("AND display_name != ?1")
            && RELINK_DB_SRC.contains("AuditMetadata::OperatorRecovery")
            && OPERATOR_HANDLER_SRC.contains("relink_db::issue_required"),
        "former direct-insert surfaces must remain centralized after the Package 5 operator boundary move"
    );

    for (name, source, current_state) in [
        (
            "template deletion",
            RFC079_EVENT_TEMPLATE_DB_SRC,
            "is_active = 1",
        ),
        (
            "invite revocation",
            INVITE_DB_SRC,
            "used_at IS NULL AND revoked_at IS NULL",
        ),
        (
            "note moderation",
            RFC079_EVENT_NOTE_DB_SRC,
            "hidden_by_admin_at IS NULL",
        ),
        (
            "attendance",
            RFC079_ATTENDANCE_DB_SRC,
            "attendances.status IS NOT 'attended'",
        ),
        ("role transfer", MEMBERSHIP_DB_SRC, "role = 'member'"),
    ] {
        assert!(
            source.contains(current_state),
            "Package 3 {name} mutation must repeat expected current state"
        );
    }

    assert!(
        RFC079_ATOMICITY_RUNNER_SRC.contains("'d1', 'migrations', 'apply'")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("'--local'")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("'--persist-to'")
            && !RFC079_ATOMICITY_RUNNER_SRC.contains("'--remote'")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("inheritNonAuthorityEnvironment")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("sentinel authority value was read")
            && RFC079_ATOMICITY_WORKER_SRC.contains("WHERE changes() = 1")
            && RFC079_ATOMICITY_WORKER_SRC.contains("db.batch(statements)")
            && !RFC079_ATOMICITY_WORKER_SRC.contains("console.")
            && RFC079_ATOMICITY_FIXTURE_SRC.contains("CREATE TRIGGER reject_proof_audit")
            && RFC079_ATOMICITY_FIXTURE_SRC
                .contains("RAISE(ABORT, 'synthetic required audit rejection')"),
        "Package 3 real-D1 proof must remain local-only, authority-isolated, privacy-bounded, and cover audit-trigger rollback"
    );
}

#[test]
fn rfc079_package4_event_calendar_and_attendance_batches_are_pinned() {
    assert!(
        RFC079_EVENT_WRITE_DB_SRC.contains("RECURRENCE_MATERIALIZATION_INSERT_CAP + 3")
            && RFC079_EVENT_WRITE_DB_SRC.contains("event create statement budget exceeded")
            && RFC079_EVENT_WRITE_DB_SRC.contains("execute_required_tail")
            && RFC079_EVENT_WRITE_DB_SRC.contains("EventOccurrenceCancelled")
            && RFC079_EVENT_WRITE_DB_SRC.contains("EventEditScope::SingleDaySchedule")
            && RFC079_EVENT_WRITE_DB_SRC.contains("AuditAction::EventCancelled"),
        "Package 4 event creation and mutation batches must remain bounded and typed"
    );
    for required in [
        "actor.role='admin'",
        "actor.removed_at IS NULL",
        "e.status='scheduled'",
        "occurrence_status='scheduled'",
        "WHERE changes()=1",
    ] {
        assert!(
            RFC079_EVENT_WRITE_DB_SRC.contains(required),
            "Package 4 event mutation boundary is missing {required:?}"
        );
    }
    for required in [
        "AND (SELECT COUNT(*) FROM event_days WHERE event_id=?5)=1",
        "AND EXISTS (SELECT 1 FROM event_days d",
        "d.occurrence_status='scheduled'",
        "d.day_date IS ?8",
        "d.starts_at_utc IS ?9 AND d.ends_at_utc IS ?10",
        "d.starts_at_utc>?4",
    ] {
        assert!(
            RFC079_EVENT_WRITE_DB_SRC.contains(required),
            "single-day edit tail must prove its exact eligible day post-state: missing {required:?}"
        );
    }
    assert!(
        RFC079_ATTENDANCE_DB_SRC.contains("ADMIN_OVERRIDE_CELL_CAP: usize = 10_000")
            && RFC079_ATTENDANCE_DB_SRC.contains("FROM json_each(?1)")
            && RFC079_ATTENDANCE_DB_SRC.contains("SELECT COUNT(*) FROM eligible")
            && RFC079_ATTENDANCE_DB_SRC.contains("attendances.status IS NOT excluded.status")
            && RFC079_ATTENDANCE_DB_SRC.contains("execute_required_attendance_override"),
        "Package 4 attendance override must remain one bounded, all-or-nothing set-based mutation"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("json_object('changed_count', changes())")
            && RFC079_AUDIT_CORE_SRC.contains("WHERE changes() BETWEEN 1 AND")
            && RFC079_AUDIT_CORE_SRC.contains("attendance audit cardinality mismatch"),
        "attendance audit metadata must use the database mutation cardinality"
    );
    assert!(
        CALENDAR_DB_SRC.contains("pub async fn rotate_required")
            && CALENDAR_DB_SRC.contains("pub async fn revoke_required")
            && CALENDAR_DB_SRC.contains("execute_required_tail")
            && CALENDAR_DB_SRC.contains("execute_required_bounded")
            && CALENDAR_DB_SRC.contains("m.removed_at IS NULL")
            && !CALENDAR_HANDLER_SRC.contains("write_calendar_token_audit"),
        "Package 4 calendar-token rotation/revocation must remain typed and atomic"
    );
    assert!(
        RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/event/audit-failure")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/edit/eligibility-loss")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/occurrence/success")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/occurrence/replay")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/occurrence/audit-failure")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/attendance/replay")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("/package4/calendar/audit-failure")
            && RFC079_ATOMICITY_RUNNER_SRC.contains("predecessorRestored: true")
            && RFC079_ATOMICITY_WORKER_SRC.contains("json_object('changed_count', changes())")
            && RFC079_ATOMICITY_WORKER_SRC.contains("proof_event_parts")
            && RFC079_ATOMICITY_FIXTURE_SRC.contains("proof_edit_days")
            && RFC079_ATOMICITY_FIXTURE_SRC.contains("proof_occurrence_exceptions")
            && RFC079_ATOMICITY_FIXTURE_SRC.contains("proof_calendar_tokens"),
        "Package 4 real-D1 proof must cover edit eligibility loss, occurrence success/no-op/rollback, multi-write rollback, database-derived attendance count/no-op, and calendar predecessor restoration"
    );
}

#[test]
fn rfc079_package5_one_winner_batches_and_correlation_are_pinned() {
    for required in [
        "pub(crate) async fn execute_asserted_required",
        "format!(\"ast_{}\", &random_token()[..22])",
        "INSERT INTO audit_change_assertions (operation_id, changed_count)",
        "VALUES (?1, changes())",
        "UPDATE audit_change_assertions SET changed_count=changes()",
        "let audit_statement = match audit.statement(db)",
        "statements.push(audit_statement)",
        "DELETE FROM audit_change_assertions WHERE operation_id=?1",
        "assertion_changes,",
        "cleanup_changes,",
        ") != (1, 1, 1, 1)",
    ] {
        assert!(
            RFC079_AUDIT_CORE_SRC.contains(required),
            "Package 5 central assertion executor is missing {required:?}"
        );
    }
    assert!(
        INVITE_DB_SRC.contains("pub async fn redeem_required")
            && INVITE_DB_SRC.contains("audit::execute_asserted_required")
            && INVITE_DB_SRC.contains("INSERT INTO users")
            && INVITE_DB_SRC.contains("INSERT INTO community_memberships")
            && INVITE_DB_SRC.contains("UPDATE invite_codes SET used_by_membership_id=?1")
            && INVITE_DB_SRC.contains("INSERT INTO sessions")
            && INVITE_DB_SRC.contains("AuditAction::InviteCodeRedeemed"),
        "Package 5 join claim, candidate identities/session, linkage, and audit must share one asserted batch"
    );
    assert!(
        RELINK_DB_SRC.contains("pub async fn redeem_required")
            && RELINK_DB_SRC.contains("audit::execute_asserted_required")
            && RELINK_DB_SRC.contains("INSERT INTO sessions")
            && RELINK_DB_SRC.contains("UPDATE sessions SET revoked_at=?1")
            && RELINK_DB_SRC.contains("AuditAction::MembershipRelinkRedeemed")
            && RELINK_DB_SRC.contains("relink_code_id: target.id.clone()"),
        "Package 5 relink claim, replacement session, predecessor revocation, and correlated audit must share one asserted batch"
    );
    assert!(
        HELP_SIGNIN_HANDLER_SRC.contains("relink_db::issue_required")
            && OPERATOR_HANDLER_SRC.contains("relink_db::issue_required")
            && RELINK_DB_SRC.contains("AuditAction::MembershipRelinkCodeCreated")
            && RELINK_DB_SRC.contains("AuditAction::OperatorRecoveryAdminRelinkCreated")
            && RELINK_DB_SRC.contains("AuditMetadata::RelinkCorrelation")
            && RELINK_DB_SRC.contains("AuditMetadata::OperatorRecovery")
            && RELINK_DB_SRC.contains("audit::execute_required_tail"),
        "help-signin and operator relink-code issuance must share guarded typed correlation"
    );
    let help_signin_post = HELP_SIGNIN_HANDLER_SRC
        .split("pub async fn post_help_signin")
        .nth(1)
        .expect("help-signin POST handler must exist");
    let guarded_issuance = help_signin_post
        .find("if !relink_db::issue_required(")
        .expect("help-signin must inspect guarded issuance success");
    let rejected_issuance = help_signin_post[guarded_issuance..]
        .find("return render::not_found();")
        .map(|offset| guarded_issuance + offset)
        .expect("help-signin must reject a zero-row guarded issuance");
    let code_response = help_signin_post
        .find("data-copy-code-value")
        .expect("help-signin success must render the generated code");
    assert!(
        help_signin_post.contains(".await?\n    {\n        return render::not_found();\n    }")
            && guarded_issuance < rejected_issuance
            && rejected_issuance < code_response,
        "help-signin must reject a zero-row guarded issuance before rendering the generated code"
    );
    assert!(
        RFC079_ASSERTION_RUNNER_SRC.contains("/flow/join/concurrent")
            && RFC079_ASSERTION_RUNNER_SRC.contains("/flow/relink/concurrent")
            && RFC079_ASSERTION_RUNNER_SRC.contains("/flow/join/audit-failure")
            && RFC079_ASSERTION_RUNNER_SRC.contains("/flow/relink/audit-failure")
            && RFC079_ASSERTION_RUNNER_SRC
                .contains("users: 1, memberships: 1, links: 1, sessions: 1, audits: 1, guards: 0")
            && RFC079_ASSERTION_RUNNER_SRC
                .contains("users: 0, memberships: 0, links: 0, sessions: 1, audits: 1, guards: 0")
            && RFC079_ASSERTION_WORKER_SRC.contains("async function runFlow")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_users")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_memberships")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_links")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_sessions")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("proof_flow_audits"),
        "Package 5 local D1 proof must show one winner/one audit, no losing candidate identity/session residue, and full audit-failure rollback"
    );
}

#[test]
fn rfc079_package6_disclosure_and_logout_boundaries_are_pinned() {
    let export_audit = EXPORT_HANDLER_SRC
        .find("audit::write_pre_disclosure(")
        .expect("community JSON export must persist pre-disclosure audit evidence");
    let export_payload = EXPORT_HANDLER_SRC
        .find("let payload = build_export(")
        .expect("community JSON export payload construction must remain explicit");
    let export_response = EXPORT_HANDLER_SRC
        .find("let mut resp = Response::ok(json)?")
        .expect("community JSON export response must remain explicit");
    assert!(
        export_audit < export_payload
            && export_payload < export_response
            && EXPORT_HANDLER_SRC.contains("AuditAction::CommunityExportAuthorized")
            && EXPORT_HANDLER_SRC.contains("return render::service_unavailable();"),
        "community JSON export must return disclosure-free 503 unless typed audit evidence is durable"
    );

    let matrix_post = COMMUNITIES_HANDLER_SRC
        .split("pub async fn post_matrix_export_audit")
        .nth(1)
        .expect("matrix export acknowledgement handler must exist");
    let matrix_audit = matrix_post
        .find("crate::audit::write_pre_disclosure(")
        .expect("matrix export acknowledgement must persist pre-disclosure evidence");
    let matrix_response = matrix_post
        .find("Response::from_json")
        .expect("matrix export acknowledgement response must remain explicit");
    assert!(
        matrix_audit < matrix_response
            && matrix_post.contains("AuditAction::CalendarMatrixCsvExportRequested")
            && matrix_post.contains("AuditMetadata::MatrixExportRequested")
            && matrix_post.contains("return json_error(503, i18n::JA_GENERAL_ERROR);"),
        "matrix export acknowledgement must return privacy-safe JSON 503 when typed audit evidence fails"
    );
    assert!(
        RENDER_SRC.contains("pub fn service_unavailable()")
            && RENDER_SRC.contains("with_status(503)"),
        "Class B HTML failure must use a generic 503 renderer"
    );

    let revoke = AUTH_HANDLER_SRC
        .find("session_db::revoke(&db, &auth.session_id).await?;")
        .expect("logout must require awaited server-side revocation");
    let secondary = AUTH_HANDLER_SRC
        .find("crate::audit::write_logout_secondary(&db, rid).await;")
        .expect("logout must await its bounded secondary audit attempt");
    let clear_cookie = AUTH_HANDLER_SRC
        .find("crate::session::clear_session_cookie")
        .expect("logout must clear the client credential");
    assert!(
        revoke < secondary && secondary < clear_cookie,
        "logout must revoke first, await secondary audit, then clear the cookie"
    );
    let logout_secondary = RFC079_AUDIT_CORE_SRC
        .split("pub(crate) async fn write_logout_secondary")
        .nth(1)
        .and_then(|source| source.split("pub(crate) fn result_changes").next())
        .expect("logout-only secondary writer must remain bounded");
    assert!(
        logout_secondary.contains("let action = AuditAction::SessionLogout;")
            && logout_secondary.contains("AuditFailureEvent::SecondaryWrite")
            && logout_secondary.contains("AuditMetadata::None")
            && !logout_secondary.contains("session_id")
            && !logout_secondary.contains("target_id"),
        "logout secondary audit must accept no credential/subject identifier and must emit its bounded incident"
    );
    assert!(
        RFC079_AUDIT_CORE_SRC.contains("audit.pre_disclosure_failed")
            && RFC079_AUDIT_CORE_SRC.contains("audit.secondary_write_failed")
            && RFC079_AUDIT_CORE_SRC.contains("failure_category={}")
            && RFC079_AUDIT_CORE_SRC.contains("route_class={}"),
        "Package 6 audit failures must emit bounded structured operational events"
    );

    let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../workers/ssr/src")
        .canonicalize()
        .expect("workers/ssr/src must exist for the Package 6 source gate");
    let mut sources = Vec::new();
    collect_rust_sources(&source_root, &source_root, &mut sources);
    let secondary_owners = sources
        .iter()
        .filter(|(_, source)| source.contains("write_logout_secondary("))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        secondary_owners,
        vec!["audit.rs", "handlers/auth.rs"],
        "logout must remain the only caller and owner of the Class C secondary-audit exception"
    );
    let pre_disclosure_owners = sources
        .iter()
        .filter(|(_, source)| source.contains("write_pre_disclosure("))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        pre_disclosure_owners,
        vec!["audit.rs", "handlers/communities.rs", "handlers/export.rs"],
        "only the two reviewed Class B surfaces may use the pre-disclosure writer"
    );
    assert!(
        PACKAGE_JSON_SRC.contains("\"test:audit-boundaries\"")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("/class-b/community/audit-failure")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("/class-b/matrix/audit-failure")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("/class-c/logout/audit-failure")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("status === 503")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("status === 303")
            && RFC079_BOUNDARY_RUNNER_SRC.contains("Max-Age=0")
            && RFC079_BOUNDARY_WORKER_SRC.contains("community.export_authorized")
            && RFC079_BOUNDARY_WORKER_SRC.contains("calendar_matrix_csv.export_requested")
            && RFC079_BOUNDARY_WORKER_SRC.contains("session.logout")
            && RFC079_BOUNDARY_FIXTURE_SRC.contains("proof_boundary_sessions")
            && RFC079_BOUNDARY_FIXTURE_SRC.contains("proof_boundary_audits")
            && !RFC079_BOUNDARY_WORKER_SRC.contains("console."),
        "Package 6 local D1 proof must cover disclosure-free Class B failure and safety-first Class C failure without Worker logging"
    );
}

#[test]
fn rfc079_package7_removal_and_documentation_boundary_are_pinned() {
    let migration_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let mut migration_filenames = std::fs::read_dir(&migration_root)
        .expect("migration directory must be readable")
        .map(|entry| entry.expect("migration entry must be readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "sql"))
        .map(|path| {
            path.file_name()
                .expect("migration must have a filename")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    migration_filenames.sort();
    let expected_migration_filenames = [
        "0001_initial.sql",
        "0002_form_tokens_nullable_user.sql",
        "0003_invite_grants_role.sql",
        "0004_calendar_tokens.sql",
        "0005_event_templates.sql",
        "0006_event_recurrence.sql",
        "0007_codlet_tables.sql",
        "0008_membership_relink_codes.sql",
        "0009_recurrence_v2.sql",
        "0010_audit_integrity.sql",
    ];
    assert_eq!(
        migration_filenames, expected_migration_filenames,
        "architecture migration inventory gate must be updated with the actual SQL ledger"
    );
    for filename in expected_migration_filenames {
        assert!(
            RFC079_ARCHITECTURE_SRC.contains(filename),
            "architecture migration inventory is missing actual ledger entry {filename}"
        );
    }
    assert!(
        !RFC079_ARCHITECTURE_SRC.contains("0007_event_day_attendance_grain.sql"),
        "architecture must not name the nonexistent migration 0007 entry"
    );

    for forbidden in [
        "pub(crate) enum LegacyAuditAction",
        "pub(crate) enum LegacyAuditMetadata",
        "pub(crate) async fn write_legacy",
        "metadata: Option<serde_json::Value>",
        "action: &str",
    ] {
        assert!(
            !RFC079_AUDIT_CORE_SRC.contains(forbidden),
            "Package 7 must remove forbidden audit surface {forbidden:?}"
        );
    }
    assert!(
        LIB_SRC.contains("event=worker.request_failed request_id={}")
            && LIB_SRC.contains("failure_category=unhandled route_class=request")
            && !LIB_SRC.contains("unhandled error: {:?}"),
        "unhandled Worker errors must use bounded categories rather than raw Debug output"
    );
    for (name, source, required) in [
        ("RFC-014", RFC079_RFC014_SRC, "Current RFC-079 boundary"),
        (
            "RFC-052",
            RFC079_RFC052_SRC,
            "RFC-079 reconciliation (2026-07-16)",
        ),
        ("RFC-071", RFC079_RFC071_SRC, "sole Class C exception"),
        (
            "RFC-050",
            RFC079_RFC050_SRC,
            "earliest deployable **code** boundary",
        ),
        (
            "threat model",
            RFC079_THREAT_MODEL_SRC,
            "closed 26-action model",
        ),
        (
            "architecture",
            RFC079_ARCHITECTURE_SRC,
            "Audit integrity boundary",
        ),
        (
            "operations",
            RFC079_OPERATIONS_SRC,
            "audit.secondary_write_failed",
        ),
        (
            "backup/recovery",
            RFC079_BACKUP_RECOVERY_SRC,
            "removed compatibility adapter",
        ),
        (
            "release checklist",
            RFC079_RELEASE_CHECKLIST_SRC,
            "RFC-079 audit integrity and redaction",
        ),
        (
            "audit/operator query policy",
            RFC079_AUDIT_POLICY_SRC,
            "Raw-history compatibility query",
        ),
    ] {
        assert!(
            source.contains(required),
            "Package 7 {name} reconciliation is missing {required:?}"
        );
    }
    for source in [
        RFC079_RFC014_SRC,
        RFC079_RFC071_SRC,
        RFC079_THREAT_MODEL_SRC,
        RFC079_ARCHITECTURE_SRC,
        RFC079_AUDIT_POLICY_SRC,
        RFC079_BACKUP_RECOVERY_SRC,
        RFC079_RELEASE_CHECKLIST_SRC,
    ] {
        assert!(
            source.contains("Class A") || source.contains("Class A/B/C"),
            "Package 7 durable audit documents must preserve the Class A/B/C model"
        );
    }
    assert!(
        RFC079_AUDIT_POLICY_SRC.contains("not deployment approval")
            && RFC079_DEPLOYMENT_SRC.contains("Package 8")
            && RFC079_RELEASE_CHECKLIST_SRC.contains("exact-candidate hosted")
            && RFC079_RFC050_SRC.contains("local fixture success"),
        "Package 7 must distinguish earliest deployable code from release/hosted authorization"
    );
    assert!(
        RFC079_BACKUP_RECOVERY_SRC.contains("D1 Time Travel")
            && RFC079_BACKUP_RECOVERY_SRC.contains("7 days on Workers Free")
            && RFC079_BACKUP_RECOVERY_SRC.contains("30 days on Workers Paid")
            && !RFC079_BACKUP_RECOVERY_SRC.contains("30 days on all plans"),
        "backup/recovery must pin plan-aware D1 Time Travel retention without a plan-independent guarantee"
    );
}

#[test]
fn rfc079_package0a_assertion_fixture_is_bounded_and_outside_the_ledger() {
    assert!(
        RFC079_ASSERTION_FIXTURE_SRC.contains("CREATE TABLE audit_change_assertions")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("length(operation_id) = 26")
            && RFC079_ASSERTION_FIXTURE_SRC.contains("changed_count = 1"),
        "Package 0A fixture must pin the reviewed operation-ID and one-row CHECK constraints"
    );
    assert!(
        !RFC079_ASSERTION_FIXTURE_SRC.contains("d1_migrations")
            && !RFC079_ASSERTION_FIXTURE_SRC.contains("audit_log"),
        "Package 0A fixture must stay outside the D1 ledger and production audit schema"
    );
    assert!(
        RFC079_ASSERTION_WORKER_SRC.contains("VALUES (?1, changes())")
            && RFC079_ASSERTION_WORKER_SRC.contains("db.batch(statements)")
            && !RFC079_ASSERTION_WORKER_SRC.contains("console."),
        "Package 0A Worker must exercise the exact D1 batch primitive without console output"
    );
    assert!(
        RFC079_ASSERTION_RUNNER_SRC.contains("cloudflareAuthorityKey")
            && RFC079_ASSERTION_RUNNER_SRC.contains("CLOUDFLARE_")
            && RFC079_ASSERTION_RUNNER_SRC.contains("Object.keys(source)")
            && !RFC079_ASSERTION_RUNNER_SRC.contains("Object.entries(process.env)")
            && RFC079_ASSERTION_RUNNER_SRC.contains("sentinel authority value was read")
            && !RFC079_ASSERTION_RUNNER_SRC.contains("genericCallSites")
            && !RFC079_ASSERTION_RUNNER_SRC.contains("directSqlOutsideAuditModule")
            && !RFC079_ASSERTION_RUNNER_SRC.contains("classifiedActions"),
        "Package 0A runner must scrub inherited Cloudflare authority and must not report declarative inventory as D1 evidence"
    );
    let proof_audit_schema = RFC079_ASSERTION_FIXTURE_SRC
        .split("CREATE TABLE proof_audits")
        .nth(1)
        .expect("proof fixture must define its synthetic audit table");
    assert!(
        !proof_audit_schema.contains("operation_id"),
        "assertion operation IDs must not enter even the synthetic audit row"
    );
}

#[test]
fn rfc079_removal_scanner_detects_forbidden_surfaces_and_action_substitution() {
    let synthetic_sources = vec![
        (
            "known.rs".to_owned(),
            "fn known() { let action = AuditAction::MembershipRemoved; }".to_owned(),
        ),
        (
            "compatibility.rs".to_owned(),
            "fn old() { write_legacy(db, LegacyAuditAction::MembershipRemoved); }".to_owned(),
        ),
        (
            "new_direct.rs".to_owned(),
            "const SQL: &str = \"INSERT INTO audit_log VALUES (...)\";".to_owned(),
        ),
        (
            "new_privacy_ref.rs".to_owned(),
            "fn leaked(operation_id: &str) { let _ = audit_change_assertions; }".to_owned(),
        ),
        (
            "ignored.rs".to_owned(),
            "fn ignored() { let _ = audit::execute_required(db, mutation, audit); }".to_owned(),
        ),
        (
            "background.rs".to_owned(),
            "fn background() { spawn_local(async move { audit_required().await; }); }".to_owned(),
        ),
    ];
    let scan = scan_audit_sources(&synthetic_sources);
    assert_eq!(scan.direct_inserts["new_direct.rs"], 1);
    assert_eq!(scan.assertion_table_refs, ["new_privacy_ref.rs"]);
    assert_eq!(scan.operation_id_refs, ["new_privacy_ref.rs"]);
    assert_eq!(scan.compatibility_refs, ["compatibility.rs"]);
    assert_eq!(scan.ignored_audit_results, ["ignored.rs"]);
    assert_eq!(scan.background_audit_refs, ["background.rs"]);
    assert_eq!(
        audit_action_owners(&synthetic_sources, "MembershipRemoved"),
        ["known.rs"]
    );
    assert!(
        audit_action_owners(&synthetic_sources, "MembershipPromotedToAdmin").is_empty(),
        "a substituted expected action owner must fail the exact inventory gate"
    );
}

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
    let redeem = INVITE_DB_SRC
        .split("pub async fn redeem_required")
        .nth(1)
        .expect("join profile required redemption helper must exist");
    let mark_used = redeem
        .find("UPDATE invite_codes SET used_at=?1")
        .expect("join profile must atomically claim the invite");
    let insert_user = redeem
        .find("INSERT INTO users")
        .expect("join profile must insert user in the asserted batch");
    let insert_membership = redeem
        .find("INSERT INTO community_memberships")
        .expect("join profile must insert membership in the asserted batch");
    let assign_used_membership = redeem
        .find("UPDATE invite_codes SET used_by_membership_id=?1")
        .expect("join profile must link the claimed invite to its membership");

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
    let redeem = INVITE_DB_SRC
        .split("pub async fn redeem_required")
        .nth(1)
        .expect("invite::redeem_required must exist");
    let claim_end = redeem
        .find("let user =")
        .expect("asserted claim must precede candidate creation");
    let mark_body = &redeem[..claim_end];
    let assign_body = &redeem[claim_end..];

    assert!(
        mark_body.contains("SET used_at=?1"),
        "mark_used should perform the atomic one-winner claim"
    );
    assert!(
        !mark_body.contains("used_by_membership_id"),
        "mark_used must not write used_by_membership_id before the membership FK target exists"
    );
    assert!(
        assign_body.contains("SET used_by_membership_id=?1"),
        "assign_used_membership should perform the post-membership FK backfill"
    );
}
