// ── The account surface (RFC-080 §6, RFC-081 §6, Handoff 055) ────────────
// Japanese-only, matching `handlers/identity/mod.rs`'s own convention
// (RFC-072 Slice D): this is a top-level, non-community-scoped tier, so
// there is no single membership's `ui_language` to resolve a locale from.
// EN_* constants still added for `en_ja_parity`/`i18n_en_ja_parity_count`
// compliance even though unused in rendering.

pub const EN_ACCOUNT_PAGE_TITLE: &str = "Account";
pub const EN_ACCOUNT_LINKED_IDENTITIES_HEADING: &str = "Linked external accounts";
pub const EN_ACCOUNT_NO_LINKED_IDENTITIES: &str = "No external account is linked.";
pub const EN_ACCOUNT_LINKED_AT_PREFIX: &str = "Linked ";
pub const EN_ACCOUNT_RECOVERY_CREDENTIAL_HEADING: &str = "Recovery credential";
pub const EN_ACCOUNT_RECOVERY_CREDENTIAL_NONE: &str = "Not set up yet.";
pub const EN_ACCOUNT_COMMUNITIES_HEADING: &str = "Your communities";
pub const EN_ACCOUNT_NO_COMMUNITIES: &str = "You do not belong to any community.";
pub const EN_ACCOUNT_FRESH_CAN_MANAGE: &str = "You can manage these settings now.";
pub const EN_ACCOUNT_STALE_SIGN_IN_AGAIN: &str = "Sign in again to manage these settings.";

// ── Linking (RFC-081 §4, Handoff 056) ─────────────────────────────────────
pub const EN_ACCOUNT_LINK_ENTRY_LABEL: &str = "Link an external account";
pub const EN_ACCOUNT_LINK_TITLE: &str = "Link an external account";
pub const EN_ACCOUNT_LINK_BODY: &str = "You will be sent to sign in with the external account you want to link. \
     No existing sign-in method is removed.";
pub const EN_ACCOUNT_LINK_SUBMIT: &str = "Continue";
pub const EN_ACCOUNT_LINK_CANCEL: &str = "Cancel";

// ── Recovery credential and unlink (RFC-081 §3, Handoff 057) ─────────────
pub const EN_ACCOUNT_RECOVERY_CREDENTIAL_EXISTS: &str = "Set up.";
pub const EN_ACCOUNT_RECOVERY_REGENERATE_LABEL: &str = "Generate a new code";
pub const EN_ACCOUNT_RECOVERY_REVEAL_WARNING: &str = "Write down or copy this code now. If you leave or reload this page, it will never be shown again.";
pub const EN_ACCOUNT_RECOVERY_REVEAL_HINT: &str = "Keep it somewhere safe, the same way you would a password. \
     Anyone who has it can sign in to your account.";
pub const EN_ACCOUNT_RECOVERY_CONTINUE: &str = "Continue to your account";
pub const EN_ACCOUNT_UNLINK_LABEL: &str = "Unlink";
pub const EN_ACCOUNT_UNLINK_TITLE: &str = "Unlink this account";
pub const EN_ACCOUNT_UNLINK_BODY: &str =
    "This removes this external account as a way to sign in. This cannot be undone.";
pub const EN_ACCOUNT_UNLINK_SUBMIT: &str = "Unlink";
pub const EN_ACCOUNT_UNLINK_CANCEL: &str = "Cancel";
pub const EN_ACCOUNT_UNLINK_REFUSED: &str =
    "This could not be unlinked. You need at least one other way to sign in.";

pub const JA_ACCOUNT_PAGE_TITLE: &str = "アカウント";
pub const JA_ACCOUNT_LINKED_IDENTITIES_HEADING: &str = "連携している外部アカウント";
pub const JA_ACCOUNT_NO_LINKED_IDENTITIES: &str = "連携している外部アカウントはありません。";
pub const JA_ACCOUNT_LINKED_AT_PREFIX: &str = "連携日: ";
pub const JA_ACCOUNT_RECOVERY_CREDENTIAL_HEADING: &str = "復旧用の認証情報";
pub const JA_ACCOUNT_RECOVERY_CREDENTIAL_NONE: &str = "まだ設定されていません。";
pub const JA_ACCOUNT_COMMUNITIES_HEADING: &str = "参加しているコミュニティ";
pub const JA_ACCOUNT_NO_COMMUNITIES: &str = "参加しているコミュニティはありません。";
pub const JA_ACCOUNT_FRESH_CAN_MANAGE: &str = "これらの設定は今すぐ管理できます。";
pub const JA_ACCOUNT_STALE_SIGN_IN_AGAIN: &str =
    "これらの設定を管理するには、もう一度サインインしてください。";

