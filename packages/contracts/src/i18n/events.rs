// ── Status labels ─────────────────────────────────────────────────────────
pub const EN_STATUS_GOING: &str = "Going";
pub const EN_STATUS_NOT_GOING: &str = "No Go";
pub const EN_STATUS_ATTENDED: &str = "Attended";
pub const EN_STATUS_NO_ANSWER: &str = "No answer";
pub const EN_STATUS_ATTENDED_DISABLED: &str = "Available after the event";

pub const JA_STATUS_GOING: &str = "参加";
pub const JA_STATUS_NOT_GOING: &str = "不参加";
pub const JA_STATUS_ATTENDED: &str = "参加済み";
pub const JA_STATUS_NO_ANSWER: &str = "未回答";
pub const JA_STATUS_ATTENDED_DISABLED: &str = "イベント終了後に利用可能";

// ── Status actions ────────────────────────────────────────────────────────
pub const EN_STATUS_CLEAR: &str = "Clear";
pub const EN_STATUS_CLEAR_LABEL: &str = "Clear answer";

pub const JA_STATUS_CLEAR: &str = "未回答に戻す";
pub const JA_STATUS_CLEAR_LABEL: &str = "回答を未回答に戻す";

// ── Admin: event management ───────────────────────────────────────────────
pub const EN_ADMIN_CREATE_EVENT_TITLE: &str = "Create Event";
pub const EN_ADMIN_CREATE_EVENT_SUBMIT: &str = "Create Event";
pub const EN_REPEAT_END_OPEN: &str = "No end date";
pub const EN_REPEAT_END_UNTIL: &str = "Until date";
pub const EN_REPEAT_END_COUNT: &str = "Number of times";
pub const EN_REPEAT_COUNT_LABEL: &str = "Repeat count";
pub const EN_REPEAT_UNTIL_LABEL: &str = "Repeat until date";
pub const EN_OCCURRENCE_CANCEL_ACTION: &str = "Cancel this date only";
pub const EN_OCCURRENCE_CANCEL_TITLE: &str = "Cancel this date";
pub const EN_OCCURRENCE_CANCEL_HELPER: &str =
    "Only this date will be cancelled. Other dates in the series stay scheduled.";
pub const EN_OCCURRENCE_CANCEL_SUBMIT: &str = "Cancel this date";
pub const EN_OCCURRENCE_CANCELLED_BADGE: &str = "This date is cancelled";
pub const EN_ADMIN_RECREATE_EVENT_ACTION: &str = "Create similar event";
pub const EN_ADMIN_RECREATE_EVENT_HELPER: &str = "Only the title, place, and description are reused. Choose the date again. Attendance answers and memos are not carried over.";
pub const EN_ADMIN_COPY_EVENT_ACTION: &str = "Copy this event";
pub const EN_ADMIN_COPY_EVENT_TITLE: &str = "Create from copied event";
pub const EN_ADMIN_COPY_EVENT_HELPER: &str =
    "Create a new event from this event. Attendance answers and memos are not copied.";
pub const EN_ADMIN_COPY_EVENT_DATE_WARNING: &str = "The date is copied too. Change it if needed.";
pub const EN_ADMIN_COPY_EVENT_MULTI_DAY_HELPER: &str =
    "This source event has multiple dates. Choose the new schedule again.";
pub const EN_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE: &str =
    "The schedule cannot be copied. Choose a new schedule.";
pub const EN_ADMIN_COPY_EVENT_RECURRING_PAST: &str =
    "The recurrence starts in the past. Choose a new start date.";
pub const EN_ADMIN_COPY_EVENT_RECURRING_WINDOW: &str =
    "The recurrence starts outside the current create window. Choose a new start date.";
