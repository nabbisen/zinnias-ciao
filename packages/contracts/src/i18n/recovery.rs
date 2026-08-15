// ── Account recovery credential consumption (RFC-081 §3, Handoff 057) ────
// Anonymous route, same shape as `access.rs`'s RELINK_* strings — one
// generic invalid message for every failure cause (unknown, consumed,
// revoked, expired code), never a distinct one per cause.

pub const EN_RECOVERY_TITLE: &str = "Recover your account";
pub const EN_RECOVERY_BODY: &str = "Enter your account recovery code.";
pub const EN_RECOVERY_CODE_LABEL: &str = "Recovery code";
pub const EN_RECOVERY_SUBMIT: &str = "Continue";
pub const EN_RECOVERY_INVALID: &str =
    "This code cannot be used. It may have already been used, or it may be incorrect.";

pub const JA_RECOVERY_TITLE: &str = "アカウントを復旧する";
pub const JA_RECOVERY_BODY: &str = "アカウント復旧用のコードを入力してください。";
pub const JA_RECOVERY_CODE_LABEL: &str = "復旧コード";
pub const JA_RECOVERY_SUBMIT: &str = "続ける";
pub const JA_RECOVERY_INVALID: &str =
    "このコードは使用できません。すでに使われているか、正しくない可能性があります。";
