// ── Event templates (RFC-032) ─────────────────────────────────────────────
pub const EN_TEMPLATES_TITLE: &str = "Event Templates";
pub const EN_TEMPLATES_DESCRIPTION: &str =
    "Save common event details as templates to create events faster.";
pub const EN_TEMPLATES_EMPTY: &str = "No templates yet.";
pub const EN_TEMPLATES_SAVE_SECTION: &str = "Save a template";
pub const EN_TEMPLATES_TITLE_LABEL: &str = "Title";
pub const EN_TEMPLATES_LOC_LABEL: &str = "Location (optional)";
pub const EN_TEMPLATES_DUR_LABEL: &str = "Default duration in minutes (optional)";
pub const EN_TEMPLATES_SAVE_BTN: &str = "Save template";
pub const EN_TEMPLATES_USE_BTN: &str = "Use";
pub const EN_TEMPLATES_DELETE_BTN: &str = "Delete";
// Handoff 074: RFC-083 Slice D1c. Proposed wording, flagged for owner
// review — see this package's review request.
pub const EN_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH: &str = "Enter a title.";
pub const EN_ADMIN_TEMPLATE_SAVED_FLASH: &str = "Template saved.";
pub const EN_ADMIN_TEMPLATE_DELETED_FLASH: &str = "Template deleted.";

pub const JA_TEMPLATES_TITLE: &str = "イベントテンプレート";
pub const JA_TEMPLATES_DESCRIPTION: &str =
    "よく使うイベント情報をテンプレートとして保存して、素早く作成できます。";
pub const JA_TEMPLATES_EMPTY: &str = "まだテンプレートがありません。";
pub const JA_TEMPLATES_SAVE_SECTION: &str = "テンプレートを保存";
pub const JA_TEMPLATES_TITLE_LABEL: &str = "タイトル";
pub const JA_TEMPLATES_LOC_LABEL: &str = "場所（任意）";
pub const JA_TEMPLATES_DUR_LABEL: &str = "デフォルトの所要時間（分、任意）";
pub const JA_TEMPLATES_SAVE_BTN: &str = "テンプレートを保存";
pub const JA_TEMPLATES_USE_BTN: &str = "使用";
pub const JA_TEMPLATES_DELETE_BTN: &str = "削除";

// Handoff 037: were raw-English `?flash=Title+required`/`Template+saved`/
// `Template+deleted` query values echoed verbatim into rendered text.
// Handoff 074 (RFC-083 Slice D1c) gave these English halves and paired
// them below; this page now resolves locale.
pub const JA_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH: &str = "タイトルを入力してください。";
pub const JA_ADMIN_TEMPLATE_SAVED_FLASH: &str = "テンプレートを保存しました。";
pub const JA_ADMIN_TEMPLATE_DELETED_FLASH: &str = "テンプレートを削除しました。";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const TEMPLATES_TITLE: super::Localized = super::Localized {
    ja: JA_TEMPLATES_TITLE,
    en: EN_TEMPLATES_TITLE,
};
pub const TEMPLATES_DESCRIPTION: super::Localized = super::Localized {
    ja: JA_TEMPLATES_DESCRIPTION,
    en: EN_TEMPLATES_DESCRIPTION,
};
pub const TEMPLATES_EMPTY: super::Localized = super::Localized {
    ja: JA_TEMPLATES_EMPTY,
    en: EN_TEMPLATES_EMPTY,
};
pub const TEMPLATES_SAVE_SECTION: super::Localized = super::Localized {
    ja: JA_TEMPLATES_SAVE_SECTION,
    en: EN_TEMPLATES_SAVE_SECTION,
};
pub const TEMPLATES_TITLE_LABEL: super::Localized = super::Localized {
    ja: JA_TEMPLATES_TITLE_LABEL,
    en: EN_TEMPLATES_TITLE_LABEL,
};
pub const TEMPLATES_LOC_LABEL: super::Localized = super::Localized {
    ja: JA_TEMPLATES_LOC_LABEL,
    en: EN_TEMPLATES_LOC_LABEL,
};
pub const TEMPLATES_DUR_LABEL: super::Localized = super::Localized {
    ja: JA_TEMPLATES_DUR_LABEL,
    en: EN_TEMPLATES_DUR_LABEL,
};
pub const TEMPLATES_SAVE_BTN: super::Localized = super::Localized {
    ja: JA_TEMPLATES_SAVE_BTN,
    en: EN_TEMPLATES_SAVE_BTN,
};
pub const TEMPLATES_USE_BTN: super::Localized = super::Localized {
    ja: JA_TEMPLATES_USE_BTN,
    en: EN_TEMPLATES_USE_BTN,
};
pub const TEMPLATES_DELETE_BTN: super::Localized = super::Localized {
    ja: JA_TEMPLATES_DELETE_BTN,
    en: EN_TEMPLATES_DELETE_BTN,
};
pub const ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH: super::Localized = super::Localized {
    ja: JA_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH,
    en: EN_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH,
};
pub const ADMIN_TEMPLATE_SAVED_FLASH: super::Localized = super::Localized {
    ja: JA_ADMIN_TEMPLATE_SAVED_FLASH,
    en: EN_ADMIN_TEMPLATE_SAVED_FLASH,
};
pub const ADMIN_TEMPLATE_DELETED_FLASH: super::Localized = super::Localized {
    ja: JA_ADMIN_TEMPLATE_DELETED_FLASH,
    en: EN_ADMIN_TEMPLATE_DELETED_FLASH,
};