pub const EN_ADMIN_EDIT_EVENT_TITLE: &str = "Edit Event";
pub const EN_ADMIN_EDIT_EVENT_SUBMIT: &str = "Save Changes";
pub const EN_ADMIN_EDIT_EVENT_HINT: &str = "Members will see the updated event details.";
pub const EN_ADMIN_EDIT_DETAILS_ONLY_HEADING: &str = "Editable details";
pub const EN_ADMIN_EDIT_SCHEDULE_HEADING: &str = "Current schedule";
pub const EN_ADMIN_EDIT_SCHEDULE_TOTAL_PREFIX: &str = "Total";
pub const EN_ADMIN_EDIT_SCHEDULE_TOTAL_SUFFIX: &str = "dates";
pub const EN_ADMIN_EDIT_SCHEDULE_FIRST: &str = "First";
pub const EN_ADMIN_EDIT_SCHEDULE_LAST: &str = "Last";
pub const EN_ADMIN_EDIT_MULTI_DAY_HELPER: &str = "This event has multiple dates. You can change only the title, location, and description here. To change dates or times, cancel this event and create it again.";
pub const EN_ADMIN_EDIT_RECURRING_HELPER: &str = "This event repeats. You can change only the title, location, and description here. To change dates, times, or the number of occurrences, cancel this event and create it again.";
pub const EN_ADMIN_EDIT_RESPONSES_PRESERVED: &str =
    "Dates stay the same, and attendance answers remain attached to those dates.";
pub const EN_ADMIN_EDIT_SCHEDULE_NOT_EDITABLE: &str =
    "Date and time cannot be changed for this event.";
pub const EN_ADMIN_CANCEL_EVENT_TITLE: &str = "Cancel this event?";
pub const EN_ADMIN_CANCEL_EVENT_BODY: &str = "Members will still see that it was cancelled.";
pub const EN_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS: &str =
    "All dates for this event will be cancelled. Attendance answers can no longer be changed.";
pub const EN_ADMIN_CANCEL_EVENT_KEEP: &str = "Back";
pub const EN_ADMIN_CANCEL_EVENT_CONFIRM: &str = "Cancel Event";
pub const EN_ADMIN_CANCEL_EVENT_CONFIRM_ALL_DAYS: &str = "Cancel all";
pub const EN_ADMIN_CANNOT_EDIT_CANCELLED: &str = "Cancelled events cannot be edited.";
pub const EN_ADMIN_CANNOT_EDIT_STARTED: &str =
    "This event has already started and cannot be edited.";
pub const EN_ADMIN_CANNOT_ATTEND_CANCELLED: &str =
    "Attendance cannot be corrected for a cancelled event.";
pub const EN_ADMIN_ATTEND_TITLE: &str = "Mark Attendance";
pub const EN_ADMIN_ATTEND_SUBMIT: &str = "Save Attendance";

pub const JA_ADMIN_CREATE_EVENT_TITLE: &str = "イベントを作成";
pub const JA_ADMIN_CREATE_EVENT_SUBMIT: &str = "イベントを作成";
pub const JA_REPEAT_END_OPEN: &str = "終了日を決めない";
pub const JA_REPEAT_END_UNTIL: &str = "この日まで";
pub const JA_REPEAT_END_COUNT: &str = "回数を指定";
pub const JA_REPEAT_COUNT_LABEL: &str = "繰り返し回数";
pub const JA_REPEAT_UNTIL_LABEL: &str = "繰り返し終了日";
pub const JA_OCCURRENCE_CANCEL_ACTION: &str = "この日だけ中止する";
pub const JA_OCCURRENCE_CANCEL_TITLE: &str = "この日だけ中止";
pub const JA_OCCURRENCE_CANCEL_HELPER: &str =
    "この日だけを中止します。同じ繰り返し予定の他の日はそのまま残ります。";
pub const JA_OCCURRENCE_CANCEL_SUBMIT: &str = "この日だけ中止する";
pub const JA_OCCURRENCE_CANCELLED_BADGE: &str = "この日は中止です";
pub const JA_ADMIN_RECREATE_EVENT_ACTION: &str = "似た内容で新しいイベントを作成";
pub const JA_ADMIN_RECREATE_EVENT_HELPER: &str = "タイトル・場所・説明だけを引き継ぎます。日程はもう一度選びます。参加の回答とメモは引き継ぎません。";
pub const JA_ADMIN_COPY_EVENT_ACTION: &str = "このイベントをコピー";
pub const JA_ADMIN_COPY_EVENT_TITLE: &str = "イベントをコピーして作成";
pub const JA_ADMIN_COPY_EVENT_HELPER: &str =
    "内容をコピーして新しいイベントを作成します。参加の回答とメモはコピーされません。";
