# RFC 059 — Calendar Create Event From Day

**Status.** Implemented (v0.43.0)
**Phase:** F8 / Calendar workflow improvement
**Project:** ciao.zinnias
**Date:** 2026-07-05
**Shipped in:** v0.43.0
**Relationship:** Follows RFC-056 and RFC-058. RFC-058 made Calendar days
selectable; this RFC connects a selected Calendar day to the admin event
creation workflow.

---

## 1. Summary

Admins can start event creation from a selected Calendar day. When an admin
opens a day on the Calendar page, the day agenda shows a quiet action:

```text
この日にイベントを作成
```

The action opens Create Event for the same active community with the date field
prefilled. The flow remains server-rendered, route-backed, no-JS compatible, and
community-scoped.

## 2. Goals

- Let active admins create an event from the Calendar day they are viewing.
- Prefill only the date; admins still choose title, time, recurrence, location,
  and description.
- Keep the action hidden from non-admin members.
- Preserve the selected date if the admin changes community from the Create
  Event header switcher.
- Validate all date view state server-side before using it in a form field or
  redirect.
- Keep Calendar cells themselves free of event titles, notes, participant
  names, and attendance details.

## 3. Non-Goals

- No inline event creation inside the Calendar page.
- No drag/drop or tap-to-place event editing.
- No automatic default time selection.
- No availability voting or conflict detection.
- No change to event creation authorization or form-token policy.
- No recurrence edit semantics change.

## 4. External Behavior

When an active admin opens:

```text
/c/:cid/communities?month=2026-07&day=2026-07-05
```

the selected-day agenda includes:

```text
この日にイベントを作成
```

The link target is:

```text
/c/:cid/admin/events/new?day=2026-07-05
```

Create Event renders the existing form with:

- the date field set to `2026-07-05`;
- start/end time still empty;
- recurrence controls unchanged;
- the existing community switcher retained.

If the admin switches community from Create Event while a valid `day` is
present, the switch target keeps the selected day:

```text
/c/:other-cid/admin/events/new?day=2026-07-05
```

Non-admin members do not see the create-from-day action on Calendar. Direct
access to Create Event remains protected by the existing admin authorization.

## 5. Internal Design

`workers/ssr/src/handlers/communities.rs`:

- resolves the active membership for the current user/community;
- treats `role == "admin"` as permission to render the selected-day action;
- renders the create link only when a valid selected day is active.

`workers/ssr/src/handlers/admin/events.rs`:

- parses optional `day=YYYY-MM-DD` query state;
- validates the exact date shape and calendar day;
- passes the valid date to `event_form_fields` as `day_date`;
- sets the community switcher next value to `admin_events_new:YYYY-MM-DD` when
  a valid day is present.

`workers/ssr/src/handlers/community.rs`:

- accepts `admin_events_new:YYYY-MM-DD` as a constrained switcher next value;
- validates the date before redirecting to the selected community's Create Event
  page;
- falls back to plain Create Event if the next value is malformed.

## 6. Safety and Privacy

- The Calendar action is UI convenience only. Create Event still requires an
  authenticated active admin and a `CREATE_EVENT` form token.
- Date query parameters are validated as view state. Invalid values are ignored
  or fall back to the unprefilled Create Event page.
- The community switcher still validates that the user belongs to the target
  community before redirecting.
- Calendar cells still show only day numbers, today text, and event markers.

## 7. Copy Contract

| Surface | Japanese copy |
|---------|---------------|
| Selected-day create action | `この日にイベントを作成` |

## 8. Acceptance Criteria

- Selected Calendar day renders an admin-only create-event link.
- Link target includes the selected `day=YYYY-MM-DD`.
- Create Event validates and prefills a valid selected day.
- Invalid date query values do not prefill the form.
- Create Event community switching preserves a valid selected day.
- Non-admin members do not receive the Calendar create action.
- Release gates cover action rendering, date prefill, switch preservation, and
  i18n parity.

## 9. Test Plan

- `cargo fmt --all -- --check`
- `cargo test -p zinnias-ciao-contracts --test release_gates -- --nocapture`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown`
- `cargo build --workspace`
- `cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts -p zinnias-ciao-ssr`
- Browser smoke with an admin session: select a Calendar day, open Create Event,
  verify the date is prefilled, and verify the Create Event community switcher
  preserves the date.
- Browser smoke with a member session: selected-day agenda does not show the
  create action.
