# RFC 075 - Render Style System and Inline Style Reduction

**Status.** Accepted 2026-07-30 — design completed and architecture-reviewed;
both open questions resolved. Authorizes incremental per-surface migration; the
terminal CSP tightening ships only when inline `style=` reaches zero.

**Target release.** Next unreleased increment after `0.60.0`; the terminal CSP
tightening is a later release and is called out separately below.

**Tracks.** Maintainability, CSS, render architecture, UI consistency, **and
Content-Security-Policy hardening** — see Security Considerations.

**Touches.** `workers/ssr/static/app.css`, render helpers, handler HTML strings,
`lib.rs`'s CSP header (terminal slice only), browser smoke, release checklist.

## Summary

Reduce inline CSS in server-rendered Rust HTML by introducing reusable CSS
classes and a small style system for common UI primitives.

The current app is server-rendered Rust string HTML with many inline `style=`
attributes. Inline styles helped early velocity, but they now create a
maintenance risk:

- repeated values drift across handlers;
- design-token use is inconsistent;
- visual changes require Rust edits;
- responsive and large-text fixes are harder to apply globally;
- review diffs mix behavior and presentation.

This RFC brings style maintenance under project control.

## Background

The project already has `workers/ssr/static/app.css` with design tokens and a
few global rules. However, most page and component styling lives in inline
attributes inside Rust render strings.

**Measured 2026-07-30, and the numbers are worse than "inconsistent":**

| | |
|---|---|
| Inline `style=` occurrences in `workers/ssr/src` | **477** |
| Hardcoded hex colours in Rust | **356** |
| *(counting method note — see below)* | |
| `--cz-*` design tokens defined in `app.css` | 35 |
| Token references (`var(--cz-…)`) **from Rust** | **0** |
| Token references from `app.js` | 0 |
| Token references anywhere (all inside `app.css` itself) | 8 |

So the design system is **decorative**. Thirty-five tokens are defined, roughly
twenty-seven are referenced nowhere at all, and the Rust code that renders
essentially the entire UI consumes none of them — it hardcodes 356 hex values
instead. The Summary's "design-token use is inconsistent" understates this: token
use in the rendering layer is zero.

**Counting method.** The two counts above use a `style=\"`-literal grep. The
ratchet gates installed in Slice 1 walk the filesystem with a broader counter and
report higher figures for the same tree. Both are internally consistent; **the
gate's own function is authoritative** for the ratchets. Do not reconcile these
numbers against each other or lower a pin to match this prose.

The trend is also wrong. `lib.rs`'s CSP comment records "~272 occurrences" of
inline `style=`; the count is now **477**. Inline styling has nearly doubled
since someone last wrote that number down, which means this is not a stable
legacy to migrate at leisure — it is actively accumulating.

This app is not currently using idiomatic Leptos component styling. The active
surface is server-rendered HTML assembled by Rust handlers and render helpers.
Therefore the practical first step is not a Leptos rewrite; it is a CSS/class
migration strategy for the existing SSR renderer.

## Goals

- Move stable visual styling from inline attributes into CSS classes.
- Keep dynamic state styling explicit where needed.
- Introduce reusable classes for common page, header, tab, button, form, list,
  calendar, and table patterns.
- Reduce repeated color/spacing/radius values in Rust strings.
- Improve reviewability by separating behavior and presentation changes.
- Preserve no-JS behavior.
- Preserve existing mobile and 200% text guarantees.

## Non-Goals

- No frontend framework rewrite.
- No visual redesign as part of the first slice.
- No requirement to remove every inline style at once.
- No Tailwind or CSS-in-JS introduction.
- No breaking existing smoke scripts by changing data attributes.

## Proposed Approach

Use incremental migration by surface.

First slice:

1. Define base classes in `app.css`:
   - page layout;
   - section layout;
   - buttons/links;
   - tabs;
   - form fields;
   - status/alert text;
   - calendar grid;
   - matrix table shell.
2. Migrate Calendar-related surfaces touched by RFC-073:
   - view tabs;
   - month navigation;
   - month grid;
   - selected-day detail;
   - Events list;
   - Matrix view shell and tab links only where touched by the tab system.
3. Keep data attributes used by JavaScript and smoke scripts stable.
4. Keep inline styles only for truly dynamic values:
   - selected/unselected state when class explosion would be worse;
   - computed colors from status summaries until converted to classes;
   - generated table dimensions where needed.
5. Add release gates or static checks only after patterns are established.

Out of scope for the first slice:

- Event Detail;
- My Page;
- admin event forms;
- member management;
- join/relink/help-signin;
- global header and bottom navigation, except where a Calendar-specific class
  is needed for layout compatibility.

## Class and Helper Contract

All app-owned classes introduced by this RFC must use the `cz-*` prefix. This
matches the existing `--cz-*` design-token prefix in `app.css`.

Examples:

```text
cz-page
cz-section
cz-tabs
cz-tab
cz-calendar-grid
cz-calendar-day
cz-calendar-day-detail
cz-event-list
cz-matrix-scroller
```

