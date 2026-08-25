// ── Account recovery credential consumption (RFC-081 §3, Handoff 057) ────
// Anonymous route, same shape as `access.rs`'s RELINK_* strings — one
// generic invalid message for every failure cause (unknown, consumed,
// revoked, expired code), never a distinct one per cause.

pub const EN_RECOVERY_TITLE: &str = "Recover your account";
pub const EN_RECOVERY_BODY: &str = "Enter the recovery code you saved earlier.";
pub const EN_RECOVERY_CODE_LABEL: &str = "Recovery code";
pub const EN_RECOVERY_SUBMIT: &str = "Sign in";
pub const EN_RECOVERY_INVALID: &str =
    "This code cannot be used. It may have already been used, or it may be incorrect.";

pub const JA_RECOVERY_TITLE: &str = "アカウントを復旧する";
pub const JA_RECOVERY_BODY: &str = "以前保存した復旧用のコードを入力してください。";
pub const JA_RECOVERY_CODE_LABEL: &str = "復旧コード";
pub const JA_RECOVERY_SUBMIT: &str = "サインイン";
pub const JA_RECOVERY_INVALID: &str =
    "このコードは使用できません。すでに使われているか、正しくない可能性があります。";

/// RFC-083 Slice D2a (Handoff 075) locale-aware pairs; see `i18n::Localized`.
pub const RECOVERY_TITLE: super::Localized = super::Localized {
    ja: JA_RECOVERY_TITLE,
    en: EN_RECOVERY_TITLE,
};
pub const RECOVERY_BODY: super::Localized = super::Localized {
    ja: JA_RECOVERY_BODY,
    en: EN_RECOVERY_BODY,
};
pub const RECOVERY_CODE_LABEL: super::Localized = super::Localized {
    ja: JA_RECOVERY_CODE_LABEL,
    en: EN_RECOVERY_CODE_LABEL,
};
pub const RECOVERY_SUBMIT: super::Localized = super::Localized {
    ja: JA_RECOVERY_SUBMIT,
    en: EN_RECOVERY_SUBMIT,
};
pub const RECOVERY_INVALID: super::Localized = super::Localized {
    ja: JA_RECOVERY_INVALID,
    en: EN_RECOVERY_INVALID,
};