pub const JA_ACCOUNT_LINK_ENTRY_LABEL: &str = "外部アカウントを連携する";
pub const JA_ACCOUNT_LINK_TITLE: &str = "外部アカウントを連携する";
pub const JA_ACCOUNT_LINK_BODY: &str = "連携したい外部アカウントでサインインする画面に移動します。既存のサインイン方法が失われることはありません。";
pub const JA_ACCOUNT_LINK_SUBMIT: &str = "続ける";
pub const JA_ACCOUNT_LINK_CANCEL: &str = "やめる";

pub const JA_ACCOUNT_RECOVERY_CREDENTIAL_EXISTS: &str = "設定済みです。";
pub const JA_ACCOUNT_RECOVERY_REGENERATE_LABEL: &str = "新しいコードを発行する";
pub const JA_ACCOUNT_RECOVERY_REVEAL_WARNING: &str = "このコードを今すぐ書き留めるか、コピーしてください。このページを離れたり再読み込みしたりすると、二度と表示されません。";
pub const JA_ACCOUNT_RECOVERY_REVEAL_HINT: &str = "パスワードと同じように、安全な場所に保管してください。このコードを持っている人は誰でもあなたのアカウントにサインインできます。";
pub const JA_ACCOUNT_RECOVERY_CONTINUE: &str = "アカウントに進む";
pub const JA_ACCOUNT_UNLINK_LABEL: &str = "連携を解除する";
pub const JA_ACCOUNT_UNLINK_TITLE: &str = "この連携を解除する";
pub const JA_ACCOUNT_UNLINK_BODY: &str =
    "この外部アカウントをサインイン方法から削除します。この操作は取り消せません。";
pub const JA_ACCOUNT_UNLINK_SUBMIT: &str = "連携を解除する";
pub const JA_ACCOUNT_UNLINK_CANCEL: &str = "やめる";
pub const JA_ACCOUNT_UNLINK_REFUSED: &str =
    "連携を解除できませんでした。他にサインインする方法が少なくとも1つ必要です。";

