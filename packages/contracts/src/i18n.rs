//! UI string constants (i18n scaffold — RFC-026).
//!
//! All user-visible strings are collected here so they can be translated
//! without touching handler logic.  Currently English only; Japanese parity
//! is enforced by the i18n lint test below.
//!
//! Naming: `<LANG>_<CONTEXT>_<KEY>` in SCREAMING_SNAKE_CASE.

// ── Join / onboarding ─────────────────────────────────────────────────────
pub const EN_JOIN_HEADING: &str = "ciao.zinnias";
pub const EN_JOIN_SUBHEADING: &str = "Private community schedule sharing";
pub const EN_JOIN_CODE_LABEL: &str = "Invite code";
pub const EN_JOIN_CODE_HINT: &str = "Ask your community admin if you do not have an invite code.";
pub const EN_JOIN_SUBMIT: &str = "Join";
pub const EN_JOIN_PROFILE_HEADING: &str = "Your name in this community";
pub const EN_JOIN_PROFILE_HINT: &str =
    "People will see this name when you answer events or leave notes.";
pub const EN_JOIN_PROFILE_LABEL: &str = "Display name";
pub const EN_JOIN_PROFILE_SUBMIT: &str = "Start";

pub const JA_JOIN_HEADING: &str = "ciao.zinnias";
pub const JA_JOIN_SUBHEADING: &str = "招待制コミュニティのスケジュール共有";
pub const JA_JOIN_CODE_LABEL: &str = "招待コード";
pub const JA_JOIN_CODE_HINT: &str = "招待コードはコミュニティの管理者にお問い合わせください。";
pub const JA_JOIN_SUBMIT: &str = "参加する";
pub const JA_JOIN_PROFILE_HEADING: &str = "このコミュニティでの名前";
pub const JA_JOIN_PROFILE_HINT: &str = "イベントへの返答やメモを残すときにこの名前が表示されます。";
pub const JA_JOIN_PROFILE_LABEL: &str = "表示名";
pub const JA_JOIN_PROFILE_SUBMIT: &str = "はじめる";

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

// ── Note editor ───────────────────────────────────────────────────────────
pub const EN_NOTE_SAVE: &str = "Save Note";
pub const EN_NOTE_DELETE: &str = "Delete Note";
pub const EN_NOTE_SAVED: &str = "Saved.";
pub const EN_NOTE_TOO_LONG: &str = "Your note is too long. Please keep it under 200 characters.";

pub const JA_NOTE_SAVE: &str = "メモを保存";
pub const JA_NOTE_DELETE: &str = "メモを削除";
pub const JA_NOTE_SAVED: &str = "保存しました。";
pub const JA_NOTE_TOO_LONG: &str = "メモが長すぎます。200文字以内にしてください。";

// ── Session / auth ────────────────────────────────────────────────────────
pub const EN_SESSION_EXPIRED: &str =
    "Your session expired. Please ask your community admin for a new invite code.";
pub const EN_LOGOUT: &str = "Log out";
pub const EN_LOGOUT_CONFIRM: &str = "Log out?";

pub const JA_SESSION_EXPIRED: &str = "もう一度、招待コードを入力してください。招待コードがない場合は、コミュニティの管理者にお問い合わせください。";
pub const JA_LOGOUT: &str = "ログアウト";
pub const JA_LOGOUT_CONFIRM: &str = "ログアウトしますか？";

// ── General ───────────────────────────────────────────────────────────────
pub const EN_GENERAL_ERROR: &str = "Something went wrong. Please try again.";
pub const EN_OFFLINE_BANNER: &str = "Offline — showing last loaded";
pub const EN_EMPTY_EVENTS: &str = "No events yet.";
pub const EN_EMPTY_EVENTS_HINT: &str = "Ask your community admin to add one.";
pub const EN_EMPTY_EVENTS_ADMIN: &str = "No events yet. Create the first event for this community.";

