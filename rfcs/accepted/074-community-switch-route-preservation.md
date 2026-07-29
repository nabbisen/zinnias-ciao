# RFC 074 - Community Switch Route Preservation

**Status.** Accepted 2026-07-29 — design completed and architecture-reviewed;
authorizes implementation in four slices with two review points.

**Target release.** Next unreleased increment on `main`; not tied to a version
transition.

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

## Background

`/switch` already exists and already preserves a subset of routes. As of
`ed549be` the handler accepts `communities`, `communities:…` (month, day,
`list`, `matrix` — extended by RFC-073), `admin_events_new`,
`admin_events_new:YYYY-MM-DD`, `admin_members`, and `admin_invites`, with
everything else falling through to the target community's Home. Admin tokens are
already gated by `is_admin_target` against the **target** community.

So this RFC is not building a mechanism from nothing. It is (a) widening the
accepted set by five tokens, (b) wiring the pages that currently pass nothing, and
(c) making the contract explicit and exhaustively tested. That framing matters
for sizing: most of the work is breadth, not depth.

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
| Copy Event / Recreate Event | target create event only if admin in target; **no** event id preserved |
| Edit/cancel/attendance/admin-note-hide event admin pages | target Home unless a future RFC defines equivalent-event preservation |
| Event Detail | target Home unless an explicit equivalent-event model is designed |
| Note delete confirmation | target Home unless an explicit equivalent-event model is designed |

**Correction, 2026-07-29.** An earlier draft of this matrix grouped Copy Event
and Recreate Event with the edit/cancel/attendance pages as Home fallbacks. That
was wrong, and implementing it literally would have been a **regression**: both
pages already pass `admin_events_new` today, and doing so is correct. They are
create-event flows, not views of an existing event; landing on the target
community's blank Create Event page is the faithful route-family preservation,
it is already admin-gated, and it preserves no event id. They now have their own
row. Do not "fix" them toward Home.

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

**Fragments — settled by RFC-073, not open.** The switch handler emits **no**
fragment, ever. `#calendar-day-detail` is not part of any accepted `next` token
and must not be generated for a validated Calendar day-detail destination
either. RFC-073 decided this and shipped it at `ed549be`: changing community is a
context change, and landing mid-page on a day detail in a different community is
disorienting because that day may hold different events, or none. The user
arrives at the top of the destination with month, day, and view preserved.

This paragraph previously left the question conditional on RFC-073's needs.
RFC-073 has since answered it, and its browser smoke asserts `location.hash === ''`
after a switch. Do not reopen it here.

## Testing Expectations

- Unit tests for every accepted `next` grammar.
- Unit tests for unsafe/unknown `next` fallback.
- Browser smoke for switching from Calendar, My Page, member management, and
  create event.
- Browser smoke or focused source gates for representative fallback-only pages
  such as Event Detail.
- No-JS switcher submit path remains valid.

## Compatibility and Migration

No schema, migration, or data-format change. `next` is a form field, not a
persisted value, so there is no stored state to migrate.

Every currently-accepted token keeps its present meaning. The change is additive:
tokens that fall through to Home today either keep doing so or begin preserving
their route family. No existing URL changes shape, and a stale `next` from a
cached page still parses or safely falls back.

## Security Considerations

**This RFC is primarily a security contract wearing a UX label.** `next` is
attacker-influenceable — it arrives in a form submission — so two properties are
load-bearing and must be preserved rather than newly invented:

1. **No open redirect.** The handler must never treat `next` as a URL, path, or
   fragment. It parses a closed token grammar and *constructs* the destination
   from the validated target community id. Any unknown token, malformed date,
   extra path component, or unexpected shape falls back to the target's Home.
   This is why the grammar is enumerated rather than derived — see Alternatives.
2. **Authorization is re-checked against the target, never inherited from the
   source.** A user who is an admin in community A must not reach an admin page
   in community B where they are only a member. Every `admin_*` token requires an
   active admin role **in the target community**, and every member-level token
   requires active membership there. The existing `is_admin_target` check is the
   correct shape and must be applied to each newly added admin token.

Two further rules:

- **Never preserve a community-scoped identifier across a switch.** Member ids,
  event ids, invite ids, and template ids are meaningless — or worse, refer to a
  different entity — in another community. The route-family matrix already
  reflects this for help-signin and the member-action confirmations; it applies
  to any token added later.
- **Preserved values must be re-validated, not trusted.** `month`, `day`, and the
  `admin_events_new` prefilled day are untrusted input even when they came from a
  page the user just legitimately viewed.

No new secret, binding, or mutation is introduced.

## Operational Considerations

None. No binding, secret, migration, or hosted resource. Switching remains a
single `303` with no additional query.

## Alternatives Considered

**Derive `next` values from route definitions to reduce drift.** This resolves
the RFC's former open question: **rejected.** A derived set makes the accepted
grammar implicit — you would have to read the route table and the derivation
rules together to know what `/switch` accepts, and a new route would silently
become switchable without anyone deciding it should be. For a parameter whose
whole job is refusing untrusted input, an explicit enumerated grammar that a
reviewer can read in one place is worth the drift risk. The drift is bounded by
the exhaustive tests this RFC requires.

