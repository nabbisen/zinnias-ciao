# RFC 041 — Atomic Invite Redemption

**Status.** Implemented (v0.23.0)
**Phase:** F7 / Stabilization (architect deep-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes deep-review finding P0-6. Refines RFC-003 (invite redemption and session authentication) and RFC-010 (admin invite management). Shares the conditional-UPDATE pattern with RFC-037.

---

## 1. Summary

Invite redemption was not safely one-time. `mark_used` was an unconditional
`UPDATE` with no affected-row check, and it ran *after* the user and membership
were created. Two profile submissions racing on the same invite could each
create a user and membership, and the later `mark_used` would overwrite the
record — two active members from one "one-time" invite. This RFC makes the
invite the single point of serialization: claim it atomically *first*, and
proceed only if the claim won.

---

## 2. Motivation

Deep-review P0-6. The redemption sequence in `post_profile` was:

1. insert user,
2. insert membership,
3. `UPDATE invite_codes SET used_at=…, used_by_membership_id=… WHERE id=…`,
4. insert session.

Two problems:

- **Step 3 had no guard.** It did not require `used_at IS NULL`, so a second
  redemption overwrote the first's `used_by_membership_id` and still "succeeded."
- **Ordering.** Because the claim came last, both racers completed steps 1–2
  (creating duplicate members) before either touched the invite.

D1 via worker-rs has no multi-statement transaction, so atomicity must come
from a conditional UPDATE plus an affected-row check — the same primitive
RFC-037 uses for form tokens. The invariant at stake is core (RFC-003 §"invite
can be redeemed at most once"; data-integrity requirement in requirements §13.4).

---

## 3. Goals

- Make invite consumption a **conditional state transition** that succeeds for
  exactly one caller.
- Claim the invite **before** creating user/membership/session; abort cleanly
  if the claim is lost.
- Return a `bool` from `mark_used` so the caller can enforce one-time use.
- Avoid duplicate members and overwritten `used_by_membership_id` under races.

---

## 4. Non-Goals

- No distributed lock or external coordination; the single-row conditional
  UPDATE on D1 suffices for 10–100-member communities.
- No change to invite generation, format, expiry, or revocation (RFC-010).
- No cleanup of orphaned `users` rows in this RFC (see §7 and §13).

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Normal redemption | Invite consumed once; one membership created; session issued; redirect to home. |
| Two submissions race the same valid invite | Exactly one becomes a member; the other is redirected to `/join` without creating a second membership. |
| Already-used / revoked / expired invite | Redemption fails; member is returned to `/join` (generic, no leak of which condition). |

User-facing copy stays generic ("Invalid or expired code.") per RFC-003 and
security requirements.

---

## 6. Internal Design

### 6.1 Conditional `mark_used`

```sql
UPDATE invite_codes
SET used_at = ?now, used_by_membership_id = ?mid
WHERE id = ?id
  AND used_at IS NULL
  AND revoked_at IS NULL
  AND expires_at > ?now
```

`mark_used` reads `D1Result::meta().changes` and returns `changed == 1`.

### 6.2 Reordered redemption

`post_profile` now:

1. look up `grants_role` from the invite (already validated in `post_join`);
2. generate `user_id` and `membership_id` up front (no DB write yet);
3. **claim the invite** via `mark_used`; if it returns `false`, redirect to
   `/join` and create nothing;
4. only on a winning claim: insert user, insert membership (with the role and
   `membership_id` already passed to `mark_used`), insert session;
5. audit "redeemed".

Because step 3 is the serialization point and uses the membership id we will
actually create, the winner's `used_by_membership_id` is correct and no loser
can create a second membership.

---

## 7. Data Model Notes

No schema change. `invite_codes` already has `used_at`, `revoked_at`,
`expires_at`, `used_by_membership_id`.

**Orphan rows:** if a later step (membership/session insert) fails *after* a
winning claim, the invite is marked used but no usable membership exists — the
member must request a new invite, and an orphaned `users` row may remain. This
is strictly better than the previous duplicate-member risk: an orphan `users`
row grants no access (access requires an active membership), and it is rare
(requires a mid-sequence D1 failure). A reaper for orphan users is deferred to
§13.

---

## 8. API and UI Contract Notes

- No endpoint shape change. The redemption remains the two-step
  `redeem` → `complete`/profile flow of external-design §13.4.
- The lost-race path reuses the existing redirect to `/join`; no new error code
  is surfaced to users.

---

## 9. Security, Privacy, and Safety

- **One-time guarantee** is now enforced by the database, not by handler
  ordering or the form token alone. (The form token still prevents same-browser
  double-submit; this defends against genuine concurrent redemption from two
  clients.)
- **No information leak:** a lost race and an invalid code both land on `/join`
  with the same generic outcome.
- **Audit:** redemption is audited only on the winning path, so audit counts
  match actual memberships.

---

## 10. Acceptance Criteria

1. An admin generates one invite; two concurrent redemptions create exactly one
   active member. (Pre-pilot gate #5.)
2. `mark_used` returns `false` for an already-used/revoked/expired invite and
   `true` exactly once for a valid one.
3. The redemption handler creates no user/membership when the claim is lost.

Item 2's logic ships in v0.23.0 (conditional UPDATE + affected-row check) and is
covered at the DB-wiring level by `cargo check`; the end-to-end race assertion
(item 1) is an integration test deferred to RFC-044.

---

## 11. Test Plan

- **Compile-level (shipped):** `mark_used` returns `bool`; caller branches on it.
- **Integration (deferred to RFC-044):** fire two concurrent `post_profile`
  submissions against a live D1 for one invite; assert one membership row and
  one `used_by_membership_id`. Requires the harness RFC-044 specifies.

---

## 12. Rollout Plan

Shipped in v0.23.0. No migration. Existing unused invites are unaffected; they
now redeem through the guarded path.

---

## 13. Open Decisions

- **Orphan-user reaping.** Whether to add a periodic cleanup (or a follow-up
  insert-failure compensation) for `users` rows left when a post-claim step
  fails. Low priority given the rarity and the no-access property; tracked for a
  future operations RFC.
- **D1 batch API.** If a future worker-rs binding exposes batched/atomic
  multi-statement execution, redemption could move user+membership+invite into
  one batch. Optional optimization, not required.
