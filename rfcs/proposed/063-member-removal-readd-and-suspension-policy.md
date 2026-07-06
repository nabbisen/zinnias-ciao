# RFC 063 — Member Removal, Re-add, and Suspension Policy

**Status.** Proposed
**Phase:** F8 / Community administration workflow
**Project:** ciao.zinnias
**Date:** 2026-07-06
**Relationship:** Follows RFC-010 admin invite/member management, RFC-024
account relinking, RFC-025 moderation/member safety, RFC-052 audit retention,
and RFC-061 community admin member-management navigation.

---

## 1. Summary

The current product has member removal, implemented as a soft remove on the
membership. The UI now uses `メンバーから外す`, which is more accurate than
"delete" and does not imply that historical records disappear.

But the broader policy is still incomplete:

- Can a removed person be added back?
- Should re-add create a new membership or reactivate the old one?
- Is there a temporary suspension state, or only removal?
- What should admins see about former members?
- How should attendance, notes, audit, and exports represent former members?

RFC-063 defines the future policy boundary for removal, re-add, and possible
suspension. It is intentionally separate from RFC-062 role transfer and RFC-024
account relinking because it concerns membership lifecycle state, not role
power or identity recovery.

## 2. Current State

Existing behavior:

- active members have a `community_memberships` row;
- member removal sets `removed_at`;
- historical event data is preserved;
- removed members cannot access community data after server-side validation;
- invite redemption can create a new membership identity;
- the UI does not provide re-enable/reactivate;
- the UI does not expose a former-members list.

This is safe enough for a first removal flow, but the product needs a policy
before adding any "bring back", "disable", "suspend", or "restore" button.

## 3. Problem Statement

Admins need plain answers to operational questions:

- "I removed the wrong person. Can I undo it?"
- "Someone left and came back. Should they keep their old history?"
- "Someone needs a temporary pause. Is that removal?"
- "Can a removed person join again with a new invite?"
- "Will old notes and attendance still be visible?"

Without a policy, small UI additions can create inconsistent identity, privacy,
and audit behavior.

## 4. Goals

- Define whether the product supports removal only, suspension, re-add, or
  reactivation.
- Preserve community isolation.
- Preserve historical attendance, notes, and audit records.
- Avoid implying that removal deletes history.
- Keep the first implementation small and understandable.
- Keep admin wording calm and non-punitive.
- Avoid accidentally implementing account relinking.
- Make re-add behavior explicit before adding UI.

## 5. Non-Goals

- No account recovery or browser/session relinking.
- No role transfer or promotion.
- No public former-member directory.
- No hard deletion self-service.
- No legal data-erasure workflow.
- No automated moderation.
- No temporary ban appeals.
- No direct messaging or notification system.

## 6. Policy Options

### Option A: Removal Only, Re-add Creates a New Membership

Removed membership remains removed. If the person returns, an admin issues a new
invite and the person joins as a new membership.

Pros:

- simplest;
- matches current invite model;
- avoids accidental identity merging;
- no reactivation UI needed.

Cons:

- duplicate person identities may exist;
- old attendance and notes do not automatically attach to the new membership;
- admins may not understand why history is split.

### Option B: Removal Plus Admin Reactivation

Removed membership can be reactivated by an admin.

Pros:

- preserves a person's previous membership identity;
- useful for mistaken removals;
- easier historical continuity.

Cons:

- needs a former-member list;
- needs clear policy for removed person's browser/session access;
- can become account recovery if not carefully separated from RFC-024;
- stronger audit and confirmation requirements.

### Option C: Add Suspension State

Add a reversible state separate from removal, such as `suspended_at`.

Pros:

- matches temporary access-stop situations;
- copy can honestly say access is paused;
- re-enable is a natural counterpart.

Cons:

- schema change;
- more state transitions;
- more UI states;
- higher risk of confusing non-technical admins;
- needs clear treatment in event visibility and exports.

## 7. Recommended First Direction

For the next implementation slice, prefer **Option A** unless review finds a
strong pilot need for reactivation.

Reasoning:

- the current product already behaves like removal-only;
- RFC-061 copy now avoids implying reversibility;
- adding reactivation risks becoming hidden account recovery;
- RFC-024 should decide identity recovery before old membership reactivation is
  exposed.

