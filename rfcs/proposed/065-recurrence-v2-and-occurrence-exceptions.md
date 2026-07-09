# RFC 065 - Recurrence v2 and Occurrence Exceptions

**Status.** Proposed  
**Target release.** v0.54.0  
**Tracks.** Calendar workflow, admin event creation, recurrence data model,
event-day attendance preservation.  
**Touches.** `migrations/`, `packages/domain`, `workers/ssr/src/db`,
`workers/ssr/src/handlers/admin/events`, `workers/ssr/src/handlers/communities`,
Calendar rendering, release gates.

## Summary

RFC-065 replaces the current bounded recurrence implementation with a
recurrence-series source of truth, rolling materialization, and occurrence
exceptions.

The current implementation stores recurrence metadata on `events`, expands a
fixed number of concrete `event_days` at creation time, and caps expansion at
52 occurrences. The Create Event form exposes this as a repeat count input with
an arbitrary default of 8. That model cannot safely express an open-ended
recurring event or "skip only this week's occurrence" without pretending that a
larger count is unlimited.

RFC-065 keeps the existing data grain that matters most:

- members answer attendance against stable `event_day_id` rows;
- existing `event_day` rows with attendance are never deleted or remapped;
- visible occurrences are still concrete `event_days`;
- open-ended recurrence creates only a bounded rolling horizon of future days.

The new source of truth is a recurrence-series record attached to an event.
Occurrence exceptions record skipped future dates and cancelled materialized
occurrences.

RFC-065 supersedes the recurrence semantics of RFC-022 while preserving
RFC-022's durable principle: members interact with concrete event instances,
not abstract recurrence rules.

## Background

RFC-022 shipped a first recurrence implementation in v0.17.0. The shipped
implementation is bounded materialization, not a full recurrence-series model:

- `migrations/0006_event_recurrence.sql` says `repeat_rule` and `repeat_count`
  are informational and actual days live in `event_days`;
- `packages/domain/src/event_admin.rs` defines `RECURRENCE_MAX_COUNT = 52`;
- `expand_recurrence(...)` materializes concrete `DayInput` rows and caps the
  count;
- the admin create form renders `repeat_count` with `value="8" min="1"
  max="52"`.

RFC-051 then made recurring events details-only on edit because moving days
after members answer could reinterpret those answers. RFC-060 made the safe
schedule-change path easier by helping admins cancel and recreate similar
events.

Those constraints remain valid. RFC-065 adds recurrence power only where the
data model can preserve existing attendance anchors.

## Goals

- Remove the arbitrary repeat-count default from the recurring event workflow.
- Support open-ended recurrence without inserting infinite or excessive
  `event_days` rows.
- Support occurrence exceptions:
  - model skip exceptions for future unmaterialized occurrences;
  - expose materialized occurrence cancellation while preserving any attendance
    rows.
- Preserve `event_day_id` as the attendance anchor.
- Migrate existing bounded recurring events without changing their visible
  occurrences.
- Keep recurrence queries bounded for Cloudflare Workers and D1.
- Provide a stable visible-month occurrence contract for the future RFC-066
  admin monthly attendance matrix.

## Non-Goals

RFC-065 does not add:

- drag/drop calendar editing;
- arbitrary RRULE import or full iCalendar recurrence compatibility;
- notifications for skipped or cancelled occurrences;
- per-occurrence title, location, description, or time edits;
- "this and future" edit semantics;
- attendance transfer between occurrences;
- per-occurrence notes;
- arbitrary future-date skip entry outside the materialized Calendar window;
- member-visible attendance matrix or CSV export;
- background cron as a hard dependency.

Future RFCs may add richer series editing after recurrence v2 is stable.

## Terms

| Term | Meaning |
|------|---------|
| Series | The recurrence rule and scheduling template for an event. |
| Occurrence | One dated instance of a series. A visible occurrence has an `event_days` row. |
| Materialization | Creating concrete `event_days` rows from a series rule for a bounded date horizon. |
| Skip exception | A date that matches the recurrence rule but should not produce an `event_days` row. |
| Cancelled occurrence | A materialized occurrence that should remain historically visible/preserved but no longer accepts attendance changes. |
| Horizon | The finite date range through which occurrences are materialized. |

## External Behavior

### Create Event

The default create-event state remains non-recurring.

If an admin chooses a recurrence frequency, the form asks for an explicit end
policy instead of silently defaulting to 8 occurrences:

- no end date;
- until a selected date;
- after a selected number of occurrences.

The first release should keep the existing frequencies:

- weekly;
- every two weeks;
- monthly.