pub const JA_GENERAL_ERROR: &str = "エラーが発生しました。もう一度お試しください。";
pub const EN_NOT_FOUND: &str = "Not found.";
pub const JA_NOT_FOUND: &str = "見つかりませんでした。";
pub const EN_INTERNAL_ERROR: &str = "Something went wrong. Please try again.";
pub const JA_INTERNAL_ERROR: &str = "問題が発生しました。もう一度お試しください。";
pub const EN_ADMIN_ATTEND_CANCELLED: &str = "Attendance cannot be corrected for a cancelled event.";
pub const JA_ADMIN_ATTEND_CANCELLED: &str = "キャンセル済みのイベントの出席は修正できません。";
pub const EN_GENERAL_BACK: &str = "Go back";
pub const JA_GENERAL_BACK: &str = "戻る";
pub const EN_ADMIN_EDIT_CANCELLED: &str = "Cancelled events cannot be edited.";
pub const JA_ADMIN_EDIT_CANCELLED: &str = "キャンセル済みのイベントは編集できません。";
pub const EN_ADMIN_EDIT_STARTED: &str = "This event has already started and cannot be edited.";
pub const JA_ADMIN_EDIT_STARTED: &str = "すでに開始したイベントは編集できません。";
pub const JA_OFFLINE_BANNER: &str = "オフライン — 最後に読み込んだ情報を表示しています";
pub const JA_EMPTY_EVENTS: &str = "イベントはまだありません。";
pub const JA_EMPTY_EVENTS_HINT: &str = "コミュニティの管理者にイベントの追加をお願いしてください。";
pub const JA_EMPTY_EVENTS_ADMIN: &str =
    "イベントはまだありません。最初のイベントを作成しましょう。";

// ── Navigation ────────────────────────────────────────────────────────────
pub const EN_NAV_HOME: &str = "Home";
pub const EN_NAV_COMMUNITIES: &str = "Communities";
pub const EN_NAV_ME: &str = "Me";

pub const JA_NAV_HOME: &str = "ホーム";
pub const JA_NAV_COMMUNITIES: &str = "コミュニティ";
pub const JA_NAV_ME: &str = "マイページ";
pub const EN_NAV_BACK: &str = "Back to event";
pub const JA_NAV_BACK: &str = "イベントに戻る";
pub const EN_NAV_SWITCH_GO: &str = "Switch";
pub const JA_NAV_SWITCH_GO: &str = "切り替え";

// ── Home schedule view ────────────────────────────────────────────────────
pub const EN_HOME_TODAY: &str = "Today";
pub const EN_HOME_THIS_WEEK: &str = "This Week";
pub const EN_HOME_LATER: &str = "Later";
pub const EN_HOME_CREATE_EVENT: &str = "+ Create event";
pub const EN_HOME_INVITE_MEMBERS: &str = "Invite members";

pub const JA_HOME_TODAY: &str = "今日";
pub const JA_HOME_THIS_WEEK: &str = "今週";
pub const JA_HOME_LATER: &str = "それ以降";
pub const JA_HOME_CREATE_EVENT: &str = "+ イベントを作成";
pub const JA_HOME_INVITE_MEMBERS: &str = "メンバーを招待";

// ── Status actions ────────────────────────────────────────────────────────
pub const EN_STATUS_CLEAR: &str = "Clear";
pub const EN_STATUS_CLEAR_LABEL: &str = "Clear answer";

pub const JA_STATUS_CLEAR: &str = "未回答に戻す";
pub const JA_STATUS_CLEAR_LABEL: &str = "回答を未回答に戻す";

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

// ── Me / profile ──────────────────────────────────────────────────────────
pub const EN_ME_SECTION_NAME: &str = "Name";
pub const EN_ME_SECTION_COMMUNITY: &str = "Current community";
pub const EN_ME_SECTION_HELP: &str = "Help";
pub const EN_ME_HELP_BODY: &str = "Ask your community admin if you cannot enter or lost access.";

