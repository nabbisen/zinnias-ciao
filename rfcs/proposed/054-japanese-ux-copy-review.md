# RFC 054 — Japanese UX Copy Review

**Status.** Proposed
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Follows RFC-049 (Japanese rendering). Required before public pilot with non-technical users.

## 1. Summary

All UI strings are now `JA_*` constants. This RFC is a human review of the Japanese copy for clarity, politeness, and absence of technical jargon — not a code change.

## 2. Scope

Review all 143 `JA_*` strings in `packages/contracts/src/i18n.rs` against the criteria:

*(Count was 120 when this RFC was written. Updated to 143 in v0.35.1 after the EN→JA inline string sweep.)*

- **Clarity:** would an IT-averse Japanese user understand what to do?
- **Politeness:** ですます style, appropriate register for general community use.
- **No technical jargon:** no セッション (session), トークン (token), 同期 (sync), HMAC, ICS in member-facing copy.
- **Action labels:** Going/No Go/Attended equivalents should match community norms; consider 参加する / 欠席する / 参加済み / 未回答.
- **Error messages:** should say what to do, not what failed technically.

## 3. Architect suggestions for review

| Current tendency | Recommended alternative |
|---|---|
| セッションが期限切れです | もう一度、招待コードを入力してください |
| 同期に失敗しました | 保存できませんでした。電波状況をご確認ください |
| 参加済み (admin tone) | 参加しました (softer, for member-facing history) |


## 5. Full string inventory for reviewer

All 143 `JA_*` strings as of v0.36.0, grouped by context. The reviewer should
assess each against the four criteria: clarity, politeness, no jargon, and
action-label convention.

### Join / onboarding

| Constant | Current Japanese |
|---|---|
| `JA_JOIN_HEADING` | ciao.zinnias |
| `JA_JOIN_SUBHEADING` | 招待制コミュニティのスケジュール共有 |
| `JA_JOIN_CODE_LABEL` | 招待コード |
| `JA_JOIN_CODE_HINT` | 招待コードはコミュニティの管理者にお問い合わせください。 |
| `JA_JOIN_SUBMIT` | 参加する |
| `JA_JOIN_PROFILE_HEADING` | このコミュニティでの名前 |
| `JA_JOIN_PROFILE_HINT` | イベントへの返答やメモを残すときにこの名前が表示されます。 |
| `JA_JOIN_PROFILE_LABEL` | 表示名 |
| `JA_JOIN_PROFILE_SUBMIT` | はじめる |
| `JA_JOIN_PAGE_TITLE` | 参加 |
| `JA_JOIN_PROFILE_PAGE_TITLE` | お名前 |

### Participation status

| Constant | Current Japanese | Reviewer note |
|---|---|---|
| `JA_STATUS_GOING` | 参加 | |
| `JA_STATUS_NOT_GOING` | 不参加 | |
| `JA_STATUS_ATTENDED` | 出席済み | Consider 参加済み (softer) |
| `JA_STATUS_NO_ANSWER` | 未回答 | |
| `JA_STATUS_ATTENDED_DISABLED` | イベント終了後に利用可能 | |
| `JA_STATUS_CLEAR` | クリア | |
| `JA_STATUS_CLEAR_LABEL` | 回答をクリア | |
| `JA_EVENT_ATTENDED_UNAVAILABLE` | イベント終了後に選択できます | |
| `JA_EVENT_ATTENDED_ADMIN_ONLY` | 出席の記録は管理者のみ行えます | |

### Memo / notes

| Constant | Current Japanese |
|---|---|
| `JA_NOTE_SECTION_LABEL` | あなたのメモ |
| `JA_NOTE_PLACEHOLDER_LABEL` | メモ（200文字以内） |
| `JA_NOTE_CHAR_HINT` | 200文字以内 |
| `JA_NOTE_VISIBILITY` | コミュニティのメンバーにこのメモが表示されます。 |
| `JA_NOTE_SAVE` | メモを保存 |
| `JA_NOTE_SAVED` | 保存しました。 |
| `JA_NOTE_DELETE` | メモを削除 |
| `JA_NOTE_DELETE_BODY` | このメモは削除されます。元に戻すことはできません。 |
| `JA_NOTE_KEEP_ACTION` | メモを保持 |
| `JA_NOTE_TOO_LONG` | メモが長すぎます。200文字以内にしてください。 |
| `JA_EVENT_NOTES_SECTION` | メモ |

