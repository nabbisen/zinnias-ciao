# RFC 045 — Pre-Pilot Runtime Verification Matrix

**Status.** Proposed (source-verification section discharged; staging-runtime section pending infrastructure)
**Phase:** F7 / Stabilization (handoff-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Directly answers the handoff-review §8 verification checklist and review findings P0-1 through P0-4. Converts handoff *claims* into *evidence*. Depends on RFC-049/RFC-044 for the live-D1 portions.

---

## 1. Summary

The v0.26.0 handoff review was explicitly a handoff-level review, not a source
audit, and conditioned production-pilot approval on verifying the handoff's
claims against (a) the actual source and (b) a real Cloudflare staging
environment. This RFC is the verification matrix. It records the **source-level
verification** (which can be done in the repository, and has been) and specifies
the **staging-runtime verification** (which requires a deployed environment and
remains pending).

## 2. Motivation

A handoff document can drift from the code. The review correctly declined to
accept claims like "token consumption is atomic" or "export is flat-batched" on
the document alone. Each such claim is cheap to confirm against source with a
targeted read, and doing so closes the source half of every P0 in the review.
The staging half cannot be faked and must wait for an environment.

## 3. Goals

- Provide a per-claim source-verification record with the exact code evidence.
- Enumerate the staging-runtime tests that remain, with precise expected results.
- Make the boundary explicit: what is proven now vs. what blocks production pilot.

## 4. Non-Goals

- Implementing the live-D1 harness itself (that is RFC-049/RFC-044).
- Re-deriving the fixes (those are RFC-037 through RFC-043, RFC-046, RFC-047).

## 5. Source-Level Verification — DISCHARGED

Each item below was confirmed against the v0.26.0/v0.27.0 source. This is the
review's §8 "Source/code verification" checklist.

| # | Claim | Evidence (file:symbol) | Result |
|---|-------|------------------------|--------|
| 1 | All token issues/consumes use `auth.user_id` as subject | On wasm32: `codlet::issue_token(env, &auth.user_id, …)` and `codlet::consume_token(env, &auth.user_id, …)` — `TokenSubject::Authenticated(SubjectId::new(user_id))` at every call site in `handlers/*.rs`. On non-wasm: `form_token::issue/consume` with `&auth.user_id`. Pre-auth join tokens use `TokenSubject::Anonymous`; flow tokens use `TokenSubject::Flow(flow_id)`. `membership_id` appears only as `bound_resource`. | ✅ (updated v0.38.2) |
| 2 | `consume` is conditional UPDATE + checks affected rows | `codlet-worker/d1/token.rs`: `UPDATE codlet_form_tokens SET consumed_at=? WHERE lookup_key=? AND subject_kind=? AND purpose=? AND bound_resource=? AND expires_at>? AND consumed_at IS NULL`; checks `meta().changes`; classified by `classify_token_consume`. Non-wasm fallback: `form_token.rs` same logic on `form_tokens`. | ✅ (updated v0.38.2) |
| 3 | Invite claimed before user/membership/session | On wasm32: `join.rs::post_profile` calls `codlet::code_auth.claim(redeemable, subject=user_id)` → `ClaimOutcome::Won` required before `membership_db::insert_user`, `insert_membership`, or `session_mgr.issue`. Codlet's `D1CodeStore::claim_code` uses conditional UPDATE on `codlet_codes`. On non-wasm: `mark_used → won` precedes `insert_user`, `insert_membership`, `session_db::insert`; aborts if `!won`. | ✅ (updated v0.38.2) |
| 4 | No plaintext invite code stored or logged | On wasm32: `codlet::code_auth.issue_code()` stores `lookup_key = HMAC(key, domain-separated-value)` in `codlet_codes`; plaintext returned as `CodeSecret` exposed only to the admin redirect. On non-wasm: `code_hmac = hmac_hex(pp, normalized)` stored in `invite_codes`. `codlet.rs` and `codlet-worker` pass the `no-plaintext-store` xtask release gate. | ✅ (updated v0.38.2) |
| 5 | Key material held by a single provider | On wasm32: `WorkerKeyProvider::from_env(env, "v1", "CODLET_HMAC_KEY_V1", &[])` is the sole HMAC key source for all codlet managers. Fails closed on missing/empty secret (INV-2). On non-wasm: `crypto::pepper(env)` reads `HMAC_PEPPER`. No `secret("HMAC_PEPPER")` or `var("HMAC_PEPPER")` call outside `crypto.rs`. | ✅ (updated v0.38.2) |
| 6 | `SESSION_COOKIE_DOMAIN` optional, host-only default | On wasm32: `codlet::build_session_mgr` uses `CookiePolicy::production_strict("ciao_sid", …)` which emits `Domain` only if explicitly set. On non-wasm: `session.rs::build_session_cookie(Option<&str>)` emits `Domain` only when non-empty. | ✅ (updated v0.38.2) |
| 7 | SW does not cache authenticated HTML | `sw.js`: no `cache.put` for any route; `/c/*`, `/`, `/join` are network-only; static assets pre-cached at install | ✅ |
| 8 | SW cache version matches package version | `sw.js CACHE_VERSION` == `Cargo.toml [workspace.package].version`; enforced by `release_gates.rs::sw_cache_version_matches_workspace_version` | ✅ |
| 9 | Export uses flat batched `IN` queries | `export.rs::build_export`: 5 prepares (members, events, 3 `IN` batches for days/attendances/notes); `for` loops are in-memory grouping | ✅ |
| 10 | Write paths use `tz::local_to_utc` | `admin.rs:164,165` (create), `:811,812` (edit) | ✅ |
| 11 | Display paths use `tz::to_local_parts` | `render.rs:487–537`, `admin.rs:685` | ✅ |

**Conclusion of source verification:** every source-checkable claim confirmed against the code.
223 tests pass; zero warnings (native); zero errors/warnings (wasm32). Updated at v0.38.2
to reflect codlet integration (v0.37.0–v0.38.2): items 1–6 revised with codlet evidence.
Items 7–11 unchanged.

## 6. Staging-Runtime Verification — PENDING (requires deployed environment)

These cannot be executed in the repository. They require a Cloudflare staging
deployment (`worker-build` + `wrangler deploy --env staging` + a staging D1).
Each maps to a review P0.

| # | Test (review ref) | Expected result | Blocks |
|---|-------------------|-----------------|--------|
| S1 | Deploy to Cloudflare staging | Worker boots; `/healthz` 200; `/version` reports current version; `codlet_codes`, `codlet_sessions`, `codlet_form_tokens` tables created by `run_d1_migrations` | P0-1 |
| S2 | `Asia/Tokyo` community, create 09:00–10:30 event, view detail (P0-2) | Detail shows `09:00–10:30` JST; stored `starts_at_utc` = `…T00:00:00.000Z` | P0-2 |
| S3 | Edit event time 09:00→13:00, re-view (P0-2) | Detail shows `13:00`; `event_days` row updated, not duplicated | P0-2 |
| S4 | Download ICS, inspect (P0-2) | DTSTART/DTEND correct for JST | P0-2 |
| S5 | Two concurrent redemptions of one invite (P0-3) | Exactly one membership + session created (`codlet_sessions`); the other gets generic invalid-or-expired. `codlet_codes.used_at` set exactly once (verify via D1 query). | P0-3 |
| S6 | Two concurrent POSTs with one `SET_STATUS` token (P0-4) | Exactly one mutation; replay is a deterministic redirect; no duplicate attendance. `codlet_form_tokens.consumed_at` set exactly once (verify via D1 query). | P0-4 |
| S7 | Same for `SAVE_NOTE`, `DELETE_NOTE`, `REMOVE_MEMBER` (P0-4) | Exactly one mutation each; audit log has no duplicate admin actions | P0-4 |
| S8 | Logout, then probe SW cache for private HTML (P0-1) | No authenticated HTML served from cache after logout | P0-1 |
| S9 | Browser with JavaScript disabled (P1-4) | Join, mark attendance, switch community, all destructive confirmations work | P1-4 |
| S10 | Real phone at 200% text scale (review §6.3) | Home, Event Detail, Join, Admin Create Event, Member Remove confirm, Me all usable | review §6.3 |
| S11 | Logpush/audit availability for admin actions | Admin actions appear in audit log; Logpush delivers to R2/S3 | P0-1 |

Tests S5–S7 are the same race regressions specified by RFC-049/RFC-044; they can
be run either against staging or against the local live-D1 harness once it
exists.

## 7. Product/Usability Verification — PENDING (human QA)

From review §8 "Product/usability verification": non-technical user joins and
marks attendance under 2 minutes; admin creates an event without guidance; admin
understands invite is one-time; member understands No Answer ≠ No Go; member
understands lost-session requires a new admin invite; error messages are
non-technical. These are observational and require pilot participants.

## 8. Acceptance Criteria

- §5 source verification complete with evidence (done).
- §6 staging matrix executed against a real staging deployment (pending infra).
- §7 usability checks observed with real users (pending pilot).
- Production pilot approved only when §6 and §7 pass.

## 9. Open Decisions

- **Where S5–S7 run.** Either staging or the RFC-049 local live-D1 harness. The
  harness is reproducible in CI and is the preferred long-term home; staging is
  acceptable for a one-time pre-pilot gate.
- **Whether S10 is a hard gate for a tiny internal pilot.** The review treats
  200% scaling as a hard gate. For a first internal staging pilot with known
  participants it may be downgraded to a fast-follow; for the public Japan pilot
  it remains a hard gate.
