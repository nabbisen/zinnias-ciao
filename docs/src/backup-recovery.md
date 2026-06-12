# Backup and Recovery (RFC-028)

## Overview

ciao.zinnias stores all persistent state in a single Cloudflare D1 database.
D1 is SQLite-compatible; its backup strategy is therefore SQLite-compatible.

Two types of protection:

| Type | Tool | When |
|------|------|------|
| Automated point-in-time | Cloudflare D1 built-in (30-day retention) | Continuous |
| Manual export | `wrangler d1 export` | Before every migration; on demand |

---

## D1 built-in backups

Cloudflare D1 automatically retains point-in-time snapshots for 30 days on all
plans. To restore from a snapshot:

1. Open the Cloudflare dashboard → **Workers & Pages → D1**.
2. Select the `zinnias-ciao` database.
3. Click **Backups** and choose a restore point.
4. Confirm the restore — this overwrites the current database.

> **Note:** D1 point-in-time restore is eventual; it may take a few minutes to
> propagate. Verify with `GET /healthz` and a test sign-in after restore completes.

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
community names, display names, and event content.

**Never commit backup files to source control.**

---

## Restore from a manual export

If a migration must be reversed and D1's point-in-time restore is unavailable:

```sh
# 1. Create a new replacement database
bunx wrangler d1 create zinnias-ciao-restored

# 2. Import the backup
bunx wrangler d1 execute zinnias-ciao-restored --env production \
  --file backup-YYYYMMDD-HHMMSS.sql

# 3. Update wrangler.toml to point to the new database ID, then deploy
#    wrangler.toml [[env.production.d1_databases]] database_id = "<new id>"
bunx wrangler deploy --env production

# 4. Verify, then delete the old broken database when confident
```

A restore replaces the database; any changes made since the backup point are lost.
Communicate downtime to community admins before restoring.

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
- Audit log entries (without note content).

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
