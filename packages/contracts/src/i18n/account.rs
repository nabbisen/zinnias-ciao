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
