# Operations

## Health check

```sh
curl https://<your-worker>.workers.dev/healthz
# {"ok":true,"service":"ciao.zinnias"}
```

## Version

```sh
curl https://<your-worker>.workers.dev/version
# {"ok":true,"version":"<build-hash>"}
```

## Bootstrap: first community and admin

The initial community and admin membership must be seeded via a direct D1 SQL operation (operator-level, not the normal UI):

```sql
INSERT INTO communities (id, name, timezone, is_active, created_at)
VALUES ('com_xxx', 'My Community', 'Asia/Tokyo', 1, datetime('now'));

INSERT INTO users (id, created_at) VALUES ('usr_xxx', datetime('now'));

INSERT INTO community_memberships
  (id, community_id, user_id, role, display_name, joined_at)
VALUES ('mem_xxx', 'com_xxx', 'usr_xxx', 'admin', 'Admin', datetime('now'));
```

Then generate an invite code for the admin via the admin UI once logged in, or seed a session directly for the first login.

## Incident response

1. Identify the affected community and actor from the `audit_log` table.
2. Revoke the affected session: `UPDATE sessions SET revoked_at = datetime('now') WHERE id = '...'`.
3. If an invite code was compromised: `UPDATE invite_codes SET revoked_at = datetime('now') WHERE id = '...'`.
4. Review `audit_log` for actions taken during the incident window.
5. Notify affected community admin.
