# RFC 062 — Admin Role Transfer and Promotion

**Status.** Implemented
**Phase:** F8 / Community administration workflow
**Project:** ciao.zinnias
**Date:** 2026-07-06
**Relationship:** Follows RFC-010 admin invite/member management, RFC-024
account relinking, RFC-025 member safety, RFC-057 community creation, and
RFC-061 community admin member-management navigation.
**Design review:** `.git-exclude/reviewed/zinnias-ciao-v0.49.0-rfc062-admin-role-transfer-design-review.md`

---

## 1. Summary

Community admins need a safe way to share or transfer administrative
responsibility inside a community.

Today, a community can have admins and members, and the app protects against
removing the last admin. But there is no friendly in-app workflow for:

- promoting a trusted member to admin;
- demoting an admin back to member;
- transferring responsibility before an admin leaves;
- reducing the risk that a sole admin loses access and no one can manage the
  community.

RFC-062 designs role transfer and promotion as an admin-only member-management
extension. It does not implement lost-session recovery; that remains RFC-024.
It also does not create a complex permission model. The product keeps the
existing two roles: `admin` and `member`.

The first slice implements both directions from the member-management page:
promote an existing active member to admin, and demote another active admin back
to member. It does not expose admin-granting invite codes in the UI; assigning
admin role remains a deliberate action on a visible, already-joined member row.

## 2. Problem Statement

The sole-admin model is fragile.

The current app can prevent an admin from removing the last admin, but it cannot
help if the last admin loses their browser/session. RFC-024 is the direct
recovery path, but role transfer reduces the likelihood of that failure by
letting an existing admin intentionally add another admin before trouble occurs.

Admins also need a routine operational path:

- a founder wants another organizer to help manage events and invites;
- a retiring organizer wants to leave after another admin is ready;
- an accidental admin assignment needs to be corrected;
- a community wants two active admins for continuity.

Without in-app role management, the operator or database becomes the fallback,
which is not friendly, auditable in product terms, or scalable for a pilot.

## 3. Goals

- Let an active admin promote an active member to admin.
- Let an active admin demote another admin to member.
- Keep the last-admin guard.
- Keep role changes community-scoped.
- Make the UI clear enough for non-technical organizers.
- Record role changes in audit logs.
- Keep the two-role model: admin/member.
- Preserve no-JS server-rendered behavior.
- Keep the feature reachable from `/c/:cid/admin/members`.
- Avoid solving account recovery or relinking in this RFC.

## 4. Non-Goals

- No granular permissions.
- No owner/super-admin/community-founder role.
- No operator impersonation workflow.
- No self-service account recovery.
- No member relinking or recovery codes.
- No admin-granting invite-code UI. Invite generation remains member-role only
  in this slice.
- No public role directory.
- No cross-community role changes.
- No bulk role changes.
- No self-demotion or retiring-admin handoff flow.

## 5. External Behavior

On the member-management page, an admin can see each member's current role:

```text
管理者
メンバー
```

For another active member, the admin can open a confirmation flow:

```text
管理者にする
```

For another active admin, the admin can open a confirmation flow:

```text
メンバーに戻す
```

The current admin should not see a self-demotion action in the first slice.
Self-demotion is easy to misunderstand and can interact poorly with the
last-admin rule. If self-demotion is desired later, it should be designed as an
explicit handoff flow.

Admin invite generation remains unchanged: generated invite codes grant member
role. Admin role assignment happens only after a person has joined the community
and appears as an active member.

## 6. Confirmation Flows

Role changes must use confirmation pages, not one-click links.

Promote confirmation:

```text
管理者にしますか？

このメンバーはイベントの作成、メンバー管理、招待コードの作成ができるようになります。

[管理者にする]
[やめる]
```

Demote confirmation:

```text
メンバーに戻しますか？

この人はイベントの作成、メンバー管理、招待コードの作成ができなくなります。
過去の参加状況やメモは残ります。

[メンバーに戻す]
[やめる]
```

If demotion would leave the community without an admin, the app must block it
with plain copy:

```text
最後の管理者はメンバーに戻せません。
```

## 7. Routes

Recommended route shape:

```text
GET  /c/:cid/admin/members/:mid/promote
POST /c/:cid/admin/members/:mid/promote
GET  /c/:cid/admin/members/:mid/demote
POST /c/:cid/admin/members/:mid/demote
```

All routes require active admin membership in `:cid`.

Direct URLs for absent, removed, cross-community, or unauthorized memberships
must use the same safe denial behavior as existing admin member routes.
Promote/demote are separate routes; the direction is not accepted from
user-submitted form data.

## 8. Data Model

No new table is required for the first slice.

The existing `community_memberships.role` field can be updated from `member` to
`admin`, or from `admin` to `member`.

Required write discipline:

