# RFC 072 - Member Language Preference and Runtime Localization

**Status.** Proposed  
**Target release.** Future candidate  
**Tracks.** Localization, member settings, accessibility, i18n, UX copy.  
**Touches.** Me/profile settings, render locale resolution, i18n contracts,
HTML shell language, membership/user preference data, release gates, docs.

## Summary

Define the user-facing language-selection feature that was intentionally
deferred by RFC-026.

The project already has an i18n scaffold: user-visible strings are collected in
EN/JA constants and parity tests require both languages to exist. Current SSR
rendering, however, is effectively Japanese-only at runtime. Most handlers use
`JA_*` constants directly, and the HTML shell renders `lang="ja"`.

This RFC brings the deferred runtime-localization work under project control.
It does not implement language switching immediately. It records the first
runtime-localization slice and the constraints that must hold before schema or
handler work begins.

## Background

RFC-026 is marked implemented for the localization scaffold, not for runtime
language selection:

- EN/JA string constants exist under `packages/contracts/src/i18n/`.
- i18n parity gates require non-empty EN/JA string pairs.
- UI code currently selects Japanese strings directly.
- The HTML shell currently uses `lang="ja"`.
- No user, membership, or community language-preference schema exists.
- No settings page lets a member choose a UI language.

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
GET  /c/:cid/me/settings/language
POST /c/:cid/me/settings/language
```

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

The first localized authenticated surface is:

- My Page (`/c/:cid/me`);
- language settings page (`/c/:cid/me/settings/language`);
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

POST `/c/:cid/me/settings/language` must:

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

## Open Questions for Review

- Should the first localized authenticated boundary include only My Page and
  language settings, or should it include additional nearby pages?
- Should clear-to-default be included in the first slice, or should members
  choose explicitly between `ja` and `en`?
- Should language change remain unaudited as proposed, or should it write a
  minimal audit action?
- How much handler refactoring is acceptable in the first implementation?
- Should this wait until after RFC-071 threat-model release, or can it be a
  later near-term user-settings theme?
