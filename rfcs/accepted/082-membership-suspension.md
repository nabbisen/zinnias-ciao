# RFC 082 - Membership Suspension

**Status.** Accepted — owner-accepted 2026-08-10. Amends **RFC-063** (Member
Removal, Re-add, and Suspension Policy), which asked whether a suspension state
should exist and deferred the answer. **Acceptance settles the policy; it confers
no implementation authority** — an implementation handoff is separate, see §10.

**Target release.** None. Design only.

**Tracks.** Membership lifecycle, authorization. Amends RFC-063. Independent of
the external-identity track (RFC-080/081) and may proceed in parallel with its
remaining slices.

**Touches.** `community_memberships` (additive column), every query that decides
membership activeness, admin member management, audit inventory, member-facing
copy.

---

## Summary

Give membership a **reversible** state alongside its terminal one.

- `suspended_at` — reversible. The row stays, access stops, history attribution
  is untouched.
- `removed_at` — terminal. Unchanged.

RFC-063 listed the questions this answers and explicitly left them open:
*"Is there a temporary suspension state, or only removal?"*, *"I removed the wrong
person. Can I undo it?"*, *"Someone needs a temporary pause. Is that removal?"*
Its accepted direction — **removal only** — codified existing behaviour rather
than deciding the policy, and said the product *"needs a policy before adding any
'bring back', 'disable', 'suspend', or 'restore' button."* This is that policy.

## Why now

Two reasons, and the second is the load-bearing one.

**Volunteer admins have no undo.** Removal is the only lever, it is terminal, and
an accidental removal is currently unrecoverable as a relationship — the person
must be re-invited as a new membership, losing their history in that community.

**More importantly: the absence of a reversible state creates pressure to corrupt
the terminal one.** During Slice 3 of the external-identity track, a D1 constraint
made the clean re-entry model expensive, and the tempting alternative was to make
`removed_at` reversible — un-removing a membership on return. That option is
permanently lossy: once a row has been un-removed, nothing can reconstruct which
stint an attendance belonged to, or whether someone was a member on a given date.

Suspension supplies the reversibility the product actually needs **without**
turning a terminal state into a toggle. It does not resolve the re-entry
question; it removes most of the reason to answer it badly.

## Goals

- A reversible pause that is visibly distinct from removal.
- No weakening of authorization: a suspended member is denied exactly as
  thoroughly as a removed one.
- No ambiguity introduced into history: suspension never rewrites the past.
- No new migration risk: additive only.

## Non-goals

- Re-entry after removal — that stays RFC-081 §1's question.
- Automatic or timed expiry of a suspension. Manual, explicit, reversible only.
- Suspension at account level. This is per-membership, like every other
  authorization fact.
- Role changes as part of suspension. Role is preserved and restored unchanged.
- Any new role granularity.

## 1. The state model

```
active            removed_at IS NULL AND suspended_at IS NULL
suspended         removed_at IS NULL AND suspended_at IS NOT NULL
removed           removed_at IS NOT NULL                        (terminal)
```

Transitions:

| From | To | Who |
|---|---|---|
| active | suspended | community admin |
| suspended | active | community admin |
| active | removed | community admin |
| suspended | removed | community admin |
| removed | anything | **nobody** — terminal |

A removed membership is never suspended, unsuspended, or restored. If
`removed_at IS NOT NULL`, `suspended_at` is meaningless and must be ignored
rather than interpreted.

## 2. Schema

```sql
ALTER TABLE community_memberships ADD COLUMN suspended_at TEXT;
ALTER TABLE community_memberships ADD COLUMN suspended_by_membership_id TEXT
    REFERENCES community_memberships(id);
```

Both nullable, both additive. **No table rebuild** — this is deliberately
implementable under D1's constraint that a referenced table cannot be dropped,
which is what blocked RFC-081 §1's partial index.

`suspended_by_membership_id` records which admin acted, consistent with
`created_by_membership_id` elsewhere in the schema. It is cleared on unsuspend
along with `suspended_at`.

### Interaction with RFC-081 §1

If RFC-081 §1's partial unique index is eventually adopted
(`… WHERE removed_at IS NULL`), suspension is consistent with it: a suspended row
is *not removed*, so it still occupies the pair. That is correct — a person cannot
simultaneously hold a suspended and an active membership in one community.

## 3. The safety problem, and the structural answer

**52 call sites across 9 files currently decide activeness with the literal
`removed_at IS NULL`.** Measured at `8d9ba2e`:

```
db/membership.rs 20   db/event_write.rs 8   db/relink.rs 7   handlers/me.rs 4
db/invite.rs 4        db/calendar.rs 3      db/attendance.rs 3
db/event_template.rs 2  db/event_note.rs 1
```

Adding a column without touching all of them means **suspended members keep
access**. That is a fail-open, and "remember to update 52 places" is not a
control.

It is also not a blanket substitution, which is why this needs a rule rather than
a find-and-replace. The 52 sites ask two genuinely different questions:

- **"May this actor do something?"** — must exclude suspended.
- **"Is this person present in the community?"** — must *include* suspended, or an
  admin cannot see them in order to unsuspend them, and an unsuspend action
  cannot find its own target.