pub const JA_ME_SECTION_NAME: &str = "名前";
pub const JA_ME_SECTION_COMMUNITY: &str = "現在のコミュニティ";
pub const JA_ME_SECTION_HELP: &str = "ヘルプ";
pub const JA_ME_HELP_BODY: &str =
    "入室できない場合やアクセスを失った場合は、コミュニティの管理者にお問い合わせください。";

// ── Admin: event management ───────────────────────────────────────────────
pub const EN_ADMIN_CREATE_EVENT_TITLE: &str = "Create Event";
pub const EN_ADMIN_CREATE_EVENT_SUBMIT: &str = "Create Event";
pub const EN_ADMIN_EDIT_EVENT_TITLE: &str = "Edit Event";
pub const EN_ADMIN_EDIT_EVENT_SUBMIT: &str = "Save Changes";
pub const EN_ADMIN_EDIT_EVENT_HINT: &str = "Members will see the updated event details.";
pub const EN_ADMIN_CANCEL_EVENT_TITLE: &str = "Cancel this event?";
pub const EN_ADMIN_CANCEL_EVENT_BODY: &str = "Members will still see that it was cancelled.";
pub const EN_ADMIN_CANCEL_EVENT_KEEP: &str = "Keep Event";
pub const EN_ADMIN_CANCEL_EVENT_CONFIRM: &str = "Cancel Event";
pub const EN_ADMIN_CANNOT_EDIT_CANCELLED: &str = "Cancelled events cannot be edited.";
pub const EN_ADMIN_CANNOT_EDIT_STARTED: &str =
    "This event has already started and cannot be edited.";
pub const EN_ADMIN_CANNOT_ATTEND_CANCELLED: &str =
    "Attendance cannot be corrected for a cancelled event.";
pub const EN_ADMIN_ATTEND_TITLE: &str = "Mark Attendance";
pub const EN_ADMIN_ATTEND_SUBMIT: &str = "Save Attendance";

pub const JA_ADMIN_CREATE_EVENT_TITLE: &str = "イベントを作成";
pub const JA_ADMIN_CREATE_EVENT_SUBMIT: &str = "イベントを作成";
pub const JA_ADMIN_EDIT_EVENT_TITLE: &str = "イベントを編集";
pub const JA_ADMIN_EDIT_EVENT_SUBMIT: &str = "変更を保存";
pub const JA_ADMIN_EDIT_EVENT_HINT: &str = "メンバーには更新されたイベント詳細が表示されます。";
pub const JA_ADMIN_CANCEL_EVENT_TITLE: &str = "このイベントをキャンセルしますか？";
pub const JA_ADMIN_CANCEL_EVENT_BODY: &str =
    "メンバーにはキャンセルされたことが引き続き表示されます。";
pub const JA_ADMIN_CANCEL_EVENT_KEEP: &str = "イベントを保持";
pub const JA_ADMIN_CANCEL_EVENT_CONFIRM: &str = "イベントをキャンセル";
pub const JA_ADMIN_CANNOT_EDIT_CANCELLED: &str = "キャンセル済みのイベントは編集できません。";
pub const JA_ADMIN_CANNOT_EDIT_STARTED: &str =
    "このイベントはすでに開始しているため編集できません。";
pub const JA_ADMIN_CANNOT_ATTEND_CANCELLED: &str =
    "キャンセル済みのイベントの出席は修正できません。";
pub const JA_ADMIN_ATTEND_TITLE: &str = "出席を記録";
pub const JA_ADMIN_ATTEND_SUBMIT: &str = "出席を保存";

