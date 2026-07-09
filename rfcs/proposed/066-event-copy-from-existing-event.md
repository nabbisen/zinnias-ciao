# RFC 066 - Event Copy From Existing Event

**Status.** Proposed  
**Target release.** v0.55.0 candidate  
**Tracks.** Admin event workflow, event creation convenience, Calendar
workflow.  
**Touches.** `workers/ssr/src/handlers/admin/events`, Event Detail rendering,
Create Event form prefill, audit policy, release gates, browser smoke.

## Summary

Admins often create events that are similar to a previous event: same title,
place, description, time, or recurrence pattern, with only a new date or small
text change. Today there are two partial tools:

- event templates, which require the admin to have saved a reusable template;
- RFC-060 cancelled-event recreate, which copies only title, location, and
  description from a cancelled source event.

RFC-066 proposes an explicit "copy this event" workflow for active community
admins. The source event can be scheduled, completed/past, or cancelled. The
copy flow opens the normal Create Event form with selected source fields
prefilled. It never copies attendance, notes, cancellation state, occurrence
exceptions, audit history, invite/session/form-token material, or `event_day_id`
identity.

This is not a one-click duplicate. The admin must review the prefilled form and
submit a new normal event.

## Background

RFC-051 intentionally limited edits for multi-day and recurring events because
changing schedule rows after members answer can reinterpret attendance. RFC-060
then added a safer assistance path for cancelled events: create a similar event
from the cancelled source, copying only title, location, and description while
requiring the admin to choose a new schedule.

That solved correction after cancellation, but it does not cover ordinary admin
workflows such as:

- copying last month's event and changing the date;
- copying a successful past event for a future rerun;
- creating a second instance of a scheduled event without cancelling the first;
- copying a recurring event's visible details while intentionally choosing a new
  recurrence.

RFC-065 also changed recurrence storage: recurrence source-of-truth now lives in
`event_series`, while concrete `event_days` remain attendance anchors. Any copy
workflow must respect that split and avoid copying existing attendance anchors.

## Goals

- Let active community admins create a new event from an existing event in the
  same community.
- Reuse the normal Create Event validation, token, and write path.
- Make copied and not-copied data explicit in UI copy.
- Avoid one-click creation of duplicate active events.
- Preserve all attendance, note, and audit boundaries.
- Keep cross-community copying out of scope unless a later RFC designs it.
- Handle one-off, multi-day, recurring, scheduled, past, and cancelled sources
  without corrupting existing `event_days`.

## Non-Goals

RFC-066 does not add:

- one-click clone-and-save;
- copying attendance answers;
- copying member notes;
- copying occurrence exceptions;
- copying audit history;
- copying cancellation state;
- cross-community event copy;
- bulk event copy;
- event drafts;
- template management changes;
- recurrence edit semantics;
- copying a single occurrence as a separate event;
- "this and future" series editing.

## Terms

| Term | Meaning |
|------|---------|
| Source event | The existing event the admin chooses to copy from. |
| New event | The event created after the admin reviews and submits the Create Event form. |
| Copy prefill | Server-rendered values placed into the Create Event form before submission. |
| Schedule template | Date/time/repeat values copied as editable form inputs, not persisted until submit. |

## External Behavior

### Entry Point

Event Detail shows an admin-only action for same-community active admins:

```text
このイベントをコピー
```

English copy:

```text
Copy this event
```

The action is shown for:

- scheduled future events;
- already-started or past events;
- cancelled events.

The action is not shown to:

- non-admin members;
- anonymous users;
- admins of another community;
- removed memberships.

For cancelled events, this may coexist with the RFC-060 "create similar event"
entry point during transition. The implementation may unify both links to the
same copy form if the UX copy remains clear.

### Copy Form

The copy action opens a normal admin Create Event form scoped to the same
community.

Preferred route:

```text
GET /c/:community_id/admin/events/:event_id/copy
```

The page title should stay close to Create Event, with a short helper:

```text
内容をコピーして新しいイベントを作成します。参加の回答とメモはコピーされません。
```

English:

```text
Create a new event from this event. Attendance answers and memos are not copied.
```

The helper must be visible before the submit button on mobile.

### Prefill Rules

The copy form pre-fills safe event-level fields:

- title;
- location;
- description.

Schedule prefill is useful but riskier because a copied date can create an
accidental duplicate on the same day. RFC-066 therefore defines two design
options for review:

#### Option A: Details-only copy

Copy only title, location, and description. Leave date, start time, end time,
repeat frequency, repeat end mode, repeat count, and repeat until blank/default.

Pros:

- safest;
- nearly identical to RFC-060 semantics;
- low implementation risk.

Cons:

- weaker than "duplicate";
- admins still retype common time and recurrence settings.

#### Option B: Details plus editable schedule template

Copy title, location, description, and editable schedule inputs:

- for single-day one-off events: copy date, start time, and end time;
- for recurring events: copy the first occurrence date/time and recurrence
  settings into the Create Event form;
- for multi-day non-recurring events: see "Multi-day source events" below.

The submit button still creates a new normal event only after admin review.

Pros:

