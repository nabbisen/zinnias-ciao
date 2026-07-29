# RFC 073 - Calendar Events List and Day Detail UX

**Status.** Accepted — architecture-reviewed and owner-accepted 2026-07-29;
implementation must follow a reviewed Developer Handoff. First product theme
after the 2026-07-29 amendment lifted the feature freeze.

**Target release.** Next product release; not tied to a version transition.

**Tracks.** Calendar UX, navigation, accessibility, route state.

**Touches.** Calendar page, Calendar view tabs, selected-day detail, monthly
events list, switcher `next` serialization, i18n parity, browser smoke,
release checklist.

## Summary

Refine the Calendar page by separating the monthly events list from the month
grid and making selected-day detail appear directly under the calendar.

Current Calendar modes are:

```text
Calendar
Attendance table
```

The Calendar mode currently combines the month grid and the monthly event list
in one view. That makes the default Calendar page longer than necessary and
makes date selection feel like it moves the user to a different page area.

This RFC proposes three Calendar tabs:

```text
Calendar
Events list / 予定一覧
Attendance table
```

The Calendar tab should focus on the month grid and selected-day detail. The
Events list tab should own the month-scoped event list.

## Background

RFC-056 made Calendar a primary community page. RFC-058 added month navigation
and selected-day filtering. RFC-067 added the attendance matrix as a second
view. RFC-068 added admin-only CSV export from the matrix.

The current route already supports:

```text
/c/:cid/communities?month=YYYY-MM
/c/:cid/communities?month=YYYY-MM&day=YYYY-MM-DD
/c/:cid/communities?month=YYYY-MM&view=matrix
```

The existing design is route-backed and no-JS compatible. This RFC should keep
that property.

## Problem

User feedback on the Calendar page:

- the monthly event list under the calendar should be a separate tab;
- when a date is clicked, that date's detail should appear under the calendar;
- selecting a date currently brings the user to the top of the page;
- the events list may become large, so its scope and growth behavior need a
  design decision.

## Goals

- Keep the Calendar month grid as the default view.
- Add an Events list tab separate from the Calendar grid.
- Keep Attendance table as a separate tab.
- Show selected-day detail under the calendar grid.
- Keep selected-day detail route-backed and no-JS compatible.
- Avoid loading unbounded event history.
- Preserve month/day/view state when switching communities where safe.
- Keep mobile and 200% text behavior usable.

## Non-Goals

- No infinite scroll in the first slice.
- No all-time event archive.
- No client-only Calendar state.
- No drag/drop calendar editing.
- No inline event editing from the Calendar grid.
- No change to matrix CSV export behavior.

## Proposed Route Model

Extend Calendar view parsing from two modes to three:

```rust
CalendarView::Calendar
CalendarView::List
CalendarView::Matrix
```

Query strings:

```text
/c/:cid/communities?month=YYYY-MM
/c/:cid/communities?month=YYYY-MM&day=YYYY-MM-DD
/c/:cid/communities?month=YYYY-MM&view=list
/c/:cid/communities?month=YYYY-MM&day=YYYY-MM-DD&view=list
/c/:cid/communities?month=YYYY-MM&view=matrix
/c/:cid/communities?month=YYYY-MM&day=YYYY-MM-DD&view=matrix
```

Invalid `view` values fall back to Calendar.

## Calendar Tab

The Calendar tab should render:

1. view tabs;
2. materialization notice, if present;
3. month navigation;
4. month grid;
5. selected-day detail below the grid.

The selected-day detail should show:

- selected date;
- event links for that date;
- cancellation markers where applicable;
- admin Create Event action for that day when the current member is an admin;
- empty-day message when no events exist.

