use super::Localized;

pub const EN_CALENDAR_MONTH_TITLE: &str = "This month";
pub const EN_CALENDAR_PREV_MONTH: &str = "Previous month";
pub const EN_CALENDAR_NEXT_MONTH: &str = "Next month";
pub const EN_CALENDAR_THIS_MONTH: &str = "This month";
pub const EN_CALENDAR_ALL_DAYS: &str = "All days";
pub const EN_CALENDAR_EMPTY_MONTH: &str = "No events this month.";
pub const EN_CALENDAR_EMPTY_DAY: &str = "No events on this day.";
pub const EN_CALENDAR_CREATE_ON_DAY: &str = "Create event on this day";
pub const EN_CALENDAR_VIEW_MONTH: &str = "Calendar";
pub const EN_CALENDAR_VIEW_LIST: &str = "Events list";
pub const EN_CALENDAR_VIEW_MATRIX: &str = "Attendance table";
pub const EN_CALENDAR_DAY_DETAIL_PROMPT: &str = "Select a date to see that day's events here.";
pub const EN_CALENDAR_MATRIX_TITLE: &str = "Monthly attendance table";
pub const EN_CALENDAR_MATRIX_TOO_LARGE: &str =
    "This month is too large for the attendance table. Use Calendar view.";
pub const EN_CALENDAR_MATRIX_NO_MEMBERS: &str = "There are no active members.";
pub const EN_CALENDAR_MATRIX_CSV_EXPORT: &str = "Save CSV";
pub const EN_CALENDAR_MATRIX_CSV_ERROR: &str = "CSV could not be saved. Please try again.";
// Label-value form for the day-cell aria-label's event count (RFC-072 Slice
// C) — "events: {count}", never "{count} events", so it needs no plural
// agreement at any count including zero and one.
pub const EN_CALENDAR_DAY_EVENTS_COUNT: &str = "events: ";
// RFC-072 Slice C: own pair, not a reuse of ROLE_MEMBER — a role badge and
// a table column header are different contexts (§5.4 of the handoff).
pub const EN_CALENDAR_MATRIX_MEMBER_COLUMN: &str = "Member";
// Handoff 036: two more bare aria-label leaks — the month-switcher <nav>
// (calendar.rs's two sites and matrix.rs's one share this pair) and the
// month/list/matrix tab <nav>. English improved over the literal that was
// there ("Calendar month"/"Calendar view") to name the landmark's purpose
// rather than its noun, per the handoff's own note.
pub const EN_CALENDAR_MONTH_NAV_ARIA_LABEL: &str = "Month navigation";
pub const EN_CALENDAR_VIEW_NAV_ARIA_LABEL: &str = "View selection";

pub const JA_CALENDAR_MONTH_TITLE: &str = "今月の予定";
pub const JA_CALENDAR_PREV_MONTH: &str = "前の月";
pub const JA_CALENDAR_NEXT_MONTH: &str = "次の月";
pub const JA_CALENDAR_THIS_MONTH: &str = "今月";
pub const JA_CALENDAR_ALL_DAYS: &str = "月全体";
pub const JA_CALENDAR_EMPTY_MONTH: &str = "今月の予定はありません。";
pub const JA_CALENDAR_EMPTY_DAY: &str = "この日の予定はありません。";
pub const JA_CALENDAR_CREATE_ON_DAY: &str = "この日にイベントを作成";
pub const JA_CALENDAR_VIEW_MONTH: &str = "カレンダー";
pub const JA_CALENDAR_VIEW_LIST: &str = "予定一覧";
pub const JA_CALENDAR_VIEW_MATRIX: &str = "回答表";
pub const JA_CALENDAR_DAY_DETAIL_PROMPT: &str = "日付を選ぶと、その日の予定がここに表示されます。";
pub const JA_CALENDAR_MATRIX_TITLE: &str = "月の回答表";
pub const JA_CALENDAR_MATRIX_TOO_LARGE: &str =
    "この月は回答表を表示するには大きすぎます。カレンダー表示をご利用ください。";