pub const JA_ADMIN_COPY_EVENT_DATE_WARNING: &str =
    "日付もコピーされています。必要に応じて変更してください。";
pub const JA_ADMIN_COPY_EVENT_MULTI_DAY_HELPER: &str =
    "複数日の予定です。日程は新しく選び直してください。";
pub const JA_ADMIN_COPY_EVENT_SCHEDULE_UNAVAILABLE: &str =
    "日程はコピーできません。新しく選び直してください。";
pub const JA_ADMIN_COPY_EVENT_RECURRING_PAST: &str =
    "繰り返しの開始日が過去のため、開始日を新しく選び直してください。";
pub const JA_ADMIN_COPY_EVENT_RECURRING_WINDOW: &str =
    "繰り返しの開始日が作成できる範囲外のため、開始日を新しく選び直してください。";
pub const JA_ADMIN_EDIT_EVENT_TITLE: &str = "イベントを編集";
pub const JA_ADMIN_EDIT_EVENT_SUBMIT: &str = "変更を保存";
pub const JA_ADMIN_EDIT_EVENT_HINT: &str = "メンバーには更新されたイベント詳細が表示されます。";
pub const JA_ADMIN_EDIT_DETAILS_ONLY_HEADING: &str = "変更できる内容";
pub const JA_ADMIN_EDIT_SCHEDULE_HEADING: &str = "現在の日程";
pub const JA_ADMIN_EDIT_SCHEDULE_TOTAL_PREFIX: &str = "全";
pub const JA_ADMIN_EDIT_SCHEDULE_TOTAL_SUFFIX: &str = "回";
pub const JA_ADMIN_EDIT_SCHEDULE_FIRST: &str = "最初";
pub const JA_ADMIN_EDIT_SCHEDULE_LAST: &str = "最後";
pub const JA_ADMIN_EDIT_MULTI_DAY_HELPER: &str = "このイベントは複数の日程があります。ここでは、タイトル・場所・説明だけを変更できます。日時を変える場合は、このイベントをキャンセルして、作り直してください。";
pub const JA_ADMIN_EDIT_RECURRING_HELPER: &str = "このイベントは繰り返しの予定です。ここでは、タイトル・場所・説明だけを変更できます。日時や回数を変える場合は、このイベントをキャンセルして、作り直してください。";
pub const JA_ADMIN_EDIT_RESPONSES_PRESERVED: &str =
    "日時は変わらず、参加の回答もそのまま残ります。";
pub const JA_ADMIN_EDIT_SCHEDULE_NOT_EDITABLE: &str = "このイベントでは日時を変更できません。";
pub const JA_ADMIN_CANCEL_EVENT_TITLE: &str = "このイベントをキャンセルしますか？";
pub const JA_ADMIN_CANCEL_EVENT_BODY: &str =
    "メンバーにはキャンセルされたことが引き続き表示されます。";
pub const JA_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS: &str =
    "このイベントのすべての日程をキャンセルします。参加の回答も、これ以上変更できなくなります。";
pub const JA_ADMIN_CANCEL_EVENT_KEEP: &str = "戻る";
pub const JA_ADMIN_CANCEL_EVENT_CONFIRM: &str = "イベントをキャンセル";
pub const JA_ADMIN_CANCEL_EVENT_CONFIRM_ALL_DAYS: &str = "すべてキャンセル";
pub const JA_ADMIN_CANNOT_EDIT_CANCELLED: &str = "キャンセル済みのイベントは編集できません。";
pub const JA_ADMIN_CANNOT_EDIT_STARTED: &str =
    "このイベントはすでに開始しているため編集できません。";
