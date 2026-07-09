# RFC 064 - Rust Module and Crate Boundary Cleanup

**Status.** Proposed
**Target release.** v0.52.0 Phase 1 candidate
**Tracks.** Maintainability, Rust workspace structure, SSR Worker internals.
**Touches.** `workers/ssr`, `packages/domain`, `packages/contracts`,
`Cargo.toml`, release gates, project documentation.

## Summary

This RFC defines a careful restructuring path for the Rust codebase. The
current workspace has three crates:

- `packages/domain`: pure business rules and validation.
- `packages/contracts`: shared constants, DTOs, error model, i18n copy, ICS
  formatting, timezone helpers, and release gates.
- `workers/ssr`: Cloudflare Worker runtime adapter, routing, D1 access, auth,
  sessions, form tokens, rate limiting, HTML rendering, and feature workflows.

The primary maintainability issue is not simply the number of crates. The
largest pressure is that `workers/ssr` contains several layers in one crate and
some files combine route handling, application workflow, persistence calls,
HTML rendering, and validation orchestration.

This RFC proposes a staged cleanup:

1. Split oversized SSR modules into clearer internal modules without changing
   crate boundaries.
2. Separate pure rendering helpers from Worker `Response` construction.
3. Clarify the intended API boundaries between domain rules, shared contracts,
   persistence, application workflows, and the Worker adapter.
4. Introduce internal crates only after module boundaries are stable and the
   value is clear.

v0.52.0 is scoped to Phase 1 only. Completing the first admin event module
split does not complete all of RFC-064. It establishes the first reviewed
pattern for behavior-preserving structural cleanup.

## Background

The project guidelines require design-first development and recommend splitting
Rust files above 300 effective lines of code, with a strong recommendation
above 500 effective lines. Current source size evidence shows several files
above that threshold:

| File | Lines observed |
|------|----------------|
| `workers/ssr/src/handlers/admin/events.rs` | 1570 |
| `workers/ssr/src/render.rs` | 723 |
| `workers/ssr/src/handlers/event.rs` | 636 |
| `packages/contracts/src/i18n.rs` | 544 |
| `workers/ssr/src/handlers/communities.rs` | 482 |
| `workers/ssr/src/handlers/export.rs` | 450 |
| `workers/ssr/src/db/membership.rs` | 441 |
| `workers/ssr/src/handlers/admin/members.rs` | 410 |
| `workers/ssr/src/handlers/calendar.rs` | 389 |
| `workers/ssr/src/handlers/community.rs` | 373 |
| `workers/ssr/src/db/attendance.rs` | 343 |
| `workers/ssr/src/handlers/templates.rs` | 324 |
| `workers/ssr/src/handlers/join.rs` | 315 |

This size alone is not the whole problem. The larger design issue is that some
files cross too many boundaries. For example, admin event handling currently
contains route entry points, form parsing, recurrence/edit semantics,
authorization checks, persistence orchestration, HTML builders, and helper
validation in one implementation file.

## Goals

- Make feature files easier to review by reducing oversized files.
- Keep the first implementation slice behavior-preserving.
- Preserve the current external routes, form fields, database schema, and
  browser-visible behavior unless a later feature RFC explicitly changes them.
- Make API boundaries explicit before adding new crates.
- Keep Cloudflare Worker-specific types at the outer adapter and persistence
  edges where practical.
- Improve test organization in line with project rules: implementation files
  should not contain inline test bodies, and `tests.rs` files should remain
  split from implementation.

## Non-Goals

- No immediate rewrite to a framework.
- No immediate replacement of D1 access.
- No route redesign.
- No schema migration solely for this cleanup.
- No change to auth, session, invite, relink, or form-token semantics.
- No new public API surface for external consumers.
- No crate split before the internal module boundaries are reviewed.

## Boundary Model

The cleanup should distinguish two kinds of boundaries.

### API Boundaries

API boundaries are source-level contracts between modules or crates. They can
exist inside one crate. They are the first priority.

Recommended API layers:

| Layer | Responsibility | Must avoid |
|-------|----------------|------------|
| Worker adapter | Fetch entry point, route matching, `Request`/`Response`, headers, env bindings | Business rules and HTML page details |
| Feature workflow | Use-case orchestration such as create event, edit event, relink, member admin | Raw SQL and large HTML strings |
| Persistence | D1 queries, row structs, SQL placeholder handling | HTML rendering and route decisions |
| Presentation | Pure HTML fragments and page bodies where possible | Direct D1 access and auth/session mutation |
| Contracts | Shared DTOs, constants, error model, stable text/copy resources | Worker runtime bindings |
| Domain | Pure validation and business rules | Worker runtime bindings, SQL, HTML |