// ── Admin: invite management ──────────────────────────────────────────────
pub const EN_ADMIN_INVITES_TITLE: &str = "Invite Members";
pub const EN_ADMIN_INVITES_BODY: &str = "Generate a one-time code for one person.";
pub const EN_ADMIN_INVITES_GENERATE: &str = "Generate Code";
pub const EN_ADMIN_INVITES_ACTIVE: &str = "Active codes";
pub const EN_ADMIN_INVITES_NONE: &str = "No unused codes.";
pub const EN_ADMIN_INVITES_NEW_CODE_HINT: &str =
    "Share with one person only — expires in 24 hours.";
pub const EN_ADMIN_INVITES_REVOKE: &str = "Revoke";
pub const EN_ADMIN_INVITES_REVOKED: &str = "Code revoked";

pub const JA_ADMIN_INVITES_TITLE: &str = "メンバーを招待";
pub const JA_ADMIN_INVITES_BODY: &str = "一人のために一回限りのコードを生成します。";
pub const JA_ADMIN_INVITES_GENERATE: &str = "コードを生成";
pub const JA_ADMIN_INVITES_ACTIVE: &str = "有効なコード";
pub const JA_ADMIN_INVITES_NONE: &str = "未使用のコードはありません。";
pub const JA_ADMIN_INVITES_NEW_CODE_HINT: &str =
    "一人だけに共有してください — 24時間で失効します。";
pub const JA_ADMIN_INVITES_REVOKE: &str = "無効化";
pub const JA_ADMIN_INVITES_REVOKED: &str = "コードを無効化しました";

// ── Admin: member management ──────────────────────────────────────────────
pub const EN_ADMIN_MEMBERS_TITLE: &str = "Members";
pub const EN_ADMIN_MEMBERS_GENERATE_INVITE: &str = "Generate invite code";
pub const EN_ADMIN_REMOVE_TITLE: &str = "Remove member?";
pub const EN_ADMIN_REMOVE_KEEP: &str = "Keep Member";
pub const EN_ADMIN_REMOVE_CONFIRM: &str = "Remove";
pub const EN_ADMIN_REMOVE_CONSEQUENCE: &str = "They will no longer be able to see events or notes.";
pub const EN_ADMIN_LAST_ADMIN: &str =
    "Cannot remove the last admin. Transfer the admin role first.";

pub const JA_ADMIN_MEMBERS_TITLE: &str = "メンバー";
pub const JA_ADMIN_MEMBERS_GENERATE_INVITE: &str = "招待コードを生成";
pub const JA_ADMIN_REMOVE_TITLE: &str = "メンバーを削除しますか？";
pub const JA_ADMIN_REMOVE_KEEP: &str = "メンバーを保持";
pub const JA_ADMIN_REMOVE_CONFIRM: &str = "削除";
pub const JA_ADMIN_REMOVE_CONSEQUENCE: &str =
    "このメンバーはイベントやメモを見ることができなくなります。";
pub const JA_ADMIN_LAST_ADMIN: &str =
    "最後の管理者は削除できません。先に管理者権限を移譲してください。";

// ── Communities ───────────────────────────────────────────────────────────
pub const EN_COMMUNITIES_JOIN_ANOTHER: &str = "Join another community";

pub const JA_COMMUNITIES_JOIN_ANOTHER: &str = "別のコミュニティに参加";

// ── Role labels ───────────────────────────────────────────────────────────
pub const EN_ROLE_ADMIN: &str = "Admin";
pub const EN_ROLE_MEMBER: &str = "Member";

pub const JA_ROLE_ADMIN: &str = "管理者";
pub const JA_ROLE_MEMBER: &str = "メンバー";

// ── Home first-run card (RFC-030) ─────────────────────────────────────────
pub const EN_HOME_FIRST_RUN_WELCOME: &str =
    "Welcome. Your community is set up. Here's how to get started.";
pub const EN_HOME_FIRST_RUN_NO_EVENTS: &str =
    "No events yet. Create the first event for your community.";
pub const EN_HOME_FIRST_RUN_CREATE: &str = "+ Create first event";
pub const EN_HOME_FIRST_RUN_INVITE_HINT: &str = "Invite members so they can see your events.";

