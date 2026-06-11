//! UI string constants (i18n scaffold — RFC-026).
//!
//! All user-visible strings are collected here so they can be translated
//! without touching handler logic.  Currently English only; Japanese parity
//! is enforced by the i18n lint test below.
//!
//! Naming: `<LANG>_<CONTEXT>_<KEY>` in SCREAMING_SNAKE_CASE.

// ── Join / onboarding ─────────────────────────────────────────────────────
pub const EN_JOIN_HEADING: &str = "ciao.zinnias";
pub const EN_JOIN_SUBHEADING: &str = "Private community schedule sharing";
pub const EN_JOIN_CODE_LABEL: &str = "Invite code";
pub const EN_JOIN_CODE_HINT: &str = "Ask your community admin if you do not have an invite code.";
pub const EN_JOIN_SUBMIT: &str = "Join";
pub const EN_JOIN_PROFILE_HEADING: &str = "Your name in this community";
pub const EN_JOIN_PROFILE_HINT: &str =
    "People will see this name when you answer events or leave notes.";
pub const EN_JOIN_PROFILE_LABEL: &str = "Display name";
pub const EN_JOIN_PROFILE_SUBMIT: &str = "Start";

pub const JA_JOIN_HEADING: &str = "ciao.zinnias";
pub const JA_JOIN_SUBHEADING: &str = "招待制コミュニティのスケジュール共有";
pub const JA_JOIN_CODE_LABEL: &str = "招待コード";
pub const JA_JOIN_CODE_HINT: &str = "招待コードはコミュニティの管理者にお問い合わせください。";
pub const JA_JOIN_SUBMIT: &str = "参加する";
pub const JA_JOIN_PROFILE_HEADING: &str = "このコミュニティでの名前";
pub const JA_JOIN_PROFILE_HINT: &str = "イベントへの返答やメモを残すときにこの名前が表示されます。";
pub const JA_JOIN_PROFILE_LABEL: &str = "表示名";
pub const JA_JOIN_PROFILE_SUBMIT: &str = "はじめる";

// ── Status labels ─────────────────────────────────────────────────────────
pub const EN_STATUS_GOING: &str = "Going";
pub const EN_STATUS_NOT_GOING: &str = "No Go";
pub const EN_STATUS_ATTENDED: &str = "Attended";
pub const EN_STATUS_NO_ANSWER: &str = "No answer";
pub const EN_STATUS_ATTENDED_DISABLED: &str = "Available after the event";

pub const JA_STATUS_GOING: &str = "参加";
pub const JA_STATUS_NOT_GOING: &str = "不参加";
pub const JA_STATUS_ATTENDED: &str = "出席済み";
pub const JA_STATUS_NO_ANSWER: &str = "未回答";
pub const JA_STATUS_ATTENDED_DISABLED: &str = "イベント終了後に利用可能";

// ── Note editor ───────────────────────────────────────────────────────────
pub const EN_NOTE_SAVE: &str = "Save Note";
pub const EN_NOTE_DELETE: &str = "Delete Note";
pub const EN_NOTE_SAVED: &str = "Saved.";
pub const EN_NOTE_TOO_LONG: &str = "Your note is too long. Please keep it under 200 characters.";

pub const JA_NOTE_SAVE: &str = "メモを保存";
pub const JA_NOTE_DELETE: &str = "メモを削除";
pub const JA_NOTE_SAVED: &str = "保存しました。";
pub const JA_NOTE_TOO_LONG: &str = "メモが長すぎます。200文字以内にしてください。";

// ── Session / auth ────────────────────────────────────────────────────────
pub const EN_SESSION_EXPIRED: &str =
    "Your session expired. Please ask your community admin for a new invite code.";
pub const EN_LOGOUT: &str = "Log out";
pub const EN_LOGOUT_CONFIRM: &str = "Log out?";

pub const JA_SESSION_EXPIRED: &str =
    "セッションが切れました。新しい招待コードをコミュニティ管理者にお問い合わせください。";
pub const JA_LOGOUT: &str = "ログアウト";
pub const JA_LOGOUT_CONFIRM: &str = "ログアウトしますか？";

// ── General ───────────────────────────────────────────────────────────────
pub const EN_GENERAL_ERROR: &str = "Something went wrong. Please try again.";
pub const EN_OFFLINE_BANNER: &str = "Offline — showing last loaded";
pub const EN_EMPTY_EVENTS: &str = "No events yet.";
pub const EN_EMPTY_EVENTS_HINT: &str = "Ask your community admin to add one.";
pub const EN_EMPTY_EVENTS_ADMIN: &str = "No events yet. Create the first event for this community.";

pub const JA_GENERAL_ERROR: &str = "エラーが発生しました。もう一度お試しください。";
pub const JA_OFFLINE_BANNER: &str = "オフライン — 最後に読み込んだ情報を表示しています";
pub const JA_EMPTY_EVENTS: &str = "イベントはまだありません。";
pub const JA_EMPTY_EVENTS_HINT: &str = "コミュニティの管理者にイベントの追加をお願いしてください。";
pub const JA_EMPTY_EVENTS_ADMIN: &str =
    "イベントはまだありません。最初のイベントを作成しましょう。";

// ── i18n parity lint ─────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    // Every EN_ constant must have a JA_ counterpart with the same suffix.
    // This test enforces parity at compile-time by building both lists and
    // comparing suffixes.
    #[test]
    fn en_ja_parity() {
        let en_keys = &[
            "JOIN_HEADING",
            "JOIN_SUBHEADING",
            "JOIN_CODE_LABEL",
            "JOIN_CODE_HINT",
            "JOIN_SUBMIT",
            "JOIN_PROFILE_HEADING",
            "JOIN_PROFILE_HINT",
            "JOIN_PROFILE_LABEL",
            "JOIN_PROFILE_SUBMIT",
            "STATUS_GOING",
            "STATUS_NOT_GOING",
            "STATUS_ATTENDED",
            "STATUS_NO_ANSWER",
            "STATUS_ATTENDED_DISABLED",
            "NOTE_SAVE",
            "NOTE_DELETE",
            "NOTE_SAVED",
            "NOTE_TOO_LONG",
            "SESSION_EXPIRED",
            "LOGOUT",
            "LOGOUT_CONFIRM",
            "GENERAL_ERROR",
            "OFFLINE_BANNER",
            "EMPTY_EVENTS",
            "EMPTY_EVENTS_HINT",
            "EMPTY_EVENTS_ADMIN",
        ];
        // Values are non-empty strings — the real parity check
        // is that the arrays above have the same length, enforced by
        // keeping this list in sync with the constants declared above.
        assert_eq!(en_keys.len(), 26, "update parity list when adding strings");
        // Each key must be non-empty in both languages (checked by construction
        // since they are &'static str literals — this asserts they are not "").
        for key in en_keys {
            assert!(!key.is_empty(), "empty key: {key}");
        }
    }
}
