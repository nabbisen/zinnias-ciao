# RFC 061 — Community Admin Member Management Navigation

**Status.** Proposed
**Phase:** F8 / Community and Calendar workflow improvement
**Project:** ciao.zinnias
**Date:** 2026-07-06
**Relationship:** Extends RFC-010 admin invite and member management,
RFC-024 account relinking, RFC-025 moderation/member safety, RFC-030 admin
onboarding, RFC-056 calendar-centered dashboard, RFC-057 community creation,
and RFC-059 calendar create-event-from-day.
**Review:** `.git-exclude/reviewed/zinnias-ciao-rfc061-community-admin-member-management-navigation-review.md`

---

## 1. Summary

The app already has admin member-management routes, but the workflow is not
friendly, discoverable, or complete enough for ordinary community admins.

The current visible admin flow mostly points admins to invite-code generation.
That is only one subtask. A community admin who wants to manage people needs a
clear place to:

- see members of the active community;
- invite a new member;
- remove a member from the community;
- understand who has admin power;
- later, help a member recover access without creating a duplicate identity.

RFC-061 turns member administration into an explicit community-admin workflow.
It does not treat the problem as a missing link. It defines a small admin
member-management surface where member management is the parent task and invite
generation is one child action.

## 2. Current State

Implemented routes already include:

```text
GET  /c/:cid/admin/members
GET  /c/:cid/admin/invites
POST /c/:cid/admin/invites
POST /c/:cid/admin/invites/:iid/revoke
GET  /c/:cid/admin/members/:mid/remove
POST /c/:cid/admin/members/:mid/remove
```

The members route can list active members and start member removal. The invites
route can generate and revoke invite codes.

However, the product path is weak:

- Home admin shortcuts link to event creation and invite generation, not to a
  member-management parent workflow.
- The bottom navigation is Home, Calendar, and Me; it has no admin destination.
- The Me page exposes data export for admins, but not member management.
- The invite page is presented as a destination rather than as a sub-action of
  managing members.
- The member-management page exists, but admins are not naturally guided to it.

This is inconvenient, less functional than the underlying routes, and far from
friendly for non-technical community organizers.

## 3. Problem Statement

Community administration is a normal recurring workflow, not an exceptional
debug page.

Admins need to answer questions such as:

- "Who is currently in this community?"
- "How do I invite one more person?"
- "How do I remove someone who should no longer have access?"
- "Who else is an admin?"
- "What should I do when a member lost their browser/session?"

The current UI makes the easiest visible action "invite members." That biases
the workflow toward adding people while hiding the broader responsibility of
maintaining the community boundary. It also makes future recovery or moderation
work harder to place because there is no clear admin home for member operations.

## 4. Goals

- Make member management discoverable for active community admins.
- Make `/c/:cid/admin/members` the primary member-management destination.
- Treat invite-code generation as a sub-action of member management.
- Add direct member-management entry points without making Calendar noisy.
- Keep actions scoped to the active community and compatible with the existing
  community switcher model.
- Keep non-admin members from seeing admin controls.
- Preserve no-JS server-rendered behavior.
- Keep Japanese copy simple, concrete, and non-technical.
- Preserve all existing security properties from RFC-010.
- Leave enough design room for RFC-024 account relinking without implementing
  it accidentally.

## 5. Non-Goals

- No public member directory.
- No member self-service profile directory.
- No email/SMS sending.
- No bulk import.
- No role hierarchy beyond existing admin/member roles.
- No admin promotion/transfer redesign in this RFC unless explicitly accepted
  during review.
- No account relinking implementation; RFC-024 remains the authority for that.
- No cross-community member management.
- No Calendar redesign beyond preserving a clean route to admin work.
- No operator-only secret or staging configuration change.

## 6. UX Direction

### 6.1 Community Admin Entry Point

For the first implementation slice, active admins should be linked directly to
member management:

```text
GET /c/:cid/admin/members
```

Do not add a new `/c/:cid/admin` hub in the first slice.

The hub is deferred because its initial children would duplicate existing
destinations: members, event creation, templates, and export. Direct links solve
the current problem with less route surface and less implementation risk.

