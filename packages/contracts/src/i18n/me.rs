use super::Localized;

// ── Me / profile ──────────────────────────────────────────────────────────
pub const EN_ME_SECTION_NAME: &str = "Name";
pub const EN_ME_CHANGE_DISPLAY_NAME: &str = "Change display name";
pub const EN_ME_DISPLAY_NAME_EDIT_TITLE: &str = "Change display name";
pub const EN_ME_DISPLAY_NAME_EDIT_SUBMIT: &str = "Save display name";
pub const EN_ME_DISPLAY_NAME_EDIT_CANCEL: &str = "Cancel";
pub const EN_ME_DISPLAY_NAME_UPDATED: &str = "Display name updated.";
pub const EN_ME_DISPLAY_NAME_ERROR: &str = "Enter a display name.";
pub const EN_ME_SECTION_COMMUNITY: &str = "Current community";
pub const EN_ME_SECTION_HELP: &str = "Help";
pub const EN_ME_HELP_BODY: &str = "Ask your community admin if you cannot enter or lost access.";

pub const JA_ME_SECTION_NAME: &str = "名前";
pub const JA_ME_CHANGE_DISPLAY_NAME: &str = "表示名を変更";
pub const JA_ME_DISPLAY_NAME_EDIT_TITLE: &str = "表示名を変更";
pub const JA_ME_DISPLAY_NAME_EDIT_SUBMIT: &str = "表示名を保存";
pub const JA_ME_DISPLAY_NAME_EDIT_CANCEL: &str = "やめる";
pub const JA_ME_DISPLAY_NAME_UPDATED: &str = "表示名を変更しました。";
pub const JA_ME_DISPLAY_NAME_ERROR: &str = "表示名を入力してください。";
pub const JA_ME_SECTION_COMMUNITY: &str = "現在のコミュニティ";
pub const JA_ME_SECTION_HELP: &str = "ヘルプ";
pub const JA_ME_HELP_BODY: &str =
    "入室できない場合やアクセスを失った場合は、コミュニティの管理者にお問い合わせください。";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const ME_SECTION_NAME: Localized = Localized {
    ja: JA_ME_SECTION_NAME,
    en: EN_ME_SECTION_NAME,
};
pub const ME_CHANGE_DISPLAY_NAME: Localized = Localized {
    ja: JA_ME_CHANGE_DISPLAY_NAME,
    en: EN_ME_CHANGE_DISPLAY_NAME,
};
pub const ME_DISPLAY_NAME_UPDATED: Localized = Localized {
    ja: JA_ME_DISPLAY_NAME_UPDATED,
    en: EN_ME_DISPLAY_NAME_UPDATED,
};
pub const ME_SECTION_COMMUNITY: Localized = Localized {
    ja: JA_ME_SECTION_COMMUNITY,
    en: EN_ME_SECTION_COMMUNITY,
};
pub const ME_SECTION_HELP: Localized = Localized {
    ja: JA_ME_SECTION_HELP,
    en: EN_ME_SECTION_HELP,
};
pub const ME_HELP_BODY: Localized = Localized {
    ja: JA_ME_HELP_BODY,
    en: EN_ME_HELP_BODY,
};

// ── Support / about (RFC-035) ─────────────────────────────────────────────
pub const EN_ME_SECTION_ABOUT: &str = "About";
pub const EN_ME_VERSION_LABEL: &str = "Version";
pub const EN_ME_REF_LABEL: &str = "Ref";
pub const EN_ME_SECTION_DATA: &str = "Data";
pub const EN_ME_EXPORT_LINK: &str = "Export community data";
pub const EN_ME_SECTION_ADMIN: &str = "Admin";
pub const EN_ME_MANAGE_MEMBERS: &str = "Manage members";

pub const JA_ME_SECTION_ABOUT: &str = "このアプリについて";
pub const JA_ME_VERSION_LABEL: &str = "バージョン";
pub const JA_ME_REF_LABEL: &str = "参照コード";
pub const JA_ME_SECTION_DATA: &str = "データ";
pub const JA_ME_EXPORT_LINK: &str = "記録をダウンロード";
pub const JA_ME_SECTION_ADMIN: &str = "管理";
pub const JA_ME_MANAGE_MEMBERS: &str = "メンバーを管理";

