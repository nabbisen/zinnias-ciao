# RFC 067 - Monthly Attendance Matrix

**Status.** Implemented in v0.56.0  
**Target release.** v0.56.0  
**Tracks.** Calendar workflow, attendance visibility, community coordination,
responsive table UX.  
**Touches.** `workers/ssr/src/handlers/communities`,
`workers/ssr/src/db/event.rs`, `workers/ssr/src/db/attendance.rs`,
`workers/ssr/src/db/membership.rs`, Calendar rendering, release gates, browser
smoke.

## Summary

RFC-067 adds a switchable monthly attendance matrix to the Calendar page for
active community members and admins.

The current Calendar page is optimized for finding event dates and opening event
detail pages. It shows a month grid and an agenda for the selected day or month.
That is useful for finding events, but members and admins also need a different
view: who has answered across the whole month, which dates still have many
no-replies, and how the community's plans look as a whole.

This RFC proposes a second Calendar mode:

- the existing month calendar remains the default view;
- active members and admins can switch to a matrix view for the same active
  community and month;
- matrix columns are dates in the selected month;
- matrix rows are active community members;
- cells summarize that member's attendance state for events on that date;
- event details stay secondary through compact cell affordances and a selected
  date detail area, not by expanding every table cell.

CSV export is deliberately out of scope for RFC-067. It should be handled by a
later RFC after the matrix data contract and user experience are stable. When
added, CSV export must be available only to community admins.

## Background

Recent Calendar work made the page the center of the community workflow:

- RFC-056 split Home and Calendar and made Calendar the monthly community view.
- RFC-058 added route-backed month navigation and day agenda filtering.
- RFC-059 added create-event-from-day.
- RFC-065 added recurrence v2 and occurrence exceptions while preserving
  `event_day_id` as the attendance anchor.
- RFC-066 added admin event copy assistance.

The next pressure is not another event creation shortcut. It is overview.
Community members need to see attendance response state across a month without
opening every event detail page one by one.

The v0.54.0 Calendar roadmap review recommended caution because a month-wide
member-by-member grid is more sensitive than a single event detail page. The
product decision for RFC-067 is that the matrix itself is part of the shared
community Calendar and is visible to active members. The privacy boundary moves
to export: CSV export is not included in RFC-067 and must be admin-only in a
later RFC.

## Goals

- Add a monthly attendance matrix mode to the existing Calendar page for active
  community members and admins.
- Keep the ordinary Calendar month grid as the default view.
- Keep view state route-backed, bookmarkable, and no-JS compatible.
- Preserve active community scoping and community switch behavior.
- Show active members as rows and selected-month dates as columns.
- Summarize each member's attendance state for each date using accessible
  symbols and labels.
- Handle dates with zero, one, or multiple event days without making cells
  expand unpredictably.
- Preserve RFC-065 recurrence materialization rules and `event_day_id`
  attendance anchoring.
- Define query-budget limits before implementation.
- Add release gates and browser smoke for desktop, mobile, and 200% text
  scaling.
- Preserve an explicit boundary that future CSV export is admin-only.

## Non-Goals

RFC-067 does not add:

- CSV export;
- CSV export for members in any future export RFC;
- print layout;
- bulk attendance edit;
- inline attendance editing from the matrix;
- event drag/drop or schedule editing;
- per-occurrence notes;
- notification or reminder sending;
- former-member historical reporting by default;
- hidden broader export payloads;
- new recurrence semantics;
- a new route family separate from the Calendar page unless review finds the
  existing route unsuitable.

CSV export should be considered as a later RFC-068 candidate after the matrix
view stabilizes, and that RFC must keep CSV export limited to community admins.

## Terms

| Term | Meaning |
|------|---------|
| Calendar mode | A route-backed view of the Calendar page, initially `calendar` or `matrix`. |
| Matrix date | One day in the selected visible month, rendered as a column. |
| Matrix member | An active community member, rendered as a row. |
| Cell summary | The compact status shown for one member on one date. |
| Date detail area | A secondary area that explains events and statuses for the selected date. |
| No reply | A member has no attendance row for a specific event day. |

## External Behavior

### Entry Point

The Calendar page remains:

```text
/c/:community_id/communities
```

