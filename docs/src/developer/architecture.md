# Architecture

## Workspace layout

```
Cargo.toml                   workspace root
packages/
  domain/                    pure business logic; native-testable; no Worker/WASM deps
  contracts/                 DTOs, error model, i18n strings (184 EN/JA pairs); native-testable
workers/
  ssr/                       Cloudflare Worker: SSR renderer + route handlers
    src/
      lib.rs                 Worker entry point + router
      db/                    D1 data-access layer (parameterised queries only)
        attendance.rs        Attendance reads + batch IN-clause helpers (RFC-029)
        calendar.rs          Calendar feed token helpers (RFC-023)
        community.rs         Community lookup + additional community creation helper (RFC-057)
        event.rs             Event + event_day queries
        event_note.rs        Note read/write + soft-delete + admin hide
        event_template.rs    Event template CRUD (RFC-032)
        event_write.rs       Event create/edit/cancel (includes repeat_rule; RFC-022)
        invite.rs            Invite code redemption + revocation
        membership.rs        Membership list + community switcher data
        session.rs           Session read + revocation
      handlers/              Route handlers
        admin.rs             Admin: event create/edit/cancel, invites, members, attendance
        auth.rs              Logout POST handler
        calendar.rs          Calendar feed page + ICS download (RFC-023)
        communities.rs       Calendar tab for active community (former Communities route)
        community.rs         Community-scoped router (dispatches admin/member routes)
        community_create.rs  Additional community creation flow (RFC-057)
        event.rs             Event detail, status update, note save/delete
        export.rs            Admin community data export (RFC-027)
        health.rs            GET /healthz  GET /version
        home.rs              Multi-community nearby-events Home + first-run admin card (RFC-056/RFC-030)
        join.rs              Invite redemption + display name collection
        me.rs                Me tab: profile, sync state, about (RFC-035)
        static_files.rs      Static asset handler
        templates.rs         Event template management (RFC-032)
      render.rs              HTML render helpers, escape_html, status_display
      session.rs             Session cookie middleware
      form_token.rs          Server-issued CSRF + idempotency tokens (AD-4)
      authz.rs               Community-scoped and active-admin-somewhere authorization guards
      audit.rs               Structured audit log writer (RFC-014)
      rate_limit.rs          KV-backed invite failure and community-creation counters
      crypto.rs              HMAC-SHA256 helpers (AD-3)
    static/
      app.css                Design tokens + base styles (RFC-011)
      app.js                 SW registration + progressive enhancement
      sw.js                  Service worker (read-only caching; no mutation queue — AD-1)
      manifest.webmanifest   PWA manifest (RFC-017)
migrations/
  0001_initial.sql           Full schema (communities, memberships, events, event_days,
                             attendances, event_notes, invite_codes, form_tokens,
                             calendar_tokens, users, sessions, audit_log)
  0002_form_tokens_nullable_user.sql
  0003_invite_grants_role.sql
  0004_calendar_tokens.sql
  0005_event_templates.sql   event_templates table (RFC-032)
  0006_event_recurrence.sql  repeat_rule / repeat_count columns on events (RFC-022)
scripts/
  setup.mjs                  Dev bootstrap: D1 migrations + seed data
docs/src/                    mdbook documentation (SUMMARY.md is the index)
rfcs/
  done/                      Implemented RFCs
  proposed/                  Backlog RFCs (see ROADMAP.md)
  README.md                  RFC index
```

## Key architecture decisions

The four locked decisions are in `docs/src/shared/ref/roadmap-and-rfcs-v1/ARCHITECTURE-DECISIONS.md`.
Summary:

- **AD-1** SSR + progressive enhancement. Forms are `<form method="post">` + 303 redirect.
  State changes never depend on client-side JavaScript. No browser WASM bundle.
  Service worker caches GET responses only; POSTs go to the network.
- **AD-2** Invite-code + cookie session now; OIDC deferred. `users.idp_subject` is nullable,
  preserving a migration path. Lost-session recovery is admin-mediated: active
  members can use RFC-024 help-signin codes, while removed members return
  through a new invite under RFC-063.
- **AD-3** Design to Workers Free (10 ms CPU). HMAC-SHA256 instead of slow KDFs.
  D1 queries and `fetch` are I/O (not CPU budget). No heavy crypto in hot paths.
- **AD-4** One server-issued form token per render = CSRF protection + idempotency.
  Token is purpose-bound, session-bound, single-use, 5-minute TTL.

## Data grain

```
Community → Event → EventDay → Attendance
                ↓
           EventNote (one per member per event)
```

Status is per `(event_day, membership)`. A one-day event has one `event_days` row;
multi-day and recurring events have N rows, all with independent attendance.
The ≤200-char note is per `(event, membership)`, not per day.

## Security model

- **Community isolation**: every query scoped by `community_id` verified against the session.
- **Secrets**: stored as `HMAC-SHA256(server_pepper, value)` — DB export alone cannot recover them.
- **XSS**: all user text passes through `render::escape_html()` — single render exit point.
- **CSRF**: form token (AD-4) + `SameSite=Strict` cookie.
- **Resource enumeration**: 404 and 403 return identical user-facing messages.
- **Rate limiting**: invite code failures counted in KV, hard-capped per IP window.

## Test strategy

```
packages/domain/   — 96 pure-Rust unit tests (validation, status transitions, recurrence)
packages/contracts/ — tests for token uniqueness, i18n parity, session gates, error model, and release gates
```

SSR handlers are not unit-tested (WASM environment); integration testing is via
the dev server (`bun run dev`) and browser-based smoke tests documented in the
release checklist.

The mandatory local verification command:

```sh
cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts
cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown
```

Both must pass with zero warnings before any commit.
