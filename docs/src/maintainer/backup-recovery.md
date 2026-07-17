# Backup and Recovery (RFC-028)

## Overview

ciao.zinnias stores all persistent state in a single Cloudflare D1 database.
D1 is SQLite-compatible; its backup strategy is therefore SQLite-compatible.

Two types of protection:

| Type | Tool | When |
|------|------|------|
| Automated point-in-time | Cloudflare D1 Time Travel (7 days on Workers Free; 30 days on Workers Paid; verify current limits) | Continuous |
| Manual export | `wrangler d1 export` | Before every migration; on demand |

---

## D1 Time Travel

Time Travel is D1's built-in point-in-time recovery mechanism for databases on
the production storage backend. It is always enabled, but retention depends on
the Workers plan: up to 7 days on Workers Free and up to 30 days on Workers
Paid, subject to the current Cloudflare D1 limits. Confirm the active plan,
database backend, and current limits in the
[Cloudflare D1 Time Travel documentation](https://developers.cloudflare.com/d1/reference/time-travel/)
before relying on a restore point.

To restore with Time Travel:

1. Run `wrangler d1 info zinnias-ciao` and confirm the database reports the
   production backend.
2. Retrieve the bookmark for the required timestamp with
   `wrangler d1 time-travel info zinnias-ciao --timestamp=<RFC3339-or-Unix>`.
3. Confirm the selected point is inside the plan's available retention window.
4. Restore with
   `wrangler d1 time-travel restore zinnias-ciao --bookmark=<bookmark>`.
   This destructively overwrites the current database and cancels in-flight
   queries.

After the command completes, verify with `GET /healthz` and a test sign-in.

---

## Manual export (before migrations)

Always export before applying any migration to production:

```sh
# Export to a local SQL file
bunx wrangler d1 export zinnias-ciao --env production \
  --output backup-$(date +%Y%m%d-%H%M%S).sql

# Verify the file is non-empty
wc -l backup-*.sql
```

The export file is a portable SQLite dump. Store it securely — it contains
community names, display names, event content, and audit history.

Any backup created before RFC-079 migration 0010 may also contain unsafe legacy
`audit_log.metadata_json`. Classify it as potentially sensitive without opening,
querying, or copying that metadata into tickets or review evidence. Restrict
access to the minimum operator group and apply the approved retention/deletion
policy to the backup as a whole.

**Never commit backup files to source control.**

---

## Restore from a manual export

If a migration must be reversed and D1 Time Travel is unavailable:

```sh
# 1. Create a new replacement database
bunx wrangler d1 create zinnias-ciao-restored

# 2. Import the backup
bunx wrangler d1 execute zinnias-ciao-restored --remote --env production \
  --file backup-YYYYMMDD-HHMMSS.sql

# 3. Apply every forward migration, including 0010, before binding a Worker.
#    Do not inspect or export legacy audit metadata during this step.
bunx wrangler d1 migrations apply zinnias-ciao-restored --remote \
  --config wrangler.production.local.toml

# 4. Update ignored wrangler.production.local.toml to point to the new database ID, then deploy
#    wrangler.production.local.toml [[env.production.d1_databases]] database_id = "<new id>"
bunx wrangler deploy --env production --config wrangler.production.local.toml

# 5. Verify, then delete the old broken database when confident
```

A restore replaces the database; any changes made since the backup point are
lost. Communicate downtime to community admins before restoring. A restored
pre-0010 database must never receive traffic until migration 0010 has reset
legacy metadata and the compatible forward-only application candidate is ready.

---

## Backup schedule recommendation

| Trigger | Action |
|---------|--------|
| Before any `bun run migrate:prod` | `wrangler d1 export` |
| Weekly (production with active users) | `wrangler d1 export` |
| Before any deployment with schema-touching code | `wrangler d1 export` |

Store exports for at least 90 days in a private location (R2 bucket, encrypted
local storage, or secure cloud storage). Never store alongside application secrets.

---

## Sensitive data in backups

D1 exports contain:

- Community names and event titles.
- Member display names and notes.
- Attendance records.
- Audit log entries. Pre-0010 backups may contain untrusted arbitrary metadata
  even though note content was prohibited by policy.

They do **not** contain:

- Session tokens (stored as HMACs; originals never persisted).
- Plaintext invite codes (stored as HMACs).
- The `HMAC_PEPPER` secret (lives in Wrangler secrets, not D1).

Treat backup files as confidential. Restrict access to operators only.

---

## Migration reversibility

D1 migrations are forward-only. To undo a migration:

1. Write a new migration that reverses the schema change.
2. Apply it via `bun run migrate:prod`.
3. Document the reversal in `CHANGELOG.md`.

Never delete rows from the `d1_migrations` table to "reset" a migration —
this will cause the migration to re-apply and may cause data loss.

Migration 0010 is a destructive privacy boundary. Roll-forward recovery must
not restore arbitrary legacy metadata, the shallow redactor, stringly typed
actions, the removed compatibility adapter, or best-effort required-audit
behavior. If a post-0010 problem occurs,
keep traffic stopped, write a new reviewed forward migration or restore into an
isolated replacement database, then apply 0010 before verification. Do not use
schema rollback to make old metadata live again.

A code rollback must remain compatible with the post-0010 schema and preserve
the RFC-079 Class A/B/C failure rules. Do not roll back to Packages 1–6 merely
because Package 7 is the earliest deployable code boundary; release/deployment
still requires exact-candidate evidence and explicit owner approval.

---

## Incident response checklist

When data loss or corruption is suspected:

- [ ] Stop writes if possible (take the Worker offline temporarily).
- [ ] Export the current database state immediately (even if corrupted).
- [ ] Check D1 dashboard for available point-in-time restore points.
- [ ] Identify the last known-good backup.
- [ ] Restore to staging first; verify with test sign-in.
- [ ] Restore to production; notify community admins.
- [ ] Write a post-incident note in `CHANGELOG.md`.
- [ ] Add a process improvement if the incident was preventable.
