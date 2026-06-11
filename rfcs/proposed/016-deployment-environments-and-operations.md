# RFC 016 — Deployment Environments and Operations

**Status.** Proposed  
**Phase:** M6 / Deployment Readiness  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Reconciled:** multi-config wrangler for the workspace's worker(s); Logpush to R2/S3 for log persistence (no filesystem on isolates).
**Related roadmap milestone:** M6 / Deployment Readiness  

---

## 1. Summary

This RFC defines deployment environments, configuration, migrations, and operational practices for Cloudflare-based deployment.

---

## 2. Goals

- Separate dev, staging, and production environments.
- Document configuration and secret handling.
- Define migration process.
- Define release and rollback procedure.
- Provide basic operational checklist.

---

## 3. Non-Goals

- No multi-region enterprise topology design beyond Cloudflare baseline.
- No custom Kubernetes or VM deployment path.
- No advanced observability stack in MVP.

---

## 4. External Behavior

Users should experience stable URLs and clear maintenance/error messages. They should never see environment names except in non-production testing.

Admins should not need to understand deployment mechanics.

---

## 5. Internal Design

Environment model:

```text
dev: local developer testing
staging: release candidate testing with non-production data
production: real communities
```

Configuration:

- D1 binding per environment;
- cookie name per environment;
- session lifetime;
- allowed origins;
- rate-limit config;
- log level;
- build version;
- Logpush destination (R2/S3) — isolates have no filesystem, so log persistence is Logpush, never file-based.

Local/dev uses `wrangler dev` with multi-config when more than one worker exists (e.g. `bun run dev`).

Migrations:

- forward migration files are versioned;
- staging migration rehearsed before production;
- destructive changes require backup/export/operator approval;
- migration failures stop deployment.

---

## 6. Data and API Design

Operational routes:

```text
GET /healthz
GET /version
```

`/version` may return build hash/version without secrets.

Deployment docs should include:

- local setup;
- D1 migration apply;
- staging deploy;
- production deploy;
- rollback notes;
- incident contact/process.

---

## 7. Security, Privacy, and Safety

- Production secrets are never committed.
- Staging must not use production invite/session data.
- Logs must be privacy-preserving.
- Rollback must consider database migration compatibility.

---

## 8. Acceptance Criteria

- Local, staging, production configs are documented.
- Production deploy command is documented.
- Migration process is documented and tested in staging.
- Health/version routes work.
- Rollback/recovery notes exist.

---

## 9. Test Plan

- Staging deployment smoke test.
- Migration rehearsal test.
- Config validation test.
- Health/version response test.
- Manual rollback tabletop exercise before first production release.

---

## 10. Open Questions / Decisions

Open decision: exact production deployment region/data residency behavior should be confirmed with project owner before real communities are onboarded.
