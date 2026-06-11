# RFC 005 — Member Home and Event Detail UI

**Status.** Proposed
**Phase:** M2 / Member MVP Flow
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** AD-1 (SSR + progressive enhancement), AD-4 (form token), Event→EventDay→Attendance grain.

---

## 1. Summary

Defines the core member UI: a Home upcoming-list and a route-backed Event Detail, both **server-rendered**. State changes are forms, not client mutations. Single-day events render exactly as before; multi-day events show per-day rows.

## 2. Goals

- Home as a large-card upcoming list, grouped Today / This Week / Later.
- Event Detail as a real route (`/c/:cid/events/:eid`), rendered server-side.
- Show the event, its day(s), current user's per-day status, per-day counts, participant list, and the one note editor.
- Community switching as a navigation, not client state.
- Thumb-friendly, 360 px, 200% text.

## 3. Non-Goals

- No calendar grid default, no live updates, no chat, no optimistic client state, no WASM bundle (AD-1).

## 4. External Behavior

Bottom tabs: Home | Communities | Me (links, not client routing). Event card (single-day shown compactly; multi-day shows a day count):

```text
[Going ✓] Neighborhood Cleanup
Today 10:00–11:30 · Park Entrance
Going 3 · No Go 1 · No answer 5
```

Event Detail is reached by following the card link; browser Back returns to Home at the prior scroll position (native, because it is a real navigation). On narrow screens it may be styled as a bottom sheet, but it is a route, not a transient modal.

Every status/note control is a `<form method="post">` with a hidden form token (AD-4). Submitting reloads the page via 303 to the canonical detail URL, showing the new state. Optional progressive enhancement may update a control in place, but the form is the source of truth.

## 5. Internal Design

No client reactive state. The SSR handler builds a view model per request:

```rust
struct EventDetailView {
    event: EventView,                 // title, location, description, status
    days: Vec<EventDayView>,          // each with per-day status buttons + counts + form token
    my_note: Option<NoteView>,        // one per event; editor form + form token
    participants_by_day: Vec<DayParticipants>,
    capabilities: Capabilities,       // server-computed; disabled controls carry a reason
}
```

Components are render functions (no `spawn_local`/signals/`ActionForm`): `app_shell`, `bottom_nav`, `community_switcher`, `event_card`, `day_status_form`, `participant_list`, `note_form`. Each form embeds a freshly issued, purpose- and resource-bound form token. Rendering supports 360 px and 200% scaling by stacking.

## 6. Data and API Design

Server routes (HTML):

```text
GET  /                              # 303 -> selected community Home
GET  /c/:cid/home                   # upcoming list (bounded window over event_days)
GET  /c/:cid/events/:eid            # detail (issues form tokens for visible actions)
GET  /c/:cid/communities            # switch
POST /c/:cid/select                 # set selected community; 303 -> Home
```

The Home query is the bounded `event_days` window from RFC-002 §6 (no N+1). Detail loads the event, its days, the day attendances, and the member's note in a small fixed number of queries.

## 7. Security, Privacy, and Safety

- The handler renders only data from authorized queries (RFC-004); no community_id is trusted from the path without membership.
- Event titles, locations, descriptions, and notes are escaped on render (RFC-012).
- Direct routes to inaccessible events return a generic not-found (RFC-004).
- Form tokens are session-bound; a missing/foreign/expired token rejects the POST.

## 8. Acceptance Criteria

- Home renders large cards grouped by date, fully usable with JS disabled.
- Event Detail reachable from a card and via direct authorized route; Back works natively.
- A multi-day event shows independent per-day status; a one-day event shows a single row.
- 200% text remains usable; community switch shows no stale prior-community data.
- Empty state uses plain language.

## 9. Test Plan

- Render tests for `event_card` by status and for single- vs multi-day detail.
- Route/back test (real navigation).
- No-access route test (generic 404).
- Form-token presence and rejection-on-tamper tests.
- a11y: labels, 44 px targets, grayscale legibility, 200% scaling.

## 10. Open Questions / Decisions

Decision: weekly/monthly grids are not MVP default. For multi-day events the card shows the nearest upcoming day plus a day-count; the detail lists all days.