### Crate Boundaries

Crate boundaries are build and dependency boundaries. They are useful only when
they enforce a stable design constraint or reduce meaningful build/review
complexity.

Initial policy:

- Prefer internal module splits first.
- Add an internal crate only after at least one module group has a clear,
  Worker-free or Worker-specific dependency shape.
- Avoid splitting `packages/domain` now; it is currently small and pure.
- Treat `packages/contracts` as a candidate for later refinement, not the first
  split target.

## Proposed Phases

### Phase 1 - Behavior-Preserving SSR Module Split

Split oversized SSR implementation files while keeping them inside the
`zinnias-ciao-ssr` crate.

Initial target:

- `workers/ssr/src/handlers/admin/events.rs`

Suggested split:

```text
workers/ssr/src/handlers/admin/events.rs              facade module
workers/ssr/src/handlers/admin/events/create.rs       create GET+POST
workers/ssr/src/handlers/admin/events/recreate.rs     recreate GET
workers/ssr/src/handlers/admin/events/edit.rs         edit GET+POST
workers/ssr/src/handlers/admin/events/cancel.rs       cancel GET+POST
workers/ssr/src/handlers/admin/events/attendance.rs   attendance GET+POST
workers/ssr/src/handlers/admin/events/notes.rs        admin note-hide GET+POST
workers/ssr/src/handlers/admin/events/forms.rs        HTML form fragments only
workers/ssr/src/handlers/admin/events/summary.rs      schedule summary HTML
workers/ssr/src/handlers/admin/events/policy.rs       editability, recurrence,
                                                      prefill, validation helpers
workers/ssr/src/handlers/admin/events/support.rs      redirects, query escaping,
                                                      small shared structs
workers/ssr/src/handlers/admin/events/tests.rs        focused tests
```

If Rust module layout makes `events.rs` plus `events/` awkward, use Rust 2018+
module style with a directory module and no `mod.rs` only when it keeps the
tree simple and standard.

Ownership rules:

- `events.rs` is a facade: module declarations plus `pub use` of route handler
  entry points only.
- Only route handlers are public to `handlers/admin.rs`.
- Cross-module helpers are `pub(super)` at most.
- `forms.rs` must not call D1, auth, audit, token, or session helpers.
- `policy.rs` and validation helpers must not construct `worker::Response`.
- `support.rs` may hold small shared utilities and structs, but must not become
  a catch-all for feature workflow logic.
- Tests should import the specific modules or helpers they exercise instead of
  relying on blanket `use super::*`.

Phase 1 acceptance criteria:

- Public handler exports remain unchanged from the caller perspective.
- Route behavior remains unchanged.
- No new crate is introduced.
- Full source gates pass.
- No child module exceeds the project 300-line guideline without a documented
  reason.
- `forms.rs`, `policy.rs`, and `support.rs` follow the ownership rules above.

### Phase 2 - Rendering Boundary Cleanup

Split `workers/ssr/src/render.rs` into smaller modules. The first goal is not
to make a new crate. The first goal is to separate concerns:

```text
workers/ssr/src/render.rs
workers/ssr/src/render/layout.rs
workers/ssr/src/render/nav.rs
workers/ssr/src/render/event.rs
workers/ssr/src/render/forms.rs
workers/ssr/src/render/errors.rs
workers/ssr/src/render/tests.rs
```

Preferred direction:

- Pure helpers return `String`.
- Only a small adapter layer creates `worker::Response`.
- Existing callers can continue using `render::page`, `render::not_found`, and
  similar helpers until a later cleanup migrates them.

Acceptance criteria:

- No visual or route behavior change.
- HTML escaping behavior remains covered.
- `worker::Response` creation is isolated more clearly than today.

### Phase 3 - Contracts Package Review

Review `packages/contracts` after SSR rendering is clearer.

Candidate outcomes:

- Keep it as one crate but split files more clearly.
- Move copy/i18n tables into narrower modules.
- Move pure ICS formatting to a dedicated module or later internal crate if it
  becomes independently useful.
- Keep auth constants and token purposes in contracts unless a stronger auth
  crate boundary emerges.

This phase should not be used to create many small crates without practical
benefit.

### Phase 4 - Optional Internal Crates

Only after phases 1-3, evaluate internal crates. Candidate crate boundaries:

