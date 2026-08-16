// ── Admin: invite management ──────────────────────────────────────────────
pub const EN_ADMIN_INVITES_TITLE: &str = "Invite Members";
pub const EN_ADMIN_INVITES_BODY: &str = "Generate a one-time code for one person.";
pub const EN_ADMIN_INVITES_GENERATE: &str = "Generate Code";
pub const EN_ADMIN_INVITES_ACTIVE: &str = "Active codes";
pub const EN_ADMIN_INVITES_NONE: &str = "No unused codes.";
pub const EN_ADMIN_INVITES_NEW_CODE_HINT: &str =
    "Share with one person only — expires in 24 hours.";
pub const EN_ADMIN_INVITES_REVEAL_WARNING: &str = "Write down or copy this code now. If you leave or reload this page, it will never be shown again.";
pub const EN_ADMIN_INVITES_REVOKE: &str = "Revoke";
pub const EN_ADMIN_INVITES_BACK_TO_MEMBERS: &str = "Back to member management";

pub const JA_ADMIN_INVITES_TITLE: &str = "メンバーを招待";
pub const JA_ADMIN_INVITES_BODY: &str = "一人のために一回限りのコードを生成します。";
pub const JA_ADMIN_INVITES_GENERATE: &str = "コードを生成";
pub const JA_ADMIN_INVITES_ACTIVE: &str = "有効なコード";
pub const JA_ADMIN_INVITES_NONE: &str = "未使用のコードはありません。";
pub const JA_ADMIN_INVITES_NEW_CODE_HINT: &str =
    "一人だけに共有してください — 24時間で失効します。";
pub const JA_ADMIN_INVITES_REVEAL_WARNING: &str = "このコードを今すぐ書き留めるか、コピーしてください。このページを離れたり再読み込みしたりすると、二度と表示されません。";
pub const JA_ADMIN_INVITES_REVOKE: &str = "無効化";
pub const JA_ADMIN_INVITES_BACK_TO_MEMBERS: &str = "メンバー管理へ戻る";
// Handoff 037: was a raw-English `?flash=Code+revoked` query value matched
// (not echoed) against the now-deleted `JA_ADMIN_INVITES_REVOKED` — matched
// by an improperly-cased key, not a lowercase snake_case code. This
// constant replaced that match arm; the orphaned pair it left behind was
// deleted in Handoff 038 (RFC-075 terminal slice).
pub const JA_ADMIN_INVITE_REVOKED_FLASH: &str = "招待コードを無効にしました。";

// ── Admin: member management ──────────────────────────────────────────────
pub const EN_ADMIN_MEMBERS_TITLE: &str = "Members";
pub const EN_ADMIN_MEMBERS_GENERATE_INVITE: &str = "Generate invite code";
pub const EN_ADMIN_MEMBERS_CURRENT_USER: &str = "You";
pub const EN_ADMIN_PROMOTE_ACTION: &str = "Make admin";
pub const EN_ADMIN_DEMOTE_ACTION: &str = "Make member";
pub const EN_ADMIN_PROMOTE_TITLE: &str = "Make admin?";
pub const EN_ADMIN_PROMOTE_CONSEQUENCE: &str =
    "This member will be able to create events, manage members, and generate invite codes.";
pub const EN_ADMIN_DEMOTE_TITLE: &str = "Make member?";
pub const EN_ADMIN_DEMOTE_CONSEQUENCE: &str = "This person will no longer be able to create events, manage members, or generate invite codes. Past attendance and notes remain.";
pub const EN_ADMIN_LAST_ADMIN_DEMOTE: &str = "Cannot make the last admin a member.";
pub const EN_ADMIN_REMOVE_TITLE: &str = "Remove member?";
pub const EN_ADMIN_REMOVE_KEEP: &str = "Keep Member";
pub const EN_ADMIN_REMOVE_CONFIRM: &str = "Remove";
pub const EN_ADMIN_REMOVE_CONSEQUENCE: &str =
    "They will no longer be able to see events or notes. Past attendance and notes remain.";
pub const EN_ADMIN_LAST_ADMIN: &str =
    "Cannot remove the last admin. Transfer the admin role first.";
pub const EN_ADMIN_HELP_SIGNIN_ACTION: &str = "Help sign in again";
pub const EN_ADMIN_HELP_SIGNIN_TITLE: &str = "Help sign in again?";
pub const EN_ADMIN_HELP_SIGNIN_CONSEQUENCE: &str = "This code lets someone sign in as this member. Share it only with the intended person. It expires in 15 minutes and can be used once.";
pub const EN_ADMIN_HELP_SIGNIN_CREATE: &str = "Create code";
pub const EN_ADMIN_HELP_SIGNIN_CODE_HINT: &str =
    "Share this code only with the intended member. It is shown once.";
