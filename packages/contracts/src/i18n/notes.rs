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
pub const JA_NOTE_DELETE_BODY: &str = "このメモは削除されます。この操作は取り消せません。";
pub const EN_NOTE_KEEP_ACTION: &str = "Keep note";
pub const JA_NOTE_KEEP_ACTION: &str = "やめる";
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

// Handoff (RFC-054 Slice 5): the admin hide-note confirm previously borrowed
// ADMIN_REMOVE_CONSEQUENCE (member removal) for its body, and NOTE_DELETE
// ("Delete Note") for its title and button — neither describes what this
// action does. It sets hidden_by_admin_at, not note_deleted_at; "hidden," not
// "deleted," matching NOTE_HIDDEN_FLASH above.
pub const EN_ADMIN_HIDE_NOTE_TITLE: &str = "Hide this note?";
pub const EN_ADMIN_HIDE_NOTE_CONSEQUENCE: &str = "This note will no longer be shown to anyone, including the member who wrote it. Their membership and other notes are unaffected. This cannot be undone.";
pub const EN_ADMIN_HIDE_NOTE_CONFIRM: &str = "Hide note";
pub const JA_ADMIN_HIDE_NOTE_TITLE: &str = "メモを非表示にしますか？";
pub const JA_ADMIN_HIDE_NOTE_CONSEQUENCE: &str = "このメモは誰にも表示されなくなります。書いた本人にも表示されません。メンバーの参加やほかのメモには影響しません。この操作は取り消せません。";
pub const JA_ADMIN_HIDE_NOTE_CONFIRM: &str = "メモを非表示にする";

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
pub const ADMIN_HIDE_NOTE_TITLE: super::Localized = super::Localized {
    ja: JA_ADMIN_HIDE_NOTE_TITLE,
    en: EN_ADMIN_HIDE_NOTE_TITLE,
};
pub const ADMIN_HIDE_NOTE_CONSEQUENCE: super::Localized = super::Localized {
    ja: JA_ADMIN_HIDE_NOTE_CONSEQUENCE,
    en: EN_ADMIN_HIDE_NOTE_CONSEQUENCE,
};
pub const ADMIN_HIDE_NOTE_CONFIRM: super::Localized = super::Localized {
    ja: JA_ADMIN_HIDE_NOTE_CONFIRM,
    en: EN_ADMIN_HIDE_NOTE_CONFIRM,
};