**Accept a relative path in `next` and validate it.** Rejected: path validation
is a well-known source of bypasses (encoding, traversal, scheme confusion), and
the token grammar avoids the entire class.

**Preserve event detail across communities by matching equivalent events.**
Rejected and already a Non-Goal — there is no equivalence relation between
events in different communities, and guessing one would show a member the wrong
event.

**Do nothing and always fall back to Home.** This is today's behavior for most
pages and it is safe; it is rejected only because it repeatedly discards work the
user can see, which is the reported complaint.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A new admin token added without a target-side role check | Low | **Privilege escalation across communities** | Every `admin_*` token routes through `is_admin_target`; a source gate asserts no admin arm exists without it |
| Grammar drift as pages are added | Medium | Switcher silently stops preserving, or preserves something unintended | Exhaustive per-token tests; the grammar lives in one place |
| Breadth causes an unreviewably large patch | Medium | Slice stops being independently reviewable | Implementation slices below |
| A community-scoped id leaks into a preserved route | Low | Wrong-entity display or a not-found in the target | Matrix rule plus a test per confirmation-page family |
| RFC-067's literal gate obstructs the switcher refactor | Medium | Forces an awkward implementation shape | Generalize that gate as part of this RFC — see Implementation Boundaries |

## Implementation Slices

The route-family matrix spans roughly fourteen page families, which is too much
for one reviewable package. Implement in this order.

**Two review points, not four.** Slice 1 is the security-critical core and is
reviewed on its own, because that is where an open redirect or a missing
target-side authorization check would live. Slices 2–4 are mechanical breadth
once the grammar is proven, and are reviewed together as one package. Four
review cycles for this theme would cost more than it returns.

1. **Grammar and handler.** Add the five new tokens (`me`, `calendar_feed`,
   `admin_export`, `admin_templates`, and an explicit `home` arm) to the closed
   parser with target-side authorization, plus exhaustive per-token unit tests
   including the fallback cases. No page wiring yet.
2. **Member-level page wiring.** My Page, Calendar feed settings, Home.
3. **Admin page wiring.** Export, Templates, Member management, Invites,
   Create Event, and the help-signin / member-action confirmations that
   deliberately degrade to member management without an id.
4. **Fallback-family gates and browser smoke.** Assert that Event Detail, event
   admin pages, and note-delete confirmation fall back to Home, and add the
   browser smoke the Testing Expectations require.

## Acceptance Criteria

1. `/switch` accepts exactly the enumerated grammar and constructs every
   destination from the validated target community id; it never treats `next` as
   a URL, path, or fragment.
2. Every `admin_*` token requires an active admin role in the **target**
   community; every member-level token requires active membership there.
3. Unknown tokens, malformed dates, a day outside its month, extra components,
   and any fragment fall back to the target's Home without error.
4. No community-scoped identifier (member, event, invite, template) is preserved
   across a switch.
5. The switch handler emits no fragment under any input.
6. Each page in the route-family matrix passes the token the matrix assigns it,
   or deliberately passes none where the matrix says fall back.
7. The no-JS switcher submit path continues to work.
8. Every accepted token and every fallback case has a unit test; the four
   representative families have browser smoke.
9. RFC-067's matrix-preservation contract still holds, asserted against the
   destination the handler produces rather than a source literal.

## Implementation Boundaries

Expected to change: the `/switch` handler and its `next` parser, the per-page
`header_with_switcher_next` call sites named in the matrix, the switcher tests,
one browser smoke, and the RFC-067 gate generalization described below.

Expected **not** to change: any schema or migration, any authorization mechanism
other than applying the existing `is_admin_target` to new tokens, event or
attendance queries, the Calendar render paths RFC-073 just shipped, and any
handler's own behavior once reached.

**Generalize the RFC-067 gate (carried from the RFC-073 review, observation O1).**
`rfc067_monthly_attendance_matrix_contract_is_guarded` currently asserts the
source literal `"&view=matrix"` in the handler. That pins a spelling rather than a
behavior, and it already forced RFC-073 into a more awkward two-`bool` shape than
the author preferred. Since RFC-074 owns the switcher, replace that assertion with
one that exercises `calendar_next_destination` and asserts the **destination URL**
it returns for a matrix token. Keep the RFC-067 guarantee identical; change only
how it is proven.

**Stop and escalate** if preserving a family appears to require trusting a
community-scoped id, accepting a path or URL in `next`, weakening a target-side
authorization check, or changing a destination page's own behavior.

## Release Implications

Additive navigation change behind no feature flag. It closes no architecture
finding, does not affect the deferred RFC-050 hosted evidence, and requires no
version-transition decision. Browser evidence is local.

## Open Questions

None. The former open question — whether a helper should derive `next` values
from route definitions — is resolved in Alternatives Considered: rejected in
favour of an explicit enumerated grammar.
