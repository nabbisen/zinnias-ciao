# RFC 051 — Multi-Day and Recurring Event Edit Semantics

**Status.** Implemented (v0.44.0)
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** 2026-07-05
**Shipped in:** v0.44.0
**Relationship:** Clarifies RFC-002 (EventDay grain), RFC-022 (recurrence),
RFC-040 (single-day event edit contract), and RFC-051 design review
`.git-exclude/reviewed/zinnias-ciao-v0.44.0-rfc051-event-edit-semantics-review.md`.

## 1. Summary

v0.44.0 adopts Option A+ from the RFC-051 design review:

- single-day non-recurring events support schedule edits before the event starts;
- multi-day and recurring events support details-only edits;
- details-only edits can change title, location, and description;
- details-only edits cannot change date, time, recurrence rule, or occurrence
  count;
- details-only edit screens show a read-only schedule summary and helper copy;
- cancellation remains whole-event only.

The goal is to prevent admins from believing that ignored date/time fields move
all occurrences. Member attendance rows remain attached to existing
`event_day_id` rows unless the event is cancelled and recreated.

## 2. Event Shape Definitions

### 2.1 Schedule-Editable Event

An event is schedule-editable only when:

```text
event_days.len() == 1
and repeat_rule == "none"
and repeat_count is NULL
```

For schedule-editable events, the edit form renders:

- title;
- location;
- description;
- date;
- start time;
- end time.

Saving updates the event row and the single `event_days` row.

### 2.2 Details-Only Event

An event is details-only for edit when it is multi-day or recurring:

- more than one `event_days` row; or
- a recurrence rule other than `none`; or
- a non-null recurrence count.

For details-only events, the edit form renders:

- a read-only schedule summary;
- helper copy;
- title;
- location;
- description.

It does not render editable `day_date`, `starts_at`, `ends_at`, `repeat_rule`,
or `repeat_count` controls.

## 3. External Behavior

### 3.1 Single-Day Edit

- Available only while all event days are upcoming.
- Shows existing title, location, description, date, start time, and end time.
- Saving updates the single `event_days` row.
- Event Detail, Home, Calendar, and ICS feed reflect the updated time through
  their existing event-day queries.

### 3.2 Multi-Day Edit

- Available only while all event days are upcoming.
- Shows title, location, and description as editable fields.
- Shows a read-only schedule summary.
- Saving does not update `event_days`.
- Existing attendance rows remain attached to their existing days.

### 3.3 Recurring Edit

- Uses the same details-only rule as multi-day edit.
- Recurrence controls are not rendered on the edit screen.
- Recurrence rule and occurrence count are not editable in v0.44.0.

### 3.4 Cancellation

Cancellation remains whole-event only:

- every event day is cancelled through the event status;
- per-day and per-occurrence cancellation are not supported;
- confirmation copy states whole-event scope for multi-day and recurring events.

## 4. Japanese Copy Contract

Details-only helper copy:

```text
このイベントは複数の日程があります。ここでは、タイトル・場所・説明だけを変更できます。日時を変える場合は、このイベントをキャンセルして、作り直してください。
```

Recurring helper copy:

```text
このイベントは繰り返しの予定です。ここでは、タイトル・場所・説明だけを変更できます。日時や回数を変える場合は、このイベントをキャンセルして、作り直してください。
```

Schedule summary heading:

```text
現在の日程
```

Editable details heading:

```text
変更できる内容
```

Preservation copy:

```text
日時は変わらず、参加の回答もそのまま残ります。
```

Whole-event cancellation copy:

```text
このイベントのすべての日程をキャンセルします。参加の回答も、これ以上変更できなくなります。
```

## 5. Server Enforcement

The server enforces the same split as the UI:

- schedule-editable events validate full event details plus date/time;
- details-only events validate title/location/description only;
- direct POST submissions containing schedule or recurrence fields for
  details-only events are rejected with plain copy;
- details-only writes pass `None` for event-day updates;
- audit metadata records whether the edit was `single_day_schedule` or
  `details_only`.

## 6. Data Safety Rationale

Attendance belongs to `event_day_id`. Moving days after members answer could
reinterpret their decisions without notification. The app does not yet have a
series exception model, per-occurrence notes, or notification delivery. For the
pilot, cancel-and-recreate is a safer schedule-correction path than silent
multi-day movement.

## 7. Non-Goals

v0.44.0 does not add:

- shift-all-occurrences edit;
- edit-this-occurrence behavior;
- this-and-future recurring edits;
- per-day cancellation;
- attendance transfer from a cancelled event to a replacement event;
- a new schema for event-series exceptions.

Those features require a later RFC after pilot feedback.

## 8. Acceptance Criteria

- Single-day non-recurring edit still renders date/time fields and updates the
  single `event_days` row.
- Multi-day edit does not render date/time or recurrence controls.
- Recurring edit does not render date/time or recurrence controls.
- Multi-day and recurring edit screens render a read-only schedule summary.
- Details-only POST validation does not require date/time fields.
- Direct details-only POST with schedule fields is rejected without changing
  `event_days`.
- Cancellation confirmation copy states all dates are cancelled for multi-day
  and recurring events.
- I18n parity includes all new EN/JA strings.
- Source gates and local browser smoke cover the workflow.

## 9. Test Plan

- `cargo fmt --all -- --check`
- `cargo test -p zinnias-ciao-contracts --test release_gates -- --nocapture`
- `cargo test -p zinnias-ciao-ssr`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown`
- `cargo build --workspace`
- `cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts -p zinnias-ciao-ssr`
- sandboxed/incognito browser smoke for single-day edit, multi-day edit,
  recurring edit, 200% text scaling, and whole-event cancellation.

## 10. Future Work

Future RFCs may consider:

- per-occurrence edit;
- per-occurrence cancellation;
- series exception data model;
- attendance transfer or copy affordances when cancelling and recreating;
- notification strategy for schedule changes.
