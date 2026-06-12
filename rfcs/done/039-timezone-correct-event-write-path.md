# RFC 039 — Timezone-Correct Event Write Path

**Status.** Implemented (v0.23.0)
**Phase:** F7 / Stabilization (architect deep-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes deep-review finding P0-4. Completes the write-side counterpart to RFC-018 (time-zone and event cutoff policy), which had specified only the read/display side.

---

## 1. Summary

Admin-entered event times were stored as UTC without converting from the
community's local timezone. The display path (RFC-018) then *added* the
community offset, producing times wrong by the full offset. This RFC adds the
inverse conversion — `local → UTC` at write time — and wires it into event
create and edit, closing the loop with RFC-018's display logic.

---

## 2. Motivation

RFC-018 established that times are stored in UTC and rendered in the community's
configured timezone via `tz::to_local_parts(utc, offset)`. The write side was
never completed: handlers built timestamps as
`format!("{date}T{time}:00.000Z", …)` — stamping a `Z` (UTC) suffix onto a
*local* wall-clock time without subtracting the offset.

Concrete failure (deep-review P0-4): a Tokyo admin enters `09:00` intending JST.
The value `2026-06-14T09:00:00.000Z` is stored. On display, RFC-018 adds +9h,
showing `18:00`. The stored value is semantically false and the displayed value
is wrong. For a notice-board product whose entire purpose is "what time is the
event," this is a hard pilot blocker in any non-UTC community.

---

## 3. Goals

- Interpret admin-entered date/time as **community-local** wall-clock time.
- Convert to true UTC at write time, the exact inverse of RFC-018's display
  conversion, so a value written and then displayed round-trips to the entered
  time.
- Apply to both event creation and event editing.
- Handle day-wrap (a local time whose UTC equivalent falls on the previous or
  next day) and month/year boundaries.
- Fall back safely (no panic, no silent corruption) on unparseable input or
  unknown timezone.

---

## 4. Non-Goals

- **No DST.** The offset table (RFC-018) uses each zone's standard-time offset.
  DST-observing zones are off by an hour during summer time. This is a known,
  documented limitation; full DST handling is a future RFC-018 amendment. The
  MVP either targets non-DST communities or accepts the documented bound.
- No per-event timezone override; the community timezone governs all its events.
- No historical backfill of events created before v0.23.0 (see §12).

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Tokyo admin creates an event at 09:00 | Stored as `…T00:00:00.000Z`; displayed back as 09:00. |
| New York admin creates an event at 20:00 | Stored as next-day `…T01:00:00.000Z`; displayed back as 20:00. |
| UTC community | Stored time equals entered time. |
| Unknown timezone name | Falls back to UTC offset (no wrong conversion); event still saves. |
| Malformed date/time | Falls back to the literal value rather than panicking; validation should already have rejected it upstream. |

---

## 6. Internal Design

### 6.1 The inverse function

`contracts::tz::local_to_utc(date, time, offset_mins) -> String` mirrors
`to_local_parts`:

- `to_local_parts` computes `local = utc + offset` (with day-wrap).
- `local_to_utc` computes `utc = local − offset` (with day-wrap), then formats
  `"YYYY-MM-DDTHH:MM:00.000Z"`.

Day-wrap reuses the same `add_days` / `days_in_month` arithmetic, so both
directions share boundary logic. On unparseable input it returns
`"{date}T{time}:00.000Z"` (the previous behavior), degrading rather than
panicking.

### 6.2 Wiring

Both handlers resolve the community timezone via `community::find_active` →
`tz::offset_minutes(tz)`:

- **Create** (`post_create_event`): each expanded day's `starts_at`/`ends_at`
  is converted with `local_to_utc` before `event_write::create_event`.
- **Edit** (`post_edit_event`): for single-day events, the edited day's time is
  converted and persisted (see RFC-040); multi-day/recurring events edit
  details only.

The edit *form* prefills by converting the stored UTC back to local with
`to_local_parts`, so the admin sees and edits true local time.

### 6.3 Tests

`local_to_utc` is unit-tested in `contracts`:

- the architect's case `09:00 Asia/Tokyo → 2026-06-14T00:00:00.000Z`;
- backward day-wrap (`06:00 JST → previous-day 21:00Z`);
- forward day-wrap (`20:00 New_York → next-day 01:00Z`);
- UTC identity;
- round-trip with `to_local_parts`;
- month-boundary backward;
- malformed-input fallback.

---

## 7. Data Model Notes

No schema change. `event_days.starts_at_utc` / `ends_at_utc` continue to hold
ISO-8601 UTC strings. The fix changes only the *value* written, not the column
shape. Events created before v0.23.0 retain their (incorrect for non-UTC
communities) stored values; see §12.

---

## 8. API and UI Contract Notes

- The external API still exchanges UTC timestamps (external-design §6.3). The
  conversion is entirely server-side at the form boundary.
- The create/edit form continues to present plain date and time inputs; no
  timezone picker is shown to admins (the community timezone is implicit).

---

## 9. Security, Privacy, and Safety

- No security surface change. The conversion is pure arithmetic on already-
  authorized input.
- Safety improvement: correct times are themselves a trust/safety property for
  a scheduling product. A wrong time is a real-world harm (members miss events).

---

## 10. Acceptance Criteria

1. An admin in `Asia/Tokyo` creating `09:00` stores `00:00Z` and the event
   displays `09:00`. (Pre-pilot gate #6.)
2. Editing a single-day event's time persists and round-trips correctly.
3. `tz::local_to_utc` tests pass, including day-wrap and round-trip.
4. Unknown timezone does not corrupt or crash; it falls back to UTC offset.

All met in v0.23.0.

---

## 11. Test Plan

- **Unit (shipped):** the seven `local_to_utc` cases above.
- **Manual (pre-pilot gate):** create and edit events in a non-UTC community on
  a deployed host; confirm displayed times match entered times.

---

## 12. Rollout Plan

Shipped in v0.23.0. **Pre-existing events:** events created before this fix in a
non-UTC community hold mis-converted UTC values. Because the affected window is
the pre-pilot development period (no real community data yet), no migration is
provided. If any non-UTC events predate a pilot, the operator should recreate
them or run a one-off correction script; this is noted in the launch runbook.

---

## 13. Open Decisions

- **DST support.** Whether to add DST-aware conversion (requires a richer zone
  table or a `tz` data dependency, weighed against the Workers bundle-size and
  CPU budget). Tracked as a future RFC-018 amendment, not here.