### Navigation

| Constant | Current Japanese |
|---|---|
| `JA_NAV_HOME` | ホーム |
| `JA_NAV_COMMUNITIES` | コミュニティ |
| `JA_NAV_ME` | マイページ |
| `JA_NAV_BACK` | イベントに戻る |
| `JA_NAV_SWITCH_GO` | 切り替え |
| `JA_GENERAL_BACK` | 戻る |

### Home screen sections

| Constant | Current Japanese |
|---|---|
| `JA_HOME_TODAY` | 今日 |
| `JA_HOME_THIS_WEEK` | 今週 |
| `JA_HOME_LATER` | それ以降 |
| `JA_HOME_CREATE_EVENT` | + イベントを作成 |
| `JA_HOME_INVITE_MEMBERS` | メンバーを招待 |
| `JA_HOME_FIRST_RUN_WELCOME` | コミュニティの設定が完了しました。はじめ方をご確認ください。 |
| `JA_HOME_FIRST_RUN_NO_EVENTS` | まだイベントがありません。コミュニティ最初のイベントを作成しましょう。 |
| `JA_HOME_FIRST_RUN_CREATE` | + 最初のイベントを作成 |
| `JA_HOME_FIRST_RUN_INVITE_HINT` | メンバーを招待して、イベントを見てもらいましょう。 |
| `JA_EMPTY_EVENTS` | イベントはまだありません。 |
| `JA_EMPTY_EVENTS_HINT` | コミュニティの管理者にイベントの追加をお願いしてください。 |
| `JA_EMPTY_EVENTS_ADMIN` | イベントはまだありません。最初のイベントを作成しましょう。 |

### Event detail

| Constant | Current Japanese |
|---|---|
| `JA_EVENT_TITLE_HEADER` | イベント |
| `JA_EVENT_WHOS_GOING` | 参加予定者 |
| `JA_EVENT_MEMBER_FALLBACK` | メンバー |
| `JA_EVENT_CANCELLED_BADGE` | このイベントはキャンセルされました。 |

### Admin: event management

| Constant | Current Japanese |
|---|---|
| `JA_ADMIN_CREATE_EVENT_TITLE` | イベントを作成 |
| `JA_ADMIN_CREATE_EVENT_SUBMIT` | イベントを作成 |
| `JA_ADMIN_EDIT_EVENT_TITLE` | イベントを編集 |
| `JA_ADMIN_EDIT_EVENT_SUBMIT` | 変更を保存 |
| `JA_ADMIN_EDIT_EVENT_HINT` | メンバーには更新されたイベント詳細が表示されます。 |
| `JA_ADMIN_CANCEL_EVENT_TITLE` | このイベントをキャンセルしますか？ |
| `JA_ADMIN_CANCEL_EVENT_BODY` | メンバーにはキャンセルされたことが引き続き表示されます。 |
| `JA_ADMIN_CANCEL_EVENT_KEEP` | イベントを保持 |
| `JA_ADMIN_CANCEL_EVENT_CONFIRM` | イベントをキャンセル |
| `JA_ADMIN_CANNOT_EDIT_CANCELLED` | キャンセル済みのイベントは編集できません。 |
| `JA_ADMIN_CANNOT_EDIT_STARTED` | このイベントはすでに開始しているため編集できません。 |
| `JA_ADMIN_ATTEND_TITLE` | 出席を記録 |
| `JA_ADMIN_ATTEND_SUBMIT` | 出席を保存 |
| `JA_ADMIN_CANNOT_ATTEND_CANCELLED` | キャンセル済みのイベントの出席は修正できません。 |
| `JA_ADMIN_EDIT_CANCELLED` | キャンセル済みのイベントは編集できません。 |
| `JA_ADMIN_EDIT_STARTED` | すでに開始したイベントは編集できません。 |
| `JA_ADMIN_ATTEND_CANCELLED` | キャンセル済みのイベントの出席は修正できません。 |