/// RFC-084 (Handoff 084) locale-aware pairs; see `i18n::Localized`. No new
/// copy — every EN/JA half above already existed (RFC-072 Slice D deferral,
/// Handoffs 055–057), added for `en_ja_parity` compliance but never paired
/// since the account tier had no locale source until now.
pub const ACCOUNT_PAGE_TITLE: super::Localized = super::Localized {
    ja: JA_ACCOUNT_PAGE_TITLE,
    en: EN_ACCOUNT_PAGE_TITLE,
};
pub const ACCOUNT_LINKED_IDENTITIES_HEADING: super::Localized = super::Localized {
    ja: JA_ACCOUNT_LINKED_IDENTITIES_HEADING,
    en: EN_ACCOUNT_LINKED_IDENTITIES_HEADING,
};
pub const ACCOUNT_NO_LINKED_IDENTITIES: super::Localized = super::Localized {
    ja: JA_ACCOUNT_NO_LINKED_IDENTITIES,
    en: EN_ACCOUNT_NO_LINKED_IDENTITIES,
};
pub const ACCOUNT_LINKED_AT_PREFIX: super::Localized = super::Localized {
    ja: JA_ACCOUNT_LINKED_AT_PREFIX,
    en: EN_ACCOUNT_LINKED_AT_PREFIX,
};
pub const ACCOUNT_RECOVERY_CREDENTIAL_HEADING: super::Localized = super::Localized {
    ja: JA_ACCOUNT_RECOVERY_CREDENTIAL_HEADING,
    en: EN_ACCOUNT_RECOVERY_CREDENTIAL_HEADING,
};
pub const ACCOUNT_RECOVERY_CREDENTIAL_NONE: super::Localized = super::Localized {
    ja: JA_ACCOUNT_RECOVERY_CREDENTIAL_NONE,
    en: EN_ACCOUNT_RECOVERY_CREDENTIAL_NONE,
};
pub const ACCOUNT_COMMUNITIES_HEADING: super::Localized = super::Localized {
    ja: JA_ACCOUNT_COMMUNITIES_HEADING,
    en: EN_ACCOUNT_COMMUNITIES_HEADING,
};
pub const ACCOUNT_NO_COMMUNITIES: super::Localized = super::Localized {
    ja: JA_ACCOUNT_NO_COMMUNITIES,
    en: EN_ACCOUNT_NO_COMMUNITIES,
};
pub const ACCOUNT_FRESH_CAN_MANAGE: super::Localized = super::Localized {
    ja: JA_ACCOUNT_FRESH_CAN_MANAGE,
    en: EN_ACCOUNT_FRESH_CAN_MANAGE,
};
pub const ACCOUNT_STALE_SIGN_IN_AGAIN: super::Localized = super::Localized {
    ja: JA_ACCOUNT_STALE_SIGN_IN_AGAIN,
    en: EN_ACCOUNT_STALE_SIGN_IN_AGAIN,
};
pub const ACCOUNT_LINK_ENTRY_LABEL: super::Localized = super::Localized {
    ja: JA_ACCOUNT_LINK_ENTRY_LABEL,
    en: EN_ACCOUNT_LINK_ENTRY_LABEL,
};
pub const ACCOUNT_LINK_TITLE: super::Localized = super::Localized {
    ja: JA_ACCOUNT_LINK_TITLE,
    en: EN_ACCOUNT_LINK_TITLE,
};
pub const ACCOUNT_LINK_BODY: super::Localized = super::Localized {
    ja: JA_ACCOUNT_LINK_BODY,
    en: EN_ACCOUNT_LINK_BODY,
};
pub const ACCOUNT_LINK_SUBMIT: super::Localized = super::Localized {
    ja: JA_ACCOUNT_LINK_SUBMIT,
    en: EN_ACCOUNT_LINK_SUBMIT,
};
pub const ACCOUNT_LINK_CANCEL: super::Localized = super::Localized {
    ja: JA_ACCOUNT_LINK_CANCEL,
    en: EN_ACCOUNT_LINK_CANCEL,
};
pub const ACCOUNT_RECOVERY_CREDENTIAL_EXISTS: super::Localized = super::Localized {
    ja: JA_ACCOUNT_RECOVERY_CREDENTIAL_EXISTS,
    en: EN_ACCOUNT_RECOVERY_CREDENTIAL_EXISTS,
};
pub const ACCOUNT_RECOVERY_REGENERATE_LABEL: super::Localized = super::Localized {
    ja: JA_ACCOUNT_RECOVERY_REGENERATE_LABEL,
    en: EN_ACCOUNT_RECOVERY_REGENERATE_LABEL,
};
pub const ACCOUNT_RECOVERY_REVEAL_WARNING: super::Localized = super::Localized {
    ja: JA_ACCOUNT_RECOVERY_REVEAL_WARNING,
    en: EN_ACCOUNT_RECOVERY_REVEAL_WARNING,
};
pub const ACCOUNT_RECOVERY_REVEAL_HINT: super::Localized = super::Localized {
    ja: JA_ACCOUNT_RECOVERY_REVEAL_HINT,
    en: EN_ACCOUNT_RECOVERY_REVEAL_HINT,
};
pub const ACCOUNT_UNLINK_LABEL: super::Localized = super::Localized {
    ja: JA_ACCOUNT_UNLINK_LABEL,
    en: EN_ACCOUNT_UNLINK_LABEL,
};
pub const ACCOUNT_UNLINK_TITLE: super::Localized = super::Localized {
    ja: JA_ACCOUNT_UNLINK_TITLE,
    en: EN_ACCOUNT_UNLINK_TITLE,
};
pub const ACCOUNT_UNLINK_BODY: super::Localized = super::Localized {
    ja: JA_ACCOUNT_UNLINK_BODY,
    en: EN_ACCOUNT_UNLINK_BODY,
};
pub const ACCOUNT_UNLINK_SUBMIT: super::Localized = super::Localized {
    ja: JA_ACCOUNT_UNLINK_SUBMIT,
    en: EN_ACCOUNT_UNLINK_SUBMIT,
};
pub const ACCOUNT_UNLINK_CANCEL: super::Localized = super::Localized {
    ja: JA_ACCOUNT_UNLINK_CANCEL,
    en: EN_ACCOUNT_UNLINK_CANCEL,
};
pub const ACCOUNT_UNLINK_REFUSED: super::Localized = super::Localized {
    ja: JA_ACCOUNT_UNLINK_REFUSED,
    en: EN_ACCOUNT_UNLINK_REFUSED,
};
