# RFC 060 — Cancel-and-Recreate Assistance

**Status.** Done  
**Phase:** F8 / Workflow improvements  
**Project:** ciao.zinnias  
**Date:** 2026-07-05  
**Shipped in:** v0.45.0  
**Relationship:** Follows RFC-051 multi-day and recurring event edit semantics,
RFC-032 event templates, RFC-040 single-day event edit contract, and RFC-009
admin event management.

## 1. Summary

RFC-051 made multi-day and recurring event edits intentionally details-only.
Admins can correct title, location, and description, but changing dates, times,
or recurrence requires cancelling the existing event and creating a replacement.

RFC-060 implements a small assistance flow for that recommended path:

- after an admin cancels an event, the cancelled event detail page offers
  `似た内容で新しいイベントを作成`;
- the replacement create form copies only safe event-level details from the
  cancelled source event;
- schedule fields are intentionally left for the admin to enter;
- attendance, notes, event days, recurrence metadata, and cancellation metadata
  are not copied;
- the new event is a normal event in the same community.

This keeps the data-safety boundary from RFC-051 while reducing repeated typing
for non-technical admins.

## 2. Goals

- Make the RFC-051 cancel-and-recreate path practical.
- Avoid hidden attendance transfer or schedule reinterpretation.
- Keep replacement creation admin-only and same-community scoped.
- Make copied vs not-copied data explicit in the UI.
- Preserve the existing Create Event validation and form-token path.
- Keep the feature understandable on mobile and at 200% text scaling.

## 3. Non-Goals

RFC-060 does not add:

- per-occurrence edit;
- per-day or per-occurrence cancellation;
- attendance transfer;
- note transfer;
- automatic cancellation plus replacement in one operation;
- a new event-series or replacement-event schema;
- notifications for the corrected schedule;
- cross-community event copying.

## 4. External Behavior

### 4.1 Entry Point

When an admin views any cancelled event in a community they administer, Event
Detail shows an action:

```text
似た内容で新しいイベントを作成
```

The action is not shown to non-admin members, active scheduled events, or events
outside the current community. It is available for single-day, multi-day, and
recurring cancelled events because it copies only title, location, and
description.

Helper copy on the replacement form:

```text
タイトル・場所・説明だけを引き継ぎます。日程はもう一度選びます。参加の回答とメモは引き継ぎません。
```

### 4.2 Replacement Create Form

The action opens an admin-only replacement create form for the same community.

The form is the normal Create Event form with these prefilled values:

- title;
- location;
- description.

The form does not prefill:

- date;
- start time;
- end time;
- repeat rule;
- repeat count.

The admin must choose the replacement schedule explicitly.

### 4.3 Save Behavior

Submitting the replacement form creates a normal new event:

- new event ID;
- new `event_days` rows from the submitted schedule/recurrence;
- no attendance rows copied from the source event;
- no member notes copied from the source event;
- no source cancellation state copied;
- created by the current admin membership.

After creation, the app redirects to the new Event Detail page.

### 4.4 Source Event State

v0.45.0 only offers the replacement action for already-cancelled source events.

This keeps the workflow explicit:

1. admin decides the old event should not remain active;
2. admin confirms cancellation;
3. admin creates a similar replacement with a new schedule.

The implementation may later add a pre-cancel shortcut from details-only edit,
but that should not silently create a duplicate active event in v0.45.0.

## 5. Proposed Routes

Preferred route:

```text
GET /c/:community_id/admin/events/:event_id/recreate
```

Behavior:

- requires authenticated active admin in `community_id`;
- fetches `event_id` through `find_for_community`;
- requires source event status `cancelled`;
- issues a normal `CREATE_EVENT` form token;
- renders the Create Event form with copied title/location/description;
- includes a short explanation of what is not copied.

POST can use the existing create endpoint:

```text
POST /c/:community_id/admin/events
```

If the form includes a `copy_source_event_id`, the server must re-check that the
source event is cancelled and belongs to the same community before using it for
audit metadata. The hidden field must not be trusted as authorization.
Invalid, cross-community, inaccessible, or active/non-cancelled source IDs are
rejected with the same generic not-found behavior as other event routes.

## 6. Data and Audit

No schema change is required for v0.45.0.

The replacement event is independent. If the implementation records provenance,
use safe audit metadata only:

```json
{
  "created_from_cancelled_event_id": "evt_..."
}
```

Do not copy or log:

- descriptions into audit metadata;
- member notes;
- attendance answers;
- invite/session/form-token material;
- raw hidden form data.

## 7. Safety Rules

- Source event must be same-community.
- User must be an active admin of that community.
- Source event must be cancelled.
- Copied fields must pass the same validation as normal event creation.
- Replacement schedule must be entered and validated through the normal create
  event path.
- Community switching from the replacement form must not carry a source event ID
  into another community.
- Direct URLs to inaccessible source events must return the same generic
  not-found behavior as other event routes.

## 8. UX Notes

The feature should feel like assistance, not recovery magic.

Use plain copy:

```text
似た内容で新しいイベントを作成
```

```text
タイトル・場所・説明だけを引き継ぎます。日程はもう一度選びます。参加の回答とメモは引き継ぎません。
```

Avoid terms such as:

- 複製;
- 復元;
- 移行;
- シリーズ;
- 参加データ.

The phrase `似た内容` is intentionally modest: it suggests reuse of visible
details, not a full clone.

## 9. Acceptance Criteria

- Cancelled event detail shows the replacement action only to active admins.
- Replacement form is same-community and admin-only.
- Replacement form copies title, location, and description.
- Replacement form does not copy date, time, repeat rule, or repeat count.
- Replacement create does not copy attendance rows or notes.
- Replacement create uses the normal create validation and token path.
- Audit metadata, if added, stores only safe source event ID metadata.
- Community switching does not leak or reuse `copy_source_event_id` across
  communities.
- Browser smoke covers mobile width, 200% text scaling, and non-admin absence.

## 10. Test Plan

- Unit/source gates for copied vs not-copied fields.
- Route/auth tests or source gates for admin-only same-community source lookup.
- Create-path test that tampered `copy_source_event_id` is rejected
  unless same-community and cancelled.
- Browser smoke:
  - admin sees replacement action on cancelled event;
  - member does not see replacement action;
  - replacement form has copied title/location/description;
  - date/time/repeat controls are blank/default, not copied from source;
  - layout has no horizontal scroll at mobile width and 200% text scaling.

## 11. Accepted Review Decisions

- v0.45.0 is cancelled-only; active events do not show or accept the flow.
- Any cancelled event may seed a replacement, including single-day events.
- The copied fields are title, location, and description only.
- Date, start time, end time, repeat rule, repeat count, event days,
  attendance, member memos, cancellation state, and audit metadata are not
  copied.
- A submitted `copy_source_event_id` is revalidated on POST and rejected unless
  it resolves to a cancelled event in the submitted community.
- Safe provenance may be recorded as
  `created_from_cancelled_event_id` in audit metadata.