pub const EN_ADMIN_HELP_SIGNIN_RELINK_HINT: &str = "Open the sign-in-again page in the other browser and enter this code. It is not an invite code.";
pub const EN_ADMIN_HELP_SIGNIN_RELINK_LINK: &str = "Open sign-in-again page";
pub const EN_ADMIN_HELP_SIGNIN_COPY_CODE: &str = "Copy code";
pub const EN_ADMIN_HELP_SIGNIN_COPY_DONE: &str = "Copied";
pub const EN_ADMIN_HELP_SIGNIN_COPY_FAILED: &str = "Could not copy";
// RFC-082 / Handoff 058: membership suspension — reversible, unlike removal.
pub const EN_ADMIN_SUSPENDED_BADGE: &str = "Suspended";
pub const EN_ADMIN_SUSPEND_ACTION: &str = "Suspend";
pub const EN_ADMIN_UNSUSPEND_ACTION: &str = "Unsuspend";
pub const EN_ADMIN_SUSPEND_TITLE: &str = "Suspend member?";
pub const EN_ADMIN_SUSPEND_CONSEQUENCE: &str = "This member will lose access to this community until unsuspended. Their role, history, and other communities are unaffected. This is reversible.";
pub const EN_ADMIN_UNSUSPEND_TITLE: &str = "Unsuspend member?";
pub const EN_ADMIN_UNSUSPEND_CONSEQUENCE: &str =
    "This member will regain access to this community with their prior role unchanged.";
pub const EN_ADMIN_LAST_ADMIN_SUSPEND: &str =
    "Cannot suspend the last admin. Transfer the admin role first.";

pub const JA_ADMIN_MEMBERS_TITLE: &str = "メンバー";
pub const JA_ADMIN_MEMBERS_GENERATE_INVITE: &str = "招待コードを生成";
pub const JA_ADMIN_MEMBERS_CURRENT_USER: &str = "あなた";
pub const JA_ADMIN_PROMOTE_ACTION: &str = "管理者にする";
pub const JA_ADMIN_DEMOTE_ACTION: &str = "メンバーに戻す";
pub const JA_ADMIN_PROMOTE_TITLE: &str = "管理者にしますか？";
pub const JA_ADMIN_PROMOTE_CONSEQUENCE: &str =
    "このメンバーはイベントの作成、メンバー管理、招待コードの作成ができるようになります。";
pub const JA_ADMIN_DEMOTE_TITLE: &str = "メンバーに戻しますか？";
pub const JA_ADMIN_DEMOTE_CONSEQUENCE: &str = "この人はイベントの作成、メンバー管理、招待コードの作成ができなくなります。過去の参加状況やメモは残ります。";
pub const JA_ADMIN_LAST_ADMIN_DEMOTE: &str = "最後の管理者はメンバーに戻せません。";
pub const JA_ADMIN_REMOVE_TITLE: &str = "メンバーから外しますか？";
pub const JA_ADMIN_REMOVE_KEEP: &str = "やめる";
pub const JA_ADMIN_REMOVE_CONFIRM: &str = "メンバーから外す";
pub const JA_ADMIN_REMOVE_CONSEQUENCE: &str =
    "このメンバーはイベントやメモを見ることができなくなります。過去の参加状況やメモは残ります。";
pub const JA_ADMIN_LAST_ADMIN: &str =
    "最後の管理者はメンバーから外せません。先に管理者権限を移譲してください。";
pub const JA_ADMIN_HELP_SIGNIN_ACTION: &str = "サインインを手伝う";
pub const JA_ADMIN_HELP_SIGNIN_TITLE: &str = "サインインし直すお手伝いをしますか？";
pub const JA_ADMIN_HELP_SIGNIN_CONSEQUENCE: &str = "このコードを使うと、このメンバーとしてサインインできます。本人にだけ渡してください。コードは15分で使えなくなり、1回だけ使えます。";
pub const JA_ADMIN_HELP_SIGNIN_CREATE: &str = "コードを作成";
pub const JA_ADMIN_HELP_SIGNIN_CODE_HINT: &str =
    "このコードは本人にだけ渡してください。ここで一度だけ表示されます。";
pub const JA_ADMIN_HELP_SIGNIN_RELINK_HINT: &str = "別のブラウザでサインインし直す画面を開き、このコードを入力してください。招待コード欄では使えません。";
pub const JA_ADMIN_HELP_SIGNIN_RELINK_LINK: &str = "サインインし直す画面を開く";
pub const JA_ADMIN_HELP_SIGNIN_COPY_CODE: &str = "コードをコピー";
pub const JA_ADMIN_HELP_SIGNIN_COPY_DONE: &str = "コピーしました";
pub const JA_ADMIN_HELP_SIGNIN_COPY_FAILED: &str = "コピーできませんでした";
pub const JA_ADMIN_SUSPENDED_BADGE: &str = "停止中";
pub const JA_ADMIN_SUSPEND_ACTION: &str = "一時停止";
pub const JA_ADMIN_UNSUSPEND_ACTION: &str = "再開";
pub const JA_ADMIN_SUSPEND_TITLE: &str = "一時停止しますか？";
pub const JA_ADMIN_SUSPEND_CONSEQUENCE: &str = "このメンバーは再開されるまでこのコミュニティにアクセスできなくなります。役割・履歴・他のコミュニティへの参加には影響しません。この操作は取り消せます。";
pub const JA_ADMIN_UNSUSPEND_TITLE: &str = "再開しますか？";
pub const JA_ADMIN_UNSUSPEND_CONSEQUENCE: &str =
    "このメンバーは元の役割のまま、このコミュニティに再びアクセスできるようになります。";
pub const JA_ADMIN_LAST_ADMIN_SUSPEND: &str =
    "最後の管理者は一時停止できません。先に管理者権限を移譲してください。";
