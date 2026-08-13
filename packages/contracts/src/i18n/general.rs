use super::Localized;

// ── Session / auth ────────────────────────────────────────────────────────
pub const EN_SESSION_EXPIRED: &str =
    "Your session expired. Use an invite code or a sign-in-again code from your community admin.";
pub const EN_LOGOUT: &str = "Log out";
pub const EN_LOGOUT_CONFIRM: &str = "Log out?";

pub const JA_SESSION_EXPIRED: &str = "時間がたったため、もう一度入る必要があります。管理者から受け取った招待コード、またはサインインし直すためのコードを使ってください。";
pub const JA_LOGOUT: &str = "ログアウト";
pub const JA_LOGOUT_CONFIRM: &str = "ログアウトしますか？";

/// RFC-072 locale-aware pair; see `i18n::Localized`.
pub const LOGOUT: Localized = Localized {
    ja: JA_LOGOUT,
    en: EN_LOGOUT,
};

// ── General ───────────────────────────────────────────────────────────────
pub const EN_GENERAL_ERROR: &str = "Something went wrong. Please try again.";
pub const EN_CONFIGURATION_UNAVAILABLE: &str =
    "The service is temporarily unavailable. Please try again later.";
pub const EN_OFFLINE_BANNER: &str = "Offline — showing last loaded";
pub const EN_EMPTY_EVENTS: &str = "No events yet.";
pub const EN_EMPTY_EVENTS_HINT: &str = "Ask your community admin to add one.";
pub const EN_EMPTY_EVENTS_ADMIN: &str = "No events yet. Create the first event for this community.";

pub const JA_GENERAL_ERROR: &str = "エラーが発生しました。もう一度お試しください。";

/// RFC-072 locale-aware pair; see `i18n::Localized`.
pub const GENERAL_ERROR: Localized = Localized {
    ja: JA_GENERAL_ERROR,
    en: EN_GENERAL_ERROR,
};
pub const JA_CONFIGURATION_UNAVAILABLE: &str =
    "ただいまサービスを利用できません。しばらくしてから、もう一度お試しください。";

/// RFC-072 locale-aware pair; see `i18n::Localized`.
pub const CONFIGURATION_UNAVAILABLE: Localized = Localized {
    ja: JA_CONFIGURATION_UNAVAILABLE,
    en: EN_CONFIGURATION_UNAVAILABLE,
};
pub const EN_NOT_FOUND: &str = "Not found.";
pub const JA_NOT_FOUND: &str = "見つかりませんでした。";
// RFC-082 §4 (owner decision): an explicit page for a suspended member, not
// an indistinguishable not-found — suspension is not a secret from the
// person suspended.
pub const EN_MEMBERSHIP_SUSPENDED: &str =
    "Your access to this community is paused. Contact your community administrator.";
pub const JA_MEMBERSHIP_SUSPENDED: &str = "このコミュニティへのアクセスは一時停止されています。コミュニティの管理者にお問い合わせください。";
pub const EN_INTERNAL_ERROR: &str = "Something went wrong. Please try again.";
pub const JA_INTERNAL_ERROR: &str = "問題が発生しました。もう一度お試しください。";
pub const EN_ADMIN_ATTEND_CANCELLED: &str = "Attendance cannot be corrected for a cancelled event.";
pub const JA_ADMIN_ATTEND_CANCELLED: &str = "キャンセル済みのイベントの出席は修正できません。";
pub const EN_GENERAL_BACK: &str = "Go back";
pub const JA_GENERAL_BACK: &str = "戻る";
pub const EN_ADMIN_EDIT_CANCELLED: &str = "Cancelled events cannot be edited.";
pub const JA_ADMIN_EDIT_CANCELLED: &str = "キャンセル済みのイベントは編集できません。";
pub const EN_ADMIN_EDIT_STARTED: &str = "This event has already started and cannot be edited.";
pub const JA_ADMIN_EDIT_STARTED: &str = "すでに開始したイベントは編集できません。";
pub const JA_OFFLINE_BANNER: &str = "オフライン — 最後に読み込んだ情報を表示しています";
pub const JA_EMPTY_EVENTS: &str = "イベントはまだありません。";
pub const JA_EMPTY_EVENTS_HINT: &str = "コミュニティの管理者にイベントの追加をお願いしてください。";
pub const JA_EMPTY_EVENTS_ADMIN: &str =
    "イベントはまだありません。最初のイベントを作成しましょう。";

