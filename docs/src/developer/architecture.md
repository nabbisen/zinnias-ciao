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
      audit.rs               Closed typed audit model, atomic helpers, bounded events (RFC-079)
      abuse_control.rs       Fail-closed abuse-control coordinator client: ingress
                             validation, HMAC subject digesting, reserve/reset (RFC-078)
      abuse_limiter.rs       AbuseLimiter Durable Object: SQLite transition, alarm cleanup,
                             private /v1/reserve + /v1/reset protocol (RFC-078)
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
  0007_codlet_tables.sql     codlet codes, sessions, and form-token auth tables
  0008_membership_relink_codes.sql
  0009_recurrence_v2.sql
  0010_audit_integrity.sql   closed audit schema, legacy metadata reset, assertion table (RFC-079)
  0011_membership_ui_language.sql  nullable ui_language on community_memberships (RFC-072 Slice A)
  0012_session_provenance.sql  provenance + scope_community_id on sessions; revokes pre-existing rows (RFC-081 §2, Handoff 048)
  0013_identity_namespaces.sql  identity_namespaces + user_identities tables, additive only, seeds the local-fake namespace (RFC-080 §3, Handoff 050)
scripts/
  setup.mjs                  Dev bootstrap: D1 migrations + seed data
docs/src/                    mdbook documentation (SUMMARY.md is the index)
rfcs/
  done/                      Implemented RFCs
  accepted/                  Reviewed, owner-approved implementation queue
  proposed/                  Design under review; implementation not authorized
  README.md                  RFC index
```

## Key architecture decisions

The four locked decisions are in `docs/src/shared/ref/roadmap-and-rfcs-v1/ARCHITECTURE-DECISIONS.md`.
Summary:

- **AD-1** SSR + progressive enhancement. Forms are `<form method="post">` + 303 redirect.
  State changes never depend on client-side JavaScript. No browser WASM bundle.
  Service worker caches GET responses only; POSTs go to the network.
- **AD-2** Invite-code + cookie session now; OIDC deferred. The originally reserved
  `users.idp_subject` (singleton, namespace-free) was rejected by RFC-080 §3.4 and
  removed; external identity's migration path is `user_identities` (migration
  0013), keyed on `(identity_namespace_id, subject_lookup)` so a subject is never
  assumed to mean the same person across two namespaces. Lost-session recovery is
  admin-mediated: active members can use RFC-024 help-signin codes, while removed
  members return through a new invite under RFC-063.
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
- **Abuse control**: invite/relink redemption and community creation are gated by the
  `AbuseLimiter` Durable Object — one SQLite-backed, HMAC-sharded object per
  scope/subject, reserved atomically before credential lookup or mutation. Fail-closed:
  a missing binding, storage failure, or malformed coordinator response is `Unavailable`,
  never `Allowed` (RFC-078, replacing the earlier fail-open KV counters).

## Audit integrity boundary

RFC-079 classifies 23 mutation actions as Class A, two export authorization
actions as Class B, and logout as the sole Class C action. Class A helpers keep
business writes and typed audit records in one D1 batch. Class B writes audit
evidence before returning protected data or acknowledgement and fails with a
generic `503`. Logout revokes first, awaits an audit attempt carrying no
session/subject identifier, emits a bounded incident on failure, and always
continues to cookie clearing.

Only `workers/ssr/src/audit.rs` owns production audit INSERT SQL. Callers use
closed `AuditAction`/`AuditMetadata` variants; arbitrary JSON, compatibility
writers, ignored results, raw-ID audit logs, and background required-audit work
are prohibited by repository-wide release gates. Passing these removal gates
is the earliest deployable code boundary, not release or deployment approval.

For the durable security map and form-review baseline, see
[Application Threat Model](security-threat-model.md).

## Test strategy

```
packages/domain/    — pure-Rust validation, status, and recurrence tests
packages/contracts/ — token/i18n/session/error contracts and repository-wide release gates
workers/ssr/        — native handler/helper unit tests plus WASM compilation
scripts/            — disposable local-D1 migration, rollback, concurrency, and response-boundary proofs
```

Worker-independent SSR helpers and render/handler logic have native unit tests.
Worker/D1 transaction semantics use disposable local Wrangler fixtures; hosted
and browser behavior remains part of the release checklist and RFC-050.

The mandatory local verification command:

```sh
cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts -p zinnias-ciao-ssr
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p zinnias-ciao-ssr --target wasm32-unknown-unknown
mdbook build docs
```

All applicable gates must pass with zero warnings before a release review.