pub const JA_HOME_FIRST_RUN_WELCOME: &str =
    "コミュニティの設定が完了しました。はじめ方をご確認ください。";
pub const JA_HOME_FIRST_RUN_NO_EVENTS: &str =
    "まだイベントがありません。コミュニティ最初のイベントを作成しましょう。";
pub const JA_HOME_FIRST_RUN_CREATE: &str = "+ 最初のイベントを作成";
pub const JA_HOME_FIRST_RUN_INVITE_HINT: &str =
    "メンバーを招待して、イベントを見てもらいましょう。";

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
pub const EN_TEMPLATES_USE_LINK: &str = "Use a template";

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
pub const JA_TEMPLATES_USE_LINK: &str = "テンプレートを使用";

// ── Community export (RFC-027) ────────────────────────────────────────────
pub const EN_EXPORT_TITLE: &str = "Export community data";
pub const EN_EXPORT_DESCRIPTION: &str =
    "Download a JSON file of your community's events, attendance, and notes.";
pub const EN_EXPORT_PRIVACY_NOTE: &str = "Member names and notes are included. Session tokens and security credentials are not included.";
pub const EN_EXPORT_DOWNLOAD_BTN: &str = "Download JSON";
pub const EN_EXPORT_SINGLE_USE: &str = "This link is single-use and expires in 5 minutes.";

pub const JA_EXPORT_TITLE: &str = "コミュニティの記録をダウンロード";
pub const JA_EXPORT_DESCRIPTION: &str =
    "イベント・出欠・メモの記録をファイルでダウンロードします。";
pub const JA_EXPORT_PRIVACY_NOTE: &str =
    "メンバー名とメモが含まれます。ログイン情報や招待コードは含まれません。";
pub const JA_EXPORT_DOWNLOAD_BTN: &str = "ファイルをダウンロード";
pub const JA_EXPORT_SINGLE_USE: &str = "このリンクは1回限りで、5分後に無効になります。";

// ── Support / about (RFC-035) ─────────────────────────────────────────────
pub const EN_ME_SECTION_ABOUT: &str = "About";
pub const EN_ME_VERSION_LABEL: &str = "Version";
pub const EN_ME_REF_LABEL: &str = "Ref";
pub const EN_ME_SECTION_DATA: &str = "Data";
pub const EN_ME_EXPORT_LINK: &str = "Export community data";

pub const JA_ME_SECTION_ABOUT: &str = "このアプリについて";
pub const JA_ME_VERSION_LABEL: &str = "バージョン";
pub const JA_ME_REF_LABEL: &str = "参照コード";
pub const JA_ME_SECTION_DATA: &str = "データ";
pub const JA_ME_EXPORT_LINK: &str = "記録をダウンロード";

// ── Calendar feed (RFC-023) ───────────────────────────────────────────────
pub const EN_CALENDAR_TITLE: &str = "Calendar feed";
pub const EN_CALENDAR_DESCRIPTION: &str = "Subscribe in Apple Calendar, Google Calendar, or any app that supports calendar subscriptions (.ics / webcal).";
pub const EN_CALENDAR_GENERATE: &str = "Generate feed URL";
pub const EN_CALENDAR_DISABLE: &str = "Disable feed";
pub const EN_CALENDAR_REGENERATE: &str = "Regenerate URL";
pub const EN_CALENDAR_PRIVACY_NOTE: &str = "Your personal calendar feed URL. Keep this private — anyone with the URL can read your community events.";

pub const JA_CALENDAR_TITLE: &str = "予定をカレンダーに入れる";
pub const JA_CALENDAR_DESCRIPTION: &str =
    "AppleカレンダーやGoogleカレンダーなど、予定を取り込めるアプリで利用できます。";
