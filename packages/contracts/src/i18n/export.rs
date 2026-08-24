// ── Community export (RFC-027) ────────────────────────────────────────────
pub const EN_EXPORT_TITLE: &str = "Export community data";
pub const EN_EXPORT_DESCRIPTION: &str =
    "Download a JSON file of your community's events, attendance, and notes.";
pub const EN_EXPORT_PRIVACY_NOTE: &str = "Member names and notes are included. Session tokens and security credentials are not included.";
pub const EN_EXPORT_DOWNLOAD_BTN: &str = "Download JSON";
pub const EN_EXPORT_SINGLE_USE: &str = "This link is single-use and expires in 5 minutes.";
// Handoff 074: RFC-083 Slice D1c. Proposed wording, flagged for owner
// review — see this package's review request. Restores the exact English
// phrasing the Handoff 036 comment below documents as the pre-Japanese
// -only original, rather than inventing new copy. Both named placeholders
// (`{events}`, `{members}`) must survive verbatim — substitution is by
// name (see `get_export_page`), so English word order is free, but
// dropping either placeholder renders it literally.
pub const EN_ADMIN_EXPORT_SUMMARY_COUNTS: &str = "{events} events · {members} active members";

pub const JA_EXPORT_TITLE: &str = "コミュニティの記録をダウンロード";
pub const JA_EXPORT_DESCRIPTION: &str =
    "イベント・出欠・メモの記録をファイルでダウンロードします。";
pub const JA_EXPORT_PRIVACY_NOTE: &str =
    "メンバー名とメモが含まれます。ログイン情報や招待コードは含まれません。";
pub const JA_EXPORT_DOWNLOAD_BTN: &str = "ファイルをダウンロード";
pub const JA_EXPORT_SINGLE_USE: &str = "このリンクは1回限りで、5分後に無効になります。";
// Handoff 036 §A: was a bare "{events} events · {members} active members" —
// found by the new default-fail gate, unnamed by the handoff itself.
// Handoff 074 (RFC-083 Slice D1c) gave this constant an English half
// (restoring that original phrasing) and paired it below; this page now
// resolves locale. Named substitution (`{events}`/`{members}`), not
// positional — see `get_export_page`.
pub const JA_ADMIN_EXPORT_SUMMARY_COUNTS: &str = "予定{events}件 · 有効メンバー{members}人";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const EXPORT_TITLE: super::Localized = super::Localized {
    ja: JA_EXPORT_TITLE,
    en: EN_EXPORT_TITLE,
};
pub const EXPORT_DESCRIPTION: super::Localized = super::Localized {
    ja: JA_EXPORT_DESCRIPTION,
    en: EN_EXPORT_DESCRIPTION,
};
pub const EXPORT_PRIVACY_NOTE: super::Localized = super::Localized {
    ja: JA_EXPORT_PRIVACY_NOTE,
    en: EN_EXPORT_PRIVACY_NOTE,
};
pub const EXPORT_DOWNLOAD_BTN: super::Localized = super::Localized {
    ja: JA_EXPORT_DOWNLOAD_BTN,
    en: EN_EXPORT_DOWNLOAD_BTN,
};
pub const EXPORT_SINGLE_USE: super::Localized = super::Localized {
    ja: JA_EXPORT_SINGLE_USE,
    en: EN_EXPORT_SINGLE_USE,
};
pub const ADMIN_EXPORT_SUMMARY_COUNTS: super::Localized = super::Localized {
    ja: JA_ADMIN_EXPORT_SUMMARY_COUNTS,
    en: EN_ADMIN_EXPORT_SUMMARY_COUNTS,
};