### Admin: event form fields

| Constant | Current Japanese |
|---|---|
| `JA_FORM_FIELD_TITLE` | タイトル |
| `JA_FORM_FIELD_DATE` | 日付 |
| `JA_FORM_FIELD_START` | 開始時刻 |
| `JA_FORM_FIELD_END` | 終了時刻 |
| `JA_FORM_FIELD_LOCATION` | 場所（任意） |
| `JA_FORM_FIELD_DESC` | 説明（任意） |
| `JA_REPEAT_LABEL` | 繰り返し |
| `JA_REPEAT_NONE` | 繰り返さない |
| `JA_REPEAT_WEEKLY` | 毎週 |
| `JA_REPEAT_BIWEEKLY` | 2週間ごと |
| `JA_REPEAT_MONTHLY` | 毎月 |
| `JA_REPEAT_COUNT_UNIT` | 回 |
| `JA_REPEAT_COUNT_HINT` | 「繰り返さない」を選択した場合、回数は無視されます。 |

### Admin: invites and members

| Constant | Current Japanese |
|---|---|
| `JA_ADMIN_INVITES_TITLE` | メンバーを招待 |
| `JA_ADMIN_INVITES_BODY` | 一人のために一回限りのコードを生成します。 |
| `JA_ADMIN_INVITES_GENERATE` | コードを生成 |
| `JA_ADMIN_INVITES_ACTIVE` | 有効なコード |
| `JA_ADMIN_INVITES_NONE` | 未使用のコードはありません。 |
| `JA_ADMIN_INVITES_NEW_CODE_HINT` | 一人だけに共有してください — 24時間で失効します。 |
| `JA_ADMIN_INVITES_REVOKE` | 無効化 |
| `JA_ADMIN_INVITES_REVOKED` | コードを無効化しました |
| `JA_ADMIN_MEMBERS_TITLE` | メンバー |
| `JA_ADMIN_MEMBERS_GENERATE_INVITE` | 招待コードを生成 |
| `JA_ADMIN_REMOVE_TITLE` | メンバーを削除しますか？ |
| `JA_ADMIN_REMOVE_KEEP` | メンバーを保持 |
| `JA_ADMIN_REMOVE_CONFIRM` | 削除 |
| `JA_ADMIN_REMOVE_CONSEQUENCE` | このメンバーはイベントやメモを見ることができなくなります。 |
| `JA_ADMIN_LAST_ADMIN` | 最後の管理者は削除できません。先に管理者権限を移譲してください。 |

### Me / profile

| Constant | Current Japanese | Reviewer note |
|---|---|---|
| `JA_ME_SECTION_NAME` | 名前 | |
| `JA_ME_SECTION_COMMUNITY` | 現在のコミュニティ | |
| `JA_ME_SECTION_HELP` | ヘルプ | |
| `JA_ME_HELP_BODY` | 入室できない場合やアクセスを失った場合は、コミュニティの管理者にお問い合わせください。 | |
| `JA_ME_SECTION_ABOUT` | このアプリについて | |
| `JA_ME_VERSION_LABEL` | バージョン | |
| `JA_ME_REF_LABEL` | 参照コード | |
| `JA_ME_SECTION_DATA` | データ | |
| `JA_ME_DATA_EXPORT` | コミュニティデータをエクスポート | Consider 記録をダウンロード |
| `JA_ME_EXPORT_LINK` | コミュニティデータをエクスポート | Same |
| `JA_ME_CALENDAR_LABEL` | カレンダーフィード | Consider 予定をカレンダーに入れる |
| `JA_LOGOUT` | ログアウト | |
| `JA_LOGOUT_CONFIRM` | ログアウトしますか？ | |

