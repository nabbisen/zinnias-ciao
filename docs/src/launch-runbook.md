# Launch Runbook

This document is the step-by-step operator guide for taking ciao.zinnias from a
clean tarball to a running production deployment. It is intended to be followed
exactly, in order, by one operator. Tick each step as you complete it.

**Version this runbook was written for:** v0.38.6  
**Estimated time:** 60–90 minutes for a first deployment.

---

## Prerequisites

Before starting, confirm you have:

- [ ] A Cloudflare account with Workers, D1, and KV enabled.
- [ ] `wrangler` CLI authenticated: `bunx wrangler whoami` shows your account.
- [ ] Rust stable + `wasm32-unknown-unknown` target + `worker-build` installed
  (see `docs/src/quick-start.md`).
- [ ] `bun` installed.
- [ ] The v0.38.6 source tarball extracted to a working directory.
- [ ] A domain or workers.dev subdomain decided for the production deployment.

---

## Phase 1 — Provision cloud resources

### 1.1 Create production D1 database

```sh
bunx wrangler d1 create zinnias-ciao
```

Note the `database_id` in the output. Edit `wrangler.toml` — find the production
`[[d1_databases]]` block and replace `"local"` with the real ID:

```toml
[env.production]
...
[[env.production.d1_databases]]
binding       = "DB"
database_name = "zinnias-ciao"
database_id   = "PASTE_REAL_ID_HERE"
```

- [ ] Done.

### 1.2 Create staging D1 database

```sh
bunx wrangler d1 create zinnias-ciao-staging
```

Replace `REPLACE_WITH_STAGING_D1_ID` in `wrangler.toml` with the real ID.

- [ ] Done.

### 1.3 Create production KV namespace for rate limiting

```sh
bunx wrangler kv:namespace create RATE_LIMIT --env production
```

Note the `id`. Add a `[[env.production.kv_namespaces]]` block in `wrangler.toml`:

```toml
[[env.production.kv_namespaces]]
binding = "RATE_LIMIT"
id      = "PASTE_REAL_KV_ID_HERE"
```

- [ ] Done.

### 1.4 Create staging KV namespace

```sh
bunx wrangler kv:namespace create RATE_LIMIT --env staging
```

Replace `REPLACE_WITH_STAGING_KV_ID` in `wrangler.toml`.

- [ ] Done.

---

## Phase 2 — Set secrets

Secrets are never committed to source. Set them once per environment.

### 2.1 Generate the HMAC pepper

The pepper is a cryptographically random 32-byte value. Generate it securely:

```sh
# macOS / Linux
openssl rand -hex 32
```

Copy the output — you will not see it again after setting it.

### 2.2 Set secrets for staging

```sh
bunx wrangler secret put HMAC_PEPPER --env staging
# paste the pepper value when prompted
```

`SESSION_COOKIE_DOMAIN` is **not a secret** — it is a plain `[vars]` binding (RFC-038).
Set it in `wrangler.toml` under `[env.staging]`:

```toml
[env.staging]
vars = { SESSION_COOKIE_DOMAIN = "zinnias-ciao-stg.workers.dev", ... }
```

Leave it unset (or set to an empty string) for a host-only cookie scoped to the
exact deployment host. Only set it if you need cross-subdomain cookie sharing.

- [ ] Done.

### 2.3 Set secrets for production

Use a **different** pepper value from staging.

```sh
openssl rand -hex 32   # generate a new pepper — different from staging

bunx wrangler secret put HMAC_PEPPER --env production
# paste the new pepper
```

Set `SESSION_COOKIE_DOMAIN` in `wrangler.toml` under `[env.production]` (same as
staging — it is a var, not a secret):

```toml
[env.production]
vars = { SESSION_COOKIE_DOMAIN = "your-domain.com", ... }
```

Or leave unset for a host-only cookie.

- [ ] Done.

---

## Phase 3 — Apply migrations

### 3.1 Apply to staging first

```sh
bun run migrate:staging
```

Confirm output shows all migrations applied (`0001` through `0007`).

- [ ] Done.

### 3.2 Apply to production

```sh
bun run migrate:prod
```

Confirm all migrations applied (`0001` through `0007`).

- [ ] Done.

---

## Phase 4 — Build and deploy

### 4.1 Install dependencies

```sh
bun install
```

- [ ] Done.

### 4.2 Deploy to staging

```sh
bunx wrangler deploy --env staging
```

- [ ] Done.

### 4.3 Smoke-test staging

```sh
STAGING_URL="https://zinnias-ciao-ssr-stg.<account>.workers.dev"

curl "$STAGING_URL/healthz"
# Expected: {"ok":true,"service":"ciao.zinnias"}

curl "$STAGING_URL/version"
# Expected: {"ok":true,"version":"staging"}
```

Open `$STAGING_URL/join` in a browser. Confirm the join form loads.

- [ ] Health check passes.
- [ ] Version check passes.
- [ ] Join form loads without error.

### 4.4 Seed the first community and admin on staging

Use `bun run setup` against the staging D1 via a temporary local proxy, or insert
directly using `wrangler d1 execute`:

