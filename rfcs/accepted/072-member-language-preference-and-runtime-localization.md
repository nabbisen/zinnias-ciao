# RFC 072 - Member Language Preference and Runtime Localization

**Status.** Accepted 2026-07-30 — design completed and architecture-reviewed;
all five previously deferred questions resolved. Authorizes implementation in
Slices A–C; Slice D is explicitly a future RFC.

**Target release.** Next unreleased increment on `main`; not tied to a version
transition.

**Tracks.** Localization, member settings, accessibility, i18n, UX copy.

**Touches.** Me/profile settings, render locale resolution, i18n contracts,
HTML shell language, membership preference data, release gates, docs.

## Summary

Define the user-facing language-selection feature that was intentionally
deferred by RFC-026.

The project already has an i18n scaffold: user-visible strings are collected in
EN/JA constants and parity tests require both languages to exist. Current SSR
rendering, however, is effectively Japanese-only at runtime. Most handlers use
`JA_*` constants directly, and the HTML shell renders `lang="ja"`.

This RFC brings the deferred runtime-localization work under project control
and, as of the 2026-07-30 design completion, specifies it end to end: the locale
seam, the membership-scoped schema, the no-JS settings form, and the order in
which pages migrate. All five questions it previously deferred to review are
resolved in Resolved Decisions.

The work is sequenced so the language switcher only becomes reachable once
enough of the member-facing UI honours it — see First Localized Boundary.

## Background

RFC-026 is marked implemented for the localization scaffold, not for runtime
language selection:

- EN/JA string constants exist under `packages/contracts/src/i18n/`.
- i18n parity gates require non-empty EN/JA string pairs.
- UI code currently selects Japanese strings directly.
- The HTML shell currently uses `lang="ja"` (`render/shell.rs:13`).
- No user, membership, or community language-preference schema exists.
- No settings page lets a member choose a UI language.

**Measured 2026-07-30, and it reframes this RFC's central question.** There are
**254 `JA_*` constants and 254 `EN_*` constants** — exact parity, spanning every
surface (`access`, `admin`, `calendar`, `community`, `events`, `export`,
`general`, `home`, `me`, `notes`, `templates`). And **exactly zero `EN_*`
constants are referenced anywhere in `workers/ssr`.**

So the English copy for the whole application already exists and is already
maintained under a parity gate. Nothing is missing but the accessor boundary:
handlers name `i18n::JA_*` directly, so there is no seam at which a locale could
be honoured. This RFC is therefore **not a translation project**. It is a
refactor that introduces one seam, plus the schema and settings UI to drive it.

**Precision added 2026-07-30 after the Slice B review.** The paragraph above is
right about *string constants* and was imprecise about *date and time
formatting*, which is computed rather than stored as a constant and so is not
covered by the parity gate. The actual state:

- `tz::date_label_en` **exists** and is tested, in the same unreachable
  condition as the 254 string constants — called from nowhere in the worker.
- It is **not at parity** with `tz::date_label_ja`: the Japanese form is
  `{m}月{d}日（{wd}）` including the weekday, the English form is `{d} {mon}`
  without it, because `weekday_en` does not exist (only `weekday_ja`).
- There is **no English form at all** for the `{year}年{month}月` month headers
  composed inline on Calendar and Matrix, nor for the composed Japanese
  aria-label sentences in `communities/matrix/cells.rs`.

This is still not a translation project — the gap is seven weekday strings, a
month-name helper, and routing three call sites through the locale. But it is
real work that the "copy already exists" framing would have let someone skip, and
it lands in **Slice C**, because the switcher cannot honestly be exposed while an
English member sees Japanese dates.

**Date format decision (2026-07-30).** An English date must **never** be
all-numeric: `08/03/2026` is ambiguous between month-first and day-first readers,
and in a scheduling application the failure mode is a member arriving on the
wrong day. That makes it a safety property, not a style preference. The month is
always spelled or abbreviated. Day labels extend `date_label_en`'s established
shape with the weekday (`Mon, 3 Aug`); month headers use the full month name
(`August 2026`), which needs a `month_name_en` beside the existing
`month_abbr_en`.

That distinction changes the sizing, and it changes the answer to "how much
handler refactoring is acceptable" — see Implementation Slices.

Recent work added a self display-name edit flow under My Page. That makes My
Page the natural place to discuss member preferences, but language preference
needs its own design because it affects every page render and every future
copy-review workflow.

## Problem

The app is prepared for translation maintenance but not for multilingual use.