pub const JA_CALENDAR_MATRIX_NO_MEMBERS: &str = "有効なメンバーがいません。";
pub const JA_CALENDAR_MATRIX_CSV_EXPORT: &str = "CSVを保存";
pub const JA_CALENDAR_DAY_EVENTS_COUNT: &str = "予定";
pub const JA_CALENDAR_MATRIX_MEMBER_COLUMN: &str = "メンバー";
pub const JA_CALENDAR_MONTH_NAV_ARIA_LABEL: &str = "月の切り替え";
pub const JA_CALENDAR_VIEW_NAV_ARIA_LABEL: &str = "表示の切り替え";
pub const JA_CALENDAR_MATRIX_CSV_ERROR: &str =
    "CSVを保存できませんでした。もう一度お試しください。";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const CALENDAR_MONTH_TITLE: Localized = Localized {
    ja: JA_CALENDAR_MONTH_TITLE,
    en: EN_CALENDAR_MONTH_TITLE,
};
pub const CALENDAR_PREV_MONTH: Localized = Localized {
    ja: JA_CALENDAR_PREV_MONTH,
    en: EN_CALENDAR_PREV_MONTH,
};
pub const CALENDAR_NEXT_MONTH: Localized = Localized {
    ja: JA_CALENDAR_NEXT_MONTH,
    en: EN_CALENDAR_NEXT_MONTH,
};
pub const CALENDAR_THIS_MONTH: Localized = Localized {
    ja: JA_CALENDAR_THIS_MONTH,
    en: EN_CALENDAR_THIS_MONTH,
};
pub const CALENDAR_ALL_DAYS: Localized = Localized {
    ja: JA_CALENDAR_ALL_DAYS,
    en: EN_CALENDAR_ALL_DAYS,
};
pub const CALENDAR_EMPTY_MONTH: Localized = Localized {
    ja: JA_CALENDAR_EMPTY_MONTH,
    en: EN_CALENDAR_EMPTY_MONTH,
};
pub const CALENDAR_EMPTY_DAY: Localized = Localized {
    ja: JA_CALENDAR_EMPTY_DAY,
    en: EN_CALENDAR_EMPTY_DAY,
};
pub const CALENDAR_CREATE_ON_DAY: Localized = Localized {
    ja: JA_CALENDAR_CREATE_ON_DAY,
    en: EN_CALENDAR_CREATE_ON_DAY,
};
pub const CALENDAR_VIEW_MONTH: Localized = Localized {
    ja: JA_CALENDAR_VIEW_MONTH,
    en: EN_CALENDAR_VIEW_MONTH,
};
pub const CALENDAR_VIEW_LIST: Localized = Localized {
    ja: JA_CALENDAR_VIEW_LIST,
    en: EN_CALENDAR_VIEW_LIST,
};
pub const CALENDAR_VIEW_MATRIX: Localized = Localized {
    ja: JA_CALENDAR_VIEW_MATRIX,
    en: EN_CALENDAR_VIEW_MATRIX,
};
pub const CALENDAR_DAY_DETAIL_PROMPT: Localized = Localized {
    ja: JA_CALENDAR_DAY_DETAIL_PROMPT,
    en: EN_CALENDAR_DAY_DETAIL_PROMPT,
};
pub const CALENDAR_MATRIX_TITLE: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_TITLE,
    en: EN_CALENDAR_MATRIX_TITLE,
};
pub const CALENDAR_MATRIX_TOO_LARGE: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_TOO_LARGE,
    en: EN_CALENDAR_MATRIX_TOO_LARGE,
};
pub const CALENDAR_MATRIX_NO_MEMBERS: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_NO_MEMBERS,
    en: EN_CALENDAR_MATRIX_NO_MEMBERS,
};
pub const CALENDAR_MATRIX_CSV_EXPORT: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_CSV_EXPORT,
    en: EN_CALENDAR_MATRIX_CSV_EXPORT,
};
pub const CALENDAR_MATRIX_CSV_ERROR: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_CSV_ERROR,
    en: EN_CALENDAR_MATRIX_CSV_ERROR,
};
pub const CALENDAR_DAY_EVENTS_COUNT: Localized = Localized {
    ja: JA_CALENDAR_DAY_EVENTS_COUNT,
    en: EN_CALENDAR_DAY_EVENTS_COUNT,
};
pub const CALENDAR_MATRIX_MEMBER_COLUMN: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_MEMBER_COLUMN,
    en: EN_CALENDAR_MATRIX_MEMBER_COLUMN,
};
pub const CALENDAR_MONTH_NAV_ARIA_LABEL: Localized = Localized {
    ja: JA_CALENDAR_MONTH_NAV_ARIA_LABEL,
    en: EN_CALENDAR_MONTH_NAV_ARIA_LABEL,
};
pub const CALENDAR_VIEW_NAV_ARIA_LABEL: Localized = Localized {
    ja: JA_CALENDAR_VIEW_NAV_ARIA_LABEL,
    en: EN_CALENDAR_VIEW_NAV_ARIA_LABEL,
};

