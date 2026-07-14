# RFC 074 - Community Switch Route Preservation

**Status.** Proposed  
**Target release.** Future candidate  
**Tracks.** Navigation, community switching, route state, UX safety.  
**Touches.** Header switcher, `/switch`, route `next` contracts, release gates,
browser smoke.

## Summary

Make community switching preserve the current page when it is safe and
understandable.

Current behavior is mixed. Some pages pass a specific `next` value to the
header switcher, while pages that use the default switcher fall back to Home.
This is safe, but it can feel disruptive when a user expects to stay on the
same type of page after changing communities.

## Problem

User feedback:

- changing community from the header can force navigation to Home;
- users often expect to stay on the current page type when switching
  communities.

The current code already has special cases for Calendar and some admin routes,
but preservation is not systematic.

## Goals

- Preserve the current route family when the target community supports it.
- Keep safe fallback behavior when the target community lacks access.
- Avoid open redirects.
- Keep no-JS switcher behavior.
- Make route-state preservation explicit and testable.

## Non-Goals

- No client-side router.
- No cross-community resource transfer.
- No preserving event detail pages across communities by guessing equivalent
  events.
- No preserving admin pages for target communities where the user is not an
  admin.

## Proposed Contract

Each page with a community switcher should choose one of:

- preserve this route family;
- preserve this route family only for admin targets;
- intentionally fall back to Home.

My Page preservation is in scope. Switching communities from My Page should
land on the target community's My Page when the user is an active member there.

Route-family matrix:

| Source page | Target behavior |
|-------------|-----------------|
| Home | target Home |
| Calendar | target Calendar with month/day/view when valid |
| My Page | target My Page |
| Calendar feed settings | target Calendar feed settings |
| Export page | target Export page only if admin in target |
| Templates page | target Templates page only if admin in target |
| Member management | target member management only if admin in target |
| Invite page | target invite page only if admin in target |
| Help-signin page | target member management if admin in target; do not preserve target member id |
| Remove/promote/demote confirmation | target member management if admin in target; do not preserve target member id |
| Create Event | target create event only if admin in target; preserve prefilled day when valid |
| Edit/cancel/copy/recreate/attendance/admin-note-hide event admin pages | target Home unless a future RFC defines equivalent-event preservation |
| Event Detail | target Home unless an explicit equivalent-event model is designed |
| Note delete confirmation | target Home unless an explicit equivalent-event model is designed |

The `/switch` handler should remain the only redirect target for the header
select form. It should validate every `next` value through a closed parser.

## Closed `next` Grammar

Accepted tokens:

```text
home
me
calendar_feed
admin_export
admin_templates
communities:YYYY-MM
communities:YYYY-MM:YYYY-MM-DD
communities:YYYY-MM:list
communities:YYYY-MM:YYYY-MM-DD:list
communities:YYYY-MM:matrix
communities:YYYY-MM:YYYY-MM-DD:matrix
admin_members
admin_invites
admin_events_new
admin_events_new:YYYY-MM-DD
```

Rules:

- `home`, `me`, and `calendar_feed` require active membership in the target
  community.
- `admin_members`, `admin_invites`, `admin_export`, `admin_templates`, and
  `admin_events_new*` require active admin role in the target community.
- Calendar month and day values must be syntactically valid, and a day must
  belong to the selected month.
- Unknown tokens, malformed dates, unsupported fragments, extra path
  components, or unsupported route families fall back to target Home.
- The switch handler must not accept arbitrary URLs, paths, query strings, or
  fragments.

Known Calendar fragment generation such as `#calendar-day-detail` is not part
of the accepted `next` token. If RFC-073 needs it, the switch handler should
generate that fragment only for a validated Calendar day-detail destination.

## Testing Expectations

- Unit tests for every accepted `next` grammar.
- Unit tests for unsafe/unknown `next` fallback.
- Browser smoke for switching from Calendar, My Page, member management, and
  create event.
- Browser smoke or focused source gates for representative fallback-only pages
  such as Event Detail.
- No-JS switcher submit path remains valid.

## Open Questions

- Should a future helper derive `next` values from route definitions to reduce
  drift?