The first slice should prefer plain `class` attributes in existing render
strings. New render helper abstractions are allowed only when narrowly scoped
to the migrated surface and clearly reduce duplication. Broad generic helpers
such as `button_link`, `tabs`, or `page_section` are deferred until the class
patterns have been proven on one surface.

## Preservation Contract

The first slice must preserve behavior-sensitive markup:

- form names, methods, and actions;
- route href semantics;
- ARIA labels, landmarks, `aria-current`, and table semantics;
- JavaScript data attributes;
- smoke-test data attributes;
- matrix CSV export attributes.

Calendar/matrix attributes that must not be removed or renamed without a
separate reviewed behavior change include:

```text
data-rfc067-matrix-scroller
data-calendar-matrix-export
data-calendar-matrix-export-button
data-calendar-matrix-export-status
data-audit-url
data-month
data-export-type
data-token
data-filename
data-date
data-member-name
data-export-value
```

The CSS migration should be visually conservative. It should not intentionally
redesign Calendar while extracting styles.

## Testing Expectations

- `mdbook build docs` only if docs are touched.
- Rust tests for render helpers if class-generating helpers are introduced.
- Browser smoke for any migrated surface at mobile width and 200% text.
- Screenshot evidence for migrated Calendar surfaces at mobile width and 200%
  text, even if RFC-075 is implemented separately from RFC-073.
- Matrix CSV smoke or focused source gates proving data attributes used by CSV
  export remain present.
- Static count of inline `style=` may be tracked as an informational metric,
  not an immediate blocking gate.

## Security Considerations

**This RFC has a security deliverable the original draft did not name.**
`workers/ssr/src/lib.rs` currently sends:

```text
style-src 'self' 'unsafe-inline'
```

with a comment saying the SSR templates use inline `style=` pervasively and that
"removing them requires a full CSS extraction pass; tracked for a future RFC."
**RFC-075 is that future RFC.**

`'unsafe-inline'` in `style-src` is the one meaningful weakening in an otherwise
strict policy — every other directive is locked down (`default-src 'self'`,
`script-src 'self'`, `object-src 'none'`, `base-uri 'none'`,
`frame-ancestors 'none'`). It permits injected style attributes and style
elements, which enables CSS-based exfiltration and UI-redressing techniques that
a strict policy would block. It does not by itself create an XSS, and this
codebase escapes rendered output (`render::escape_html`), so **the current
exposure is a weakened mitigation layer, not a live vulnerability** — the same
honest framing used for the `0.60.0` cache-key drift.

The endgame is therefore concrete: **when inline `style=` reaches zero, drop
`'unsafe-inline'` and gate its absence.** That is the terminal slice, not the
first one, and it is the reason the migration must be sequenced to actually
finish rather than stall at the interesting surfaces. A migration that removes
400 of 477 inline styles delivers maintainability but **zero** security benefit,
because the directive can only be dropped at zero.

No other security property changes. Classes carry no authorization meaning, and
no rendered decision may depend on a class name.

## Accessibility is a hard constraint, not a nice-to-have

`release_gates.rs:2378–2385` protects a real WCAG property on **exactly the
surface slice 1 targets**:

> "Event presence must use a visible marker, not color alone"
> "Today styling must stay calmer than selected-day styling and distinct from
> ordinary event days"

It does so by pinning `#FAFAFB`, `#6E6E73`, and the inline
`border:{border_width} solid {border}` construction. A class migration will break
those literals.

**The gate must be re-expressed, never deleted.** Assert the property against the
rendered class set — that a today cell and a selected cell carry distinguishable
classes, and that event presence is conveyed by a marker element rather than a
colour rule alone. This is the same move made for the RFC-067 gate during RFC-074:
keep the guarantee identical, change only how it is proven. Deleting an
accessibility gate to make a CSS refactor pass would be the worst possible
outcome of this RFC.

## Compatibility and Migration

No schema, route, form, or data change. Presentation only.

Every behaviour-sensitive attribute listed in the Preservation Contract stays
byte-identical. Because migration is per-surface, a partially-migrated tree is
always internally consistent — a page either uses classes or inline styles, never
a broken mix that renders wrong.

`app.css` grows; `app.js` is untouched. Note that `app.css` is a cached asset
covered by the `0.60.0` cache-drift gate, so **every slice must move the cache
key** — the gate will say so, but it prompts rather than enforces.

## Operational Considerations

None. No binding, secret, migration, or hosted resource. One caveat worth
stating: moving bytes from inline attributes into `app.css` moves them from a
per-response cost to a once-cached cost, which is a small win on repeat views and
a small loss on first paint. Not worth measuring at this app's size.

## Alternatives Considered

**Adopt Tailwind or a CSS-in-JS system.** Rejected, and already a Non-Goal. Both
would add a build step to a Worker that currently ships static CSS, and Tailwind's
utility classes reproduce the "styling lives in the markup string" problem this
RFC exists to solve.