pub const EN_ME_CALENDAR_LABEL: &str = "Calendar feed";
pub const JA_ME_CALENDAR_LABEL: &str = "予定をカレンダーに入れる";
pub const EN_ME_DATA_EXPORT: &str = "Export community data";
pub const JA_ME_DATA_EXPORT: &str = "記録をダウンロード";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const ME_SECTION_ABOUT: Localized = Localized {
    ja: JA_ME_SECTION_ABOUT,
    en: EN_ME_SECTION_ABOUT,
};
pub const ME_VERSION_LABEL: Localized = Localized {
    ja: JA_ME_VERSION_LABEL,
    en: EN_ME_VERSION_LABEL,
};
pub const ME_REF_LABEL: Localized = Localized {
    ja: JA_ME_REF_LABEL,
    en: EN_ME_REF_LABEL,
};
pub const ME_SECTION_ADMIN: Localized = Localized {
    ja: JA_ME_SECTION_ADMIN,
    en: EN_ME_SECTION_ADMIN,
};
pub const ME_MANAGE_MEMBERS: Localized = Localized {
    ja: JA_ME_MANAGE_MEMBERS,
    en: EN_ME_MANAGE_MEMBERS,
};
pub const ME_CALENDAR_LABEL: Localized = Localized {
    ja: JA_ME_CALENDAR_LABEL,
    en: EN_ME_CALENDAR_LABEL,
};
pub const ME_DATA_EXPORT: Localized = Localized {
    ja: JA_ME_DATA_EXPORT,
    en: EN_ME_DATA_EXPORT,
};

// ── Language settings (RFC-072) ──────────────────────────────────────────
// Copy decided in the Slice A review (Handoff 021 §19 / Handoff 022 §7.2).
// `ME_LANGUAGE_CANCEL` is deliberately its own pair, not a reuse of
// `ME_DISPLAY_NAME_EDIT_CANCEL`, even though the values match today —
// reusing it would couple two unrelated surfaces through a name that
// describes only one of them.
pub const EN_ME_LANGUAGE_TITLE: &str = "Language";
pub const EN_ME_LANGUAGE_SUBMIT: &str = "Save";
pub const EN_ME_LANGUAGE_UPDATED: &str = "Language updated.";
pub const EN_ME_LANGUAGE_CANCEL: &str = "Cancel";

pub const JA_ME_LANGUAGE_TITLE: &str = "言語";
pub const JA_ME_LANGUAGE_SUBMIT: &str = "保存";
pub const JA_ME_LANGUAGE_UPDATED: &str = "表示言語を変更しました。";
pub const JA_ME_LANGUAGE_CANCEL: &str = "やめる";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const ME_LANGUAGE_TITLE: Localized = Localized {
    ja: JA_ME_LANGUAGE_TITLE,
    en: EN_ME_LANGUAGE_TITLE,
};
pub const ME_LANGUAGE_SUBMIT: Localized = Localized {
    ja: JA_ME_LANGUAGE_SUBMIT,
    en: EN_ME_LANGUAGE_SUBMIT,
};
pub const ME_LANGUAGE_UPDATED: Localized = Localized {
    ja: JA_ME_LANGUAGE_UPDATED,
    en: EN_ME_LANGUAGE_UPDATED,
};
pub const ME_LANGUAGE_CANCEL: Localized = Localized {
    ja: JA_ME_LANGUAGE_CANCEL,
    en: EN_ME_LANGUAGE_CANCEL,
};

/// A language's own name for itself does not vary with the render locale,
/// so these deliberately do not fit the `Localized` shape (Slice A review,
/// Q2) — plain, unpaired, always-rendered-as-is literals.
pub const LANGUAGE_OPTION_JA: &str = "日本語";
pub const LANGUAGE_OPTION_EN: &str = "English";
