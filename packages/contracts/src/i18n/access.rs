// ── Join / onboarding ─────────────────────────────────────────────────────
pub const EN_JOIN_HEADING: &str = "ciao.zinnias";
pub const EN_JOIN_SUBHEADING: &str = "Private community schedule sharing";
pub const EN_JOIN_CODE_LABEL: &str = "Invite code";
pub const EN_JOIN_CODE_HINT: &str = "Ask your community admin if you do not have an invite code.";
pub const EN_JOIN_RELINK_HINT: &str = "Have a sign-in-again code?";
pub const EN_JOIN_RELINK_LINK: &str = "Open sign-in-again page";
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
pub const JA_JOIN_RELINK_HINT: &str = "サインインし直すためのコードをお持ちですか？";
pub const JA_JOIN_RELINK_LINK: &str = "サインインし直す画面を開く";
pub const JA_JOIN_SUBMIT: &str = "参加する";
pub const JA_JOIN_PROFILE_HEADING: &str = "このコミュニティでの名前";
pub const JA_JOIN_PROFILE_HINT: &str = "イベントへの返答やメモを残すときにこの名前が表示されます。";
pub const JA_JOIN_PROFILE_LABEL: &str = "表示名";
pub const JA_JOIN_PROFILE_SUBMIT: &str = "はじめる";

// ── Relink / access recovery ─────────────────────────────────────────────
pub const EN_RELINK_TITLE: &str = "Sign in again";
pub const EN_RELINK_BODY: &str = "Enter the code from your community admin.";
pub const EN_RELINK_CODE_LABEL: &str = "Code";
pub const EN_RELINK_SUBMIT: &str = "Sign in";
pub const EN_RELINK_INVALID: &str = "This code is invalid or has expired.";

pub const JA_RELINK_TITLE: &str = "サインインし直す";
pub const JA_RELINK_BODY: &str = "コミュニティの管理者から受け取ったコードを入力してください。";
pub const JA_RELINK_CODE_LABEL: &str = "コード";
pub const JA_RELINK_SUBMIT: &str = "サインイン";
pub const JA_RELINK_INVALID: &str = "このコードは無効か、有効期限が切れています。";

// ── Join page (RFC-003) ────────────────────────────────────────────────────
pub const EN_JOIN_PAGE_TITLE: &str = "Join";
pub const EN_JOIN_PROFILE_PAGE_TITLE: &str = "Your name";

pub const JA_JOIN_PAGE_TITLE: &str = "参加";
pub const JA_JOIN_PROFILE_PAGE_TITLE: &str = "お名前";

// ── External identity sign-in (RFC-080, Handoff 054) ─────────────────────
// One generic outcome for every distinct rejection reason in the callback
// contract (RFC-080 §5.2): sign-in failure text must never confirm account
// existence, a linked provider, invite validity, or any internal detail.
pub const EN_IDENTITY_SIGN_IN_LINK: &str = "Sign in with an external account";
pub const EN_IDENTITY_SIGN_IN_FAILED_TITLE: &str = "Sign-in could not be completed";
pub const EN_IDENTITY_SIGN_IN_FAILED_BODY: &str =
    "Sign-in could not be completed. You can try again, or cancel and return.";
pub const EN_IDENTITY_SIGN_IN_RETRY: &str = "Try again";
pub const EN_IDENTITY_SIGN_IN_CANCEL: &str = "Cancel";

pub const JA_IDENTITY_SIGN_IN_LINK: &str = "外部アカウントでサインイン";
pub const JA_IDENTITY_SIGN_IN_FAILED_TITLE: &str = "サインインを完了できませんでした";
pub const JA_IDENTITY_SIGN_IN_FAILED_BODY: &str =
    "サインインを完了できませんでした。もう一度お試しいただくか、やめてお戻りください。";
pub const JA_IDENTITY_SIGN_IN_RETRY: &str = "もう一度試す";
pub const JA_IDENTITY_SIGN_IN_CANCEL: &str = "やめる";

// Handoff 072 (RFC-083 Slice D1b): the admin-facing help-signin page
// resolves locale now; this is the only constant in this file with an
// admin (not anonymous-route) caller.
pub const RELINK_CODE_LABEL: super::Localized = super::Localized {
    ja: JA_RELINK_CODE_LABEL,
    en: EN_RELINK_CODE_LABEL,
};