### The rule

Two named predicates, defined once:

```rust
/// Authorization. Fail-closed default: if you are unsure which to use, use this.
const MEMBERSHIP_ACTIVE: &str = "removed_at IS NULL AND suspended_at IS NULL";

/// Presence. For listing and for targeting an admin action at a suspended member.
const MEMBERSHIP_PRESENT: &str = "removed_at IS NULL";
```

Every query interpolates one of the two. **No query spells the predicate
inline.**

A release gate asserts that no SQL string under `workers/ssr/src` contains
`removed_at IS NULL` outside these two constants — the same default-fail shape as
the session-minting and fixture gates, and it must be proven firing in both
directions.

**The implementation package must classify all 52 sites with evidence, one at a
time.** It must not inherit a classification from this RFC — this document
deliberately does not enumerate them, because six enumerations written from grep
output in this project's recent history have been wrong, and a wrong one here is
a silent authorization failure rather than a wrong count.

## 4. What a suspended member experiences

**Owner decision required.** Two defensible answers:

- **Explicit** — a plain-Japanese "access is paused; contact your community
  administrator" page for that community. Honest, and it stops the volunteer
  admin fielding "the app is broken" messages.
- **Indistinguishable** — the same not-found as a non-member, consistent with the
  enumeration-hiding rules in RFC-081.

**Recommendation: explicit.** The non-disclosure rules exist to stop one person
learning about *another* account or community. Suspension is not a secret from
the person suspended, and hiding it converts a clear situation into a support
burden on exactly the volunteers this product exists to help.

The member's other communities are unaffected and must remain reachable —
suspension is per-membership.

## 5. What an admin sees

A suspended member appears in the member list, marked as suspended, with an
unsuspend action. This is the reason `MEMBERSHIP_PRESENT` exists.

Removed members remain absent from the list; RFC-063 chose not to expose a
former-members list and this RFC does not revisit that.

## 6. Audit

Class A candidates: membership suspended, membership unsuspended. Both record the
acting admin.

Extend `AuditAction` and its pinned `ALL` count. Standard prohibitions carry
over: no session IDs, no raw identifiers, no user-controlled strings in metadata.

## 7. Sessions

Suspension needs **no session revocation**. Sessions carry a principal, not an
authorization; a suspended membership simply fails the authorization check on
every request. This falls out of the separation Slice 1 established, and is worth
stating so nobody adds a revocation sweep that is not needed.

## 8. Acceptance criteria

1. `suspended_at` and `suspended_by_membership_id` added additively; no table
   rebuild.
2. The two predicates exist as single definitions; no query spells either inline.
3. All 52 sites classified individually, with evidence, in the implementation
   package.
4. The gate exists, is default-fail, and is proven firing.
5. A suspended member is denied authorization exactly as a removed one is, proven
   by test **and** by a browser smoke.
6. A suspended member is visible and targetable by an admin, proven the same way.
7. A suspended member's other communities remain reachable.
8. Removed memberships are unaffected by every suspension path.
9. Audit actions added; `AuditAction::ALL` updated.
10. No role change on suspend or unsuspend.

## 9. Security considerations

**The fail-open is the whole risk.** A missed site means a suspended member keeps
an access path — and the ones most likely to be missed are the least-trafficked,
which are also the ones nobody would notice. The gate is the control; the
per-site classification is the evidence.

**Default fail-closed.** Where a site's intent is unclear, it takes
`MEMBERSHIP_ACTIVE`. A wrongly-denied admin listing is a visible bug; a wrongly-
allowed suspended member is not.

**Suspension must not become a soft delete.** It is reversible by design, so it
must never be used to satisfy a retention or erasure obligation. Removal and the
RFC-019 retention path remain the only mechanisms for that.

## 10. What acceptance does not authorize

No implementation. Acceptance settles the policy RFC-063 deferred; an
implementation handoff is separate. B1, B3, B4, and B5 remain open; production,
public-pilot, and first-real-community deployment remain **No-Go**.

## 11. Owner decisions carried

**Resolved by acceptance, 2026-08-10.** §4 was drafted with a stated
recommendation, and acceptance adopts it:

1. **§4 — a suspended member sees an explicit "access is paused; contact your
   community administrator" page**, not an indistinguishable not-found. The
   non-disclosure rules exist to stop one person learning about *another*
   account or community; suspension is not a secret from the person suspended,
   and hiding it converts a clear situation into a support burden on the
   volunteers this product exists to help.

**Still open — no default was drafted:**

2. **Sequencing: parallel with external-identity Slices 4–5, or after them.**
   This RFC is technically independent of that track — its schema change is
   additive and touches no identity code — so parallel is possible. The
   constraint is not technical but reviewing capacity: both tracks return
   packages to the same reviewer, and the suspension implementation is the more
   dangerous of the two to review while distracted, because its failure mode is
   a silent fail-open across 52 call sites.

   **Decided 2026-08-10: after.** The external-identity track finishes first
   (Slices 4 and 5), then suspension gets undivided review attention. Nothing
   here is urgent — RFC-063's gap has been open since v0.50.0, and the
   reversibility pressure that made it sharp came from a decision now resolved
   (RFC-081 §1.2a).