- role changes are scoped by both `membership_id` and `community_id`;
- removed memberships cannot be promoted or demoted;
- last-admin demotion is blocked by the role-change write itself;
- the current admin cannot demote themself in the first slice;
- all mutations use form-token discipline.
- no-op safety is structural: duplicate or stale requests must not rely only on
  form-token single-use;
- target resolution uses a scoped active-role lookup; absent, removed, and
  cross-community target ids return the generic not-found response.

Demote must use a single conditional statement that re-checks the active admin
count inside the update:

```sql
UPDATE community_memberships
SET role = 'member'
WHERE id = ?1
  AND community_id = ?2
  AND removed_at IS NULL
  AND role = 'admin'
  AND (
    SELECT COUNT(*)
    FROM community_memberships
    WHERE community_id = ?2
      AND role = 'admin'
      AND removed_at IS NULL
  ) > 1;
```

Promote must also be conditional:

```sql
UPDATE community_memberships
SET role = 'admin'
WHERE id = ?1
  AND community_id = ?2
  AND removed_at IS NULL
  AND role = 'member';
```

The handler branches on rows affected:

- `1` row changed: write the audit row and redirect to member management;
- `0` rows changed and target is already in the desired role: redirect with no
  audit;
- `0` rows changed because demotion would leave no admin: render the
  last-admin block copy;
- `0` rows changed because the target is absent, removed, cross-community, or
  otherwise invalid: generic not-found.

This release must also retrofit admin removal so removing an admin cannot race
with another remove/demote request and leave the community with zero admins.
The existing pre-check-then-update pattern is not sufficient for admin removal
or demotion.

## 9. Audit

Role changes must be audited.

Audit actions:

```text
membership.promoted_to_admin
membership.demoted_to_member
```

Audit metadata should avoid user-entered notes or private details. Target
membership id is acceptable as target id because audit logs already use internal
ids. The action name encodes the direction, so old-role/new-role metadata is not
needed in this first slice. UI shown to admins should use display names, not raw
ids.

## 10. Security and Safety

- Role changes are admin-only server-side actions.
- UI hiding is not authorization.
- Cross-community target ids must not work.
- Removed members cannot be promoted or demoted.
- Last-admin guard is mandatory.
- Self-demotion is out of scope for the first slice and must be rejected
  server-side, not only hidden in the UI.
- Admin-granting invite codes stay out of the UI; invite generation continues
  to grant member role.
- Promote/demote POSTs require form tokens.
- Duplicate submit must be harmless: applying the same role twice should not
  create inconsistent state.
- The feature must not bypass RFC-024 recovery rules.
- Role changes never create, merge, or relink user identities.

## 11. Relationship to RFC-024

RFC-062 reduces sole-admin risk but does not solve lost-session recovery.

If all admins lose access, promotion cannot help because promotion itself
requires a working admin session. RFC-024 remains necessary for account
relinking or admin-mediated recovery.

RFC-062 should be implemented before or near RFC-024 if the pilot depends on
community continuity, but it must not add recovery codes or identity merging.
It operates only on existing active memberships.

## 12. Acceptance Criteria

- Admin can promote another active member to admin.
- Admin can demote another active admin to member.
- Self-demotion is not shown in the first slice.
- Self-demotion is rejected server-side.
- Last-admin demotion is blocked server-side.
- Concurrent demote/remove attempts cannot leave a community with zero admins.
- Removed, absent, cross-community, and unauthorized target ids are denied
  safely.
- Duplicate submits and already-applied role changes are harmless no-ops.
- Member-management page shows role-change actions only where allowed.
- Role-change confirmation pages use plain Japanese copy.
- Role changes are audited.
- Admin invite generation still grants member role.
- Release gates or tests cover last-admin guard, cross-community safety, and
  source discoverability from member management.
- Browser smoke covers mobile width and 200% text scaling.

## 13. Test Plan

- Unit/source gates for route registration and member-page links.
- Authorization tests or source gates for admin-only access.
- Role update tests:
  - member -> admin;
  - admin -> member;
  - no-op duplicate submit;
  - removed target rejected;
  - cross-community target rejected;
  - self-demotion rejected;
  - last-admin demotion rejected.
  - concurrent double-demote or demote/remove leaves at least one active admin.
- Removal update test:
  - concurrent admin removal cannot leave zero admins.
- Audit tests or source gates for action names and target scoping.
- Source gate that generated invite codes remain member-role only.
- Browser smoke:
  - promote flow;
  - demote flow;
  - last-admin blocked copy;
  - non-admin cannot see role actions;
  - layout at mobile width and 200% text scaling.

## 14. Deferred Questions

- Whether admin invite codes should ever be able to grant admin role from UI.
  This is deferred because admin invites are bearer tokens of standing admin
  power and do not help promote an existing member.
- Whether self-demotion should be allowed later through a dedicated handoff
  flow.
- Should role changes require a second confirmation phrase for very small
  communities?
- Should role-change audit be visible to admins in a future history view?