// ── Matrix cell aria-label templates (RFC-072 Slice C) ────────────────────
// Positional `{}` placeholders substituted in order by
// `matrix/cells.rs::substitute_positional` — not a mechanical `i18n::t`
// swap, because these carry counts. The English forms use label-value
// pairs ("events: {}", "cancelled: {}", …), never "{} events", so no
// plural agreement is needed at any count including zero and one. Every
// pair here must keep the same number of `{}` placeholders on both sides —
// see `cell_label_templates_have_matching_placeholder_counts`.
pub const EN_CALENDAR_MATRIX_CELL_NO_EVENTS: &str = "{}, {}, no events";
pub const EN_CALENDAR_MATRIX_CELL_CANCELLED: &str = "{}, {}, cancelled";
pub const EN_CALENDAR_MATRIX_CELL_SINGLE_STATUS: &str = "{}, {}, {}";
pub const EN_CALENDAR_MATRIX_CELL_BREAKDOWN: &str =
    "{}, {}, events: {}, cancelled: {}, going: {}, not going: {}, attended: {}, no answer: {}";

pub const JA_CALENDAR_MATRIX_CELL_NO_EVENTS: &str = "{}、{}、予定なし";
pub const JA_CALENDAR_MATRIX_CELL_CANCELLED: &str = "{}、{}、中止";
pub const JA_CALENDAR_MATRIX_CELL_SINGLE_STATUS: &str = "{}、{}、{}";
pub const JA_CALENDAR_MATRIX_CELL_BREAKDOWN: &str =
    "{}、{}、予定{}件、中止{}件、参加{}件、不参加{}件、参加済み{}件、未回答{}件";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const CALENDAR_MATRIX_CELL_NO_EVENTS: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_CELL_NO_EVENTS,
    en: EN_CALENDAR_MATRIX_CELL_NO_EVENTS,
};
pub const CALENDAR_MATRIX_CELL_CANCELLED: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_CELL_CANCELLED,
    en: EN_CALENDAR_MATRIX_CELL_CANCELLED,
};
pub const CALENDAR_MATRIX_CELL_SINGLE_STATUS: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_CELL_SINGLE_STATUS,
    en: EN_CALENDAR_MATRIX_CELL_SINGLE_STATUS,
};
pub const CALENDAR_MATRIX_CELL_BREAKDOWN: Localized = Localized {
    ja: JA_CALENDAR_MATRIX_CELL_BREAKDOWN,
    en: EN_CALENDAR_MATRIX_CELL_BREAKDOWN,
};

// ── Recurrence materialization notices ───────────────────────────────────
pub const EN_CALENDAR_OUT_OF_RANGE: &str =
    "Recurring dates are prepared only for the next several months.";
pub const EN_CALENDAR_MATERIALIZATION_LIMIT: &str = "Some recurring dates are still being prepared. Please try again later or ask an admin to review.";

pub const JA_CALENDAR_OUT_OF_RANGE: &str =
    "繰り返し予定は、近い月から順に表示できるように準備します。";