For active community members and admins, the page shows a compact view switcher
near the month navigation:

```text
カレンダー
回答表
```

English reference copy:

```text
Calendar
Attendance table
```

The default mode remains the existing calendar grid:

```text
/c/:community_id/communities?month=2026-07
```

The matrix mode is selected by query parameter:

```text
/c/:community_id/communities?month=2026-07&view=matrix
```

If a day is selected, the selected day may be preserved:

```text
/c/:community_id/communities?month=2026-07&day=2026-07-05&view=matrix
```

Invalid `view` values fall back to the default Calendar mode. Invalid month or
day values follow RFC-058 behavior.

### Authorization

The matrix is visible to active community members and admins.

- Active community members and admins see the view switcher and can open matrix
  mode.
- Anonymous users follow the existing session-expired behavior.
- Removed memberships follow the existing not-found or inaccessible behavior.
- Users without membership in the active community cannot view the matrix.

This follows the current event detail privacy model where active community
members can see event participant state inside their community. RFC-067 extends
that visibility to a monthly scanning view, so it must avoid adding notes,
internal IDs, feed tokens, audit data, or export payloads.

CSV export is a separate permission boundary. It is not included in RFC-067 and
must be community-admin-only when designed later.

### Community Switcher

The existing community switcher keeps the selected community as the access
boundary.

When a member or admin switches communities from matrix mode:

- if the target community membership is active, preserve `month`, optional
  `day`, and `view=matrix`;
- if the target community membership is missing or removed, follow the existing
  community switcher fallback.

The switcher must not expose the matrix for communities where the current user
is not an active member.

### Matrix Layout

The matrix is date-first:

- columns are dates of the selected month;
- rows are active members;
- the first column is the member display name;
- the date header is sticky horizontally where the browser supports it;
- the member name column is sticky vertically/horizontally where practical, but
  this must not break mobile layout;
- cells have stable dimensions and do not grow to fit event titles.

The table may scroll horizontally. That is acceptable because a 31-day matrix
cannot fit narrow screens without losing meaning. The UI must make horizontal
scrolling discoverable without instructional clutter.

### Cell Semantics

For each member/date cell, compute the set of event days in the active
community on that date.

If the date has no event days:

- show an empty or muted placeholder;
- do not imply that the member failed to answer.

If the date has one event day:

- show the member's status for that event day:
  - going;
  - not going;
  - attended, when applicable after event completion;
  - no reply.
- v0.56.0 visual symbols:
  - `○` for going;
  - `×` for not going;
  - `済` for attended;
  - `?` for no reply;
  - `中` for a cancelled occurrence where attendance is not actionable.

If the date has multiple event days:

- show a compact answered-count summary in the fixed shape `answered/total`;
- `answered` is the number of non-cancelled event days where the member has an
  explicit `going`, `not_going`, or `attended` status;
- `total` is the number of non-cancelled event days on that date;
- if every event day on the date is cancelled, show `中`;
- the accessible label must include the date, member display name, total event
  count, cancelled count, going count, not-going count, attended count, and
  no-reply count in plain text;
- the selected date detail area must expose the per-event status breakdown so
  the compact cell is never the only way to understand the date.

Cancelled occurrences:

- stay visible when the underlying Calendar month query returns them;
- use a subdued cancelled marker;
- do not ask users to infer cancellation from missing data.

### Date Detail Area

The matrix should include a selected date detail area below or beside the table.

The detail area lists the selected date's event days with:

- title;
- time;
- cancellation state if applicable;
- links to event detail pages;
- aggregate counts for going, not going, attended, and no reply.

This keeps event titles out of dense cells while still letting the user
understand what a compact cell means.

If no date is selected, the detail area may show the first date with events or a
plain empty state.

### Sorting

Members should be sorted predictably:

1. current local convention if already established;
2. active members by display name;
3. stable tie-breaker by membership id, not shown.

The implementation should avoid surprising reordering based on attendance
status. The matrix is for scanning the month, not ranking people.

## Internal Design

### Route State

Extend Calendar state parsing in `workers/ssr/src/handlers/communities.rs` with
an internal mode enum:

```text
CalendarMode::Calendar
CalendarMode::Matrix
```

