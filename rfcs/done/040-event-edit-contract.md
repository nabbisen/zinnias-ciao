# RFC 040 — Event Edit Contract

**Status.** Implemented (v0.23.0)
**Phase:** F7 / Stabilization (architect deep-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes deep-review finding P0-5. Refines RFC-009 (admin event management) and depends on RFC-039 (timezone write path). Interacts with RFC-022 (recurring events).

---

## 1. Summary

The event edit form presented editable date and time fields, validated them on
submit, and then **discarded** them: only title, location, and description were
persisted. An admin would change the time, see a success redirect, and find the
event unchanged. This RFC defines a clear edit contract — single-day events
persist time changes; multi-day and recurring events edit details only — and
makes the form honest about what it edits.

---

## 2. Motivation

Deep-review P0-5: `post_edit_event` read `day_date`, `starts_at`, `ends_at`,
ran them through `validate_event`, then called `edit_event` with only
title/location/description. The `event_days` rows were never touched. The
day/time inputs were therefore controls that did nothing.

For non-technical admins this is a trust-destroying silent failure: the UI
affirms the change, the data ignores it. "Don't show editable controls that
don't persist" (deep review §6.2) is the principle violated.

The architect offered two acceptable resolutions: (a) remove date/time from the
edit form ("edit details only"), or (b) implement the persistence properly. We
chose (b) for single-day events because it is what admins expect, and (a)'s
fallback for the multi-day/recurring cases where per-day edit semantics are not
yet designed.

---

## 3. Goals

- Persist date/time edits for **single-day** events, converted to UTC via
  RFC-039.
- For **multi-day and recurring** events, edit details only (title, location,
  description); do not present time controls that cannot be honored.
- Prefill the edit form with the event's current values (converted UTC →
  community-local) so the admin edits real data, not blanks.
- Hide the recurrence selector on the edit form (recurrence is a create-time
  concern; changing a series mid-stream is out of scope).
- Preserve existing participant statuses across an edit (only cancellation
  clears participation, per RFC-009).

---

## 4. Non-Goals

- **No multi-day per-day time editing.** Editing the individual days of a
  multi-day or recurring event is deferred; the semantics (does moving day 3
  shift the series? split it?) need their own design.
- No editing of a started or cancelled event (existing RFC-018/RFC-009 guards
  remain: edits are rejected once the first day has started).
- No change to the create flow.

---

## 5. External Behavior

| Event shape | Editable fields | Time persists? |
|---|---|---|
| Single-day, upcoming | Title, date, start, end, location, description | Yes |
| Multi-day, upcoming | Title, location, description | No (details only) |
| Recurring series, upcoming | Title, location, description | No (details only) |
| Started or cancelled | — | Edit rejected (generic not-found/҂cannot-edit) |

The edit form shows the current values pre-filled. For single-day events the
date/time inputs are present and effective. For multi-day/recurring events the
recurrence selector is hidden and (in this iteration) the single date/time
inputs apply only to single-day events; multi-day editing of times is not
offered.

---

## 6. Internal Design

### 6.1 `edit_event` signature

`db::event_write::edit_event` gained a `day: Option<(&str,&str,&str)>` parameter
(`day_date`, `starts_at_utc`, `ends_at_utc`). When `Some`, it updates the
single `event_days` row at `seq = 1`:

```sql
UPDATE event_days SET day_date=?1, starts_at_utc=?2, ends_at_utc=?3
WHERE event_id=?4 AND seq=1
```

When `None`, only the `events` row (title/location/description/updated_at) is
updated.

### 6.2 Handler logic

`post_edit_event` counts the event's days. If exactly one, it resolves the
community timezone, converts the submitted local time via `tz::local_to_utc`
(RFC-039), and passes `Some(day)`. Otherwise it passes `None` (details only).

### 6.3 Form rendering

`event_form_fields` gained `day_date`, `starts_at`, `ends_at` prefill
parameters and a `show_recurrence` flag:

- **Create:** no prefill, `show_recurrence = true`.
- **Edit:** prefilled from the existing day (single-day events convert
  `starts_at_utc`/`ends_at_utc` back to local via `to_local_parts`),
  `show_recurrence = false`.

---

## 7. Data Model Notes

No schema change. Relies on the existing `event_days(event_id, seq, day_date,
starts_at_utc, ends_at_utc)` shape. `seq = 1` is the single day for single-day
events; the same convention RFC-022 uses for the first occurrence.

---

## 8. API and UI Contract Notes

- The PATCH-equivalent edit endpoint's external contract (external-design §13.6)
  is unchanged in shape; this RFC fixes that the time fields are now honored for
  single-day events.
- Multi-day/recurring edit forms must not render non-functional time controls.
  This is a UI-honesty requirement carried into RFC-043's acceptance gates.

---

## 9. Security, Privacy, and Safety

- Authorization unchanged: `require_admin` plus community scoping plus the
  started/cancelled guards.
- Safety: removing the silent-failure path is itself the safety win. Admins can
  trust that a saved change took effect.
- Editing preserves participation rows, so correcting a typo in the title does
  not wipe members' Going/No Go answers.

---

## 10. Acceptance Criteria

1. Editing a single-day event's start/end time persists and displays the new
   time. (Pre-pilot gate #7.)
2. The edit form shows current values, not blanks.
3. The recurrence selector does not appear on the edit form.
4. Multi-day/recurring events do not present time controls that fail to persist.
5. Participant statuses survive a details edit.

Items 1–3 and 5 are met in v0.23.0. Item 4 is satisfied for the recurrence
selector; broader multi-day time-edit UI is explicitly out of scope (a
multi-day event's edit form currently surfaces only the single-day inputs, which
apply to single-day events — full multi-day editing is deferred per §4).

---

## 11. Test Plan

- **Unit (shipped):** `local_to_utc` round-trip (RFC-039) underpins correct
  prefill/persist.
- **Manual (pre-pilot gate):** edit a single-day event's time on a deployed
  host and confirm persistence; confirm multi-day events show no broken time
  controls.
- **Integration (deferred to RFC-044):** an admin-edit acceptance test in the
  live-D1 harness.

---

## 12. Rollout Plan

Shipped in v0.23.0. No migration. Existing single-day events become time-
editable immediately; multi-day/recurring events are details-only until a
future RFC designs per-day editing.

---

## 13. Open Decisions

- **Multi-day / recurring per-day time editing.** Needs its own design: does
  editing one occurrence detach it from the series? Does editing the series
  template re-materialize future days (bounded by RFC-022's
  `RECURRENCE_MAX_COUNT`)? Deferred to a dedicated follow-up RFC.