The form must not submit an implicit `repeat_count=8`. If the admin chooses
"after a selected number of occurrences", the count is required and bounded.
If the admin chooses "no end date", the server stores an open-ended series and
materializes only the configured rolling horizon.

### Member View

Members continue to see concrete event dates.

Members do not see or interact with abstract recurrence rules when answering
attendance. A member status form remains bound to one `event_day_id`.

### Admin View

Admins can see that an event is recurring and can inspect the current
materialized occurrences.

For a materialized occurrence, admins can cancel that one occurrence without
cancelling the entire event series. The UI copy must distinguish:

- cancelling this date only;
- cancelling the whole event/series.

In v0.54.0, occurrence exception UI is limited to visible/materialized
occurrences. The admin action is "cancel this date only". Arbitrary future-date
skip entry is deferred because it needs a separate date-rule validation UI and
stronger operator expectations around the materialization window.

Open-ended recurrence is available to active community admins in v0.54.0. No
feature flag is required by this RFC.

### Calendar Page

Calendar month queries must see concrete `event_days` for the requested month
when the requested month is inside the recurrence materialization window.

Member-facing Calendar requests must not materialize arbitrary future months.
The global materialization window for v0.54.0 is from the community-local today
through the end of the month six calendar months ahead.

If a Calendar request is outside that window:

- it must not call the recurrence materializer;
- it may render existing one-off, multi-day, and already-materialized
  `event_days` rows;
- it must show a plain out-of-range note for recurring events rather than
  writing new future rows.

No v0.54.0 route gives ordinary members a way to extend the window through
repeated far-future month requests.

If an in-window member-facing Calendar request reaches the 64-insert
materialization cap, the page must not silently show a partial recurring month
as if it were complete. The implementation must either:

- avoid member-triggered writes and show only already-materialized rows with a
  plain note; or
- show a plain non-technical note that recurring dates are still being prepared
  and ask an admin/operator to retry or review.

Cancelled occurrences should be shown where the occurrence would otherwise
appear, with a subdued cancelled label and disabled attendance controls.

This makes the exception visible without deleting history or silently hiding a
date that members may have already seen.

## Data Model

### Series Table

Add a recurrence-series table. Names are illustrative; implementation may adjust
them if a clearer local naming pattern exists.

```sql
event_series (
  id TEXT PRIMARY KEY,
  event_id TEXT NOT NULL UNIQUE REFERENCES events(id),
  community_id TEXT NOT NULL REFERENCES communities(id),
  frequency TEXT NOT NULL CHECK(frequency IN ('weekly','biweekly','monthly')),
  start_day_date TEXT NOT NULL,
  starts_at_local TEXT,
  ends_at_local TEXT,
  timezone TEXT NOT NULL,
  end_mode TEXT NOT NULL CHECK(end_mode IN ('after_count','until_date','open_ended')),
  occurrence_count INTEGER,
  until_day_date TEXT,
  materialized_through_day_date TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

`events` remains the event-level record for title, description, location,
community, creator, and whole-event cancellation state. In v0.54.0, a recurring
event has one `events` row, one `event_series` row, and many `event_days` rows.

One-off and ordinary multi-day events do not need an `event_series` row.

For newly-created recurring events, `starts_at_local` and `ends_at_local` are
required application inputs and are written with the admin's local form values.
Legacy migrated series may store these fields as null when the original local
clock text cannot be safely recovered from existing UTC-only rows. Such legacy
series remain readable through their existing `event_days`, but must not
materialize additional future rows from guessed local times.

### Occurrence Status

Add a per-day occurrence status to `event_days`:

```sql
ALTER TABLE event_days
  ADD COLUMN occurrence_status TEXT NOT NULL DEFAULT 'scheduled'
  CHECK(occurrence_status IN ('scheduled','cancelled'));
```

This is intentionally separate from `events.status`, which remains whole-event
status.

Add recurrence identity columns to `event_days` for generated series
occurrences:

```sql
ALTER TABLE event_days ADD COLUMN series_id TEXT REFERENCES event_series(id);
ALTER TABLE event_days ADD COLUMN series_occurrence_date TEXT;

CREATE UNIQUE INDEX idx_event_days_series_occurrence
  ON event_days(series_id, series_occurrence_date)
  WHERE series_id IS NOT NULL;