Only active community members may keep `CalendarMode::Matrix`. Non-member
requests follow existing Calendar access behavior.

The community switcher `next` value is extended with an exact grammar. Existing
Calendar values remain valid:

```text
communities:YYYY-MM
communities:YYYY-MM:YYYY-MM-DD
```

Matrix mode uses `matrix` as a final literal segment:

```text
communities:YYYY-MM:matrix
communities:YYYY-MM:YYYY-MM-DD:matrix
```

No other mode literal is accepted in v0.56.0. The parser must reject malformed
dates, out-of-month day values, unknown modes, duplicate mode segments, empty
segments, and ambiguous segment counts.

### Data Shape

The matrix can reuse existing data grains:

- active members from `membership_db::list_all_active`;
- visible month event days from `event_db::calendar_month_for_community`;
- attendance rows from `attendance_db::list_for_event_days`;
- status counts from an existing or new batched count helper.

The implementation should build a server-side view model:

```text
MatrixMonth {
  year,
  month,
  dates,
  members,
  event_days_by_date,
  attendance_by_day_and_member,
}
```

The view model must not include notes, invite data, session data, calendar feed
tokens, audit log rows, or hidden export-only fields.

### Query Budget

The acceptable first implementation query shape is bounded:

1. authenticate and authorize membership using existing route behavior;
2. fetch the active community record;
3. fetch active memberships once;
4. fetch visible-month event days once;
5. fetch attendance rows for all visible event-day IDs with one batched query;
6. optionally fetch grouped counts for all visible event-day IDs with one
   batched query, or derive counts from the attendance rows and active member
   count.

No member x date x event-day query loop is acceptable.

Initial v0.56.0 caps:

- member rows: render active members up to 100;
- event-day rows: render visible-month event days up to the existing Calendar
  month cap of 300;
- dates: one calendar month only.

If caps are exceeded, render a plain message that the month is too large for the
matrix and keep the ordinary Calendar view available.

### Rendering Boundary

The current Calendar handler is already a large file. RFC-067 requires splitting
the Calendar handler before or as part of implementation. The implementation
must not add matrix view-model construction and rendering to the existing
monolithic `workers/ssr/src/handlers/communities.rs`.

Required implementation direction:

```text
workers/ssr/src/handlers/communities.rs          route entry/facade
workers/ssr/src/handlers/communities/calendar.rs existing month grid/list
workers/ssr/src/handlers/communities/matrix.rs   matrix view model/rendering
workers/ssr/src/handlers/communities/tests.rs    focused tests
```

If Rust module layout requires a slightly different filename shape, the same
boundary is still required: route parsing, DB access, view-model construction,
and HTML rendering must not all be expanded in one oversized function.

## Security, Privacy, and Safety

- Matrix mode is visible only to active members of the active community.
- Data remains scoped to active community membership.
- The matrix must not show members from other communities.
- Former or removed members are not shown by default.
- Member notes are not included.
- Private calendar feed tokens are not included.
- Internal IDs are not shown in visible cells.
- Status symbols must not rely on color alone.
- No hidden CSV/export payload is included in RFC-067.
- Future CSV export must be community-admin-only.
- Matrix cells link only to event detail pages already authorized for that
  community.

Former-member historical reporting may be useful later, but it should be a
separate design because it exposes membership history. If it exports data, that
export must be admin-only.

## Accessibility and Responsive Design

The matrix is inherently dense. Acceptance must include explicit responsive
checks rather than assuming a table works on mobile.

Requirements:

- semantic table markup where practical;
- clear row and column headers;
- accessible labels for compact symbols;
- no color-only status meaning;
- keyboard-reachable event/date detail links;
- no horizontal page overflow outside the intended matrix scroller;
- usable at 200% text scaling;
- stable cell dimensions so hover/focus/state changes do not resize the grid;
- selected date state distinct from today and event markers.

For mobile, horizontal scrolling is acceptable. Collapsing the whole matrix into
cards is not required in v0.56.0 unless review finds the table unusable.

## Copy Contract

Japanese copy should stay short because the matrix is dense.

