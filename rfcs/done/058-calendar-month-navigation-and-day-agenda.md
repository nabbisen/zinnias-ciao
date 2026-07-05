# RFC 058 — Calendar Month Navigation and Day Agenda

**Status.** Implemented (v0.42.0)
**Phase:** F8 / Calendar workflow improvement
**Project:** ciao.zinnias
**Date:** 2026-07-05
**Shipped in:** v0.42.0
**Relationship:** Follows RFC-056. RFC-056 split Home and Calendar and kept
Calendar day cells non-interactive in v0.40.0; this RFC adds route-backed month
navigation and day filtering to the Calendar page.

---

## 1. Summary

The Calendar page can now move between months and filter the agenda to a single
day. The behavior remains server-rendered and no-JS compatible: all state is
held in `month=YYYY-MM` and optional `day=YYYY-MM-DD` query parameters.

The Calendar community switcher preserves the selected month and day after
switching to another community. The resulting page still shows only events from
the selected active community.

## 2. Goals

- Add previous month, this month, and next month navigation.
- Let a member tap a day cell to show that day's agenda below the grid.
- Let a member clear the day filter and return to the full visible-month agenda.
- Preserve selected month/day when changing active community from the Calendar
  header switcher.
- Keep event queries scoped to the active community and visible month.
- Keep the flow server-rendered, bookmarkable, and no-JS compatible.
- Keep today calculated from the active community timezone.
- Add i18n and release-gate coverage for the new Calendar controls.

## 3. Non-Goals

- No drag/drop or inline calendar editing.
- No client-side month rendering.
- No recurrence edit semantics changes.
- No external calendar import or OAuth integration.
- No change to Home's read-only month overview.

## 4. External Behavior

On `/c/:cid/communities`, members see:

1. A centered monthly calendar for the selected active community.
2. Previous month, this month, and next month links.
3. Day cells that link to the same Calendar page with a day filter.
4. A visible "月全体" action when a day filter is active.
5. An agenda below the grid that shows either the full visible month or the
   selected day.

The route shape is:

```text
/c/:cid/communities?month=2026-07
/c/:cid/communities?month=2026-07&day=2026-07-05
```

Invalid or out-of-month `day` values are ignored. Invalid `month` values fall
back to the community-local current month.

## 5. Internal Design

`workers/ssr/src/handlers/communities.rs` owns Calendar rendering.

The handler:

- derives the community-local current date;
- parses optional `month` and `day` query parameters;
- queries `calendar_month_for_community(db, community_id, month_start,
  next_month_start)`;
- renders the month grid with route-backed links;
- filters the agenda in memory when `day` is present.

The community switcher uses a constrained `next` value:

```text
communities:YYYY-MM
communities:YYYY-MM:YYYY-MM-DD
```

`workers/ssr/src/handlers/community.rs` validates that shape before redirecting.
It rejects malformed months, invalid dates, and day values outside the selected
month.

## 6. Safety and Privacy

- The Calendar query remains scoped by authenticated membership and
  `community_id`.
- Calendar cells show only day numbers, a today label, and an event marker.
- Event titles and locations appear only in the agenda list below the grid.
- Community switching accepts only known member communities and validates the
  Calendar `next` payload before building a redirect.
- Query parameters are treated as view state only; they do not mutate data.

## 7. Copy Contract

| Surface | Japanese copy |
|---------|---------------|
| Month heading | `今月の予定` |
| Previous month | `前の月` |
| Next month | `次の月` |
| This month | `今月` |
| Clear day filter | `月全体` |
| Empty month | `今月の予定はありません。` |
| Empty day | `この日の予定はありません。` |

## 8. Acceptance Criteria

- Calendar supports `month=YYYY-MM` and optional `day=YYYY-MM-DD`.
- Previous/next/current links are route-backed and work without JavaScript.
- Tapping a day filters the agenda to that date.
- Clearing the filter restores the visible-month agenda.
- Community switching preserves selected month/day.
- Calendar events remain scoped to the selected active community.
- Day cells expose accessible labels and selected-day state.
- Release gates cover the route-backed navigation, day agenda, switcher state,
  and i18n parity.

## 9. Test Plan

- `cargo fmt --all -- --check`
- `cargo test -p zinnias-ciao-contracts --test release_gates -- --nocapture`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown`
- `cargo build --workspace`
- `cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts -p zinnias-ciao-ssr`
- Browser smoke at 360-428px mobile width for month navigation and day agenda.
- Browser smoke with JavaScript disabled to verify route-backed navigation and
  the visible community switch submit fallback.
