# RFC 003 — Invite Redemption and Session Authentication

**Status.** Proposed
**Phase:** M1 / Trust Boundary Foundation
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** aligned to ADR (AD-2 invite + deferred OIDC, AD-3 CPU/HMAC). The original "invite redemption is the entire identity boundary" stands for MVP; OIDC is reserved, not built.

---

## 1. Summary

Defines invite-only onboarding and the cookie session. MVP identity is an invite code redeemed into a community membership plus a server-side cookie session (AD-2). There is no password and no mandatory external account. OIDC is the intended future and is reserved in the data model (`users.idp_subject`) but not implemented here.

## 2. Goals

- Join a community with one short, one-time invite code.
- Create user (if new) + membership + session atomically.
- Store invite codes and session secrets safely and within the CPU budget.
- Finite, understandable session lifetime; clean logout.
- Leave a clean seam for deferred OIDC.

## 3. Non-Goals

- No password login, passkeys, or self-service identity recovery in MVP.
- No OIDC implementation yet (reserved only).
- No exposing used invite codes to members.

## 4. External Behavior

Join (all server-rendered, AD-1):

1. Anonymous visitor opens `/join` (a form).
2. Enters the invite code; server validates without revealing whether a community exists.
3. If valid, `/join/profile` collects a display name.
4. Server creates user (if new) + membership + session, sets the cookie, and 303-redirects to Home.

Session expiry: "Your session expired. Please ask your community admin for a new invite code." Private cached shell is cleared/locked; user returns to `/join`.

## 5. Internal Design

Redemption is one logical transaction (D1 batch if needed; never a state where the code is consumed but no membership/session exists):

```text
begin
  look up invite by code_hmac = HMAC(pepper, normalize(code))   -- AD-3: fast, peppered
  verify unexpired, unused, unrevoked
  create user if new anonymous join (idp_subject = NULL)
  create community_membership (role = member)
  mark invite used_by_membership / used_at
  create session: store HMAC(pepper, random_secret); set the cookie secret
  write audit event (no plaintext code)
commit
```

**Session cookie:** opaque high-entropy random secret; `HttpOnly; Secure; SameSite=Strict`; server stores only the HMAC.

**Lifetime (critical, AD-2/AD-3).** `expires_at = now + SESSION_TTL_SECONDS` with `SESSION_TTL_SECONDS = 86400`. The session lifetime is **decoupled from any token's `exp`**. Do not derive cookie `Max-Age`/expiry from an upstream token's expiry: a token at the edge of validation leeway can yield a near-zero or zero lifetime, and browsers discard a `Max-Age=0` cookie immediately, logging the user straight back out. Cookie `Max-Age` is computed from `SESSION_TTL_SECONDS`, full stop.

**Deferred OIDC seam.** When OIDC lands (future RFC), a first IdP login creates/looks up a `users` row by `idp_subject` and issues the same kind of session; invite redemption then binds an *authenticated* identity to a membership instead of creating an anonymous user. Nothing in the MVP cookie/session model needs to change for that.

## 6. Data and API Design

Server-rendered routes (forms, not JSON):

```text
GET  /join              # code entry form (issues a form_token)
POST /join              # validate + redeem; 303 -> /join/profile or Home
GET  /join/profile      # display-name form
POST /join/profile      # set name; 303 -> Home
POST /logout            # revoke session, clear cookie; 303 -> /join
```

Generic failure copy only: "Invalid or expired code." No JSON identity API in MVP.

## 7. Security, Privacy, and Safety

- Invite codes and session secrets never stored or logged in plaintext; HMAC only (AD-3).
- Normalize code input (uppercase, strip separators, drop ambiguous chars per policy) before hashing.
- Rate-limit redemption by IP/attempt pattern (RFC-012); generic errors so a code's prior validity cannot be inferred.
- POST routes carry the form token (AD-4) for CSRF.
- Logout revokes the server session; expiry locks/clears the private shell.

## 8. Acceptance Criteria

- Valid invite creates membership + session atomically; used invite cannot be reused.
- Expired/revoked/unknown codes fail with one generic message.
- Cookie has `HttpOnly; Secure; SameSite=Strict` and a `Max-Age` derived only from `SESSION_TTL_SECONDS`.
- A token at validation-leeway edge never produces a zero-lifetime cookie (regression test).
- Logout clears the cookie and revokes the session row.
- `users.idp_subject` exists and is NULL for invite-only members.

## 9. Test Plan

- Unit: code normalization + HMAC; session lifetime computation independent of any token exp.
- Integration: redeem success, duplicate-redeem failure, atomic rollback on mid-flow failure.
- Security: generic-error indistinguishability; cookie flag assertions; rate-limit behavior.
- Regression: leeway-edge token does not yield `Max-Age<=0` (the original cookie-discard bug).
- Audit-record-exists test (no plaintext code in audit).

## 10. Open Questions / Decisions

Decision: no self-service recovery in MVP. Lost session -> admin re-invite (or the optional relink path, RFC-024). OIDC stays deferred (AD-2); when adopted it makes recovery self-healing via a stable `idp_subject`.
