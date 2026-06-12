# RFC 012 — Security Hardening and Abuse Controls

**Status.** Implemented (v0.5.0)
**Phase:** M5 / UX and Release Hardening
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** AD-4 (CSRF via the server-issued form token), AD-1 (strict CSP is easy with little/no inline JS), AD-2 (OAuth/OIDC deferred, not forbidden), AD-3 (HMAC, not slow KDF).

---

## 1. Summary

Security controls beyond authorization: CSRF, XSS prevention, invite-code abuse resistance, secure headers/CSP, cache safety, and dependency hygiene.

## 2. Goals

- Protect state-changing cookie-authenticated requests via the form token.
- Prevent XSS from titles, descriptions, names, notes.
- Rate-limit invite redemption.
- Keep secrets out of logs/cache/errors.
- Establish security review gates.

## 3. Non-Goals

- No enterprise SSO, no public API tokens, no anomaly-detection dashboard. OAuth/OIDC is **deferred** (AD-2), not a permanent non-goal.

## 4. External Behavior

Plain messages only: "Invalid or expired code.", "Please try again later.", "You no longer have access to this community.", "Your session expired." No stack traces, SQL/platform errors, or internal IDs in the UI.

## 5. Internal Design

- Session cookie `HttpOnly; Secure; SameSite=Strict`.
- **CSRF = the AD-4 form token**: every state-changing `POST` carries a hidden, session-bound, single-use token (no fetch/header scheme needed because there is no SPA). Server rejects POSTs whose token is missing, foreign, expired, or already consumed; combined with `SameSite=Strict` and an Origin/Referer check.
- **XSS**: all user text rendered through escaping; notes/titles/locations/descriptions/cancellation notes are text nodes, never HTML.
- **CSP**: strict policy; trivial because the app ships little/no inline JS (AD-1). Any progressive-enhancement script is a hashed/nonced external asset.
- **Invite abuse**: rate-limit redemption by IP/attempt pattern; generic errors; peppered HMAC storage (AD-3); normalized input.
- Dependency audit in CI.

Error model separates internal reason from external message:

```rust
struct AppError { internal_code, external_code, user_message: &'static str, log_level }
```

## 6. Data and API Design

Affected: all state-changing `POST` routes (join/redeem, status, note, admin). Each requires a valid form token. The error model is shared via `packages/contracts` (RFC-013).

## 7. Security, Privacy, and Safety

- Private event/community existence never revealed via errors (generic 404).
- Invite brute force slowed and observable (RFC-014) without storing attempted plaintext.
- Production logs redact secrets and note bodies.
- Offline cache must not survive logout in readable form (RFC-017).

## 8. Acceptance Criteria

- A state-changing POST without a valid form token is rejected.
- Cross-origin POST with a stolen cookie but no valid token is rejected.
- Script-like notes render harmlessly as text.
- Invite brute force triggers rate limiting.
- No stack traces or secrets reach the UI or logs.

## 9. Test Plan

- Form-token CSRF tests (missing/foreign/expired/replayed).
- XSS payload render tests across all user-text fields.
- Rate-limit tests; error-redaction tests.
- Dependency audit; manual header/CSP/cookie-flag inspection.

## 10. Open Questions / Decisions

Decision: CSRF is the form token, not a separate header scheme — one mechanism, also providing idempotency (AD-4). UI security wording stays plain-language.
