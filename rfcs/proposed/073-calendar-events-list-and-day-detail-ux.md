# RFC 073 - Calendar Events List and Day Detail UX

**Status.** Proposed  
**Target release.** Future candidate  
**Tracks.** Calendar UX, navigation, accessibility, route state.  
**Touches.** Calendar page, Calendar view tabs, selected-day detail, monthly
events list, browser smoke, release checklist.

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

When no date is selected, the detail area may show a calm prompt or remain
minimal. It should not duplicate the whole monthly events list.

## Events List Tab

The Events list tab should render the month-scoped event list:

- same selected month as Calendar;
- full-month list by default, even when the Calendar tab currently has a
  selected day;
- event links with date/time and location;
- cancellation markers where applicable;
- month navigation.

The first implementation may continue to accept
`view=list&day=YYYY-MM-DD` as a valid URL for direct links or future use, but
the tab link itself should point to the full-month list:

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

Fragment behavior is scoped to Calendar grid date links only. The switcher
should not accept arbitrary fragments in `next`. If a switched destination is a
Calendar day-detail view and review confirms the fragment is useful, the switch
handler may generate the known `#calendar-day-detail` fragment itself.

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

## Open Questions

- Should the selected-day detail show attendance counts, or only event links?
