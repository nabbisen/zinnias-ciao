# ciao.zinnias — Roadmap

## Status

**v0.33.1** — 42 of 55 RFCs implemented (13 proposed). 213 passing unit tests. Zero warnings.

**Pre-pilot hardening complete.** All in-repo code work for the pilot is done.
RFC-055 (offline read-only contract) closed with a one-line app.js enhancement
that disables status/note submit buttons while offline and shows Japanese copy.
The ICS feed scope (RFC-053) was verified in source: title and times only,
no participant or member data.

All remaining pilot gates require a live Cloudflare environment, human review,
or explicit product/operator decisions — none can be progressed in-repo:
- RFC-050: staging deployment and runtime evidence pack
- RFC-051: multi-day edit semantics (product decision needed)
- RFC-052: audit retention policy (document first)
- RFC-053: ICS privacy UX copy (needs RFC-054)
- RFC-054: Japanese copy review (native-speaker reviewer)
- RFC-044/049: live-D1 integration harness (gates beta)
- Operator tasks: secrets, migrations, Logpush

---

## Implemented (done/)

| RFC | Feature | Shipped |
|-----|---------|---------|
| 001 | Project bootstrap: Cloudflare Workers, D1, SSR | v0.1.0 |
| 002 | Data model and D1 migrations | v0.1.0 |
| 003 | Invite redemption and session auth | v0.2.0 |
| 004 | Community isolation and authorization | v0.2.0 |
| 005 | Member home and event detail UI | v0.3.0 |
| 006 | Participation status lifecycle | v0.3.0 |
| 007 | Notes and comment safety | v0.3.0 |
| 008 | Offline cache and mutation queue | v0.4.0 |
| 009 | Admin event management | v0.4.0 |
| 010 | Admin invite and member management | v0.4.0 |
| 011 | Accessibility and design system | v0.4.0 |
| 012 | Security hardening and abuse controls | v0.5.0 |
| 013 | API contracts, error model, idempotency | v0.5.0 |
| 014 | Observability, audit, privacy logging | v0.5.0 |
| 015 | Testing, QA, and release gates | v0.6.0 |
| 016 | Deployment environments and operations | v0.6.0 |
| 017 | PWA installability and service worker | v0.6.0 |
| 018 | Timezone and event cutoff policy | v0.7.0 |
| 019 | Retention, soft-delete, data lifecycle | v0.7.0 |
| 022 | Recurring events (bounded materialization) | v0.17.0 |
| 023 | Calendar export and ICS interop | v0.10.0 |
| 025 | Community moderation, abuse response | v0.13.0 |
| 026 | Multi-language and plain-language localisation | v0.10.0 |
| 027 | Import/export, data portability | v0.15.0 |
| 028 | Backup, restore, disaster recovery | v0.14.0 |
| 029 | Scalability and query performance | v0.12.0 |
| 030 | Admin onboarding and first-run experience | v0.14.0 |
| 032 | Event templates and quick-create | v0.16.0 |
| 035 | Support diagnostics and user help | v0.15.0 |
| 036 | Public release readiness | v0.15.0 |
| 037 | Token subject normalization and form-token atomicity | v0.23.0 |
| 038 | Session and secret binding hardening | v0.23.0 |
| 039 | Timezone-correct event write path | v0.23.0 |
| 040 | Event edit contract | v0.23.0 |
| 041 | Atomic invite redemption | v0.23.0 |
| 042 | Pilot offline and private cache contract | v0.23.0 |

---

## Backlog (proposed/) — blocked

| RFC | Feature | Blocker |
|-----|---------|---------|
| 020 | Design assets and prototype handoff | Design team deliverable; not code |
| 021 | Post-MVP notification strategy | Notification infrastructure not yet set up |
| 024 | Display name recovery and account relinking | Self-healing once OIDC (AD-2) lands; defer |
| 031 | Consentful contact channels | Requires notification infrastructure (RFC-021 first) |
| 033 | Subgroups and event visibility | Needs explicit product decision on scope |
| 034 | Notification-free quiet mode | Depends on RFC-021 notification system |
| 043 | Pilot UX acceptance and error feedback | Error banners (v0.23.0) + no-JS confirmations (v0.24.0) done; device QA pending |
| 044 | D1 query-budget gate and integration test harness | CI tooling; gates beta (not first pilot) |

---

## Before first pilot deployment

These are the remaining gates before the first real community can use the service.

### Operator tasks (not in code)

- [ ] Apply all 6 D1 migrations to staging; rehearse rollback.
- [ ] Set `HMAC_PEPPER` secret via `wrangler secret put` (one per environment, different values).
- [ ] Set `SESSION_COOKIE_DOMAIN` as a **`[vars]` binding** in `wrangler.toml` (not a secret — see RFC-038; leave unset for host-only cookie).
- [ ] Configure Logpush for production (Cloudflare dashboard → R2 or S3).
- [ ] Run security review against the release checklist.

### Browser / device QA (not automatable in CI)

- [ ] Core join-and-mark-attendance flow under 2 minutes on a real phone.
- [ ] Event Detail readable and usable at 200% system text scaling.
- [ ] No-JS destructive confirmations work without scripting (cancel event, remove member, delete own note, admin remove note). *(implementation ships with v0.23.x; verify on a browser with scripting disabled)*

### Release gate (process)

- **Do not deploy to production** (or tag v1.0.0) without explicit confirmation from nabbisen.

---

## After first pilot

Once a pilot community has been running for at least 4 weeks, revisit:

1. **RFC-033 (Subgroups)** — only if privacy needs emerge from real usage.
2. **RFC-021 (Notifications)** — only if sync-based checking proves insufficient.
3. **RFC-024 (Account relinking)** — superseded if OIDC is added first; useful if not.

The guiding principle remains: add only what is needed. Every feature added
is a feature that must be maintained, explained, and trusted.