A future admin hub remains acceptable once there are multiple admin-only
workflows that do not fit naturally elsewhere, such as recovery or moderation.
If that hub is later built, it must be implemented in its own handler file
rather than being added to `handlers/admin/members.rs`.

### 6.2 Home Page Entry

For active admins, Home should provide a clear member-management action.

The recurring admin shortcut changes from invite-first framing to:

```text
メンバーを管理
```

The target is:

```text
/c/:cid/admin/members
```

First-run Home may still emphasize inviting the first members, but the link
target should land on the member-management page. The member-management page can
then present invite creation prominently.

Home remains event/member overview, not an admin dashboard.

### 6.3 Me Page Entry

The Me page already contains admin-only operational actions. Add a visible
admin tools section for active admins of the current community.

Required structure:

```text
管理
メンバーを管理
データを書き出す
```

`メンバーを管理` links to `/c/:cid/admin/members`.

### 6.4 Calendar Page

Calendar is primarily for viewing events of the active community. It should not
become crowded with member-management controls.

RFC-061 does not add a member-management button into every Calendar cell or day
agenda. The acceptable Calendar impact is limited to consistent app navigation:
an admin should be able to reach the admin/member workflow through the app's
normal shell without leaving the active community context.

The selected-day create-event action from RFC-059 remains separate.

## 7. Member-Management Page

`GET /c/:cid/admin/members` becomes the primary page for member administration.

Required content:

- community name or active community switcher;
- list of active members;
- display name;
- role label;
- clear indication of the current admin's own membership;
- admin-safe remove action where allowed;
- invite action;
- link to active invite codes if shown on a separate page.

Required top action:

```text
招待コードを作成
```

This action opens the existing invite-code page:

```text
/c/:cid/admin/invites
```

Inline invite-code generation on the members page is deferred. Keeping the
existing invite page as the child page preserves the current token and revoke
flow while fixing discoverability.

The members page should not expose internal membership IDs, user IDs, token
subjects, HMACs, or audit IDs to normal admins.

## 8. Invite-Code Page

`GET /c/:cid/admin/invites` remains valid, but it should behave like a child
page of member management.

Required changes:

- provide a clear link back to member management;
- keep plaintext invite code visible only immediately after generation;
- keep active invite revocation;
- make the page title/copy clear that the code is for one person;
- avoid presenting invite generation as the whole admin workflow.

Suggested back link:

```text
メンバー管理へ戻る
```

## 9. Remove vs Disable Language

The existing implementation soft-removes membership. UX language needs review.

For non-technical admins, "remove" can imply deletion of history, while
"disable access" can imply a reversible suspension. The current product has no
admin UI to re-enable a removed member, so the first implementation uses:

```text
メンバーから外す
```

The confirmation copy must state that past attendance, notes, and audit records
are preserved. Final wording should still be checked through RFC-054 Japanese
copy review, but implementation should not remain blocked by parallel candidate
phrases.

Rejected for the first slice:

```text
アクセスを止める
```

because it suggests a temporary access toggle that the app does not provide.

## 10. Recovery and Reissue Boundary

Admins also need a way to help members who lose access. That capability is not
implemented by RFC-061.

For v0.48.0-sized scope, RFC-061 only prepares the UI shape:

- reserve a member-row action area where future recovery can live;
- do not add a fake "reissue token" action that creates a new member identity;
- do not add relink, session recovery, admin-mediated identity recovery, or
  token reissue code paths;
- link future work to RFC-024.

If review later decides recovery must ship, then RFC-024 must be updated or
superseded with a concrete relink-code design before coding.

### 10.1 Sole-Admin Recovery Pilot Risk

RFC-061 does not solve the sole-admin recovery problem, but it must name it.

Current facts:

- the app protects against removing the last admin;
- session recovery is not implemented;
- a user who loses their browser/session can only join again through a new
  invite, which creates a new identity;
- if the sole admin loses their session, no in-app admin remains to invite,
  promote, or relink that person.

This is a real pilot risk and belongs to RFC-024 or a future role-transfer /
recovery RFC. RFC-061 reserves UI room for that future workflow but must not
pretend the problem is solved.

## 11. Authorization and Safety

All admin pages and actions require active admin membership in the target
community. UI hiding is not authorization.

Required rules:

- non-admin access to `/c/:cid/admin/members` and `/c/:cid/admin/invites`
  returns the existing safe denial behavior;