// RFC-083 Slice D2a (Handoff 075) locale-aware pairs — the four anonymous
// routes now resolve a locale (RFC-083 §8.1 rung 2, Accept-Language, since
// none of them have a membership to read rung 1 from).
pub const JOIN_HEADING: super::Localized = super::Localized {
    ja: JA_JOIN_HEADING,
    en: EN_JOIN_HEADING,
};
pub const JOIN_SUBHEADING: super::Localized = super::Localized {
    ja: JA_JOIN_SUBHEADING,
    en: EN_JOIN_SUBHEADING,
};
pub const JOIN_CODE_LABEL: super::Localized = super::Localized {
    ja: JA_JOIN_CODE_LABEL,
    en: EN_JOIN_CODE_LABEL,
};
pub const JOIN_CODE_HINT: super::Localized = super::Localized {
    ja: JA_JOIN_CODE_HINT,
    en: EN_JOIN_CODE_HINT,
};
pub const JOIN_RELINK_HINT: super::Localized = super::Localized {
    ja: JA_JOIN_RELINK_HINT,
    en: EN_JOIN_RELINK_HINT,
};
pub const JOIN_RELINK_LINK: super::Localized = super::Localized {
    ja: JA_JOIN_RELINK_LINK,
    en: EN_JOIN_RELINK_LINK,
};
pub const JOIN_SUBMIT: super::Localized = super::Localized {
    ja: JA_JOIN_SUBMIT,
    en: EN_JOIN_SUBMIT,
};
pub const JOIN_PROFILE_HEADING: super::Localized = super::Localized {
    ja: JA_JOIN_PROFILE_HEADING,
    en: EN_JOIN_PROFILE_HEADING,
};
pub const JOIN_PROFILE_HINT: super::Localized = super::Localized {
    ja: JA_JOIN_PROFILE_HINT,
    en: EN_JOIN_PROFILE_HINT,
};
pub const JOIN_PROFILE_LABEL: super::Localized = super::Localized {
    ja: JA_JOIN_PROFILE_LABEL,
    en: EN_JOIN_PROFILE_LABEL,
};
pub const JOIN_PROFILE_SUBMIT: super::Localized = super::Localized {
    ja: JA_JOIN_PROFILE_SUBMIT,
    en: EN_JOIN_PROFILE_SUBMIT,
};
pub const JOIN_PAGE_TITLE: super::Localized = super::Localized {
    ja: JA_JOIN_PAGE_TITLE,
    en: EN_JOIN_PAGE_TITLE,
};
pub const JOIN_PROFILE_PAGE_TITLE: super::Localized = super::Localized {
    ja: JA_JOIN_PROFILE_PAGE_TITLE,
    en: EN_JOIN_PROFILE_PAGE_TITLE,
};
pub const RELINK_TITLE: super::Localized = super::Localized {
    ja: JA_RELINK_TITLE,
    en: EN_RELINK_TITLE,
};
pub const RELINK_BODY: super::Localized = super::Localized {
    ja: JA_RELINK_BODY,
    en: EN_RELINK_BODY,
};
pub const RELINK_SUBMIT: super::Localized = super::Localized {
    ja: JA_RELINK_SUBMIT,
    en: EN_RELINK_SUBMIT,
};
pub const RELINK_INVALID: super::Localized = super::Localized {
    ja: JA_RELINK_INVALID,
    en: EN_RELINK_INVALID,
};
pub const IDENTITY_SIGN_IN_FAILED_TITLE: super::Localized = super::Localized {
    ja: JA_IDENTITY_SIGN_IN_FAILED_TITLE,
    en: EN_IDENTITY_SIGN_IN_FAILED_TITLE,
};
pub const IDENTITY_SIGN_IN_FAILED_BODY: super::Localized = super::Localized {
    ja: JA_IDENTITY_SIGN_IN_FAILED_BODY,
    en: EN_IDENTITY_SIGN_IN_FAILED_BODY,
};
pub const IDENTITY_SIGN_IN_RETRY: super::Localized = super::Localized {
    ja: JA_IDENTITY_SIGN_IN_RETRY,
    en: EN_IDENTITY_SIGN_IN_RETRY,
};
pub const IDENTITY_SIGN_IN_CANCEL: super::Localized = super::Localized {
    ja: JA_IDENTITY_SIGN_IN_CANCEL,
    en: EN_IDENTITY_SIGN_IN_CANCEL,
};
/// RFC-084 (Handoff 084): paired for `account/mod.rs::render_freshness`'s
/// stale-session re-sign-in link — the only remaining bare reference to
/// this pair in the corpus.
pub const IDENTITY_SIGN_IN_LINK: super::Localized = super::Localized {
    ja: JA_IDENTITY_SIGN_IN_LINK,
    en: EN_IDENTITY_SIGN_IN_LINK,
};
