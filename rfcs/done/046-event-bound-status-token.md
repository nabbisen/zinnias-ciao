# RFC 046 — Event-Bound Status Token

**Status.** Implemented (v0.27.0)
**Phase:** F7 / Stabilization (handoff-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes handoff-review finding P1-1 (per-day `SET_STATUS` token issuance as the last hot-path scalability smell). Refines AD-4 (single-use form tokens) and RFC-029 (query performance discipline). Builds on the batched query work in RFC-044.

---

## 1. Summary

Event Detail previously issued one `SET_STATUS` form token **per event day**,
inside the day-render loop. For a recurring event with up to 52 occurrences this
produced up to 52 `form_tokens` row inserts during a single page render — the
last loop-based D1 write in the hot path.

This RFC binds a **single** `SET_STATUS` token to the **event** instead of the
day. The token is issued once before the day loop and reused by every day's
status form on the page. The POST handler continues to identify the specific day
from the URL path and validates day-and-event-and-community ownership before
mutating, so the security properties of AD-4 are preserved.

## 2. Motivation

The per-day token issuance was a write amplification proportional to event
recurrence count. While each individual write is small, 52 sequential D1 inserts
during one GET render is both a latency cost and a needless consumption of the
Workers subrequest budget. The architect handoff review (P1-1) flagged this as
the last remaining hot-path scalability smell and recommended fixing it before
beta.

## 3. Goals

- Issue exactly one `SET_STATUS` token per Event Detail render, regardless of the
  number of days.
- Preserve all AD-4 guarantees: CSRF protection, single-use idempotency,
  session-binding, purpose-binding, 1-hour TTL.
- Preserve day-level authorization: a submitted day must belong to the bound
  event, which must belong to the current community.

## 4. Non-Goals

- Changing the URL scheme. The status form action remains
  `/c/:cid/events/:eid/days/:did/my-status`; the day is identified by the path.
- Changing the attendance data model or the status transition rules (RFC-006).
- Allowing a single token to mutate multiple days in one request. The token is
  single-use; one submit changes one day, exactly as before.

## 5. External Behavior

No visible change for users. Each day still shows its own status control. A user
who opens Event Detail, changes status on day 1, then on day 2, behaves exactly
as before — except the second change now requires a fresh page load (since the
single token is consumed by the first change). This matches the existing AD-4
single-use behavior; the prior per-day tokens were independently single-use, so
in practice users already reload between mutations because the page re-renders on
the 303 redirect after each submit.

## 6. Internal Design

### Issue (GET `get_event_detail`)

The token is issued once, before the day loop, bound to `event_id`:

```rust
let status_token = form_token::issue(
    &db, &pp, &auth.user_id,
    token_purpose::SET_STATUS, Some(event_id),
).await.unwrap_or_default();
```

The same `status_token` value is passed to `render::status_form` for every day.

### Consume (POST `post_my_status`)

The token is consumed bound to `event_id` (not `day_id`):

```rust
let replay = form_token::consume(
    &db, &pp, &auth.user_id,
    token_purpose::SET_STATUS, &raw_token, Some(event_id),
).await?;
```

### Day-ownership validation (unchanged, already present)

After consuming the token, the handler validates the day chain before any
mutation:

1. `require_membership(env, auth, community_id)` — entry gate (community scope).
2. `days_for_event(event_id)` then `days.iter().find(|d| d.id == day_id)` — the
   submitted day must belong to the bound event, else generic not-found.
3. `find_for_community(event_id, community_id)` — the event must belong to the
   current community, else generic not-found.
4. `validate_status_transition(...)` — time/role/cancellation rules (RFC-006).
5. Only then `attendance_db::upsert(...)`.

This satisfies the architect's design rule: token bound resource = `event_id`;
day identified separately; handler rejects mismatches with generic not-found.

## 7. Data Model Notes

No schema change. The `form_tokens.bound_resource` column now stores `event_id`
for `SET_STATUS` tokens instead of `event_day_id`. No migration is needed because
the column is an opaque string.

## 8. API and UI Contract Notes

The `render::status_form` signature is unchanged — it still takes a `day_id` for
the form action URL and a `token`. The caller now passes the same token to every
invocation.

## 9. Security, Privacy, and Safety

The single event-bound token is at least as safe as the prior per-day tokens:

- **CSRF:** unchanged — a forged cross-site POST still lacks a valid token.
- **Idempotency:** unchanged — the token is single-use; a double-submit of the
  same token yields exactly one mutation and a deterministic replay redirect.
- **Authorization:** strengthened in clarity — the day-ownership check is now the
  explicit guard rather than being implied by the token binding. The check was
  already present; this RFC makes it the load-bearing control.

The only behavioral difference: with per-day tokens, a user could (in principle,
with two browser tabs opened from one render) hold two distinct unconsumed tokens
for two different days simultaneously. With one event-bound token, the first
submit consumes it and the second tab's submit becomes a replay (redirect, no
mutation). This is strictly safer and matches user expectation (reload to act
again).

## 10. Acceptance Criteria

- Event Detail issues exactly one `SET_STATUS` token regardless of day count.
- A status submit on any day succeeds with the event-bound token.
- A submit for a `day_id` that does not belong to the event returns generic
  not-found and does not mutate.
- A replayed token yields a redirect with no second mutation.
- Query budget for max-recurring Event Detail drops (token issues: 52 → 1).

## 11. Test Plan

- Unit: existing `status` transition tests unchanged (RFC-006).
- Unit: token uniqueness and `classify_token_consume` exhaustion unchanged.
- Compile: `cargo check --target wasm32-unknown-unknown`, zero warnings.
- Deferred to RFC-045/049 live-D1 harness: concurrent double-submit of one
  event-bound token across two days must yield exactly one mutation.

## 12. Rollout Plan

Shipped in v0.27.0. No migration. No operator action. Backward compatible: any
in-flight per-day tokens from a prior render simply fail to match the new
event-bound consume and the user reloads — acceptable for a token with a
1-hour TTL during a deploy.

## 13. Open Decisions

None. The day-ownership validation already existed; this RFC repurposes the token
binding and documents the guard as the authoritative control.
