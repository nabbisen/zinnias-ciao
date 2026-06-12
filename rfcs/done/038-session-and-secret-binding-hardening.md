# RFC 038 — Session and Secret Binding Hardening

**Status.** Implemented (v0.23.0)
**Phase:** F7 / Stabilization (architect deep-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes deep-review findings P0-3, P1-1, and P1-5. Refines RFC-003 (sessions) and RFC-012 (security hardening); references AD-3 (HMAC pepper).

---

## 1. Summary

This RFC fixes three binding and lifetime defects around sessions and the HMAC
pepper:

1. **Cookie domain binding.** `SESSION_COOKIE_DOMAIN` was read as a `[vars]`
   binding but documented as a secret, and the cookie unconditionally appended
   `Domain=localhost` as a fallback — which is invalid on any deployed host.
2. **Pepper access inconsistency.** Six handlers read `HMAC_PEPPER` through a
   mix of `env.secret()` and `env.var()` with two different dev fallbacks.
3. **Session lifetime.** The 24-hour TTL was too short for an invite-only
   product with no password and no self-service recovery.

The result is host-only cookies by default, a single source of truth for the
pepper, and a 30-day session that remains server-side revocable.

---

## 2. Motivation

- **P0-3 (cookie domain).** If an operator follows the documentation and sets
  `SESSION_COOKIE_DOMAIN` via `wrangler secret put`, the code reading it with
  `env.var()` will not see it and will fall back to `Domain=localhost`. A cookie
  scoped to `localhost` is rejected by the browser on a real domain, so login
  silently fails in staging/production while working locally — the worst kind of
  environment-specific bug.

- **P1-1 (pepper inconsistency).** `join`, `event`, `admin`, `me` used
  `env.secret("HMAC_PEPPER")` with fallback `"dev-pepper-change-in-production"`;
  `calendar`, `export`, `templates` used `env.var("HMAC_PEPPER")` with fallback
  `"dev-pepper"`. A deployment that sets the pepper as a secret would leave the
  `var`-reading handlers on the fallback, so calendar/export/template form
  tokens would be computed with a *different* pepper than the rest of the app.
  Divergent fallbacks make this silent.

- **P1-5 (session lifetime).** Members join via a one-time invite code, have no
  password, and (in the MVP) no self-service recovery: a lost session means
  asking an admin for a fresh invite. A 24-hour expiry turns that into a
  recurring chore for every community. The session is already revocable on
  logout, so a longer window does not weaken the revocation story.

---

## 3. Goals

- Emit a **host-only** cookie by default (no `Domain` attribute), and only emit
  `Domain=…` when explicitly configured for cross-subdomain sharing.
- Read `SESSION_COOKIE_DOMAIN` consistently and document it correctly as a
  non-secret `[vars]` binding.
- Provide one `crypto::pepper(env)` helper and use it everywhere, including
  `require_auth`.
- Set a 30-day session TTL; keep it server-side revocable; keep the form-token
  TTL strictly shorter.

---

## 4. Non-Goals

- No sliding/rolling expiry on every request (rejected — see §6.4; it would add
  a D1 write to every authenticated GET, the exact pressure RFC-044 guards
  against).
- No move to signed/stateless session tokens. Sessions stay server-side and
  revocable per RFC-003.
- No OIDC work (deferred per AD-2).

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Member logs in on the deployed domain | Session cookie is accepted and round-trips (was broken when `Domain=localhost`). |
| Member stays away up to 30 days | Still logged in on return. |
| Member logs out | Session revoked server-side; cookie cleared; cached private data purged. |
| Operator misconfigures the domain var | Cookie is host-only and still works (no invalid `Domain`). |
| Session expires (>30 days) | Plain message guiding the member to ask an admin for a new code; safe local draft text preserved. |

---

## 6. Internal Design

### 6.1 Host-only cookies

`build_session_cookie` and `clear_session_cookie` take `domain: Option<&str>`.
When `None` or empty, no `Domain` attribute is emitted, producing a host-only
cookie scoped to the exact deployment host — the correct default for a
single-host service. A `Domain` is emitted only when a non-empty value is
configured.

```rust
let domain_part = domain.filter(|d| !d.is_empty())
    .map(|d| format!("; Domain={d}")).unwrap_or_default();
format!("{NAME}={secret}; Max-Age={ttl}; Path=/; HttpOnly; Secure; SameSite=Strict{domain_part}")
```

`get_domain` in `join.rs` (and the equivalent in `auth.rs`) reads
`SESSION_COOKIE_DOMAIN` as a var, returning `Option<String>` filtered to
non-empty. `wrangler.toml` documents it as a `[vars]` binding, set per
environment, left unset for host-only.

### 6.2 Centralized pepper

`crypto::pepper(env)` is the single accessor:

1. `env.secret("HMAC_PEPPER")` if present and non-empty;
2. else `env.var("HMAC_PEPPER")` if present and non-empty (some local setups);
3. else the dev sentinel `"dev-pepper-change-in-production"`.

All six handlers and `session::require_auth` now call it; the per-handler
helpers and inline reads were removed. A single fallback string eliminates the
divergence.

### 6.3 Session TTL

`SESSION_TTL_SECONDS = 30 * 86_400`. The bounds test asserts
`3600 ≤ TTL ≤ 31 days` and `FORM_TOKEN_TTL_SECONDS < SESSION_TTL_SECONDS`.

### 6.4 Why not sliding expiry

A rolling session (extend `expires_at` on access) would require a D1 UPDATE on
every authenticated request. For a service targeting the Workers Free CPU/IO
budget and explicitly worried about D1 query pressure (RFC-029, RFC-044), a
fixed 30-day window is the better trade: members active within any 30-day
period stay logged in, with zero per-request write cost. The unused
`session::touch` helper is retained for a future RFC should rolling sessions
become desirable.

---

## 7. Data Model Notes

No schema change. `sessions` already has `expires_at`, `revoked_at`, and
`last_seen_at`. `last_seen_at` remains available but is not written per-request
under the fixed-window decision.

---

## 8. API and UI Contract Notes

- No endpoint changes.
- Session-expiry copy continues to follow RFC-026 plain-language rules:
  "For safety, please enter a new invite code from your community admin."

---

## 9. Security, Privacy, and Safety

- **Cookie attributes** remain `HttpOnly; Secure; SameSite=Strict`. Host-only
  scoping is *stricter* than a shared `Domain`, reducing the cookie's reach.
- **Pepper integrity.** A single accessor ensures all HMACs (invite codes,
  sessions, form tokens, calendar bearer tokens) are computed with the same
  pepper. Operators must set `HMAC_PEPPER` as a secret in staging/production;
  the launch runbook and release checklist enforce this. The dev fallback exists
  only for `wrangler dev`.
- **Longer sessions** are bounded (30 days), revocable on logout, and do not
  store any secret in JavaScript-accessible storage (the secret lives only in
  the HttpOnly cookie; the server stores only its HMAC).

---

## 10. Acceptance Criteria

1. On a non-localhost host, login sets a cookie with no `Domain` (or the
   configured one) and the session round-trips.
2. No code path emits `Domain=localhost`.
3. `grep` finds no `env.secret("HMAC_PEPPER")` / `env.var("HMAC_PEPPER")` reads
   outside `crypto::pepper`; no `"dev-pepper"` divergent fallback remains.
4. `SESSION_TTL_SECONDS == 30 days`; bounds and ordering tests pass.

All met in v0.23.0.

---

## 11. Test Plan

- **Unit (shipped):** session TTL bounds and form-token-shorter-than-session
  ordering in both `auth.rs` and `release_gates.rs`.
- **Manual (pre-pilot gate):** deploy to a staging Cloudflare host, confirm
  login round-trips and that an unset `SESSION_COOKIE_DOMAIN` yields a host-only
  cookie (inspect `Set-Cookie`). Listed in the launch runbook.

---

## 12. Rollout Plan

Shipped in v0.23.0. Operators must ensure `HMAC_PEPPER` is set as a secret per
environment before deploy (pre-existing requirement, now uniformly enforced).
No data migration. Existing sessions keep their original `expires_at`; only new
sessions get the 30-day window.

---

## 13. Open Decisions

- Whether to adopt rolling sessions later (would supersede §6.4) — left to a
  future RFC if the re-invite burden proves higher than expected even at 30
  days.