Without a runtime language model:

- English constants are mostly a scaffold, not reachable UI;
- `html lang` cannot reflect the rendered language;
- future user settings might add ad hoc preferences without a clear locale
  precedence rule;
- tests can prove string parity but not that the app renders the selected
  language;
- Japanese copy review and English copy changes can drift in purpose;
- support for multilingual communities remains ambiguous.

The project needs a controlled RFC before adding schema or UI.

## Goals

- Define whether language preference is member-scoped, user-scoped,
  community-scoped, or a combination.
- Define a deterministic locale resolution order.
- Add a discoverable settings path for a signed-in member to change their UI
  language.
- Make rendered `html lang` match the chosen language.
- Keep no-JS form behavior.
- Preserve community isolation.
- Reuse existing i18n constants and parity gates where practical.
- Avoid machine translation and arbitrary admin-provided UI templates.
- Keep Japanese and English copy safety equivalent for warnings, errors, and
  destructive confirmations.

## Non-Goals

- No machine translation of event text, notes, comments, or admin-entered
  content.
- No per-message mixed-language rendering.
- No community-specific arbitrary UI templates.
- No change to internal enum values or database status codes.
- No language-dependent business rules.
- No browser-only language switch that bypasses server rendering.
- No full redesign of every page in the first slice.
- No support for more than the reviewed language set until EN/JA runtime
  selection is stable.

## Decision

The first slice is membership-scoped runtime language preference.

Add a no-JS member setting:

```text
GET  /c/:cid/me/language
POST /c/:cid/me/language
```

**Route corrected 2026-07-30.** An earlier draft proposed
`/c/:cid/me/settings/language`. The codebase has no `settings/` segment: member
routes are flat and kebab-cased — `me`, `me/display-name`, `me/calendar`,
`me/calendar/regenerate`, `me/calendar/revoke` (`handlers/community.rs`
`dispatch_get`/`dispatch_post`). Introducing a one-member `settings/` tier for a
single page would be inconsistent for no benefit. If a settings *index* is ever
warranted, moving flat routes under it is a separate, deliberate change.

The preference should apply to the current signed-in member's experience in the
current community. This matches the current membership-centered data model:
display name, role, access, calendar feed, and member settings are all tied to
`community_memberships`.

The first reviewed language set is:

```text
ja
en
```

The implementation must use stable locale codes, not display names, as stored
values. There is no community default language in the first slice, and no
global user preference in the first slice.

## First Localized Boundary

The first implementation must not set `html lang="en"` on pages that still
render Japanese UI strings.

**Decision, 2026-07-30 — the user-visible setting ships with enough coverage to
be honest, not with the first migrated page.** An earlier reading of this section
would have shipped a language switcher that only My Page and the settings page
honoured. A member who selects English and then finds Home, Calendar, and Event
Detail still in Japanese has been made a promise the product does not keep, and
that is worse than not offering the choice yet. Because all 254 English strings
already exist, the constraint is refactor sequencing, not translation — so the
honest sequence is available at ordinary cost.

The mechanism, the schema, and the migration therefore land **before** the
switcher is reachable. See Implementation Slices: the setting page is built in
Slice A and routed, but is not linked from My Page until Slice C, when the
member-facing core renders localized. Slices A and B are shippable and reviewable
without exposing a half-honoured preference.

The first localized authenticated surface is:

- My Page (`/c/:cid/me`);
- language settings page (`/c/:cid/me/language`);
- shared navigation/header labels needed by those pages;
- the language-setting no-JS form and its success/error states.

Pages outside that boundary may continue to render Japanese and keep
`html lang="ja"` until they are explicitly migrated. The renderer must make the
selected locale an explicit input for localized pages instead of changing the
global shell default for every route at once.

Anonymous and static pages are deferred in the first slice:

- `/join` remains deployment-default Japanese;
- `/relink` remains deployment-default Japanese;
- static offline HTML remains `lang="ja"`;
- no `Accept-Language` behavior is implemented in the first slice.

## Locale Resolution

The long-term precedence order is:

1. Active membership language preference, if set.
2. Community default language, if a future RFC adds one.
3. Browser `Accept-Language`, only for anonymous/public routes where no
   membership preference exists.
4. Deployment default, currently Japanese.

The first slice implements only:

1. active membership preference;
2. Japanese fallback.

However, the code shape should not block later community defaults or
`Accept-Language` handling.

## Data Model Contract

Add a nullable membership-scoped preference:

