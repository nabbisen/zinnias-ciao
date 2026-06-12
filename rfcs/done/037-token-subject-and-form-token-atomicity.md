# RFC 037 — Token Subject Normalization and Form-Token Atomicity

**Status.** Implemented (v0.23.0)
**Phase:** F7 / Stabilization (architect deep-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes deep-review findings P0-1, P0-2, P0-7, and P1-2. Refines RFC-004 (authorization) and RFC-013 (idempotency) without changing their external contracts.

---

## 1. Summary

This RFC fixes two classes of defect in the single-use form-token mechanism (AD-4):

1. **Subject inconsistency.** Two handlers issued tokens keyed on the
   community-scoped `membership_id` while every consume site looked them up by
   the global `user_id`. The lookup could never match, so the affected member
   actions silently failed even though the page rendered correctly.
2. **Non-atomic consume.** Token consumption was a SELECT-then-UPDATE sequence
   with no conditional guard and no affected-row check, so two concurrent
   submissions of the same token could both observe it as unused and both
   proceed.

It also promotes the previously raw-string `generate_invite` purpose to a
contract constant, closing a gap in the token-purpose registry, and extracts
the consume decision into a pure, unit-tested classifier.

The fix is implementation-only. No API path, form field, or user-visible
behavior changes except that the affected actions now work.

---

## 2. Motivation

Form tokens are the project's combined CSRF and idempotency primitive (AD-4):
one server-issued token per rendered form, purpose-bound, subject-bound,
single-use, with a one-hour TTL. The entire safety of state-changing POSTs
rests on the token being (a) found by the same key it was stored under and
(b) consumable exactly once.

Both properties were violated:

- **P0-1 / P0-2 (subject mismatch).** `SET_STATUS` (attendance) and
  `DELETE_NOTE` tokens were issued with `membership.membership_id` as the
  subject but consumed with `auth.user_id`. Because the two identifiers differ
  (a `user` may hold different `membership` rows per community), the consume
  SELECT returned no row and the handler rejected every attempt. Setting
  Going / No Go / Attended — the single most important member action — was
  broken in normal operation. The page rendered fine, so the defect was
  invisible to code review that checked only that purposes matched.

- **P0-7 (non-atomic consume).** Consume did `SELECT … WHERE token_hmac=…`
  then, separately, `UPDATE … SET consumed_at=…`. Under interleaving, two
  requests could both pass the SELECT (both see `consumed_at IS NULL`) and both
  run the UPDATE, executing the guarded action twice. Idempotency must not
  depend on each handler being individually idempotent.

- **P1-2 (registry gap).** `generate_invite` was a string literal at both the
  issue and consume sites, bypassing the `token_purpose` registry and its
  uniqueness test. This also revealed that `remove_member` had never actually
  been added to the uniqueness test lists despite a prior changelog claim.

These are correctness and security defects, not enhancements, and they block a
pilot.

---

## 3. Goals

- Normalize **every** form-token subject to `auth.user_id`. Authorization and
  database writes continue to use `membership_id`; the token subject is purely
  the identity key for token storage and lookup.
- Make consume a single atomic conditional UPDATE whose affected-row count is
  authoritative for "did this call win".
- Preserve idempotent replay: a second submission returns the prior result
  reference rather than re-executing or erroring.
- Register all token purposes as contract constants; enforce uniqueness in a
  test that lists every purpose.
- Extract the consume decision so it is unit-testable without a database.

---

## 4. Non-Goals

- No change to the token TTL, HMAC scheme, or cookie handling (see RFC-038).
- No move to D1 transactions or batch API (D1 via worker-rs offers no
  multi-statement transaction; conditional UPDATE is the chosen primitive).
- No change to which actions require a token.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Member sets Going/No Go/Attended | Status persists; page reflects it after the 303 redirect. |
| Member deletes own note | Note removed; confirmed via flash. |
| Double-submit (double tap, retry) of the same token | First wins; second is an idempotent replay returning the prior result — never a second mutation, never a user-facing error. |
| Stale or forged token | Generic "could not be completed" message; no state change. |
| Token used for the wrong bound resource | Treated as invalid (rejected), not as a replay. |

User-facing copy is unchanged and remains non-technical (no "CSRF", "token",
"mutation").

---

## 6. Internal Design

### 6.1 Subject normalization

All `form_token::issue(...)` calls take `&auth.user_id` as the subject. The two
offending sites in `handlers/event.rs` (`SET_STATUS` at the event-detail render
and `DELETE_NOTE` for the member's own note) were changed from
`&membership.membership_id` to `&auth.user_id`. `SAVE_NOTE` was already correct
and serves as the reference pattern. Resource scoping continues via the token's
`bound_resource` (the `event_day_id` for status, the `event_id` for notes).

### 6.2 Atomic consume

`form_token::consume` is now:

```sql
UPDATE form_tokens SET consumed_at = ?now
WHERE token_hmac = ?h
  AND user_id    = ?subject
  AND purpose    = ?purpose
  AND expires_at > ?now
  AND consumed_at IS NULL
  AND COALESCE(bound_resource, '') = ?bound
```

The worker-rs `D1Result::meta().changes` count is read:

- `changes == 1` → this call won; return `Ok(None)` ("proceed").
- `changes == 0` → a follow-up **SELECT** classifies the cause (replay vs
  invalid vs expired). The follow-up is read-only and race-free because the
  decisive write already happened (or didn't).

### 6.3 Pure classifier

The decision is encoded in `contracts::auth::classify_token_consume`:

```rust
pub enum TokenConsumeOutcome { Proceed, Replay, Invalid }

pub fn classify_token_consume(
    changed: usize, found: bool, already_consumed: bool, binding_ok: bool,
) -> TokenConsumeOutcome
```

`consume` maps the DB state onto these four inputs and matches on the result.
Because the classifier is pure (no Worker/D1 types), it is unit-tested in the
`contracts` crate, including an exhaustive check that **no** `changed == 0`
state can ever yield `Proceed`.

### 6.4 Purpose registry

`token_purpose::GENERATE_INVITE` was added. The admin invite handler uses the
constant at both issue and consume. The uniqueness tests in
`release_gates.rs` and `token_and_color_regression.rs` now enumerate every
purpose, including the previously-missing `REMOVE_MEMBER` and the new
`GENERATE_INVITE`.

---

## 7. Data Model Notes

No schema change. `form_tokens` already carried `consumed_at`, `bound_resource`,
`result_ref`, and `expires_at`. The fix relies only on the conditional UPDATE
and the `changes` metadatum, both supported by the existing `worker 0.8` D1
binding.

`form_tokens` remains a short-lived operational table with no FK on `user_id`
(migration 0002 made it nullable for pre-auth join tokens).

---

## 8. API and UI Contract Notes

- No endpoint, method, or field changes.
- The replay contract is unchanged from RFC-013: a replayed token returns the
  recorded `result_ref` so the second POST is a benign no-op from the user's
  perspective.
- The generic failure message is reused for invalid/expired/binding-mismatch to
  avoid leaking which condition occurred.

---

## 9. Security, Privacy, and Safety

- **CSRF.** Unchanged: a same-origin, single-use, session-bound token is still
  required for every state-changing POST.
- **Idempotency under races.** Now guaranteed by the database, not by handler
  discipline. Two writers cannot both see `changes == 1`.
- **Subject confusion.** Eliminating the `membership_id`/`user_id` split removes
  a class of "token of user A in community X accidentally matching" risk; the
  subject is now always the stable global identity, and community scoping is
  enforced separately by `require_membership` / `require_admin`.
- **Registry discipline.** Every purpose is a constant with a uniqueness gate,
  preventing accidental collisions (e.g. two actions sharing a purpose string,
  which would let a token for one action authorize another).

---

## 10. Acceptance Criteria

1. A member can set Going, reload, and still see Going. (Was broken.)
2. A member can delete their own note. (Was broken.)
3. Submitting the same status token twice changes the attendance row at most
   once and never shows an error on the second submit.
4. `cargo test -p zinnias-ciao-contracts` includes classifier tests, including
   the exhaustive "changed==0 never proceeds" guard.
5. The token-purpose uniqueness test lists every purpose constant, including
   `GENERATE_INVITE` and `REMOVE_MEMBER`; no duplicates.
6. No raw-string token purposes remain in `workers/ssr/src`.

All six are met in v0.23.0 (173 contract+domain tests passing, zero warnings).

---

## 11. Test Plan

- **Unit (shipped):** `classify_token_consume` across all input combinations;
  `local_to_utc` is covered by RFC-039. Purpose uniqueness via `HashSet` over
  the full constant list.
- **Integration (deferred to RFC-044):** a live-D1 test that renders Event
  Detail, extracts `_token`, POSTs a status, and asserts the attendance row
  changed; plus a concurrent double-POST asserting exactly one mutation. This
  requires a `wrangler dev` + test-runner harness the project does not yet have;
  RFC-044 specifies it. Until then, the DB wiring is verified by `cargo check
  --target wasm32-unknown-unknown` and the pure classifier tests.

---

## 12. Rollout Plan

Shipped in v0.23.0 as part of the stabilization pass. No migration, no config
change, no data backfill. Existing unconsumed tokens remain valid; the new
conditional consume simply enforces the guard that the old code intended.

---

## 13. Open Decisions

None. The deferred integration harness is tracked in RFC-044, not here.