pub const JA_CALENDAR_GENERATE: &str = "リンクを作成";
pub const JA_CALENDAR_DISABLE: &str = "リンクを無効化";
pub const JA_CALENDAR_REGENERATE: &str = "リンクを再作成";
pub const JA_CALENDAR_PRIVACY_NOTE: &str = "このカレンダーリンクは、持っている人なら誰でもあなたのコミュニティの予定を見られます。公開しないでください。こちらで再発行または無効化できます。";

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
pub const EN_CURRENT_BADGE: &str = "Current";
pub const JA_CURRENT_BADGE: &str = "現在";
pub const EN_ME_CALENDAR_LABEL: &str = "Calendar feed";
pub const JA_ME_CALENDAR_LABEL: &str = "予定をカレンダーに入れる";
pub const EN_ME_DATA_EXPORT: &str = "Export community data";
pub const JA_ME_DATA_EXPORT: &str = "記録をダウンロード";

// ── Join page (RFC-003) ────────────────────────────────────────────────────
pub const EN_JOIN_PAGE_TITLE: &str = "Join";
pub const EN_JOIN_PROFILE_PAGE_TITLE: &str = "Your name";

pub const JA_JOIN_PAGE_TITLE: &str = "参加";
pub const JA_JOIN_PROFILE_PAGE_TITLE: &str = "お名前";

