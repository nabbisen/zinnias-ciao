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
| 1 | All token issues/consumes use `auth.user_id` | every `form_token::issue`/`consume` call in `handlers/*.rs` passes `&auth.user_id` as subject; `membership_id` appears only as `bound_resource` (e.g. `REMOVE_MEMBER`) | ✅ |
| 2 | `consume` is conditional UPDATE + checks affected rows | `form_token.rs`: `UPDATE … WHERE … consumed_at IS NULL …`; gates on `meta().changes == 1`; zero-row case classified via `classify_token_consume` | ✅ |
| 3 | Invite claimed before user/membership/session | `join.rs`: `mark_used → won` (212) precedes `insert_user` (218), `insert_membership` (223), `session_db::insert` (237); aborts if `!won` (213) | ✅ |
| 4 | No plaintext invite code stored or logged | `admin.rs`: stored as `code_hmac = hmac_hex(pp, normalized)`; lookups by HMAC; `join.rs` header comment confirms never logged | ✅ |
| 5 | `crypto::pepper(env)` sole pepper path | no `secret("HMAC_PEPPER")`/`var("HMAC_PEPPER")` outside `crypto.rs` | ✅ |
| 6 | `SESSION_COOKIE_DOMAIN` optional, host-only default | `session.rs::build_session_cookie(Option<&str>)` emits `Domain` only when non-empty; `get_domain` returns `Option` | ✅ |
| 7 | SW does not cache authenticated HTML | `sw.js`: no `cache.put` for any route; `/c/*`, `/`, `/join` are network-only; static assets pre-cached at install | ✅ |
| 8 | SW cache version matches package version | `sw.js CACHE_VERSION` == `Cargo.toml [workspace.package].version`; enforced by `release_gates.rs::sw_cache_version_matches_workspace_version` | ✅ |
| 9 | Export uses flat batched `IN` queries | `export.rs::build_export`: 5 prepares (members, events, 3 `IN` batches for days/attendances/notes); `for` loops are in-memory grouping | ✅ |
| 10 | Write paths use `tz::local_to_utc` | `admin.rs:164,165` (create), `:811,812` (edit) | ✅ |
| 11 | Display paths use `tz::to_local_parts` | `render.rs:487–537`, `admin.rs:685` | ✅ |

**Conclusion of source verification:** every source-checkable claim in the
handoff is confirmed true against the code. 184 tests pass; zero warnings.

## 6. Staging-Runtime Verification — PENDING (requires deployed environment)

These cannot be executed in the repository. They require a Cloudflare staging
deployment (`worker-build` + `wrangler deploy --env staging` + a staging D1).
Each maps to a review P0.

| # | Test (review ref) | Expected result | Blocks |
|---|-------------------|-----------------|--------|
| S1 | Deploy to Cloudflare staging | Worker boots; `/healthz` 200; `/version` reports v0.27.0 | P0-1 |
| S2 | `Asia/Tokyo` community, create 09:00–10:30 event, view detail (P0-2) | Detail shows `09:00–10:30` JST; stored `starts_at_utc` = `…T00:00:00.000Z` | P0-2 |
| S3 | Edit event time 09:00→13:00, re-view (P0-2) | Detail shows `13:00`; `event_days` row updated, not duplicated | P0-2 |
| S4 | Download ICS, inspect (P0-2) | DTSTART/DTEND correct for JST | P0-2 |
| S5 | Two concurrent redemptions of one invite (P0-3) | Exactly one membership + session created; the other gets generic invalid-or-expired | P0-3 |
| S6 | Two concurrent POSTs with one `SET_STATUS` token (P0-4) | Exactly one mutation; replay is a deterministic redirect; no duplicate attendance | P0-4 |
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
