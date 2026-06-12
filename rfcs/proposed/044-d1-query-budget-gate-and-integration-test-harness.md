# RFC 044 — D1 Query-Budget Release Gate and Integration Test Harness

**Status.** Proposed
**Phase:** F7 / Stabilization (architect deep-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes deep-review finding P1-4 (D1/subrequest pressure) and provides the live-D1 integration harness that RFC-037, RFC-040, and RFC-041 defer their end-to-end tests to. Refines RFC-029 (query performance discipline) and RFC-015 (testing and release gates).

> **Proposed (partial).** The compile-level query-budget constants shipped in
> v0.25.0 (§6.1 option a) and the SW version gate shipped in v0.26.0 (§11 step 1).
> The integration harness (§6.2) and the race-regression tests (§11 steps 3–5)
> remain unimplemented. They gate beta, not the first pilot.

---

## 1. Summary

Two related gaps:

1. **No per-route D1 query budget.** SSR pages issue sequential D1 operations
   (data fetches plus per-render form-token issues). On Workers Free with D1's
   single-writer model, unbounded sequential queries hurt latency and risk
   subrequest/query limits. There is no gate that catches a regression that,
   say, reintroduces an N+1 on event detail.
2. **No live-D1 integration harness.** All current tests live in the pure
   `domain`/`contracts` crates; the `ssr` worker crate is only `cargo check`ed.
   The architect's requested regression tests (form-token race, invite race,
   admin edit persistence) cannot be expressed without a runnable D1.

This RFC specifies a query-budget release gate and a `wrangler dev`-based
integration harness, then enumerates the deferred tests to implement on it.

---

## 2. Motivation

- **P1-4.** RFC-029 already states the budgets aspirationally; nothing enforces
  them. Event detail is a core, frequently hit page that fans out per-day and
  per-note queries plus per-form token issuance — exactly where an N+1 can
  creep back in. Export walks events/days/attendance/notes and must use batched
  `IN` queries before beta.
- **Deferred tests.** RFC-037 (token race/idempotency), RFC-041 (invite
  one-time under races), and RFC-040 (single-day edit persistence) each have an
  end-to-end assertion that needs a real database. Their logic is unit-tested
  where pure, but the wiring is currently only compile-checked. A harness closes
  that gap and guards against regressions in the exact bugs v0.23.0 fixed.

---

## 3. Goals

- Define explicit per-route D1 operation budgets and a mechanism to **measure
  and assert** them (a counting shim around the D1 binding in tests, or query
  logging parsed in CI).
- Enforce batched `IN` queries for export and any list endpoint that would
  otherwise N+1.
- Stand up an integration harness that runs the worker against a local D1
  (`wrangler dev` + a test runner, or `workerd` with a seeded SQLite) and can
  drive real HTTP requests.
- Implement the deferred regression tests on that harness.
- Add a release gate that fails CI if a budget is exceeded or a regression test
  fails.

---

## 4. Non-Goals

- No production performance monitoring/telemetry (that is RFC-014 / operations).
- No load testing or benchmarking at scale; budgets are per-request operation
  counts, not throughput targets.
- No change to runtime code paths solely to satisfy measurement (the shim is
  test-only).

---

## 5. External Behavior

None directly. This RFC is about CI gates and test infrastructure; user-facing
behavior is unchanged. Indirectly, it protects the latency and correctness
properties users depend on.

---

## 6. Internal Design (proposed)

### 6.1 Query budgets

Adopt RFC-029's targets as enforced gates:

| Route | Target D1 operations |
|---|---:|
| Home | ≤ 8 |
| Event detail, single-day | ≤ 12 |
| Event detail, 7-day event | ≤ 25 |
| Admin invites | ≤ 10 + active invite count |
| Export | higher allowed, but must use batched `IN` queries |

**Measurement options** (decide during implementation):

- **(a) Counting shim.** A thin wrapper over the D1 binding used in integration
  tests that increments a counter per `prepare`/`run`/`first`/`all`. The test
  drives a route and asserts the counter is within budget.
- **(b) Query-log parse.** Run under `wrangler dev` / `workerd` with query
  logging and parse the count in CI.

(a) is preferred: deterministic, no log scraping, lives with the tests.

### 6.2 Integration harness

- Seed a local D1 (SQLite file) with the migrations and a fixture community,
  admin, member, invite, and event.
- Boot the worker (via `wrangler dev --local` or `workerd`) bound to that D1 and
  a test `HMAC_PEPPER`.
- Provide helpers to: issue an HTTP GET (parse the rendered `_token`), POST a
  form, and read back DB rows for assertions.

### 6.3 Deferred regression tests to implement

1. **Token race / idempotency (RFC-037):** render Event Detail, extract
   `_token`, POST a status, assert the attendance row changed; POST the same
   token again, assert no second change and no error (replay).
2. **Invite one-time (RFC-041):** fire two concurrent `post_profile`
   submissions for one invite; assert exactly one membership and one
   `used_by_membership_id`.
3. **Single-day edit persistence (RFC-040):** edit an event's time; assert the
   `event_days` row updated and round-trips to the entered local time.
4. **SW version gate:** assert `sw.js` `CACHE_VERSION` equals the package
   version (a trivial string check; can run without the harness).

---

## 7. Data Model Notes

The harness needs a deterministic seed/fixture script (reuse `setup.mjs` or a
test-specific seeder). No production schema change.

---

## 8. API and UI Contract Notes

None. The harness exercises existing routes; it does not add any.

---

## 9. Security, Privacy, and Safety

- The harness uses a throwaway pepper and local DB; no production secrets.
- The query-budget gate indirectly protects availability (a runaway N+1 under
  load is a denial-of-service risk on a constrained tier).
- The regression tests guard the exact security/correctness invariants fixed in
  v0.23.0 (one-time invite, single-use token), preventing silent reintroduction.

---

## 10. Acceptance Criteria

1. CI runs an integration suite against a local D1.
2. Each budgeted route has an asserted operation count within its target.
3. Export uses batched `IN` queries (no per-row fetch) and is covered by a test.
4. The three deferred regression tests (token race, invite race, edit
   persistence) pass.
5. A release gate fails if any budget is exceeded or any regression test fails.
6. SW `CACHE_VERSION` matches the package version (automated check).

---

## 11. Test Plan

This RFC *is* largely a test plan. Implementation order:

1. SW version check (cheap, no harness).
2. Counting-shim budgets on Home and Event detail (highest-traffic).
3. Integration harness boot + fixture seed.
4. Token-race and invite-race regression tests.
5. Edit-persistence test; export `IN`-batching test.
6. Wire all into CI as a release gate.

---

## 12. Rollout Plan

Implement incrementally; each step is independently valuable. Prioritize the SW
version check and the Home/Event-detail budgets (fast wins), then the harness
and the race regressions (highest correctness value). Gate the beta — not
necessarily the first pilot — on the full suite.

---

## 13. Open Decisions

- **Harness runtime:** `wrangler dev --local` vs `workerd` directly vs a
  Miniflare-style in-process runner. Pick the one that runs cleanly in CI
  without flakiness.
- **Budget measurement:** counting shim (preferred) vs query-log parsing.
- **Concurrency simulation:** how to realistically interleave two requests
  against local D1 to exercise the race paths (e.g. barrier-synchronized
  parallel requests) given the local single-writer behavior may serialize them;
  may require a deterministic injection point rather than true parallelism.