**Decision — the detail section always renders.** When no date is selected it
shows a short prompt (for example "日付を選ぶと、その日の予定がここに表示され
ます") rather than being omitted. The section must exist in the DOM on every
Calendar-tab render, because `#calendar-day-detail` is a link target: if the
section only appeared once a day was selected, the first click would land on a
dangling anchor and fall back to the top of the page — the exact behavior this
RFC exists to fix. It must not duplicate the whole monthly events list.

**Decision — selected-day detail shows event links only, not attendance
counts.** This resolves the former open question. Three reasons: the product
principle is minimum information per screen; per-event attendance aggregation is
not among the data the current day-detail helper fetches, so adding it would
introduce new per-day queries whose cost belongs to the RFC-044 query-budget
discussion; and the Attendance table tab already exists for precisely that job.
Revisit only if pilot feedback asks for it.

## Events List Tab

The Events list tab should render the month-scoped event list:

- same selected month as Calendar;
- full-month list by default, even when the Calendar tab currently has a
  selected day;
- event links with date/time and location;
- cancellation markers where applicable;
- month navigation.

**Decision — `day` is carried but never filters the Events list.**
`view=list&day=YYYY-MM-DD` is a valid URL and must not error, but it renders the
**same full-month list** as `view=list` alone. The `day` value is preserved only
so that switching back to the Calendar tab restores the user's selection, and so
community switching can carry complete state. It never changes what the Events
list tab displays.

The rationale is predictability: if `day` filtered the list, the tab's contents
would depend on state the user cannot see from within that tab, and the same tab
would sometimes show a month and sometimes a single day. The tab link itself
always points at the full-month form:

```text
/c/:cid/communities?month=YYYY-MM&view=list
```

This keeps the Events list tab predictable: it is the month list, not a
selected-day detail duplicate.

The first slice should keep the list bounded to the selected month. If users
need more than a month list later, add a separate archive or pagination RFC.

## Date Click and Scroll Behavior

Date cells should keep no-JS route-backed behavior.

The first acceptable fix is to use a fragment target:

```text
/c/:cid/communities?month=YYYY-MM&day=YYYY-MM-DD#calendar-day-detail
```

The detail section should have:

```html
id="calendar-day-detail"
```

This avoids landing at the top of the page after a date click. Exact scroll
position preservation is a separate progressive-enhancement problem and is out
of scope for the first slice.

## Community Switch Behavior

Switcher `next` state must preserve:

- month;
- selected day;
- view mode, including `list`;

If the target community is active for the user, the switch should remain on the
same Calendar view. If not, existing safe fallback behavior applies.

**Decision — the switcher never emits a fragment.** Fragment behavior is scoped
to Calendar grid date links only, and `next` must not accept arbitrary fragments.
The switcher must not generate `#calendar-day-detail` either: changing community
is a context change, and landing mid-page on a day detail in a different
community is disorienting — that day may hold entirely different events, or
none. The user should arrive at the top of the destination Calendar with month,
day, and view preserved, and scroll if they choose.

**Scope boundary against RFC-074.** RFC-073 only extends the *existing*
Calendar-specific switcher serialization to carry one more value (`view=list`).
It does **not** build general cross-page preservation. Making preservation
systematic across page families is RFC-074's scope and stays there. An
implementer working from this RFC should touch only the Calendar `next`
serialization that already handles `month`, `day`, and `view=matrix`, and should
stop and escalate if the change appears to require a general mechanism.

## Copy Contract

Add a new i18n pair for the third tab:

```text
EN_CALENDAR_VIEW_LIST = "Events list"
JA_CALENDAR_VIEW_LIST = "予定一覧"
```

The strings must be included in the existing EN/JA parity gate.

## Testing Expectations

- Unit tests for Calendar view parsing including `list`.
- Unit tests for switcher `next` serialization and destination parsing.
- i18n parity gate includes the Events list tab label.
- Render tests or release gates proving three tabs exist.
- Browser smoke for:
  - Calendar default grid without monthly list duplication;
  - date click lands at selected-day detail, not top;
  - Events list tab shows month-scoped list;
  - Matrix tab still works;
  - community switch preserves `view=list`, `view=matrix`, month, and day;
  - mobile and 200% text usability.

## Compatibility and Migration

No schema change, no migration, no data-format change.

Existing bookmarked and shared URLs continue to work unchanged:
`?month=`, `?month=&day=`, and `?month=&view=matrix` all render as they do
today. The only behavioral difference for an existing URL is that
`?month=&day=` now renders the day's detail beneath the grid instead of a
filtered list inside the combined view — which is the intended improvement.
`view=list` is purely additive, and an unrecognized `view` value still falls back
to Calendar, so no URL can 404 as a result of this change.

## Security Considerations

This RFC adds no new authorization boundary, no new data exposure, and no new
form or mutation. It changes rendering and route parsing within an already
authenticated, membership-scoped page.

Two properties must be preserved rather than newly established:

- the community-scoped authorization check that already gates this page applies
  unchanged to all three tabs; a `view` value must never widen what a member can
  see;
- `day`, `month`, and `view` remain untrusted query input, parsed and validated
  as they are today, with `day` still required to fall inside the selected month.
  The added `list` variant must be parsed by the same closed-enum path, never by
  string comparison at a render site.

Fragments are client-side only and are never sent to the server, so
`#calendar-day-detail` carries no security consequence.

## Operational Considerations

None. No new binding, secret, migration, or hosted resource. The Events list tab
reuses the month-bounded query the Calendar tab already performs, so this adds no
new query per render; the month bound is what keeps event loading bounded.

## Alternatives Considered

**Keep one combined Calendar view and only move the day detail.** Rejected: it
addresses the scroll complaint but leaves the default page as long as it is
today, which is half the reported problem.

**Make the Events list tab day-filtered when a day is selected.** Rejected — see
the Events List Tab decision. It makes tab contents depend on invisible state.

**Client-side scroll or expanding the day detail in place without navigation.**
Rejected: it breaks the route-backed, no-JS contract that AD-1 and this page's
existing design require.

**Infinite scroll or an all-time archive for the Events list.** Out of scope by
Non-Goals; if a month is genuinely insufficient, that is a separate RFC with its
own pagination and query-budget analysis.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Fragment target missing on first render, so date clicks still jump to top | Low | Defeats the RFC's main purpose | The detail section always renders (Calendar Tab decision); browser smoke asserts it |
| Three tabs crowd the mobile viewport at 200% text | Medium | Usability regression on the primary screen | Tabs must wrap rather than truncate; 200%-scaling check is a required acceptance item |
| Scope creep into RFC-074's general switcher work | Medium | Slice stops being independently reviewable | Explicit scope boundary above plus a stop condition |
| Matrix or CSV export behavior disturbed | Low | Breaks an accepted RFC-068 contract | Non-Goal; existing matrix gates must stay green unchanged |

## Acceptance Criteria

1. `CalendarView` parses exactly three variants through one closed enum, with
   any unrecognized `view` falling back to Calendar.
2. The Calendar tab renders the month grid and a day-detail section that is
   present on every render, and does not render the monthly events list.
3. A date-cell click navigates to that day's detail without landing at the top
   of the page.
4. The Events list tab renders the full month regardless of `day`, and its own
   tab link omits `day`.
5. The Attendance table tab and admin CSV export behave exactly as before.
6. Community switching preserves `month`, `day`, and `view` — including
   `view=list` — and emits no fragment.
7. The Events list tab label exists as an EN/JA pair and passes the parity gate.
8. All three tabs remain usable on a 360 px viewport and at 200% text scaling.
9. No new query is introduced per Calendar render beyond those performed today.

## Implementation Boundaries

Expected to change: Calendar view parsing, the three tab links, Calendar-tab
render composition, the day-detail section and its `id`, the Events list render
path, Calendar switcher `next` serialization, one EN/JA i18n pair, and the
corresponding tests and browser smoke.

Expected **not** to change: event or attendance queries, the matrix render path,
CSV export, any handler outside the Calendar page, any schema, any form or
mutation, and the general switcher mechanism (RFC-074).

**Stop and escalate** if the change appears to require a general cross-page
switcher mechanism, a new query per render, a schema change, or any alteration
to matrix/CSV behavior.

## Release Implications

Additive UI change to an existing page, behind no feature flag. It does not
close any architecture finding, does not affect the remediation hold's hosted
evidence items, and does not require a version-transition decision. Browser
evidence for this RFC is local; it does not substitute for RFC-050 hosted
evidence.

## Open Questions

None. The former open question — whether selected-day detail should show
attendance counts — is resolved in the Calendar Tab section.