**Rewrite the render layer into idiomatic Leptos components first.** Rejected for
now — the Background is right that the active surface is Rust string HTML, and a
framework migration is a far larger, riskier change that would not by itself
remove one inline style.

**Big-bang extraction of all 477 in one package.** Rejected: unreviewable, and it
would touch every handler simultaneously while the accessibility gates above are
being re-expressed.

**Leave it and stop adding to it (freeze-only).** Tempting given the doubling, and
the ratchet below captures most of its value — but on its own it never reaches
zero, so the CSP directive stays forever. Adopted *as part of* the plan, not
instead of it.

## Resolved Decisions

**1. Calendar UX and Calendar CSS are two releases — already settled by events.**
RFC-073 shipped the Calendar UX work in `0.60.0` (`ed549be`). This RFC's Calendar
CSS migration therefore lands separately by fait accompli, which was the better
answer anyway: it keeps a behaviour change and a presentation change in different
reviewable units, which is one of this RFC's own stated goals.

**2. Not a threshold — a ratchet, plus a terminal gate.** "What threshold should
become a gate?" invites arguing about a number that will be wrong. Two objective
gates instead:

- **Inline-style ratchet.** Pin the current count (477 at the time of writing;
  re-measure when the first slice lands) and assert it never *increases*. Cheap,
  unarguable, and it stops the doubling that got us here. Each slice lowers the
  pin.
- **Hardcoded-colour ratchet.** Same shape on the 356 hex literals in Rust, so
  new code reaches for a token rather than a hex value.

Then the terminal gate: **when inline `style=` reaches zero, drop
`'unsafe-inline'` from `style-src` and assert the CSP no longer contains it.**
That converts a maintainability programme into a security outcome with a
verifiable finish line.

Informational counts, as the draft proposed, are not enough — nothing stops a
number from drifting upward, which is precisely what happened between 272 and 477.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| An accessibility gate is deleted rather than re-expressed | Medium | **WCAG regression on Calendar** | Named above as a hard constraint; the gate must assert the class set |
| Migration stalls partway | **High** — this is the usual fate | Maintainability gain, zero security gain; `'unsafe-inline'` stays forever | The ratchet prevents backsliding; the terminal gate defines "done" |
| A visual regression slips in during "conservative" extraction | Medium | Broken layout for real members | Screenshot evidence at mobile width and 200% text per migrated surface |
| The cache key is stale at **release** time | Medium | Members served stale CSS once deployed | The version bump at release moves `CACHE_VERSION` with it; mid-slice, the re-pinned digest records the content change |
| Class explosion for dynamic state | Medium | Worse than the inline styles it replaces | The draft's rule stands: keep inline for genuinely dynamic values |

## Acceptance Criteria

Per slice, unless marked terminal.

1. Every behaviour-sensitive attribute in the Preservation Contract is unchanged
   — forms, hrefs, ARIA, `aria-current`, table semantics, and every listed
   `data-*`.
2. All app-owned classes use the `cz-*` prefix.
3. The CSS rules a migrated surface uses reference `--cz-*` tokens rather than
   introducing new hardcoded hex values. **Clarified 2026-07-30:** this means the
   `cz-*` rules in `app.css`, not the Rust strings — Rust emits class names and
   should contain no `var(--cz-…)` at all.
4. The Calendar accessibility gate is re-expressed against the rendered class
   set, with its guarantee unchanged and demonstrably still failing when the
   property is violated.
5. The inline-style and hardcoded-colour ratchets exist and are lowered, never
   raised.
6. No rendered authorization, validation, or routing decision depends on a class
   name.
7. Screenshot evidence at mobile width and 200% text for each migrated surface.
8. Any `app.css` change re-pins `cached_asset_content_matches_pinned_hash`.
   **Corrected 2026-07-30 after the Slice 1 review:** an earlier form of this
   criterion said the cache *key* must move in the same commit. It must not, for
   a mid-RFC slice — the key is tied to the workspace version, so moving it forces
   a version bump with no tag and no changelog, which is its own drift. The key
   must be correct at **deploy** time; a release bumps the version and carries
   `CACHE_VERSION` with it. Mid-slice: re-pin the digest, nothing else.
9. **Terminal:** inline `style=` reaches zero, `'unsafe-inline'` is removed from
   `style-src`, and a gate asserts its absence.

## Implementation Boundaries

Expected to change: `app.css`, the render strings of the migrated surface, the
re-expressed gates, the ratchets, and the pinned asset digest.

Expected **not** to change: any handler's behaviour, any form or route, any
`data-*` attribute, `app.js`, any schema, or the CSP header — until the terminal
slice.

**Stop and escalate** if extraction appears to require changing a
behaviour-sensitive attribute, deleting rather than re-expressing an
accessibility gate, or introducing a build step.

## Release Implications

Presentation-only until the terminal slice, which is a genuine security
improvement and should be called out as such in that release's changelog. No
architecture finding is closed by this RFC; B1/B3/B4/B5 are unaffected. Evidence
is local — screenshots and gates, no hosted requirement.

## Open Questions

None. Both former questions are resolved in Resolved Decisions above.
