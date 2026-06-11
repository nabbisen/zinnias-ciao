# RFC 018 — Time-Zone and Event Cutoff Policy

**Status.** Proposed  
**Phase:** M6 / Deployment Readiness  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Reconciled:** time state and cutoffs are computed per event_day (Event→EventDay→Attendance grain).
**Related roadmap milestone:** M6 / Deployment Readiness  

---

## 1. Summary

This RFC defines how event times are stored, displayed, and used for status cutoffs. Time behavior must be deterministic because the app is calendar-centered.

---

## 2. Goals

- Store event times in UTC.
- Display event times according to community timezone policy.
- Define event state: upcoming, started, ended, cancelled.
- Define cutoff checks for member status changes and admin attendance correction.
- Avoid ambiguous local-time behavior.

---

## 3. Non-Goals

- No per-user travel timezone customization in MVP.
- No recurring event timezone rules.
- No calendar import/export.

---

## 4. External Behavior

Users see event times in the community's configured timezone, with simple labels such as Today, Tomorrow, This Week, and Past.

If status action is unavailable due to time, UI explains:

```text
This event has already started.
```

For admin attendance correction:

```text
Mark who attended
Available after the event ends.
```

---

## 5. Internal Design

Data:

- `communities.timezone` stores IANA timezone name.
- `event_days.starts_at_utc` / `event_days.ends_at_utc` store UTC instants (times live on days, not events).
- Server computes canonical event time state.
- Client may render friendly labels but must not be the only source of cutoff enforcement.

Per-day time state (event is cancelled if `events.status == cancelled`):

```text
upcoming if now < day.starts_at
started  if day.starts_at <= now < day.ends_at
ended    if now >= day.ends_at
```

Member status cutoff and admin Attended availability are evaluated per day.

---

## 6. Data and API Design

API DTO includes:

```json
{
  "starts_at_utc": "...",
  "ends_at_utc": "...",
  "display_timezone": "Asia/Tokyo",
  "time_state": "upcoming"
}
```

Mutating status endpoints should use server time for validation.

---

## 7. Security, Privacy, and Safety

- Do not trust client clock for permissions.
- Avoid leaking server internals in time validation errors.
- Logs should use UTC for operational consistency.
- Display strings should avoid ambiguity for users near midnight.

---

## 8. Acceptance Criteria

- Events are stored in UTC.
- Home grouping uses community timezone.
- Server rejects disallowed status transition based on server time.
- Admin attendance correction is available after event end.
- Cancelled state overrides normal time state.

---

## 9. Test Plan

- Unit tests for event time state.
- Tests around midnight in community timezone.
- Tests for invalid end-before-start event.
- Status cutoff tests using controlled server time.
- Manual UI review for Today/Tomorrow labels.

---

## 10. Open Questions / Decisions

Open decision: default community timezone at community creation. Recommendation: deployment default may be Asia/Tokyo for the current project context, but communities should store timezone explicitly.