```sql
ALTER TABLE community_memberships
ADD COLUMN ui_language TEXT
CHECK(ui_language IN ('ja', 'en') OR ui_language IS NULL);
```

Rules:

- no backfill writes are required for existing memberships;
- `NULL` means Japanese fallback in the first slice;
- submitted values outside `ja`, `en`, or the reviewed clear-to-default value
  are rejected without writing;
- unsupported stored values must not be silently rendered if introduced by a
  future bug or manual repair; defensive read code should fall back safely and
  make the condition testable or observable in review.

Rejected first-slice alternatives:

### User-scoped preference

`users.ui_language` is deferred.

Reason:

- the project does not yet have a full global account profile model;
- invite-era users and relink behavior need careful review;
- per-community language needs may be harder.

### Community default only

Community-level default language is deferred.

Reason:

- it does not solve individual member preference;
- admin choice could make UI harder for some members;
- changing a community default could be surprising;
- it can be added later to the locale resolution order without blocking the
  membership preference.

## POST Contract

POST `/c/:cid/me/language` must:

1. Require a valid session.
2. Require active membership in `:cid`.
3. Treat submitted fields as attacker-controlled.
4. Accept only:
   - `ui_language=ja`;
   - `ui_language=en`;
   - a reviewed clear-to-default value, if the UI supports clearing.
5. Reject unsupported values without writing.
6. Consume a form token with purpose:

```text
CHANGE_UI_LANGUAGE
```

Declared alongside the existing purposes in
`packages/contracts/src/auth.rs::token_purpose`, following that module's
convention of a screaming-snake constant whose value is snake_case:
`pub const CHANGE_UI_LANGUAGE: &str = "change_ui_language";`. `CHANGE_DISPLAY_NAME`
(RFC-070) is the precedent to copy for the whole flow — same membership binding,
same no-JS POST-and-303 shape.

7. Bind that token to the active `membership_id`.
8. Write only the current active membership row:

```sql
id = <active membership id>
community_id = <cid>
user_id = <authenticated user id>
removed_at IS NULL
```

9. Treat same-value submission as a no-op.
10. Store deterministic replay result refs for consumed outcomes:
    - `ui_language_updated`;
    - `ui_language_unchanged`;
    - `ui_language_cleared`, if clear-to-default is supported.
11. Redirect back to My Page or the language settings page with a fixed flash
    code.

Audit decision for the first slice:

- language change does not require an audit row unless implementation review
  finds a stronger operational need;
- if audit is added, metadata must contain only the stable locale code or an
  explicit clear marker, never raw submitted text or localized labels.

This POST must cite the RFC-071 form-security baseline during implementation
review.

## Rendering Contract

The implementation should introduce a small locale boundary before broad
handler edits:

- define a small `UiLanguage` or `Locale` type in `packages/contracts`;
- validate and parse only `ja` and `en`;
- expose an i18n accessor boundary so localized render code does not grow
  repeated `match locale { Ja => JA_*, En => EN_* }` branches at every call
  site;
- make localized page rendering pass the locale into the shell;
- make `html lang` come from the same locale as the rendered strings;
- keep non-migrated pages on the Japanese shell until their strings are
  actually locale-selected.

Error pages that cannot determine a membership preference should remain
deployment-default Japanese in the first slice.

## Settings UX

The language setting should be discoverable from My Page near other member
profile/settings actions.

The settings form should:

- show language-native choices such as `日本語` and `English`;
- make the current selection visible;
- work without JavaScript;
- use a normal POST and 303 redirect;
- after a successful change, render the destination in the selected language if
  that destination is inside the first localized boundary;
- optionally support clearing the preference only if the clear-to-default value
  and copy are reviewed.

## Security and Privacy

- The preference is not secret, but it is personal data and should not be
  unnecessarily included in audit metadata or exports.
- POST must require a valid session and active membership in `:cid`.
- The submitted locale is attacker-controlled and must be validated against a
  small allow-list.
- The form must use a purpose-bound form token.
- Hidden fields must not determine the target membership.
- If changing language is audited, metadata should contain only the new stable
  locale code or possibly no metadata, depending on review.
- Error and warning translations must preserve safety meaning across languages.
- **No authorization, validation, or error-classification decision may branch on
  locale.** Locale selects rendered text and nothing else. A localized string may
  differ; the decision that produced it must not. This is the invariant that
  keeps localization out of the security surface, and Acceptance Criterion 8
  restates it as a gate.
