// ── Note editor ───────────────────────────────────────────────────────────
pub const EN_NOTE_SAVE: &str = "Save Note";
pub const EN_NOTE_DELETE: &str = "Delete Note";
pub const EN_NOTE_TOO_LONG: &str = "Your note is too long. Please keep it under 200 characters.";

pub const JA_NOTE_SAVE: &str = "メモを保存";
pub const JA_NOTE_DELETE: &str = "メモを削除";
pub const JA_NOTE_TOO_LONG: &str = "メモが長すぎます。200文字以内にしてください。";

// ── Note editor (additional) ──────────────────────────────────────────────
pub const EN_NOTE_SECTION_LABEL: &str = "Your note";
pub const EN_NOTE_PLACEHOLDER_LABEL: &str = "Note (up to 200 characters)";
pub const EN_NOTE_CHAR_HINT: &str = "Up to 200 characters";
pub const EN_NOTE_VISIBILITY: &str = "Community members can see this note.";

pub const JA_NOTE_SECTION_LABEL: &str = "あなたのメモ";
pub const JA_NOTE_PLACEHOLDER_LABEL: &str = "メモ（200文字以内）";
pub const JA_NOTE_CHAR_HINT: &str = "200文字以内";
pub const JA_NOTE_VISIBILITY: &str = "コミュニティのメンバーにこのメモが表示されます。";
pub const JA_NOTE_DELETE_BODY: &str = "このメモは削除されます。元に戻すことはできません。";
pub const EN_NOTE_KEEP_ACTION: &str = "Keep note";
pub const JA_NOTE_KEEP_ACTION: &str = "メモを保持";
pub const EN_NOTE_DELETE_BODY: &str = "Your note will be removed. This cannot be undone.";

// Handoff 037: were raw-English `?flash=saved`/`?flash=Note+removed` query
// values echoed verbatim into rendered text on Event Detail — member-facing,
// so `Localized` pairs, resolved through a per-surface code mapper
// (`note_flash_message`, `event.rs`), the `calendar_flash_message` pattern.
// "Hidden", not "removed": the action is `ADMIN_HIDE_NOTE` — the note is
// hidden, not deleted, and the old English said something untrue.
pub const EN_NOTE_SAVED_FLASH: &str = "Note saved.";
pub const EN_NOTE_HIDDEN_FLASH: &str = "Note hidden.";
pub const JA_NOTE_SAVED_FLASH: &str = "メモを保存しました。";
pub const JA_NOTE_HIDDEN_FLASH: &str = "メモを非表示にしました。";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const NOTE_DELETE: super::Localized = super::Localized {
    ja: JA_NOTE_DELETE,
    en: EN_NOTE_DELETE,
};
pub const NOTE_DELETE_BODY: super::Localized = super::Localized {
    ja: JA_NOTE_DELETE_BODY,
    en: EN_NOTE_DELETE_BODY,
};
pub const NOTE_KEEP_ACTION: super::Localized = super::Localized {
    ja: JA_NOTE_KEEP_ACTION,
    en: EN_NOTE_KEEP_ACTION,
};
pub const NOTE_SAVE: super::Localized = super::Localized {
    ja: JA_NOTE_SAVE,
    en: EN_NOTE_SAVE,
};
pub const NOTE_SECTION_LABEL: super::Localized = super::Localized {
    ja: JA_NOTE_SECTION_LABEL,
    en: EN_NOTE_SECTION_LABEL,
};
pub const NOTE_PLACEHOLDER_LABEL: super::Localized = super::Localized {
    ja: JA_NOTE_PLACEHOLDER_LABEL,
    en: EN_NOTE_PLACEHOLDER_LABEL,
};
pub const NOTE_CHAR_HINT: super::Localized = super::Localized {
    ja: JA_NOTE_CHAR_HINT,
    en: EN_NOTE_CHAR_HINT,
};
pub const NOTE_VISIBILITY: super::Localized = super::Localized {
    ja: JA_NOTE_VISIBILITY,
    en: EN_NOTE_VISIBILITY,
};
pub const NOTE_SAVED_FLASH: super::Localized = super::Localized {
    ja: JA_NOTE_SAVED_FLASH,
    en: EN_NOTE_SAVED_FLASH,
};
pub const NOTE_HIDDEN_FLASH: super::Localized = super::Localized {
    ja: JA_NOTE_HIDDEN_FLASH,
    en: EN_NOTE_HIDDEN_FLASH,
};