### Calendar feed

| Constant | Current Japanese | Reviewer note |
|---|---|---|
| `JA_CALENDAR_TITLE` | カレンダーフィード | Consider 予定をカレンダーに入れる |
| `JA_CALENDAR_DESCRIPTION` | Appleカレンダー、Googleカレンダー、またはiCS/webcalに対応したアプリでご利用いただけます。 | `iCS` casing odd |
| `JA_CALENDAR_GENERATE` | フィードURLを生成 | |
| `JA_CALENDAR_DISABLE` | フィードを無効化 | |
| `JA_CALENDAR_REGENERATE` | URLを再生成 | |
| `JA_CALENDAR_PRIVACY_NOTE` | 個人のカレンダーフィードURLです。このURLを知っている人はコミュニティのイベントを閲覧できます。公開しないでください。 | |

### Export

| Constant | Current Japanese | Reviewer note |
|---|---|---|
| `JA_EXPORT_TITLE` | コミュニティデータのエクスポート | Technical term |
| `JA_EXPORT_DESCRIPTION` | コミュニティのイベント・出欠・メモをJSONファイルでダウンロードします。 | JSON visible to user |
| `JA_EXPORT_PRIVACY_NOTE` | メンバー名とメモが含まれます。セッショントークンやセキュリティ情報は含まれません。 | セッショントークン is technical |
| `JA_EXPORT_DOWNLOAD_BTN` | JSONをダウンロード | JSON visible to user |
| `JA_EXPORT_SINGLE_USE` | このリンクは1回限りで、5分後に無効になります。 | |

### Communities

| Constant | Current Japanese |
|---|---|
| `JA_COMMUNITIES_JOIN_ANOTHER` | 別のコミュニティに参加 |
| `JA_CURRENT_BADGE` | 現在 |
| `JA_ROLE_ADMIN` | 管理者 |
| `JA_ROLE_MEMBER` | メンバー |

### Templates

| Constant | Current Japanese |
|---|---|
| `JA_TEMPLATES_TITLE` | イベントテンプレート |
| `JA_TEMPLATES_DESCRIPTION` | よく使うイベント情報をテンプレートとして保存して、素早く作成できます。 |
| `JA_TEMPLATES_EMPTY` | まだテンプレートがありません。 |
| `JA_TEMPLATES_SAVE_SECTION` | テンプレートを保存 |
| `JA_TEMPLATES_TITLE_LABEL` | タイトル |
| `JA_TEMPLATES_LOC_LABEL` | 場所（任意） |
| `JA_TEMPLATES_DUR_LABEL` | デフォルトの所要時間（分、任意） |
| `JA_TEMPLATES_SAVE_BTN` | テンプレートを保存 |
| `JA_TEMPLATES_USE_BTN` | 使用 |
| `JA_TEMPLATES_DELETE_BTN` | 削除 |
| `JA_TEMPLATES_USE_LINK` | テンプレートを使用 |

### Error and system messages

| Constant | Current Japanese | Reviewer note |
|---|---|---|
| `JA_SESSION_EXPIRED` | セッションが切れました。新しい招待コードをコミュニティ管理者にお問い合わせください。 | セッション is technical |
| `JA_GENERAL_ERROR` | エラーが発生しました。もう一度お試しください。 | |
| `JA_NOT_FOUND` | 見つかりませんでした。 | |
| `JA_INTERNAL_ERROR` | 問題が発生しました。もう一度お試しください。 | |
| `JA_OFFLINE_BANNER` | オフライン — 最後に読み込んだ情報を表示しています | オフライン is used colloquially |
| `JA_TZ_ERROR` | コミュニティのタイムゾーンが正しく設定されていません。運営者にお問い合わせください。 | タイムゾーン may be acceptable |

## 4. Blocker

Requires a Japanese native speaker familiar with the target community type. Output: a list of string changes applied to i18n.rs.