- The stored locale is read on every localized render, so a value outside the
  allow-list must fail safe to Japanese rather than panicking — a panic in a
  render path is an SEC-5 violation.
- This RFC cites the RFC-071 form-security baseline. Implementation review must
  identify the affected assets, actors, trust boundaries, controls, and
  evidence for the language-settings form.

## Testing Expectations

Candidate gates:

- domain/contract tests for accepted locale codes and fallback behavior;
- i18n parity tests continue to require EN/JA keys;
- render tests for `html lang`;
- release gates ensuring handlers no longer hardcode `JA_*` on localized pages;
- form-token and authorization gates for the language settings POST;
- migration test or source gate for the `ui_language` closed-set constraint;
- replay tests for same-value and duplicate-submit behavior;
- browser smoke for changing language and seeing navigation, My Page, and one
  form render in the selected language;
- no-JS smoke for the language form;
- 200% text check because English/Japanese label lengths differ.

## Relationship to RFC-026 and RFC-054

RFC-026 remains the scaffold and plain-language localization foundation.
RFC-072 is the runtime preference and rendering-selection layer.

RFC-054 remains important. Runtime language selection should not reduce the
need for Japanese copy review. It increases the need to keep Japanese and
English safety copy aligned.

## Compatibility and Migration

One additive migration, `migrations/0011_membership_ui_language.sql` (0010 is the
current head). The column is nullable with no backfill, so every existing
membership row keeps rendering Japanese with no write. Rollback is dropping an
unread column.

No existing route changes shape. No enum value, status code, or stored business
value is localized — locale codes are a new, separate vocabulary. The parity gate
continues to require both languages for every key, unchanged.

Non-migrated pages keep `lang="ja"` and Japanese strings until their slice lands,
so a partially-migrated tree is always internally consistent per page.

## Operational Considerations

No binding, secret, hosted resource, or Durable Object. One D1 column, read on
render for localized pages, written only by the settings POST.

The locale read joins the existing membership lookup that localized pages already
perform — it must not introduce a second query. This matters against the RFC-029
/ RFC-044 per-route D1 budgets, which the static await-count gate enforces.

## Alternatives Considered

**User-scoped (`users.ui_language`) instead of membership-scoped.** Deferred, and
the RFC's existing reasoning stands: there is no global account profile model
yet, and invite-era and relink semantics need their own review. Membership scope
also matches every other member-level preference in the schema.

**Community default language only.** Deferred — it does not solve individual
preference, and an admin choosing for members can make the UI *less* usable for
some of them. It slots into the resolution order later without rework.

**`Accept-Language` in the first slice.** Deferred. It is the right answer for
anonymous routes (`/join`, `/relink`), but those are outside the first boundary,
and implementing header negotiation before any authenticated page is localized
would add a second resolution path with nothing to test it against.

**Clear-to-default in the first slice — rejected**, see Resolved Decisions.

**A translation framework / message catalogue with runtime lookup by key.**
Rejected for now. The 254 constant pairs are compile-time checked and gate-parity
enforced; swapping to runtime key lookup would trade a compile error for a
missing-key defect at render time, on a service with no hosted error visibility
yet. Revisit if a third language is ever added.

## Resolved Decisions

The five questions this RFC previously deferred to review are resolved here.
Implementers must not reopen them.

**1. Scope of the first localized boundary.** My Page, the language settings
page, and the shared navigation/header those two need — as already written. But
the *switcher is not exposed* until the member-facing core is localized; see the
decision in First Localized Boundary and Slice C below.

**2. Clear-to-default: not in the first slice.** Members choose explicitly
between `日本語` and `English`. With exactly two languages, "clear to default" is
behaviourally identical to "choose Japanese" for every current member, so it buys
nothing while adding a third form value, a third result ref, a third copy string,
and a persistent `NULL`-versus-`'ja'` ambiguity that every read path would have to
carry. `NULL` still means Japanese for existing rows — the RFC simply offers no UI
route back to it. Clearing becomes meaningful only once a community default
exists, and should be designed with it.

**3. No audit row for a language change.** This is a personal display preference:
no security consequence, no safety consequence, not visible to other members, and
of no operator value during an incident. The audit stream exists for accountable
and incident-relevant actions (RFC-014, RFC-052, RFC-079); adding a row per
preference toggle dilutes it and creates a personal-data retention question for
no benefit. If a future need appears, metadata carries the stable locale code and
nothing else — never a localized label or raw submitted text.

