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

**Local development:** `bun run setup` seeds one community, one admin membership,
and one bootstrap invite code with `grants_role = 'admin'`. The invite code is
printed at the end. Visit `/join` and enter it — the first sign-in creates an
admin membership.

**Production:** follow `docs/src/launch-runbook.md` §4.4 and §5 for the full
procedure. The key point is that the bootstrap invite must be inserted with
`grants_role = 'admin'` (migration 0003 column) so the first sign-in creates an
admin rather than a member.

```sql
-- Minimal production seed (use launch-runbook.md for the full procedure)
INSERT INTO communities (id, name, timezone, is_active, created_at)
VALUES ('com_xxx', 'My Community', 'Asia/Tokyo', 1, datetime('now'));

INSERT INTO users (id, created_at) VALUES ('usr_xxx', datetime('now'));

INSERT INTO community_memberships
  (id, community_id, user_id, role, display_name, joined_at)
VALUES ('mem_xxx', 'com_xxx', 'usr_xxx', 'admin', 'Admin', datetime('now'));

-- Invite code: HMAC must be HMAC-SHA256(pepper, uppercase(code))
-- grants_role = 'admin' so the first redemption creates an admin
INSERT INTO invite_codes
  (id, community_id, code_hmac, created_by_membership_id, expires_at, grants_role, created_at)
VALUES ('inv_xxx', 'com_xxx', '<computed_hmac>', 'mem_xxx', '2099-12-31T23:59:59Z', 'admin', datetime('now'));
```

## Session revocation

To revoke a compromised session immediately:

```sh
bunx wrangler d1 execute zinnias-ciao --env production --command \
  "UPDATE sessions SET revoked_at = datetime('now') WHERE id = '<session_id>'"
```

## Invite code revocation

Via the admin UI: Communities → Invite Members → Revoke button on the active code.

Via SQL (emergency):

```sh
bunx wrangler d1 execute zinnias-ciao --env production --command \
  "UPDATE invite_codes SET revoked_at = datetime('now') WHERE id = '<invite_id>'"
```

## Incident response

1. Identify the affected community and actor from the `audit_log` table:
   ```sql
   SELECT * FROM audit_log WHERE community_id = '<cid>' ORDER BY created_at DESC LIMIT 50;
   ```
2. Revoke the affected session (see above).
3. If an invite code was compromised, revoke it (see above).
4. Review `audit_log` for actions taken during the incident window.
5. Notify the affected community admin.

## Log access

V8 isolates have no filesystem. Logs are accessible via:

```sh
# Real-time (development / incident investigation)
bunx wrangler tail --env production
```

For persistent log storage, configure Logpush (see `docs/src/launch-runbook.md` §6).
Logs should never contain plaintext invite codes, session tokens, or note content —
this is enforced by the audit writer (RFC-014).