pub const JA_ADMIN_CANNOT_ATTEND_CANCELLED: &str =
    "キャンセル済みのイベントの出席は修正できません。";
pub const JA_ADMIN_ATTEND_TITLE: &str = "出席を記録";
pub const JA_ADMIN_ATTEND_SUBMIT: &str = "出席を保存";

// ── Recurrence fields (RFC-022) ───────────────────────────────────────────
pub const EN_REPEAT_LABEL: &str = "Repeat";
pub const EN_REPEAT_NONE: &str = "Do not repeat";
pub const EN_REPEAT_WEEKLY: &str = "Every week";
pub const EN_REPEAT_BIWEEKLY: &str = "Every 2 weeks";
pub const EN_REPEAT_MONTHLY: &str = "Every month";
pub const EN_REPEAT_COUNT_UNIT: &str = "times";
pub const EN_REPEAT_COUNT_HINT: &str =
    "Number of times ignored when \"Do not repeat\" is selected.";

pub const JA_REPEAT_LABEL: &str = "繰り返し";
pub const JA_REPEAT_NONE: &str = "繰り返さない";
pub const JA_REPEAT_WEEKLY: &str = "毎週";
pub const JA_REPEAT_BIWEEKLY: &str = "2週間ごと";
pub const JA_REPEAT_MONTHLY: &str = "毎月";
pub const JA_REPEAT_COUNT_UNIT: &str = "回";
pub const JA_REPEAT_COUNT_HINT: &str = "「繰り返さない」を選択した場合、回数は無視されます。";
pub const EN_FORM_FIELD_TITLE: &str = "Title";
pub const JA_FORM_FIELD_TITLE: &str = "タイトル";
pub const EN_FORM_FIELD_DATE: &str = "Date";
pub const JA_FORM_FIELD_DATE: &str = "日付";
pub const EN_FORM_FIELD_START: &str = "Start time";
pub const JA_FORM_FIELD_START: &str = "開始時刻";
pub const EN_FORM_FIELD_END: &str = "End time";
pub const JA_FORM_FIELD_END: &str = "終了時刻";
pub const EN_FORM_FIELD_LOCATION: &str = "Location (optional)";
pub const JA_FORM_FIELD_LOCATION: &str = "場所（任意）";
pub const EN_FORM_FIELD_DESC: &str = "Description (optional)";
pub const JA_FORM_FIELD_DESC: &str = "説明（任意）";

// ── Event detail page (RFC-006 / RFC-025) ─────────────────────────────────
pub const EN_EVENT_TITLE_HEADER: &str = "Event";
pub const EN_EVENT_ATTENDED_UNAVAILABLE: &str = "Available after the event";
pub const EN_EVENT_ATTENDED_ADMIN_ONLY: &str = "Only admins can mark Attended";
pub const EN_EVENT_MEMBER_FALLBACK: &str = "Member";

pub const JA_EVENT_TITLE_HEADER: &str = "イベント";
pub const JA_EVENT_ATTENDED_UNAVAILABLE: &str = "イベント終了後に選択できます";
pub const JA_EVENT_ATTENDED_ADMIN_ONLY: &str = "出席の記録は管理者のみ行えます";
pub const JA_EVENT_MEMBER_FALLBACK: &str = "メンバー";
pub const EN_EVENT_CANCELLED_BADGE: &str = "This event was cancelled.";
pub const JA_EVENT_CANCELLED_BADGE: &str = "このイベントはキャンセルされました。";
pub const EN_EVENT_WHOS_GOING: &str = "Who's going?";
pub const JA_EVENT_WHOS_GOING: &str = "参加予定者";
pub const EN_EVENT_NOTES_SECTION: &str = "Notes";
pub const JA_EVENT_NOTES_SECTION: &str = "メモ";
pub const EN_TZ_ERROR: &str = "Community timezone is not configured correctly. Please ask the operator to set a valid timezone.";
pub const JA_TZ_ERROR: &str =
    "コミュニティのタイムゾーンが正しく設定されていません。運営者にお問い合わせください。";
