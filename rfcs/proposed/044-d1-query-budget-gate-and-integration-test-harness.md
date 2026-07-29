# RFC 044 — D1 Query-Budget Release Gate and Integration Test Harness

**Status.** Proposed
**Phase:** F7 / Stabilization (architect deep-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes deep-review finding P1-4 (D1/subrequest pressure) and provides the live-D1 integration harness that RFC-037, RFC-040, and RFC-041 defer their end-to-end tests to. Refines RFC-029 (query performance discipline) and RFC-015 (testing and release gates).

> **Proposed (narrowed 2026-07-29).** Most of this RFC is now discharged. The
> compile-level query-budget constants shipped in v0.25.0, the SW version gate in
> v0.26.0 (§11 step 1), and the static source query-count gates in v0.34.0
> (§6.1 — home, event detail, export await-count assertions via `include_str!`).
> The i18n parity gate now covers all 141 EN/JA constant pairs.
>
> The **integration harness (§6.2) and every deferred regression test (§6.3)
> were subsequently built under RFC-050's local evidence tooling** and committed
> at `c55787a` — see §6.4 for the item-by-item mapping and evidence pointers.
>
> **What remains is one narrow item:** the runtime counting shim of §6.1 option
> (a), giving exact per-route D1 operation counts. Everything else in this RFC is
> either shipped or superseded. The remaining item still gates beta, not the
> first pilot.

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

(a) is preferred: deterministic, no log scraping, lives with the tests. **This is
now the decision, not a preference** — see §6.4.

**This is the only part of this RFC that remains open.** What exists today is
`packages/contracts/tests/release_gates.rs` (~L590–663): the budget constants
above, plus a static `include_str!` await-count gate per budgeted route. That
gate is deliberately conservative — it fires only when the counted awaits exceed
**2× the budget**, because a source-level await count is a proxy for runtime
operations, not a measurement of them.

The residual gap is therefore precise: **an N+1 regression that stays under 2× of
its route's budget is not caught today.** The counting shim closes it by
measuring actual `prepare`/`run`/`first`/`all` calls against the exact budget.

This is much cheaper to build than when this RFC was written, because its hard
prerequisite — a runnable worker bound to a real local D1 (§6.2) — now exists.
The shim is a wrapper around that harness's D1 binding plus one assertion per
budgeted route. It remains a **beta** gate, not a first-pilot gate: the static
gate already catches the gross regressions, and this closes the narrow band
between "exact budget" and "2× budget."

### 6.2 Integration harness — **superseded, built**

`scripts/lib/isolated-worker-test.mjs` (`c55787a`) discharges this section. It
provisions a **disposable isolated Worker plus its own D1 database**, seeds
migrations and fixtures, binds a test pepper, and tears both down
unconditionally. `prepareIsolatedWorkerTest(name)` is the entry point; the E3/E4
collectors use it today.

That is stronger than what this section specified — per-run database isolation
rather than a shared local SQLite file, so runs cannot contaminate each other.
The original text is retained below for provenance only. **Do not implement it.**

- Seed a local D1 (SQLite file) with the migrations and a fixture community,
  admin, member, invite, and event.
- Boot the worker (via `wrangler dev --local` or `workerd`) bound to that D1 and
  a test `HMAC_PEPPER`.
- Provide helpers to: issue an HTTP GET (parse the rendered `_token`), POST a
  form, and read back DB rows for assertions.

### 6.3 Deferred regression tests — **all four discharged**

See §6.4 for the mapping. The original list is retained below for provenance.
**Do not re-implement these.**

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

### 6.4 Overlap check against RFC-050 (performed 2026-07-29)

`ROADMAP.md` required this RFC's remaining scope to be checked for overlap with
or supersession by the revised RFC-050 before treating it as a prerequisite.
That check is complete. The result: **RFC-050's local evidence *tooling*
discharged nearly all of it.** Note the distinction — this is discharged by
tooling that is built and committed, not by the deferred hosted campaign, so
none of it is waiting on a deployment.

| Item | Disposition | Discharged by |
|---|---|---|
| §6.2 integration harness | **Superseded** | `scripts/lib/isolated-worker-test.mjs` (`c55787a`) |
| §6.3.1 token race / idempotency | **Discharged** | `scripts/collect-evidence-e4-concurrency.mjs` — concurrent form-token bursts on attendance, note, and a destructive admin action |
| §6.3.2 invite one-time race | **Discharged** | same collector — concurrent invite redemption |
| §6.3.3 single-day edit persistence | **Discharged** | `scripts/collect-evidence-e3-flows.mjs`, `S4.event_create_and_edit_asia_tokyo` |
| §6.3.4 SW version gate | **Shipped** v0.26.0 | — |
| §10 AC 3 export batched `IN` | **Satisfied in source** | `workers/ssr/src/handlers/export.rs` — three `IN` batches, no per-row fetch |
| §6.1 runtime counting shim (option a) | **Remains** | — |

Two items were discharged *more strongly* than this RFC asked:

- §6.3.3 asked only that an edited time "round-trips to the entered local time."
  E3 asserts the exact UTC conversion of an Asia/Tokyo `09:00–10:30` event and of
  its `13:00–14:30` edit, which also covers the timezone-conversion path.
- §6.3.1/§6.3.2 asked for two concurrent submissions. E4 pre-issues single-use
  tokens sequentially, fires the burst with `Promise.all`, then asserts **exact**
  admitted counts plus D1 postconditions and audit cardinality.

**§13's three open decisions are therefore moot.** The harness runtime is settled
(isolated Worker + per-run D1). Budget measurement is settled (option (a),
below). Concurrency simulation is settled by E4's pre-issue-then-burst pattern,
which is the answer to "how do we interleave requests when local D1 serializes
writers" — you remove token issuance from the raced window.

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

Marked against the §6.4 disposition.

1. ~~CI runs an integration suite against a local D1.~~ **Met** — `c55787a`.
2. **Open.** Each budgeted route has an asserted operation count within its
   target, measured at runtime rather than proxied by a source await count.
3. ~~Export uses batched `IN` queries (no per-row fetch) and is covered by a
   test.~~ **Met** — `export.rs` uses three `IN` batches; the export await-count
   gate covers it.
4. ~~The three deferred regression tests (token race, invite race, edit
   persistence) pass.~~ **Met** — E4, E4, and E3 respectively.
5. **Partially met.** A release gate already fails on the static await-count
   ceiling and on regression-test failure. It does not yet fail on an exact
   budget overrun; that is criterion 2.
6. ~~SW `CACHE_VERSION` matches the package version.~~ **Met** — v0.26.0.

Only criteria 2 and the residual half of 5 remain, and both are the same piece
of work: the counting shim.

---

## 11. Test Plan

Steps 1 and 3–5 of the original plan are complete (§6.4). What remains:

1. Add a counting shim over the isolated harness's D1 binding, incrementing per
   `prepare`/`run`/`first`/`all`.
2. Assert exact operation counts for Home and Event detail first — highest
   traffic, and the routes where an N+1 is most likely to reappear.
3. Extend to the remaining budgeted routes.
4. Wire the exact-budget assertion into CI alongside the existing static gate.
   **Keep the static gate**; it runs without the harness and is the cheaper first
   line of defence.

---

## 12. Rollout Plan

The incremental rollout is largely done. The remaining counting-shim work is a
single coherent package and should be taken as one, since splitting a shim from
the assertions it exists to enable would produce a slice with no observable
value. It gates **beta**, not the first pilot, and it sits behind current product
themes in `ROADMAP.md` — the static gate holds the line until then.

---

## 13. Open Decisions

**None. All three were resolved by the 2026-07-29 overlap check (§6.4)**, and are
recorded here as decisions rather than deleted, so the reasoning survives.

- **Harness runtime — resolved.** A disposable isolated Worker with a per-run D1
  database, via `scripts/lib/isolated-worker-test.mjs`. This beat the three
  options originally listed by giving per-run isolation, so a failed run cannot
  contaminate the next.
- **Budget measurement — resolved.** Option (a), the counting shim. Query-log
  parsing is rejected: it depends on log formatting and would be the only gate in
  the repo that scrapes text to establish a numeric property.
- **Concurrency simulation — resolved.** Pre-issue single-use tokens and tickets
  sequentially, then fire only the claiming request concurrently with
  `Promise.all`, and assert exact admitted counts. This sidesteps the concern
  that local D1's single-writer behavior would serialize the race: token issuance
  is removed from the raced window, so what is raced is the claim itself. No
  deterministic injection point was needed. Proven in
  `scripts/collect-evidence-e4-concurrency.mjs`, which generalized the pattern
  from `scripts/smoke/abuse-controls.mjs`.
