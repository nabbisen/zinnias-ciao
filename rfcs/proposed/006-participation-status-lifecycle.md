# RFC 006 — Participation Status Lifecycle

**Status.** Proposed
**Phase:** M2 / Member MVP Flow
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** AD-1/AD-4 (SSR form + form token, no optimistic UI), per-day grain, AD-3 (server-time cutoff).

---

## 1. Summary

Defines the lifecycle of attendance status — No answer, Going, No Go, Attended — **per event day**. Status is set by submitting a form; the server validates the transition against server time and persists, then re-renders. Admin sets final Attended after a day ends.

## 2. Goals

- Status values and transitions, per `event_day`.
- Preserve `No answer` as a real state.
- Server-authoritative cutoff (no client clock trust).
- Admin attendance correction after a day ends, audited.

## 3. Non-Goals

- No analytics, waitlists, maybe/tentative state, GPS proof, or optimistic client updates.

## 4. External Behavior

Per day, three buttons in a form: Going | No Go | Attended (icon + label + color). Before a day starts a member may set Going or No Go (and may clear to No answer if the clear control is shown). Attended is disabled before the day ends, with the reason shown ("Available after the event"), never hidden in a way that shifts layout. Submitting POSTs and 303-redirects back to the detail; the new state is what the server returns.

## 5. Internal Design

```rust
enum Status { NoAnswer, Going, NotGoing, Attended }   // DB NULL = NoAnswer

fn validate_status_transition(
    actor_role: Role,
    day_time_state: DayTimeState,   // upcoming | started | ended (server time, RFC-018)
    current: Status,
    requested: Status,
) -> Result<(), Rejection>
```

The member status form carries a `set_status` form token bound to `event_day_id` (AD-4). The handler: validate token (CSRF + single-use) → re-authorize membership (RFC-004) → `validate_status_transition` against server time → upsert the `(event_day_id, membership_id)` attendance row → 303. A consumed token replay returns the already-applied state, never a duplicate row. Admin attendance correction writes an audit record (actor, target, event_day, prev→new).

## 6. Data and API Design

```text
POST /c/:cid/events/:eid/days/:dayId/my-status      # body: token, status
POST /c/:cid/admin/events/:eid/days/:dayId/attendance # admin; token, [membership->status]
```

No client mutation_id; idempotency is the form token. Cutoff uses server time only.

## 7. Security, Privacy, and Safety

- Server, not the form, enforces transitions; a member cannot set another member's status.
- Admin override is community-scoped and audited.
- Token binding prevents replay across days/members.

## 8. Acceptance Criteria

- No answer renders distinctly per day.
- Member can set Going/No Go before a day's cutoff; disallowed transitions show a plain reason.
- Admin can mark Attended after a day ends.
- Per-day counts reflect the accepted change after reload.
- Double-submit (same token) does not create a duplicate attendance row.

## 9. Test Plan

- Unit: transition matrix × day time-state.
- Integration: member self-status set; forbidden cross-member set; admin override + audit.
- Token replay / double-submit idempotency test.
- Cutoff tests with controlled server time across a multi-day event.

## 10. Open Questions / Decisions

Decision: clearing to No answer is allowed before cutoff via an explicit small control (not by re-tapping the active status, to avoid accidental clears).
