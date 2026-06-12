# Architecture

## Workspace layout

```
Cargo.toml                   workspace root
packages/
  domain/                    pure business logic; native-testable
  contracts/                 DTOs, error model, i18n strings; native-testable
workers/
  ssr/                       Cloudflare Worker: SSR renderer + handlers
    src/
      lib.rs                 Worker entry point + router
      db/                    D1 data-access layer (parameterised queries)
      handlers/              Route handlers (home, event, admin, join, auth, …)
      render.rs              HTML render helpers + escape_html
      session.rs             Session middleware
      form_token.rs          Server-issued CSRF + idempotency tokens (AD-4)
      authz.rs               Community-scoped authorization guard
      audit.rs               Structured audit log writer
      rate_limit.rs          KV-backed invite failure counter
    static/
      app.css                Design tokens + base styles
      app.js                 SW registration + progressive enhancement
      sw.js                  Service worker (read-only caching)
      manifest.webmanifest   PWA manifest
migrations/
  0001_initial.sql           Full schema
```

## Key architecture decisions

See `rfcs/proposed/ARCHITECTURE-DECISIONS.md` for the four locked decisions:

- **AD-1** SSR + progressive enhancement (no Leptos hydration/WASM).
- **AD-2** Invite-code + cookie session now; OIDC deferred with seam in `users.idp_subject`.
- **AD-3** Design to Workers Free (10 ms CPU); HMAC-SHA256 not slow KDFs.
- **AD-4** One server-issued form token per render = CSRF protection + idempotency.

## Data grain

`Community → Event → EventDay → Attendance`

Status is per `event_day`; the single ≤200-char note is per `event`.
A one-day event has one `event_days` row; multi-day is native.

## Security model

- Community isolation enforced at every query via `community_memberships`.
- Secrets stored as `HMAC-SHA256(server_pepper, value)` — DB leak alone cannot recover them.
- All user text escaped at render via `render::escape_html` — single exit point.
- CSRF = the form token (AD-4); combined with `SameSite=Strict` cookie.
- Generic 404 for missing/inaccessible resources — no resource-existence leak.