- closer to the admin meaning of duplicate;
- most useful for copying a past event forward;
- preserves the Create Event path and validation.

Cons:

- can accidentally create a second event on the same date if the admin does not
  adjust it;
- recurrence copy must respect RFC-065 materialization rules.

#### Recommended v0.55.0 behavior

Use Option B for single-day one-off events and recurring events, with explicit
warning copy near the date field:

```text
日付もコピーされています。必要に応じて変更してください。
```

Use Option A for multi-day non-recurring events unless the Create Event form is
extended to support multi-day input. This avoids pretending that a multi-day
event can be faithfully copied through the current single-day create form.

Recurring copy uses Option B only after the normalization rules in "Recurring
Source Events" below. Past or out-of-window recurring source dates must not be
prefilled into an immediately invalid Create Event form.

### Multi-day Source Events

Current Create Event UI is single base-day plus recurrence controls. It does not
have a general multi-day schedule editor. Therefore v0.55.0 should not silently
collapse a multi-day source into one copied day without explanation.

Recommended behavior:

- copy title, location, and description;
- leave date/time blank;
- show helper copy:

```text
複数日の予定です。日程は新しく選び直してください。
```

English:

```text
This source event has multiple dates. Choose the new schedule again.
```

If a later RFC adds a multi-day create/edit control, RFC-066 can be extended to
copy multi-day schedules into that control.

### Recurring Source Events

For recurring source events created under RFC-065:

- copy title, location, and description;
- copy the recurrence frequency;
- copy the source series local start/end times;
- prefill the base date and recurrence end controls only through the
  normalization rules below.

The admin can then change the base date or recurrence settings before submit.

For legacy migrated recurring events whose `event_series.starts_at_local` or
`ends_at_local` is null, do not guess local times from UTC. Fall back to
details-only copy and show plain helper copy that the schedule must be entered
again.

#### Recurring copy normalization

The copy form must compute community-local today using the same community
timezone basis as Create Event validation. It must also use the RFC-065
materialization window that Create Event uses for recurring starts.

Let:

- `source_base_date` be `event_series.start_day_date`;
- `today_local` be the community-local date at render time;
- `window_through` be the current RFC-065 materialization-window end date.

The copy form applies these rules before rendering:

1. If `source_base_date` is before `today_local`, do not prefill `day_date`.
   Keep copied title/location/description, recurrence frequency, and local
   start/end times. Reset recurrence end controls to the normal Create Event
   default and show helper copy that the source recurrence is in the past and a
   new start date must be chosen.
2. If `source_base_date` is after `window_through`, do not prefill `day_date`.
   Keep copied title/location/description, recurrence frequency, and local
   start/end times. Reset recurrence end controls to the normal Create Event
   default and show helper copy that the source recurrence starts outside the
   current create window and a new start date must be chosen.
3. If `source_base_date` is between `today_local` and `window_through`
   inclusive, prefill `day_date` from `source_base_date` and show the date
   warning copy.
4. When the base date is prefilled, copy recurrence end controls as follows:
   - `open_ended`: copy as `open_ended`;
   - `after_count`: copy the occurrence count, still subject to normal Create
     Event validation and caps;
   - `until_date`: copy only when the source `until_day_date` is present and is
     on or after the copied base date. If it is missing or before the copied
     base date, reset recurrence end controls to the normal Create Event
     default and show schedule-unavailable helper copy.
5. When the base date is not prefilled, do not copy `until_day_date` or
   occurrence count. This avoids rendering an end date or count that looks
   attached to the old source schedule.

These rules intentionally allow past recurring events to be copied as useful
templates while preventing the form from opening in a state that the existing
Create Event POST will immediately reject. POST still runs normal RFC-065
recurrence validation; the prefill rules are a UX and safety contract, not a
validation substitute.

Do not copy:

- materialized occurrence rows as rows;
- `event_series.id`;
- `materialized_through_day_date`;
- occurrence exceptions;
- cancelled occurrence state.

The new event receives its own `event_series` row if the submitted form remains
recurring.

### Cancelled Source Events

Cancelled source events are eligible. The new event is scheduled unless the
admin separately cancels it after creation.

Do not copy:

- `events.status = 'cancelled'`;
- `cancelled_at`;
- `cancelled_by_membership_id`;
- cancelled occurrence state.

### Save Behavior

POST uses the existing Create Event endpoint:

```text
POST /c/:community_id/admin/events
```

The form includes a hidden source marker:

```text
copy_source_event_id=<source_event_id>
copy_mode=event_copy
```

The server must re-check the source event on POST:

- source event exists;
- source event belongs to the same community;
- current user is still an active admin of that community;
- source event is accessible through normal scoped event lookup.

The hidden source fields are provenance and validation aids only. They are not
authorization.

`copy_mode=event_copy` must use the broader RFC-066 source eligibility rules.
It must not reuse the RFC-060 cancelled-event-only predicate. During any
transition where RFC-060 cancelled-event recreate still posts
`copy_source_event_id`, the POST branch must distinguish RFC-060 provenance from
RFC-066 event-copy provenance explicitly.

Submitting creates a new normal event:

