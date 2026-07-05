# RFC 056 — Calendar-Centered Home Dashboard

**Status.** Implemented (v0.40.0)
**Phase:** F8 / Member Workflow Improvement
**Project:** ciao.zinnias
**Date:** 2026-07-02
**Relationship:** Amends RFC-005. RFC-005 shipped a single-community list-first Home and a Communities switcher page for MVP; this RFC makes Home a multi-community nearby-events dashboard and changes the former Communities page into a Calendar page for the active community.

---

## 1. Summary

Home opens with all active communities shown one by one. Each community section contains nearby upcoming events as links into Event Detail. Home does not show the community switcher; it is a cross-community overview.

The Calendar tab replaces the previous Communities page. It shows a month-shaped overview for the active community and keeps the community switcher in the header, so members can change the active community while staying in calendar context.

## 2. Goals

- Show active communities one by one on `/c/:cid/home`.
- Show nearby upcoming event links under each community.
- Remove the Home header community switcher.
- Change `/c/:cid/communities` into the Calendar page for the active community.
- Keep the Calendar page header community switcher, and keep the user on Calendar after switching.
- Highlight today on the Calendar page using the community timezone.
- Mark event days from the active community's visible month.
- Keep Home and Calendar server-rendered and no-JS compatible.
- Keep Japanese-only pilot copy covered by i18n parity gates.

## 3. Non-Goals

- No month navigation in this release.
- No client-side filtering, drag/drop, or calendar editing.
- No external calendar import or OAuth integration.
- No change to Event Detail, attendance, notes, or admin event creation semantics.

## 4. External Behavior

On Home, members see:

1. Header without the community switcher.
2. One section per active community.
3. Nearby upcoming event links in each section.
4. Empty-state copy in sections with no nearby events.
5. Admin shortcuts when applicable.

On Calendar, members see:

1. Header with the community switcher.
2. A centered "今月のこれからの予定" calendar-shaped overview for the active community.
3. A helper line: "予定がある日に印をつけています。詳しくは下の一覧をご覧ください。"

The Calendar page is active-community scoped. It marks event days from the visible month in that community, independent from Home's nearby-events window.

Days with events show a simple marker. Today is identified with visible text and styling. Calendar day cells are non-interactive in v0.40.0: they are not links or buttons and must not look tappable. Home event links remain the place to open Event Detail and answer attendance.

## 5. Internal Design

`workers/ssr/src/handlers/home.rs` renders the Home dashboard by:

- Loading active user communities.
- Fetching nearby events across those communities with a single batched query.
- Rendering each community as its own section.
- Rendering event links directly to `/c/:cid/events/:eid`.

`workers/ssr/src/handlers/communities.rs` now renders the Calendar page for the active community. It uses the calendar helper from `home.rs` and renders the same active community's current-month event links below the grid:

- Parse the community-local current date.
- Count `event_days` rows from the selected community's visible month.
- Render a stable seven-column CSS grid with weekday labels and fixed minimum cell heights.

The Calendar handler reuses the existing active-community Home query:

```text
calendar_month_for_community(db, community_id, month_start, next_month_start)
```

Home uses `home_upcoming_for_communities` so it does not issue one event query per community. The Calendar page remains active-community scoped and uses a separate month query so the selected community's visible month is complete.

## 6. Safety and Privacy

- Home shows only communities returned by the authenticated user's active memberships.
- Event links are community-scoped.
- The calendar uses only rows for the active community.
- Event titles, notes, participant names, and private detail are not placed in the calendar cells.
- The agenda links remain the route-backed Event Detail path from RFC-005.
- Rendered text remains Japanese for the pilot.
- Calendar cells are non-interactive and not focusable in v0.40.0.

## 7. Copy Contract

| Surface | Japanese copy |
|---------|---------------|
| Calendar heading | `今月のこれからの予定` |
| Calendar helper | `予定がある日に印をつけています。詳しくは下の一覧をご覧ください。` |
| Agenda heading | `予定の一覧` |
| Empty member | `これからの予定はまだありません。` |

## 8. Acceptance Criteria

- Home renders community sections and does not render the community switcher.
- Home event links point to community-scoped Event Detail routes.
- Calendar page renders the active-community month grid and active-community event links, and keeps the community switcher.
- Calendar uses a stable seven-column grid.
- Today is calculated from community-local display time.
- Release gates cover Home multi-community layout, Calendar ownership of the grid/switcher, and i18n parity.
- Weekday labels are Japanese and Sunday-first: 日 / 月 / 火 / 水 / 木 / 金 / 土.
- Today is identified by more than color alone.
- Event presence is identified by more than color alone.
- At 200% text scaling on a 360px viewport, day numbers and markers do not overlap.
- The calendar does not cause horizontal scrolling.
- Non-interactive cells are not focusable.
- Calendar cells do not contain event titles, notes, participant names, or attendance details.

## 9. Test Plan

- `cargo test -p zinnias-ciao-contracts --test release_gates`
- `cargo test -p zinnias-ciao-ssr`
- `cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown`
- Standard release gates: fmt, clippy, workspace build, workspace tests.
- Browser visual smoke test at 360-428px mobile width for Home and Calendar.
- Browser visual smoke test at 200% text scaling for Home and Calendar.
- Home screenshot with multiple communities and nearby event links.
- Calendar screenshot with today highlighted and at least one marked event day.
- Empty-state screenshot.
- Privacy check that calendar cells do not include title, note, participant, or attendance detail.

## 10. Future Work

- Month navigation.
- Day tap/filter behavior.
- Broader device QA beyond the sandboxed incognito Chromium smoke recorded for
  v0.40.0.
- Product decision on whether a future true current-month history calendar is needed.