- community switching to an admin destination must validate admin role in the
  target community, not only membership;
- if the selected target community is valid for membership but not admin role,
  switcher handling should fall back to the target community Home instead of
  showing an admin-denied page;
- direct URLs must not reveal whether another community, member, or invite code
  exists;
- invite codes remain short-lived, one-time, and stored only as protected
  server-side values;
- remove actions must keep the last-admin guard;
- admin actions are audited without secret material.

## 12. Data Model

No schema change is required for the initial navigation and workflow cleanup.

The initial implementation can use existing tables and soft-removal behavior.
If implementation adds recovery/relink actions, that becomes a schema-affecting
change and must follow RFC-024 or an approved successor.

## 13. Routes

Recommended route set:

```text
GET  /c/:cid/admin/members
GET  /c/:cid/admin/invites
POST /c/:cid/admin/invites
POST /c/:cid/admin/invites/:iid/revoke
GET  /c/:cid/admin/members/:mid/remove
POST /c/:cid/admin/members/:mid/remove
```

Existing routes should remain stable. A future `/c/:cid/admin` hub would be an
additive change and must not break bookmarked member or invite links.

## 14. Copy Contract

Initial Japanese copy:

| Surface | Copy |
|---------|----------------|
| Admin tools section | `管理` |
| Members action | `メンバーを管理` |
| Invite action | `招待コードを作成` |
| Back to members | `メンバー管理へ戻る` |
| Current user marker | `あなた` |
| Admin role | `管理者` |
| Member role | `メンバー` |
| Remove action | `メンバーから外す` |

Copy must be reviewed for plainness and emotional safety through RFC-054, but
these defaults are the implementation baseline. The UI should avoid technical
terms such as token, subject, membership ID, HMAC, and session unless the page
is explicitly operator-facing.

## 15. Acceptance Criteria

- Active community admins can discover member management without typing a URL.
- Home no longer makes invite-code generation the only visible member-related
  admin action.
- Me exposes a community admin/member-management path for admins.
- `/c/:cid/admin/members` is the parent member-management page.
- Invite generation is reachable from member management.
- Invite page links back to member management.
- Non-admin members do not see admin links and cannot access admin routes.
- Community switching preserves the intended admin page only when the target
  community also grants admin role; otherwise it falls back to that community
  Home.
- Remove behavior keeps last-admin protection.
- No plaintext invite code is stored, logged, or shown after the generation
  response.
- Browser smoke covers mobile width and large text for the admin entry, members
  page, invite generation, and remove confirmation.

## 16. Test Plan

- Source/release gates for route discoverability:
  - Home admin link target includes member management;
  - Me admin tools include member management;
  - invite page links back to members.
- Authorization tests or source gates:
  - non-admin cannot access members/invites;
  - cross-community direct URLs do not expose data.
  - admin-destination community switching requires admin role and falls back to
    Home for member-only target communities.
- Existing invite tests remain valid:
  - generated code is one-time;
  - generated code expires;
  - active code can be revoked;
  - plaintext code is only shown immediately.
- Existing removal tests remain valid:
  - admin can remove another member;
  - admin cannot remove the last admin;
  - removed member loses access on subsequent server validation.
- Browser smoke:
  - admin reaches member management from normal UI;
  - admin creates and revokes an invite from the member workflow;
  - admin opens remove confirmation and cancels safely;
  - member session does not show admin links;
  - layout has no horizontal scroll at mobile width and 200% text scaling.

## 17. Rollout Notes

This is a workflow improvement and can be released as a minor version because it
changes user-facing navigation and admin behavior.

The first implementation should stay small:

1. add the discoverable admin/member entry points;
2. make members the parent page;
3. add invite-page back navigation;
4. tighten copy and tests.

Do not add `/c/:cid/admin` in this first slice. Recovery/relink remains a
follow-up under RFC-024 or a successor RFC.

## 18. Open Questions

- What exact confirmation copy best explains that `メンバーから外す` preserves
  past attendance, notes, and audit records?
- Should role transfer/promotion get its own RFC, or be folded into RFC-024?
- When should RFC-024 be advanced so sole-admin recovery has a complete design
  before a pilot community hits the failure mode?