```sh
# Generate a random community ID and membership ID (any unique strings)
COMMUNITY_ID="com_$(openssl rand -hex 8)"
USER_ID="usr_$(openssl rand -hex 8)"
MEMBERSHIP_ID="mem_$(openssl rand -hex 8)"
INVITE_ID="inv_$(openssl rand -hex 8)"
NOW=$(date -u +"%Y-%m-%dT%H:%M:%S.000Z")
EXPIRES="2099-12-31T23:59:59.000Z"

# Community
bunx wrangler d1 execute zinnias-ciao-staging --env staging --command \
  "INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('$COMMUNITY_ID', 'Staging Community', 'Asia/Tokyo', 1, '$NOW')"

# User
bunx wrangler d1 execute zinnias-ciao-staging --env staging --command \
  "INSERT INTO users (id, created_at) VALUES ('$USER_ID', '$NOW')"

# Membership (admin)
bunx wrangler d1 execute zinnias-ciao-staging --env staging --command \
  "INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('$MEMBERSHIP_ID', '$COMMUNITY_ID', '$USER_ID', 'admin', 'Admin', '$NOW')"
```

Then generate the bootstrap invite code. The code HMAC requires the staging pepper.
Use the setup script against a local wrangler instance pointed at staging D1, or
run a one-time seeding script. The simplest approach for staging is to use
`bun run setup` locally after pointing the D1 binding at the remote staging DB by
temporarily editing `wrangler.toml` — or seed it via `wrangler d1 execute` after
computing the HMAC externally.

**Practical shortcut for staging:** run `bun run setup -y --reset` against the
local dev DB, confirm the flow works, then replay the same SQL against staging.

- [ ] Staging community and admin seeded.

### 4.5 Run the full QA checklist against staging

Work through all `[~]` items in `docs/src/release-checklist.md`:

- Join with the staging invite code on a real phone.
- Mark Going on an event.
- Save a note.
- Go offline; confirm the offline banner appears and no false-success on form submit.
- Test at 200% text scaling.
- Check reduced-motion mode.
- Test grayscale legibility of status chips.
- Confirm the 2-minute join-to-attendance flow.

- [ ] All `[~]` QA items passed on staging.

### 4.6 Deploy to production

Only after staging QA passes:

```sh
bunx wrangler deploy --env production
```

- [ ] Done.

### 4.7 Smoke-test production

```sh
PROD_URL="https://your-production-domain.com"

curl "$PROD_URL/healthz"
curl "$PROD_URL/version"
```

- [ ] Health check passes.
- [ ] Version check passes.
- [ ] Join form loads in a browser.

---

## Phase 5 — Seed production community

Use `wrangler d1 execute` against the production D1 as in §4.4, but with the
production community name and a fresh bootstrap invite. The invite code HMAC must
use the **production** pepper.

**Recommended procedure:** compute the HMAC locally using the same algorithm as the
server (`HMAC-SHA256(pepper, uppercase(code))`), or temporarily use `bun run setup`
against a local wrangler instance with the production D1 ID bound. Clear the local
state afterward.

- [ ] Production community and admin seeded.
- [ ] First admin can sign in via the bootstrap invite code.
- [ ] Admin confirms they can create an event and generate a new invite code.

---

## Phase 6 — Configure log persistence (Logpush)

V8 isolates have no filesystem. `console.log` output is visible in `wrangler tail`
during development but not persisted without Logpush.

1. In the Cloudflare dashboard: Workers → your worker → Observability → Logpush.
2. Add a destination: R2 bucket or S3-compatible endpoint.
3. Select fields: `timestamp`, `outcome`, `scriptName`, `logs`, `exceptions`.
4. Enable.

```sh
# Confirm logs are flowing (requires wrangler tail for real-time; Logpush for persistence)
bunx wrangler tail --env production
# make a test request to the production URL
# confirm you see a log entry
```

- [ ] Logpush destination configured.
- [ ] Test log entry confirmed flowing.

---

## Phase 7 — Final security review

Before sharing the service with real users, confirm:

- [ ] Production secrets are not in source control, `.env` files, or Slack history.
- [ ] `wrangler.toml` contains no hardcoded D1 database IDs that shouldn't be public
  (IDs in `wrangler.toml` are semi-public via the Cloudflare dashboard; the actual
  protection is the secret pepper — confirm it is set and not shared).
- [ ] Rate limiting is active: attempt 11 failed invite codes from a single IP and
  confirm the 12th is rejected with the rate-limit message.
- [ ] Cross-community isolation: sign in as a member of community A; attempt to
  access a direct URL for an event in community B; confirm generic 404.
- [ ] Removed member: remove a test member; confirm their session returns 404 on the
  next community-scoped request.
- [ ] Session revocation: log out; confirm the session cookie is cleared; confirm the
  old session ID no longer grants access.

- [ ] Security review complete. No critical issues found.

---

## Rollback procedure

If a production issue is found after deployment:

```sh
# Revert the Worker to the previous version
bunx wrangler rollback --env production

# If the issue is in a migration:
# 1. Write a new forward migration that undoes the change.
# 2. Apply it: bun run migrate:prod
# DO NOT roll back migrations by deleting rows from d1_migrations.
```

The deployed Worker version and the migration state are independent. Rollback
reverts the code; it does not revert the database. Write forward migrations only.

---

## Post-launch monitoring

```sh
# Real-time log tail (development / incident investigation)
bunx wrangler tail --env production

# Check worker metrics in the Cloudflare dashboard:
# Workers → zinnias-ciao-ssr-prd → Metrics
# Review: request count, error rate, CPU time, D1 query latency
```

Alert threshold recommendations:
- Error rate > 1% over 5 minutes → investigate.
- CPU time p99 > 8 ms → review recent changes.
- D1 query latency p99 > 200 ms → check indexes and query patterns.
