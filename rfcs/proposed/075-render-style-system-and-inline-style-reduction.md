# RFC 075 - Render Style System and Inline Style Reduction

**Status.** Proposed  
**Target release.** Future candidate  
**Tracks.** Maintainability, CSS, render architecture, UI consistency.  
**Touches.** `workers/ssr/static/app.css`, render helpers, handler HTML strings,
browser smoke, release checklist.

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

## Open Questions

- Should Calendar UX and Calendar CSS migration be one release or two?
- What inline-style threshold should become a future release gate, if any?