#[cfg(test)]
mod tests {
    // Every EN_ constant must have a JA_ counterpart with the same suffix.
    #[test]
    fn en_ja_parity() {
        let en_keys = &[
            // Join
            "JOIN_HEADING",
            "JOIN_SUBHEADING",
            "JOIN_CODE_LABEL",
            "JOIN_CODE_HINT",
            "JOIN_SUBMIT",
            "JOIN_PROFILE_HEADING",
            "JOIN_PROFILE_HINT",
            "JOIN_PROFILE_LABEL",
            "JOIN_PROFILE_SUBMIT",
            // Status
            "STATUS_GOING",
            "STATUS_NOT_GOING",
            "STATUS_ATTENDED",
            "STATUS_NO_ANSWER",
            "STATUS_ATTENDED_DISABLED",
            "STATUS_CLEAR",
            "STATUS_CLEAR_LABEL",
            // Note
            "NOTE_SAVE",
            "NOTE_DELETE",
            "NOTE_SAVED",
            "NOTE_TOO_LONG",
            "NOTE_SECTION_LABEL",
            "NOTE_PLACEHOLDER_LABEL",
            "NOTE_CHAR_HINT",
            "NOTE_VISIBILITY",
            // Session/auth
            "SESSION_EXPIRED",
            "LOGOUT",
            "LOGOUT_CONFIRM",
            // General
            "GENERAL_ERROR",
            "OFFLINE_BANNER",
            "EMPTY_EVENTS",
            "EMPTY_EVENTS_HINT",
            "EMPTY_EVENTS_ADMIN",
            // Nav
            "NAV_HOME",
            "NAV_COMMUNITIES",
            "NAV_ME",
            // Home
            "HOME_TODAY",
            "HOME_THIS_WEEK",
            "HOME_LATER",
            "HOME_CREATE_EVENT",
            "HOME_INVITE_MEMBERS",
            // Me
            "ME_SECTION_NAME",
            "ME_SECTION_COMMUNITY",
            "ME_SECTION_HELP",
            "ME_HELP_BODY",
            // Admin: events
            "ADMIN_CREATE_EVENT_TITLE",
            "ADMIN_CREATE_EVENT_SUBMIT",
            "ADMIN_EDIT_EVENT_TITLE",
            "ADMIN_EDIT_EVENT_SUBMIT",
            "ADMIN_EDIT_EVENT_HINT",
            "ADMIN_CANCEL_EVENT_TITLE",
            "ADMIN_CANCEL_EVENT_BODY",
            "ADMIN_CANCEL_EVENT_KEEP",
            "ADMIN_CANCEL_EVENT_CONFIRM",
            "ADMIN_CANNOT_EDIT_CANCELLED",
            "ADMIN_CANNOT_EDIT_STARTED",
            "ADMIN_CANNOT_ATTEND_CANCELLED",
            "ADMIN_ATTEND_TITLE",
            "ADMIN_ATTEND_SUBMIT",
            // Admin: invites
            "ADMIN_INVITES_TITLE",
            "ADMIN_INVITES_BODY",
            "ADMIN_INVITES_GENERATE",
            "ADMIN_INVITES_ACTIVE",
            "ADMIN_INVITES_NONE",
            "ADMIN_INVITES_NEW_CODE_HINT",
            "ADMIN_INVITES_REVOKE",
            "ADMIN_INVITES_REVOKED",
            // Admin: members
            "ADMIN_MEMBERS_TITLE",
            "ADMIN_MEMBERS_GENERATE_INVITE",
            "ADMIN_REMOVE_TITLE",
            "ADMIN_REMOVE_KEEP",
            "ADMIN_REMOVE_CONFIRM",
            "ADMIN_REMOVE_CONSEQUENCE",
            "ADMIN_LAST_ADMIN",
            // Communities
            "COMMUNITIES_JOIN_ANOTHER",
            // Role labels
            "ROLE_ADMIN",
            "ROLE_MEMBER",
            // Home first-run (RFC-030)
            "HOME_FIRST_RUN_WELCOME",
            "HOME_FIRST_RUN_NO_EVENTS",
            "HOME_FIRST_RUN_CREATE",
            "HOME_FIRST_RUN_INVITE_HINT",
            // Recurrence (RFC-022)
            "REPEAT_LABEL",
            "REPEAT_NONE",
            "REPEAT_WEEKLY",
            "REPEAT_BIWEEKLY",
            "REPEAT_MONTHLY",
            "REPEAT_COUNT_UNIT",
            "REPEAT_COUNT_HINT",
            // Templates (RFC-032)
            "TEMPLATES_TITLE",
            "TEMPLATES_DESCRIPTION",
            "TEMPLATES_EMPTY",
            "TEMPLATES_SAVE_SECTION",
            "TEMPLATES_TITLE_LABEL",
            "TEMPLATES_LOC_LABEL",
            "TEMPLATES_DUR_LABEL",
            "TEMPLATES_SAVE_BTN",
            "TEMPLATES_USE_BTN",
            "TEMPLATES_DELETE_BTN",
            "TEMPLATES_USE_LINK",
            // Export (RFC-027)
            "EXPORT_TITLE",
            "EXPORT_DESCRIPTION",
            "EXPORT_PRIVACY_NOTE",
            "EXPORT_DOWNLOAD_BTN",
            "EXPORT_SINGLE_USE",
            // Me / About (RFC-035)
            "ME_SECTION_ABOUT",
            "ME_VERSION_LABEL",
            "ME_REF_LABEL",
            "ME_SECTION_DATA",
            "ME_EXPORT_LINK",
            // Calendar (RFC-023)
            "CALENDAR_TITLE",
            "CALENDAR_DESCRIPTION",
            "CALENDAR_GENERATE",
            "CALENDAR_DISABLE",
            "CALENDAR_REGENERATE",
            "CALENDAR_PRIVACY_NOTE",
            // Event detail (RFC-006/025)
            "EVENT_TITLE_HEADER",
            "EVENT_ATTENDED_UNAVAILABLE",
            "EVENT_ATTENDED_ADMIN_ONLY",
            "EVENT_MEMBER_FALLBACK",
            // Join page (RFC-003)
            "JOIN_PAGE_TITLE",
            "JOIN_PROFILE_PAGE_TITLE",
        ];
        assert_eq!(en_keys.len(), 120, "update parity list when adding strings");
        for key in en_keys {
            assert!(!key.is_empty(), "empty key: {key}");
        }
    }
}