| Surface | Japanese copy | English reference |
|---------|---------------|-------------------|
| Calendar mode | `カレンダー` | Calendar |
| Matrix mode | `回答表` | Attendance table |
| Matrix heading | `月の回答表` | Monthly attendance table |
| Empty month | `この月の予定はありません。` | There are no events this month. |
| No members | `有効なメンバーがいません。` | There are no active members. |
| Too large | `この月は回答表を表示するには大きすぎます。カレンダー表示をご利用ください。` | This month is too large for the attendance table. Use Calendar view. |
| Going | `参加` | Going |
| Not going | `不参加` | Not going |
| Attended | `参加済み` | Attended |
| No reply | `未回答` | No reply |
| Cancelled | `中止` | Cancelled |

The v0.56.0 symbols are part of this RFC's contract. Visual styling and
accessible labels should be reviewed during implementation.

## Acceptance Criteria

- Active members and admins can switch Calendar between month grid and monthly
  matrix modes.
- Users outside the active community cannot see monthly member-by-member matrix
  data.
- Matrix route state is bookmarkable through `view=matrix`.
- Community switching preserves matrix mode for active target community
  memberships and follows existing fallback for inaccessible targets.
- Community switcher `next` accepts only the RFC-067 grammar:
  `communities:YYYY-MM`, `communities:YYYY-MM:YYYY-MM-DD`,
  `communities:YYYY-MM:matrix`, and
  `communities:YYYY-MM:YYYY-MM-DD:matrix`.
- Matrix columns cover dates in the selected month.
- Matrix rows cover active community members.
- Cells summarize no event, no reply, going, not going, attended, cancelled,
  and multi-event dates without growing unpredictably.
- Single-event cells use the v0.56.0 symbols `○`, `×`, `済`, `?`, and `中`.
- Multi-event cells use `answered/total` visually and expose the full status
  breakdown through an accessible label and selected date detail area.
- A selected date detail area links to the relevant event detail pages.
- Query shape is batched and avoids member/date/event N+1 loops.
- Matrix caps are fixed at 100 active members and 300 visible-month event days,
  with clear too-large UI behavior.
- Calendar handler code is split into focused modules as part of this RFC.
- Calendar recurrence materialization rules from RFC-065 are preserved.
- Browser smoke covers desktop, mobile, and 200% text scaling.
- CSV export remains absent from this release and reserved as an admin-only
  future feature.

## Test Plan

- Unit tests for Calendar mode parsing and switcher `next` validation.
- Switcher parser tests for all accepted grammar shapes, malformed dates,
  out-of-month day values, unknown modes, duplicate mode segments, empty
  segments, and ambiguous segment counts.
- Rendering tests for admin vs member matrix visibility.
- Rendering tests for single-event, multi-event, no-event, cancelled, and no
  reply cells.
- Rendering tests for the exact single-event symbols and multi-event
  `answered/total` accessible label breakdown.
- Query/helper tests for attendance grouping from batched rows.
- Contract/release gate tests for i18n copy and no stale cache/version strings
  during release prep.
- Browser smoke:
  - member opens matrix for a month with multiple members and multiple events;
  - admin opens the same matrix successfully;
  - non-member cannot access matrix data through direct URL;
  - community switcher preserves matrix for another active-member target;
  - selected date detail links to event detail;
  - mobile viewport around 390px remains usable;
  - 200% text scaling has no unintended page-wide horizontal overflow beyond
    the matrix scroller.
- Source gates:
  - `cargo fmt --all -- --check`
  - `cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts -p zinnias-ciao-ssr`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo build --workspace`
  - `cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown`

## Rollout Notes

RFC-067 should be implemented as a normal source release, not behind a
production-only feature flag. The behavior is membership-scoped, route-backed,
and does not introduce new persisted data.

Hosted staging smoke is recommended before public pilot use because dense table
layout and Cloudflare-hosted recurrence materialization are both runtime
concerns.

## Deferred Decisions

- The selected date detail default may be today, the first date with events, or
  no date; implementation should choose the least surprising no-JS behavior.
- Role emphasis is visual polish only. The matrix must remain sorted
  predictably and must not rank members by attendance status.
- Future admin-only RFC-068 CSV export should decide whether to use only the
  visible matrix data or define a separate export-focused data contract.