/// RFC-072 locale-aware pair; see `i18n::Localized`.
pub const EMPTY_EVENTS_HINT: Localized = Localized {
    ja: JA_EMPTY_EVENTS_HINT,
    en: EN_EMPTY_EVENTS_HINT,
};

// ── Navigation ────────────────────────────────────────────────────────────
pub const EN_NAV_HOME: &str = "Home";
pub const EN_NAV_COMMUNITIES: &str = "Calendar";
pub const EN_NAV_ME: &str = "Me";

pub const JA_NAV_HOME: &str = "ホーム";
pub const JA_NAV_COMMUNITIES: &str = "カレンダー";
pub const JA_NAV_ME: &str = "マイページ";
pub const EN_NAV_BACK: &str = "Back to event";
pub const JA_NAV_BACK: &str = "イベントに戻る";
pub const EN_NAV_SWITCH_GO: &str = "Switch";
pub const JA_NAV_SWITCH_GO: &str = "切り替え";
pub const EN_NAV_SWITCH_ARIA_LABEL: &str = "Switch community";
pub const JA_NAV_SWITCH_ARIA_LABEL: &str = "コミュニティを切り替え";
// Handoff 036: was a bare "Main" aria-label on every page's bottom nav —
// the string a screen reader announces there, not just page text.
pub const EN_NAV_MAIN_ARIA_LABEL: &str = "Main navigation";
pub const JA_NAV_MAIN_ARIA_LABEL: &str = "メインナビゲーション";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const NAV_HOME: Localized = Localized {
    ja: JA_NAV_HOME,
    en: EN_NAV_HOME,
};
pub const NAV_COMMUNITIES: Localized = Localized {
    ja: JA_NAV_COMMUNITIES,
    en: EN_NAV_COMMUNITIES,
};
pub const NAV_ME: Localized = Localized {
    ja: JA_NAV_ME,
    en: EN_NAV_ME,
};
pub const NAV_SWITCH_GO: Localized = Localized {
    ja: JA_NAV_SWITCH_GO,
    en: EN_NAV_SWITCH_GO,
};
pub const NAV_SWITCH_ARIA_LABEL: Localized = Localized {
    ja: JA_NAV_SWITCH_ARIA_LABEL,
    en: EN_NAV_SWITCH_ARIA_LABEL,
};
pub const NAV_MAIN_ARIA_LABEL: Localized = Localized {
    ja: JA_NAV_MAIN_ARIA_LABEL,
    en: EN_NAV_MAIN_ARIA_LABEL,
};

// ── Role labels ───────────────────────────────────────────────────────────
pub const EN_ROLE_ADMIN: &str = "Admin";
pub const EN_ROLE_MEMBER: &str = "Member";

pub const JA_ROLE_ADMIN: &str = "管理者";
pub const JA_ROLE_MEMBER: &str = "メンバー";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const ROLE_ADMIN: Localized = Localized {
    ja: JA_ROLE_ADMIN,
    en: EN_ROLE_ADMIN,
};
pub const ROLE_MEMBER: Localized = Localized {
    ja: JA_ROLE_MEMBER,
    en: EN_ROLE_MEMBER,
};

// ── Shared badges ────────────────────────────────────────────────────────
pub const EN_CURRENT_BADGE: &str = "Current";
pub const JA_CURRENT_BADGE: &str = "現在";

// ── Shared date/calendar vocabulary ────────────────────────────────────────
// RFC-072 Slice C: moved here from i18n/home.rs — used on Calendar's month
// grid, not Home-specific. Rename only; values unchanged.
pub const EN_TODAY: &str = "Today";
pub const JA_TODAY: &str = "今日";

/// RFC-072 locale-aware pair; see `i18n::Localized`.
pub const TODAY: Localized = Localized {
    ja: JA_TODAY,
    en: EN_TODAY,
};