| Candidate crate | Possible contents | Reason to split | Reason not to split yet |
|-----------------|-------------------|-----------------|--------------------------|
| `packages/views` | Pure HTML builders and view helpers | Keeps presentation Worker-free and testable | Current rendering still produces Worker responses |
| `packages/worker-d1` or `packages/cloudflare-d1` | D1 row structs and query functions | Separates SQL from route workflows | It remains Cloudflare-specific and may not reduce dependencies |
| `packages/app` | Feature workflows independent of HTTP | Stronger use-case testing | Current workflows rely heavily on Worker request/form/env types |

The first internal crate, if any, must have a short dependency list and a clear
owner responsibility. A module group may become a crate only when all of these
are true:

- It has a stable public API used by at least two feature areas, or it is
  independently testable with a materially smaller dependency graph.
- It can avoid `worker::{Request, Response, Env}` unless the crate name
  explicitly says it is Worker- or D1-specific.
- It does not require broad `pub` exposure of internals just to cross the crate
  boundary.
- The crate name honestly describes platform coupling. While persistence
  depends directly on `worker::D1Database`, it should remain in `workers/ssr`;
  if extracted later, prefer a Cloudflare-specific name such as
  `packages/worker-d1` or `packages/cloudflare-d1` over a generic
  `packages/persistence`.

## Implementation Rules

- Preserve current route names, redirects, form field names, token purposes,
  audit actions, and database writes unless separately approved.
- Use small, mechanical moves first.
- Keep each change reviewable by behavior area.
- Avoid changing formatting and logic in the same patch when a pure move is
  possible.
- Use existing module naming patterns.
- Keep tests split into `tests.rs` or `tests/` modules rather than inline test
  bodies.
- Run `cargo fmt` after implementation, then the relevant test/build gates.

## Test and Gate Expectations

For behavior-preserving module splits:

- `cargo fmt --all -- --check`
- `cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts -p zinnias-ciao-ssr`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --workspace`
- `cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown`
- `git diff --check`

Browser smoke is not required for a pure module move unless routing, rendering
output, form names, or static assets change. If rendering output is changed,
run the relevant smoke script or request a targeted smoke plan.

## Risks

- Mechanical file moves can hide accidental behavior changes.
- Adding crates too early can increase Cargo/workspace complexity without
  improving architecture.
- Moving rendering helpers without a clear adapter boundary can spread
  `worker::Response` dependencies rather than contain them.
- Splitting persistence into a crate may make testing easier, but it may also
  cement Cloudflare D1 specifics into a broad package name.
- Large diffs may be difficult to review unless changes are phased by feature
  area.

## Review Questions

- Is the first target correctly chosen as `handlers/admin/events.rs`, or should
  `render.rs` be split first?
- Should `packages/contracts/src/i18n.rs` be treated as a structural problem
  in this RFC, or left to RFC-054 Japanese UX copy review?
- What threshold should trigger a new internal crate instead of another module
  split?
- Should persistence remain inside `workers/ssr` while it depends directly on
  `worker::D1Database`?
- Are there boundary names the project should reserve now to avoid confusing
  future developers?

## Open Decisions

- Whether Phase 1 implementation reveals a better filename split than the
  target shape above. Any deviation should be documented before release.
- Whether Phase 2 should split `render.rs` in v0.53.0 or wait for another
  product slice.
- Whether crate extraction remains deferred after Phase 2 or needs a follow-up
  RFC with a narrower target.

## Acceptance Criteria

v0.52.0 Phase 1 is complete when:

- `workers/ssr/src/handlers/admin/events.rs` is split into smaller modules
  without behavior change.
- The facade/re-export surface remains stable for `handlers/admin.rs`.
- Full source gates pass for the moved code.
- No child module exceeds the project 300-line guideline without a documented
  reason.
- No new crate is introduced.
- Release notes describe the change as Phase 1 of RFC-064, not the whole RFC.

RFC-064 overall is complete when:

- Phase 1 is complete.
- Phase 2 rendering-boundary cleanup is completed or an explicit decision is
  recorded to defer it.
- The `packages/contracts/src/i18n.rs` structural question is resolved, either
  by RFC-054 copy review, by a later structural cleanup, or by an explicit
  deferral.
- Crate extraction is either explicitly deferred using the trigger criteria in
  this RFC or advanced through a follow-up RFC with a narrower target.
- `rfcs/README.md`, `ROADMAP.md`, and release notes reflect the completed
  structural scope accurately.
