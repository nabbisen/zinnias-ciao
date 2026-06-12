# RFC 048 — Pilot Security Headers and Cache-Control Gate

**Status.** Implemented (v0.30.0)
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Hardening RFC. Closes architect-review v0.29.0 finding P1 (Cache-Control no-store for authenticated HTML, CSP expansion, Permissions-Policy). Extends the existing `attach_security_headers` in `lib.rs`. Accompanies RFC-049 (language) as a joint pre-pilot milestone.

---

## 1. Summary

The v0.29.0 architect review identified two gaps in browser security headers:

1. Authenticated HTML pages did not send `Cache-Control: no-store`. Although the service worker already refuses to cache authenticated HTML (RFC-042), the browser's own HTTP cache and any intermediary proxy could still cache responses without an explicit no-store directive.
2. The Content Security Policy was missing `base-uri 'self'`, `form-action 'self'`, and `object-src 'none'` directives, and the `Referrer-Policy` was `strict-origin-when-cross-origin` rather than the stricter `same-origin` recommended for a private community service.
3. `Permissions-Policy` was absent entirely.

This RFC adds all missing headers to `attach_security_headers` in `lib.rs`, which is called on every response.

## 2. Motivation

ciao.zinnias handles community membership, attendance status, and notes that users expect to remain private. Browser cache leakage after logout is a real failure mode on shared devices — a family member or colleague could see a cached community page without authenticating. `no-store` prevents this at the HTTP layer.

For the Japan-first pilot targeting IT-averse users who may use shared devices, this is not a theoretical concern.

## 3. Goals

- Add `Cache-Control: no-store` as the default for all responses that do not already set a cache header (static assets retain `public, max-age=N`).
- Tighten CSP with `base-uri`, `form-action`, and `object-src`.
- Add `Permissions-Policy` to disable camera, microphone, and geolocation APIs.
- Change `Referrer-Policy` to `same-origin` (no referrer on cross-origin requests).
- Document the `style-src 'unsafe-inline'` exception explicitly.

## 4. Non-Goals

- Removing `style-src 'unsafe-inline'`. The SSR templates use ~272 inline `style=` attributes; extracting them to CSS classes is a separate refactor. The exception is documented in the code.
- Nonce-based CSP (future RFC).
- HSTS (`Strict-Transport-Security`): already enforced by Cloudflare for Workers deployments; no action needed in application code.

## 5. External Behavior

| Header | Before | After |
|---|---|---|
| `Cache-Control` | Not set on HTML pages | `no-store` (default for all routes without explicit caching) |
| `Content-Security-Policy` | `default-src 'self'…` without `base-uri`/`form-action`/`object-src` | Full baseline CSP |
| `Permissions-Policy` | Absent | `camera=(), microphone=(), geolocation=()` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | `same-origin` |
| Static assets | `public, max-age=N` (set by handler) | Unchanged (handler sets before `attach_security_headers` runs; `no-store` only applies when no `Cache-Control` already present) |
| ICS/Export | `no-store, private` (set by handler) | Unchanged |

## 6. Internal Design

`attach_security_headers` in `lib.rs` is called on every response path (success and error). Cache-Control is set only when the handler has not already set one:

```rust
if h.get("Cache-Control").ok().flatten().is_none() {
    h.set("Cache-Control", "no-store")?;
}
```

Static asset handlers (CSS, JS, manifest) set `Cache-Control: public, max-age=N` before returning, so the conditional gate preserves their headers. The service worker handler sets `no-cache` (correct for SW update behavior) — also preserved.

## 7. Security Analysis

**`base-uri 'self'`:** prevents `<base>` tag injection redirecting form submissions to an attacker's domain.

**`form-action 'self'`:** ensures all form POSTs must target the same origin. ciao.zinnias only posts to its own paths; this is a defense-in-depth restriction.

**`object-src 'none'`:** disables Flash, Java applets, and similar plugin content. Correct for a pure HTML service.

**`Permissions-Policy`:** reduces the browser API surface exposed to any injected content.

**`same-origin` Referrer-Policy:** no referrer sent cross-origin, preventing community IDs or usernames in the URL from leaking to third-party resources. ciao.zinnias loads no third-party resources in HTML, but this protects against future changes.

**`style-src 'unsafe-inline'` exception:** documented in code with the count of inline style= attributes. Not a new security surface — the existing XSS protection (`escape_html` from `contracts::html`, used at every user-content insertion) prevents content injection regardless of whether inline styles are allowed.

## 8. Acceptance Criteria

- Browser DevTools show `Cache-Control: no-store` on authenticated HTML pages.
- Browser DevTools show `Cache-Control: public, max-age=N` on CSS/JS.
- CSP visible in browser DevTools Security tab with all five directives.
- `Permissions-Policy` present.
- After logout, refreshing a previously viewed page does not serve from browser cache.

## 9. Test Plan

- Compile-level: `cargo check --target wasm32-unknown-unknown`, zero warnings.
- Runtime: staging smoke test (RFC-050 §S8 in RFC-045).
- Release gate: add a named check to `release_gates.rs` documenting the expected header set (assertion would need a running worker; documented as a staging item).

## 10. Open Decisions

- **Nonce-based CSP to remove `unsafe-inline`:** deferred to a future RFC after the pilot. The MVP inline-style volume makes this a non-trivial refactor.