- new `events.id`;
- new `event_days.id` rows;
- new `event_series.id` if recurring;
- no attendance rows;
- no notes;
- no exception rows;
- no copied audit history.

After successful create, redirect to the new Event Detail page.

## Authorization

All copy routes require:

- authenticated session;
- active membership in the community;
- admin role in the community.

Cross-community copy is not allowed in v0.55.0. Direct URLs to inaccessible
source events return the same generic not-found behavior as other event routes.

Community switcher behavior from the copy form must not carry the source event
ID into another community. If the admin switches communities from the copy form,
the destination should be the normal Create Event page without source-copy
state.

## Data Model

No schema change is required for v0.55.0.

Copy prefill needs a scoped source aggregate/helper that loads only the data
needed for the form:

- event-level title, location, description, status, and recurrence compatibility
  fields;
- scoped event-day rows needed to determine single-day versus multi-day source
  shape;
- RFC-065 `event_series` metadata needed for recurring source normalization.

The source aggregate/helper must not load attendance rows, member notes, invite
state, session state, form-token state, or audit history.

Audit metadata on create must record safe source provenance:

```json
{
  "copy_source_event_id": "evt_...",
  "copy_mode": "event_copy"
}
```

Do not store:

- title/location/description copies in audit metadata;
- attendance values;
- note bodies;
- event day IDs;
- event series IDs;
- exception IDs;
- token/session material.

If future analytics need copy lineage, that should be a separate RFC and schema
decision. Minimal audit provenance is enough for this release.

## UI Copy

Proposed Japanese strings:

| Key | Copy |
|-----|------|
| Event Detail action | `このイベントをコピー` |
| Copy form title | `イベントをコピーして作成` |
| Helper | `内容をコピーして新しいイベントを作成します。参加の回答とメモはコピーされません。` |
| Date warning | `日付もコピーされています。必要に応じて変更してください。` |
| Multi-day helper | `複数日の予定です。日程は新しく選び直してください。` |
| Schedule unavailable helper | `日程はコピーできません。新しく選び直してください。` |
| Recurring past helper | `繰り返しの開始日が過去のため、開始日を新しく選び直してください。` |
| Recurring window helper | `繰り返しの開始日が作成できる範囲外のため、開始日を新しく選び直してください。` |

Avoid UI wording that implies attendance or notes are transferred:

- 復元;
- 移行;
- 引き継ぐ, unless the sentence names only the copied fields;
- 参加データ;
- メモをコピー.

## Relationship to RFC-060

RFC-060 remains valid as cancelled-event recreate assistance. RFC-066 is broader:

| Concern | RFC-060 | RFC-066 |
|---------|---------|---------|
| Source state | Cancelled only | Scheduled, past, or cancelled |
| Source scope | Same community | Same community |
| Copied fields | Title/location/description | Details plus schedule template for supported source shapes |
| Attendance/notes | Never copied | Never copied |
| Purpose | Recover from cancellation workflow | Admin convenience duplication workflow |

The implementation may reuse route helpers and form rendering from RFC-060, but
the UX should not imply that an active source event is being cancelled or
replaced.

## Security and Privacy

- Copy routes must not reveal whether an event exists outside the current
  community.
- Copy form must not expose notes or participant attendance.
- Audit metadata must not include copied description text.
- Form tokens must use the existing `CREATE_EVENT` purpose or a narrowly scoped
  new purpose if review decides the copy route needs one.
- Server-side create validation must be identical to normal Create Event.
- A copied recurrence must go through RFC-065 recurrence validation and
  materialization limits.

## Acceptance Criteria

- Event Detail shows "copy this event" only to active admins.
- Copy form is same-community and admin-only.
- Copy form copies title, location, and description.
- Single-day source copy pre-fills editable date/time values if Option B is
  accepted.
- Recurring source copy pre-fills editable recurrence settings only according
  to the recurring copy normalization rules.
- Past recurring source copy opens a usable form without an invalid prefilled
  base date.
- Recurring `until_date` is not copied when it would be before the copied base
  date.
- Multi-day source copy does not silently collapse multiple dates into one
  copied date.
- Copy create does not copy attendance rows, notes, occurrence exceptions,
  cancellation state, audit history, or existing IDs.
- POST re-checks source event community and admin authorization.
- POST distinguishes RFC-066 `copy_mode=event_copy` from RFC-060 cancelled-event
  recreate provenance.
- Audit metadata records `copy_source_event_id` and `copy_mode` without copied
  descriptions, notes, attendance, or event-day IDs.
- Community switching from the copy form does not carry source-copy state across
  communities.
- Browser smoke covers admin visibility, non-admin absence, source field
  prefill, past recurring source copy, no attendance/note copy, and mobile 200%
  text scaling.

## Open Questions for Review

1. Should v0.55.0 implement the recommended hybrid behavior, or should the first
   release stay details-only for all source shapes?
2. Should active scheduled source events show stronger warning copy to avoid
   accidental same-date duplicates?
3. Should cancelled events keep the RFC-060 "similar event" action separately,
   or should it be replaced by the broader copy action?
4. Does audit provenance need a distinct action name, or is normal `event`
   `created` with required minimal metadata sufficient?