Option A still needs documentation and UI clarity:

- removal preserves history;
- removed people lose access;
- returning people use a new invite;
- old and new memberships are not automatically merged.

## 8. External Behavior for Option A

Member removal confirmation:

```text
メンバーから外しますか？

このメンバーはイベントやメモを見ることができなくなります。
過去の参加状況やメモは残ります。

[メンバーから外す]
[やめる]
```

After removal:

- admin returns to member-management page;
- removed person no longer appears in active members list;
- old event records remain internally linked to the removed membership;
- if the person should return, admin creates a new invite code.

Do not show "restore", "undo", "pause", or "reactivate" controls unless this
RFC is revised to accept Option B or C.

## 9. Former Members

Option A does not require a former-members page.

A future former-members page may be useful for audit/support, but it should not
be added casually because it can expose names of people no longer in the
community. If added later, it must answer:

- who can see former members;
- what fields are shown;
- whether notes/attendance are summarized;
- whether re-add or relink actions are available;
- how long former-member data remains visible.

## 10. Re-add Behavior

Under Option A, re-add means invite again.

The returning person receives a normal invite code and joins through the normal
invite redemption flow. This creates a new active membership. The app must not
merge old and new memberships by display name.

If the project later wants "return as the same membership", that is either:

- Option B reactivation; or
- RFC-024 account relinking.

Those require separate design before implementation.

## 11. Data Model

Option A requires no schema change.

Existing `removed_at` remains the removal marker.

Do not add `suspended_at`, `reactivated_at`, or `removed_reason` in the first
slice unless this RFC is revised after review.

If Option B or C is accepted later, likely schema additions include:

- `reactivated_at`;
- `reactivated_by_membership_id`;
- `suspended_at`;
- `suspended_by_membership_id`;
- reason codes with a restricted enum, not free text.

## 12. Audit

Removal is already auditable and should remain so.

If reactivation or suspension is added later, audit action names should be
explicit:

```text
membership.removed
membership.reactivated
membership.suspended
membership.unsuspended
```

Audit metadata must not store private notes or arbitrary admin-entered free
text in the first slice.

## 13. Security and Privacy

- Removed members must not access community data.
- Removed members must not be returned by active-member queries.
- Historical data must remain scoped to the original community.
- Re-add through new invite must not merge identities based on display name.
- Former-member visibility must be explicitly designed before any UI shows it.
- Hard deletion remains an operator/legal process, not a normal admin action.

## 14. Relationship to RFC-024

RFC-024 is the authority for account relinking and lost-session recovery.

RFC-063 must not add "restore access to the same membership" as a small UI
patch, because that can become account recovery without the safeguards RFC-024
requires.

If RFC-024 accepts relink codes, RFC-063 should be revisited to decide whether
reactivation and relinking share UI or remain separate.

## 15. Acceptance Criteria for Option A

- Removal copy clearly says access ends and past records remain.
- Active members list excludes removed members.
- No re-enable, restore, undo, or suspension controls are shown.
- Re-add guidance, if documented, says to issue a new invite.
- Display-name matching does not merge old and new memberships.
- Release gates or tests preserve the removal-only policy.
- Browser smoke confirms removal confirmation copy at mobile width and 200%
  text scaling.

## 16. Test Plan

- Source gates:
  - removal copy uses `メンバーから外す`;
  - confirmation copy states past records remain;
  - no restore/reactivate/suspend links are present.
- Existing removal tests:
  - member is soft-removed;
  - removed member loses access;
  - last-admin guard remains.
- Join/re-add tests:
  - a removed person using a new invite creates a new membership;
  - old membership is not reactivated by display-name match.
- Browser smoke:
  - remove confirmation copy fits at mobile width and 200% text;
  - removed member disappears from active member list;
  - no restore/suspend controls appear.

## 17. Open Questions

- Is Option A acceptable for pilot, or does mistaken removal require
  reactivation before launch?
- Should the docs explicitly say "invite again" for returning former members?
- Should former-member visibility be completely deferred, or should admins get
  a minimal audit-only view later?
- Should removal reason codes be added, or would they create unnecessary
  sensitivity for small communities?