```

Rules:

- `series_id` and `series_occurrence_date` are null for one-off and ordinary
  multi-day events.
- Generated recurring occurrences set both columns.
- `series_occurrence_date` is the occurrence's local date in the community
  timezone.
- The existing `(event_id, seq)` uniqueness remains.
- For generated series rows, `seq` is the recurrence ordinal from the series
  start, not "max existing seq plus one". This avoids fragile sequence
  allocation during concurrent materialization and preserves stable ordering.
- Because skip exceptions can create gaps, UI labels and tests must not assume
  generated recurring days are numbered contiguously from 1 to N. `seq` is for
  stable ordering, not for user-facing occurrence counts.

Materialization must use this database-enforced identity with insert-or-ignore,
upsert, or an equivalent transactional pattern. App-level "check then insert"
without a unique occurrence key is not acceptable.

If an occurrence already has a stable `event_day_id`, cancellation sets
`occurrence_status = 'cancelled'` instead of deleting the row. Attendance rows
remain attached to the same `event_day_id`.

Event notes remain event-level in v0.54.0, matching the current model.
Cancelling one occurrence does not delete or split notes. Per-occurrence notes
are a non-goal.

### Exception Table

Add an exception table:

```sql
event_series_exceptions (
  id TEXT PRIMARY KEY,
  series_id TEXT NOT NULL REFERENCES event_series(id),
  community_id TEXT NOT NULL REFERENCES communities(id),
  exception_day_date TEXT NOT NULL,
  action TEXT NOT NULL CHECK(action IN ('skip','cancel')),
  event_day_id TEXT REFERENCES event_days(id),
  created_by_membership_id TEXT NOT NULL REFERENCES community_memberships(id),
  created_at TEXT NOT NULL,
  CHECK (
    (action = 'skip' AND event_day_id IS NULL)
    OR (action = 'cancel' AND event_day_id IS NOT NULL)
  ),
  UNIQUE(series_id, exception_day_date)
);
```

Rules:

- `skip` means no `event_days` row should be materialized for that date.
  v0.54.0 stores and honors this shape but does not expose arbitrary
  future-date skip entry in the UI.
- `cancel` means an existing materialized `event_days` row is cancelled and
  preserved.
- `event_day_id` is required for `cancel` and null for `skip`.
- Exceptions are community-scoped for query safety and authorization checks.

## Migration

Existing data must remain stable.

### Existing one-off and multi-day events

Events with `repeat_rule = 'none'` and `repeat_count IS NULL` remain unchanged.
They do not receive an `event_series` row.

### Existing bounded recurring events

For each event with `repeat_rule != 'none'` or `repeat_count IS NOT NULL`:

1. Create one `event_series` row.
2. Set `frequency` from `repeat_rule`.
3. Set `end_mode = 'after_count'`.
4. Set `occurrence_count` from `repeat_count`.
5. Set `materialized_through_day_date` to the max existing
   `event_days.day_date`.
6. Backfill existing generated `event_days` rows with the new `series_id` and
   `series_occurrence_date`.
7. Leave existing `event_days.id` values and attendance rows unchanged.

The migration must not derive `starts_at_local` or `ends_at_local` by taking the
clock substring from `starts_at_utc` / `ends_at_utc`. That would store a UTC
clock as if it were community-local time. If the original local clock text is
not safely available, leave the legacy series local-time fields null and do not
materialize future rows for that legacy series.

For v0.54.0, the old `events.repeat_rule` and `events.repeat_count` columns
remain compatibility summary fields:

- new recurring writes set `events.repeat_rule` to the selected frequency;
- `events.repeat_count` is set only for `end_mode = 'after_count'`;
- `events.repeat_count` is null for `open_ended` and `until_date`;
- recurrence source-of-truth reads use `event_series`.

This lets existing helpers that only need a broad "is recurring" signal keep
working while new recurrence behavior moves to `event_series`-aware reads. A
later cleanup may deprecate the old columns after compatibility risk is gone.

### Invalid legacy shapes

If a legacy row has inconsistent recurrence metadata, the migration must not
invent a dangerous series. It should either:

- treat the event as non-recurring if `event_days` has only one row and the rule
  is absent/invalid; or
- fail loudly during migration with a documented operator action.

If legacy recurring rows cannot be backfilled into unique
`(series_id, series_occurrence_date)` identities, the migration must fail loudly
rather than generating duplicate or unstable occurrences.

The implementation review should verify this against actual migration code.

## Materialization Policy

Materialization must be bounded per request and bounded globally by a forward
window.

Recommended v0.54.0 policy:

- materialization window: community-local today through the end of the month six
  calendar months ahead;
- create recurring event: materialize from the start date through the smaller
  of the series end condition and the materialization window;
- Calendar month render: materialize only if the requested month intersects the
  materialization window;
- event detail/admin series page: ensure materialization through a bounded
  upcoming horizon inside the same window;
- each materialization run has a hard cap of 64 inserted rows;
- if the cap is reached, show a plain admin-visible warning rather than looping.

For new open-ended series, the first occurrence date must not be far enough in
the past that materialization spends the 64-row cap before reaching current or
upcoming visible dates. The implementation should reject such starts with plain
copy or clamp first materialization to the current window while documenting the
behavior in tests.

Materialization must:

- compute dates in the community timezone;
- skip dates present as `skip` exceptions;
- not duplicate existing `(series_id, series_occurrence_date)` rows;
- use recurrence ordinal as `event_days.seq`;
- update `materialized_through_day_date` only after successful inserts;
- remain idempotent when two requests materialize the same horizon;
- never materialize dates beyond the global materialization window from a
  member-facing Calendar request.

Admin-only explicit horizon extension beyond the global window is not part of
v0.54.0. If added later, it must have stricter caps, audit, and operator-facing
copy.

No v0.54.0 behavior should require Cloudflare Cron. Cron-based pre-generation
can be a later optimization.

## Exception Rules

### Skip a future unmaterialized occurrence

The data model supports skip exceptions for future unmaterialized occurrences,
but v0.54.0 does not expose arbitrary future-date skip entry in the UI.

If a future implementation accepts a target date that matches the series rule
and has no `event_days` row yet:

1. Insert a `skip` exception.
2. Future materialization does not create an occurrence for that date.
3. No attendance or notes are affected.

### Cancel a materialized occurrence

If the target occurrence already has an `event_days` row:

1. Insert a `cancel` exception linked to that `event_day_id`.
2. Set `event_days.occurrence_status = 'cancelled'`.
3. Preserve all attendance rows.
4. Prevent new member attendance changes for that occurrence.
5. Keep admin audit metadata for who cancelled the occurrence.

If the occurrence has no attendance, the implementation may still use the same
cancelled-row path for simplicity and auditability. Hard deletion of
materialized occurrences is not part of v0.54.0.

This is the only occurrence exception UI required for v0.54.0.

### Whole-event cancellation

Whole-event cancellation remains available and continues to use `events.status =
'cancelled'`.

Whole-event cancellation should supersede individual occurrence state for user
interaction: no occurrence in a cancelled event accepts attendance changes.

Status-transition callers must treat an occurrence as cancelled when either:

```text
events.status == "cancelled"
or event_days.occurrence_status == "cancelled"
```

The status validation path should receive this combined cancellation boolean or
an equivalent explicit cancellation state. Tests must cover both whole-event
and one-occurrence cancellation.

## Authorization and Audit

Only active admins of the event's community can create, update, or cancel
recurrence exceptions.

Each exception mutation should write an audit record without storing sensitive
or excessive metadata. Safe metadata examples:

```json
{
  "series_id": "ser_...",
  "event_id": "evt_...",
  "event_day_id": "day_...",
  "exception_day_date": "2026-08-15",
  "action": "cancel"
}
```

Do not log member notes, invite/session/form-token material, or raw submitted
form bodies.

## UI and Copy Direction

The create form should avoid the arbitrary repeat-count-first model.

Preferred Japanese copy direction:

| Concept | Direction |
|---------|-----------|
| No repeat | `繰り返しなし` |
| Weekly | `毎週` |
| Every two weeks | `隔週` |
| Monthly | `毎月` |
| No end date | `終了日を決めない` |
| Until date | `この日まで` |
| After count | `回数を指定` |
| Skip/cancel this date only | `この日だけ中止する` |

Final UI text should be reviewed for Japanese naturalness when implemented.

Important copy rule: avoid making "unlimited" sound like infinite work. The
user-facing concept is "no end date"; the system still creates only nearby
visible dates as needed.

## Matrix Dependency Contract

RFC-066 may rely on this contract:

- Calendar/month queries can request a visible month and receive concrete
  `event_days` rows for active series occurrences in that month when the month
  is inside the RFC-065 materialization window.
- Each visible occurrence has a stable `event_day_id`.
- Cancelled materialized occurrences remain identifiable and can be excluded or
  labelled by the matrix.
- Open-ended series do not require matrix queries to expand unbounded history or
  future dates. Future months outside the materialization window are out of
  scope for matrix export until materialized through an approved path.

RFC-066 should not reopen recurrence source-of-truth design unless RFC-065
implementation deviates from this contract.

## Query and Runtime Constraints

Recurrence v2 must stay within Cloudflare Worker and D1 constraints.

- No infinite expansion.
- No unbounded loops over future dates.
- No member x occurrence expansion during recurrence materialization.
- Calendar rendering may materialize only months inside the six-month forward
  materialization window.
- Materialization should use batched inserts where practical and must have a
  hard maximum of 64 inserts per request.
- Generated rows must have database-enforced occurrence identity through
  `event_days.series_id` and `event_days.series_occurrence_date`.
- D1 queries must remain community-scoped.

If materialization cannot complete within the cap, the admin should see a plain
message and the release gate should capture the behavior.

## Design Review Decisions Applied

The v0.54.0 design review required these changes before implementation, and
this revision adopts them:

- Member-facing Calendar requests are limited by a six-month forward
  materialization window and cannot write arbitrary future months.
- Generated series occurrences use database-enforced identity through
  `event_days.series_id` and `event_days.series_occurrence_date`.
- Status-transition callers must treat either whole-event cancellation or
  occurrence cancellation as cancellation.
- v0.54.0 exposes exception UI only for visible/materialized occurrences.
- New recurring writes keep `events.repeat_rule` and `events.repeat_count` as
  compatibility summary fields while `event_series` becomes the source of
  truth.

## Acceptance Criteria

- Existing one-off and multi-day events continue to render and edit as before.
- Existing bounded recurring events migrate to an `event_series` source of truth
  without changing existing `event_days` IDs, and with unique generated
  occurrence identity backfilled.
- New recurring events do not use an implicit repeat-count default of 8.
- Admins can create open-ended weekly, biweekly, and monthly series.
- Open-ended series materialize only within the six-month forward window.
- Repeated far-future Calendar month requests do not create recurring
  `event_days` outside the materialization window.
- In-window member-facing Calendar requests that hit the materialization cap do
  not silently present partial recurring output as complete.
- Concurrent or repeated materialization of the same horizon is idempotent by
  database uniqueness, not only by application checks.
- Recurrence `seq` gaps from skipped occurrences do not break ordering, labels,
  tests, or exports.
- Calendar month rendering shows materialized occurrences for selected months
  inside the window.
- New open-ended recurring events with past start dates are rejected, clamped,
  or otherwise handled so the materialization cap cannot hide current/upcoming
  occurrences.
- Admins can cancel a visible/materialized occurrence without cancelling the
  whole series.
- Cancelled materialized occurrences preserve attendance rows and stop new
  attendance changes.
- Status-transition validation treats either whole-event cancellation or
  occurrence cancellation as cancellation.
- Whole-event cancellation still works and remains distinct from one-occurrence
  cancellation.
- Community isolation and admin-only authorization cover series and exceptions.
- Audit records exist for occurrence exception mutations.
- Browser smoke covers create recurring event, open-ended recurrence, one-date
  exception, Calendar visibility, and mobile/200% text layout.

## Test Plan

- Domain tests:
  - recurrence date generation for weekly, biweekly, monthly;
  - month-end monthly clamping;
  - open-ended generation with bounded horizons;
  - skip exceptions;
  - recurrence ordinal gaps after skips;
  - materialization-window calculation;
  - idempotent materialization inputs.
- Migration tests or SQL verification:
  - legacy recurring event becomes an `event_series`;
  - existing `event_days.id` values are preserved;
  - legacy recurring event days receive unique `series_id` and
    `series_occurrence_date` identities;
  - duplicate generated occurrence identity is rejected or ignored safely;
  - one-off events do not gain series rows.
- SSR route/db tests:
  - admin-only series/exception mutations;
  - cross-community exception tampering is rejected;
  - far-future Calendar requests do not materialize recurring rows;
  - in-window materialization cap behavior is visible and not silently partial;
  - past-start open-ended creation is rejected, clamped, or documented by test;
  - cancelled occurrence blocks member status changes;
  - whole-event cancellation still blocks all days.
- Release/source gates:
  - `cargo fmt --all -- --check`;
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo build --workspace`;
  - `cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts -p zinnias-ciao-ssr`;
  - release gate/source-contract tests.
- Browser smoke:
  - create open-ended recurring event;
  - view Calendar month with generated occurrences;
  - cancel one occurrence;
  - verify the rest of the series remains visible;
  - request a far-future month and verify no write-driven materialization
    occurs;
  - verify mobile width and 200% text scaling.

## Rollout Plan

1. Add migrations and compatibility reads for current recurring events.
2. Implement recurrence-domain generation, materialization-window calculation,
   and idempotent materialization helpers.
3. Update create-event UI and server handling.
4. Add admin exception mutation path.
5. Update Calendar/event-detail rendering for per-occurrence status.
6. Run local source gates and browser smoke.
7. Request implementation review before release preparation.