pub const JA_CALENDAR_MATERIALIZATION_LIMIT: &str =
    "一部の繰り返し予定はまだ準備中です。時間をおいて再度確認するか、管理者に確認してください。";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const CALENDAR_OUT_OF_RANGE: Localized = Localized {
    ja: JA_CALENDAR_OUT_OF_RANGE,
    en: EN_CALENDAR_OUT_OF_RANGE,
};
pub const CALENDAR_MATERIALIZATION_LIMIT: Localized = Localized {
    ja: JA_CALENDAR_MATERIALIZATION_LIMIT,
    en: EN_CALENDAR_MATERIALIZATION_LIMIT,
};

// ── Calendar feed (RFC-023) ───────────────────────────────────────────────
pub const EN_CALENDAR_TITLE: &str = "Calendar feed";
pub const EN_CALENDAR_DESCRIPTION: &str = "Subscribe in Apple Calendar, Google Calendar, or any app that supports calendar subscriptions (.ics / webcal).";
pub const EN_CALENDAR_GENERATE: &str = "Generate feed URL";
pub const EN_CALENDAR_DISABLE: &str = "Disable feed";
pub const EN_CALENDAR_REGENERATE: &str = "Regenerate URL";
pub const EN_CALENDAR_PRIVACY_NOTE: &str = "Your personal calendar feed URL. Keep this private — anyone with the URL can read your community events.";
pub const EN_CALENDAR_GENERATED_FLASH: &str = "Calendar link created.";
pub const EN_CALENDAR_REVOKED_FLASH: &str = "Calendar link disabled.";

pub const JA_CALENDAR_TITLE: &str = "予定をカレンダーに入れる";
pub const JA_CALENDAR_DESCRIPTION: &str =
    "AppleカレンダーやGoogleカレンダーなど、予定を取り込めるアプリで利用できます。";
pub const JA_CALENDAR_GENERATE: &str = "リンクを作成";
pub const JA_CALENDAR_DISABLE: &str = "リンクを無効化";
pub const JA_CALENDAR_REGENERATE: &str = "リンクを再作成";
pub const JA_CALENDAR_PRIVACY_NOTE: &str = "このカレンダーリンクは、持っている人なら誰でもあなたのコミュニティの予定を見られます。公開しないでください。こちらで再発行または無効化できます。";
pub const JA_CALENDAR_GENERATED_FLASH: &str = "カレンダーリンクを作成しました。";
pub const JA_CALENDAR_REVOKED_FLASH: &str = "カレンダーリンクを無効化しました。";

/// RFC-072 locale-aware pairs; see `i18n::Localized`.
pub const CALENDAR_TITLE: Localized = Localized {
    ja: JA_CALENDAR_TITLE,
    en: EN_CALENDAR_TITLE,
};
pub const CALENDAR_DESCRIPTION: Localized = Localized {
    ja: JA_CALENDAR_DESCRIPTION,
    en: EN_CALENDAR_DESCRIPTION,
};
pub const CALENDAR_GENERATE: Localized = Localized {
    ja: JA_CALENDAR_GENERATE,
    en: EN_CALENDAR_GENERATE,
};
pub const CALENDAR_DISABLE: Localized = Localized {
    ja: JA_CALENDAR_DISABLE,
    en: EN_CALENDAR_DISABLE,
};
pub const CALENDAR_REGENERATE: Localized = Localized {
    ja: JA_CALENDAR_REGENERATE,
    en: EN_CALENDAR_REGENERATE,
};
pub const CALENDAR_PRIVACY_NOTE: Localized = Localized {
    ja: JA_CALENDAR_PRIVACY_NOTE,
    en: EN_CALENDAR_PRIVACY_NOTE,
};
pub const CALENDAR_GENERATED_FLASH: Localized = Localized {
    ja: JA_CALENDAR_GENERATED_FLASH,
    en: EN_CALENDAR_GENERATED_FLASH,
};
pub const CALENDAR_REVOKED_FLASH: Localized = Localized {
    ja: JA_CALENDAR_REVOKED_FLASH,
    en: EN_CALENDAR_REVOKED_FLASH,
};
