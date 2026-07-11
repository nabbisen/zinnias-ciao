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

**Production:** follow `docs/src/maintainer/launch-runbook.md` §4.4 and §5 for the full
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

## Additional community creation

RFC-057 adds an in-app route for additional communities:

```text
GET/POST /communities/new
```

This does not replace first-community bootstrap. The first production and
staging community still comes from the operator runbook above. The in-app route
is only for authenticated users who are already active admins in at least one
community.

Runtime flag:

```toml
COMMUNITY_CREATION_ENABLED = "true"  # dev/staging review
COMMUNITY_CREATION_ENABLED = "false" # production default
```

When enabled, eligible admins see `新しいコミュニティを作る` on the Me page.
Creation writes a new `communities` row, the creator's first admin membership,
and audit events `community.created` and `membership.created_first_admin`.
It does not copy members/events/templates/notes and does not generate invite
codes. The new admin uses the existing invite screen intentionally.

### Timezone scope (important)

The `communities.timezone` column accepts an IANA zone name (e.g. `Asia/Tokyo`).
Event times entered by an admin are converted from community-local time to UTC
at write time and back to local at display time.

> **MVP timezone conversion is validated for fixed-offset zones such as
> `Asia/Tokyo`. Zones that observe daylight saving time (e.g.
> `America/New_York`, `Europe/London`) are not yet supported for correct
> summer-time scheduling** — conversion uses each zone's standard-time offset
> year-round, so events in a DST-observing zone will be off by one hour during
> that zone's summer. For the Japan-first pilot this is not a concern (Japan
> does not observe DST). Do not provision communities in DST-observing zones
> until DST support lands (tracked as a future RFC-018 amendment).

## Session revocation

To revoke a compromised session immediately:

```sh
bunx wrangler d1 execute zinnias-ciao --remote --env production --command \
  "UPDATE sessions SET revoked_at = datetime('now') WHERE id = '<session_id>'"
```

## Invite code revocation

Via the admin UI: Communities → Invite Members → Revoke button on the active code.

Via SQL (emergency):

```sh
bunx wrangler d1 execute zinnias-ciao --remote --env production --command \
  "UPDATE invite_codes SET revoked_at = datetime('now') WHERE id = '<invite_id>'"
```

## Helping an active member sign in again

Use this only for an active member who lost browser or device access. In the
member-management page, choose `サインインを手伝う` for the intended active member
and share the generated code only with that person.

The code is a bearer credential: anyone with it can sign in as that member. It
is shown once, expires after 15 minutes, and can be used once. After successful
redemption, the app creates a new session for the member and revokes other
active sessions for that same invite-era user identity.

This is not a removed-member return flow. Removed members still receive a new
invite and join as a new membership; old and new memberships are not merged by
display name.

## Total community access recovery

Use this only when every reachable community admin and member has lost browser
or session access, and no valid invite or relink code exists. This is an
operator procedure, not a public self-service recovery feature.

Before mutation:

1. Confirm the request through the external operator policy.
2. Identify the community by `community_id`, not display name only.
3. Identify one existing active admin membership by `membership_id`.
4. Review recent audit rows:
   ```sql
   SELECT * FROM audit_log
   WHERE community_id = '<community_id>'
   ORDER BY created_at DESC
   LIMIT 50;
   ```

Open the temporary recovery window in the ignored Wrangler config for the
target environment:

```toml
COMMUNITY_RECOVERY_ENABLED = "true"
```

Set a short-lived operator bearer secret for the same environment:

```sh
COMMUNITY_RECOVERY_TOKEN="$(openssl rand -hex 32)"
printf '%s\n' "$COMMUNITY_RECOVERY_TOKEN" | bunx wrangler secret put \
  COMMUNITY_RECOVERY_TOKEN --env staging --config wrangler.staging.local.toml
```

Use `--env production --config wrangler.production.local.toml` for an approved
production recovery. Do not commit the enabled flag or the token. Deploy after
changing the flag:

```sh
bunx wrangler deploy --env staging --config wrangler.staging.local.toml
```

Run the maintained recovery script:

```sh
bun run recover:community-access -- \
  --target staging \
  --url https://<staging-worker-url> \
  --community-id com_... \
  --admin-membership-id mem_... \
  --operator-label INC-1234
```

Production additionally requires `--confirm-production` and an interactive
confirmation. The script prints one `/relink` code; give it only to the intended
existing active admin. Do not redirect the script output into evidence files.

After successful recovery, close the temporary window:

```toml
COMMUNITY_RECOVERY_ENABLED = "false"
```

```sh
bunx wrangler secret delete COMMUNITY_RECOVERY_TOKEN --env staging \
  --config wrangler.staging.local.toml

bunx wrangler deploy --env staging --config wrangler.staging.local.toml
```

Then verify `POST /operator/recovery/community-access` returns the same generic
not-found response, and review `audit_log` for
`operator_recovery.admin_relink_created` plus the following
`membership.relink_redeemed` row with the same `relink_code_id`.

Do not use `scripts/bootstrap-cloudflare.mjs` for routine community access
recovery. Bootstrap rotates `HMAC_PEPPER`, which invalidates existing sessions,
invite codes, relink codes, and form tokens for the whole environment.

## Returning removed members

Member removal stops access but preserves past attendance, notes, and audit
records on the old membership. To bring someone back, send a new invite. The
returning person joins as a new membership; past records stay on the old
membership and are not merged by display name.

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

For persistent log storage, configure Logpush (see `docs/src/maintainer/launch-runbook.md` §6).
Logs should never contain plaintext invite codes, session tokens, or note content —
this is enforced by the audit writer (RFC-014).
