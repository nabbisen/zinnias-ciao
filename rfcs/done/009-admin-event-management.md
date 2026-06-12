# RFC 009 — Admin Event Management

**Status.** Implemented (v0.4.0)
**Phase:** M4 / Admin MVP Flow
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** AD-1/AD-4 (SSR forms + token, no optimistic UI), Event→EventDay→Attendance grain.

---

## 1. Summary

Admin create / edit / cancel events and post-event attendance correction, all via server-rendered forms. An event has one or more days; the common case is one day. Tools stay simple enough for non-technical organizers.

## 2. Goals

- Create an event with one or more days (date + start/end each).
- Edit before the first day starts; preserve attendance on edit.
- Cancel (soft) rather than hard delete.
- Per-day attendance correction after a day ends; audited.

## 3. Non-Goals

- No recurring series (RFC-022 future), calendar import, polls, payments, or capacity. No optimistic client updates.

## 4. External Behavior

Create Event form: title, location (optional), description (optional), and one or more day rows (date, start, end), with an "add day" control (progressive enhancement adds a row client-side; with JS off the form supports a small fixed number of day rows and an "add day" submit). Cancel confirmation:

```text
Cancel this event?
Members will still see that it was cancelled.
[Keep Event] [Cancel Event]
```

Attendance correction is per day: a form listing active members with Attended / No Go / Clear, saved explicitly.

## 5. Internal Design

Server-side validation: title required/bounded; each day `ends_at > starts_at`; days ordered by `seq`; event community must be the admin's active community; cancelled events reject new member status; edits preserve existing attendance rows. Each admin form carries a purpose-bound form token (AD-4). Attendance correction writes audit rows (actor, target membership, event_day, prev→new). Create/edit/cancel write audit rows.

## 6. Data and API Design

```text
POST /c/:cid/admin/events                         # create event + its day(s)
POST /c/:cid/admin/events/:eid                    # edit (before first day start)
POST /c/:cid/admin/events/:eid/cancel             # soft cancel
GET  /c/:cid/admin/events/:eid/days/:dayId/attendance
POST /c/:cid/admin/events/:eid/days/:dayId/attendance
```

View models carry `capabilities` so disabled controls show a reason (RFC-013).

## 7. Security, Privacy, and Safety

- Only active admins reach admin routes (RFC-004); community-scoped.
- Confirmations on destructive/irreversible actions; cancellation auditable.
- Descriptions/titles/locations are escaped text.
- Hard deletion excluded from normal admin UI (RFC-019).

## 8. Acceptance Criteria

- Admin creates a valid one-day and a valid multi-day event.
- Invalid time range (any day) rejected.
- Member cannot reach admin event routes.
- Cancel requires confirmation; cancelled event renders muted to members.
- Per-day attendance correction works after a day ends and is audited.

## 9. Test Plan

- Day validation (single + multi-day) tests.
- Authorization (member denied) tests.
- Cancel state + audit tests.
- Per-day attendance correction + audit tests.
- Form-token presence/replay tests.

## 10. Open Questions / Decisions

Decision: cancellation (soft) is the normal "delete"; operator hard-deletion is outside admin UI. Editing days after the first day has started is restricted per RFC-018 cutoff.