**4. Handler refactoring is bounded by the accessor boundary, not by page count.**
The mechanism must make migrating a page a mechanical, single-pass substitution
(`i18n::JA_X` → a locale-aware accessor). No page may need touching twice. Given
that, the slice plan below migrates by surface rather than rationing pages.

**5. RFC-071 is not a blocker — it is a shipped prerequisite.** That question is
moot: RFC-071 landed at `1b12d96` and is in `rfcs/done/`. Its form-security
baseline is active and this RFC's POST must be reviewed against it, as the
Security section already requires.

## Implementation Slices

Four slices, each independently reviewable. **Slices A–C are this RFC; Slice D is
a future RFC and is listed only to show where the boundary falls.**

- **Slice A — the seam.** `Locale` type in `packages/contracts` parsing only `ja`
  and `en`; the i18n accessor boundary; locale as an explicit input to the shell
  so `html lang` and the strings come from one source; migration 0011; the
  resolution function (membership preference, else Japanese); the settings
  `GET`/`POST` with its form token. My Page migrated as the proof the seam works.
  **Not linked from anywhere yet.**
- **Slice B — the member-facing core.** Migrate Home, Calendar (all three views),
  Event Detail, and the shared error/flash surfaces. This is the coverage that
  makes the preference honest.
- **Slice C — expose it.** Link the setting from My Page, browser smoke for
  changing language and seeing it applied, no-JS smoke, and the 200%-text check
  that matters because English labels are frequently longer than Japanese.
- **Slice D — future RFC.** Admin surfaces, anonymous routes (`/join`,
  `/relink`), static offline HTML, `Accept-Language`, community default.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A member selects English and most pages stay Japanese | **Was high** | Broken product promise | Slice ordering: the switcher is not reachable until Slice C |
| `html lang` and rendered strings drift apart | Medium | Screen readers announce the wrong language — an accessibility defect, not cosmetic | Both come from one locale value passed into the shell; render gate asserts it |
| The locale read adds a query per render | Medium | D1 budget regression | Join the existing membership lookup; the await-count gate covers the budgeted routes |
| An unsupported locale reaches storage | Low | Render-time fallback or panic | `CHECK` constraint, allow-list on write, defensive fallback on read, all tested |
| English copy longer than Japanese breaks layout at 200% | Medium | Accessibility regression (RFC-011) | Slice C's 200% check |
| Migration touches a page twice | Medium | Slice churn, review fatigue | Requirement 4 above: the accessor must make it single-pass |

## Acceptance Criteria

1. A `Locale` type in `packages/contracts` parses exactly `ja` and `en` and
   rejects everything else; no locale value reaches a render path unvalidated.
2. `migrations/0011_*` adds a nullable `ui_language` to `community_memberships`
   with a closed `CHECK` set; no existing row is written.
3. `POST /c/:cid/me/language` requires a valid session and active membership in
   `:cid`, consumes a `change_ui_language` form token bound to the active
   `membership_id`, and writes only that membership row.
4. Submitted values outside the allow-list are rejected without writing; a
   same-value submission is a no-op; replayed tokens are detected via
   `ConsumeResult`.
5. No hidden form field determines which membership is written.
6. For every localized page, `html lang` and the rendered strings derive from the
   same resolved locale.
7. A stored value outside the allow-list falls back safely at render time rather
   than panicking, and that fallback is tested.
8. **No authorization, validation, or error-classification decision branches on
   locale** — only the text rendered for it.
9. Non-migrated pages continue to render Japanese with `lang="ja"`, and the
   language setting is not reachable from the UI until Slice C.
10. Locale resolution adds no additional D1 query to any budgeted route.

## Implementation Boundaries

Expected to change: `packages/contracts` (the `Locale` type, `token_purpose`,
the i18n accessor), `render/shell.rs`, the handlers migrated in each slice,
`migrations/0011_*`, the settings handler, and the corresponding gates and smoke.

Expected **not** to change: any enum value, status code, or stored business
value; any authorization mechanism; the existing 254 constant pairs or the parity
gate; the form-token mechanism itself; any route's shape other than the two new
ones.

**Stop and escalate** if localizing a page appears to require changing a business
rule, branching authorization on locale, adding a query to a budgeted route,
touching an already-migrated page a second time, or storing a display label
rather than a stable code.

## Release Implications

Additive; no version-transition decision. Closes no architecture finding and does
not affect the deferred RFC-050 hosted evidence. Evidence is local. The feature
becomes user-visible only at Slice C, so Slices A and B can land without any
release-note claim about language support.
