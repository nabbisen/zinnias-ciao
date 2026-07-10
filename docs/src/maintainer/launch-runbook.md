# Launch Runbook

This document is the step-by-step operator guide for taking ciao.zinnias from a
clean tarball to a running production deployment. It is intended to be followed
exactly, in order, by one operator. Tick each step as you complete it.

**Version this runbook was written for:** v0.58.0
**Estimated time:** 60–90 minutes for a first deployment.

---

## Prerequisites

Before starting, confirm you have:

- [ ] A Cloudflare account with Workers, D1, and KV enabled.
- [ ] `wrangler` CLI authenticated: `bunx wrangler whoami` shows your account.
- [ ] Rust stable + `wasm32-unknown-unknown` target + `worker-build` installed
  (see `docs/src/developer/quick-start.md`).
- [ ] `bun` installed.
- [ ] The v0.58.0 source tarball extracted to a working directory.
- [ ] A domain or workers.dev subdomain decided for the production deployment.

---

## Phase 1 — Provision cloud resources

### 1.0 Create local Wrangler config

Keep tracked `wrangler.toml` as the canonical config shape with placeholder
Cloudflare resource IDs. Create ignored local copies for real hosted IDs:

```sh
cp wrangler.toml wrangler.staging.local.toml
cp wrangler.toml wrangler.production.local.toml
git check-ignore -v wrangler.staging.local.toml wrangler.production.local.toml
```

Edit only ignored local config files in the steps below:

- `wrangler.staging.local.toml` for staging;
- `wrangler.production.local.toml` for production.

For hosted staging, replace the placeholders in `[[env.staging.d1_databases]]`
and `[[env.staging.kv_namespaces]]`:

```toml
[[env.staging.d1_databases]]
binding       = "DB"
database_name = "zinnias-ciao-staging"
database_id   = "PASTE_STAGING_D1_DATABASE_ID_HERE"

[[env.staging.kv_namespaces]]
binding = "RATE_LIMIT"
id      = "PASTE_STAGING_RATE_LIMIT_KV_ID_HERE"
```

Do not introduce another hosted deployment configuration layer.

- [ ] Done.

### 1.1 Create production D1 database

```sh
bunx wrangler d1 create zinnias-ciao
```

Note the `database_id` in the output. Edit `wrangler.production.local.toml` —
find the production `[[env.production.d1_databases]]` block and replace the
placeholder with the real ID:

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

Replace `REPLACE_WITH_STAGING_D1_ID` in `wrangler.staging.local.toml` with the real ID.

- [ ] Done.

### 1.3 Create production KV namespace for rate limiting

```sh
bunx wrangler kv namespace create RATE_LIMIT --env production \
  --config wrangler.production.local.toml
```

Note the `id`. Replace the production `RATE_LIMIT` placeholder in
`wrangler.production.local.toml`:

```toml
[[env.production.kv_namespaces]]
binding = "RATE_LIMIT"
id      = "PASTE_REAL_KV_ID_HERE"
```

- [ ] Done.

### 1.4 Create staging KV namespace

```sh
bunx wrangler kv namespace create RATE_LIMIT --env staging \
  --config wrangler.staging.local.toml
```

Replace `REPLACE_WITH_STAGING_KV_ID` in `wrangler.staging.local.toml`.

- [ ] Done.

---

## Phase 2 — Set secrets

Secrets are never committed to source. Set them once per environment.

### 2.1 Generate the HMAC pepper

The pepper is a cryptographically random 32-byte value. Generate a different
pepper per environment. Do not reuse staging's pepper in production.

The commands below generate the pepper and send it directly to Wrangler without
printing it or storing it in shell history.

### 2.2 Set secrets for staging

For hosted staging with bootstrap login testing, `bun run bootstrap:staging`
will generate and set a fresh staging `HMAC_PEPPER` later in §4.3. Use the
standalone command below only when running unauthenticated staging checks without
bootstrap seeding.

```sh
openssl rand -hex 32 | bunx wrangler secret put HMAC_PEPPER --env staging \
  --config wrangler.staging.local.toml
```

This rotates staging HMAC material. On an existing staging database, old
sessions, invite codes, and form tokens issued with the previous pepper will no
longer validate. That is acceptable for fresh staging setup; plan rotation if
staging already has test users.

`SESSION_COOKIE_DOMAIN` is **not a secret** — it is a plain `[vars]` binding (RFC-038).
Set it in `wrangler.staging.local.toml` under `[env.staging]`:

```toml
[env.staging]
vars = { SESSION_COOKIE_DOMAIN = "zinnias-ciao-stg.workers.dev", ... }
```

Leave it unset (or set to an empty string) for a host-only cookie scoped to the
exact deployment host. Only set it if you need cross-subdomain cookie sharing.

- [ ] Done.

### 2.3 Set secrets for production

For initial production release with first-admin bootstrap, `bun run
bootstrap:production` will generate and set a fresh production `HMAC_PEPPER`
later in §4.7. Use the standalone command below only when production bootstrap
seeding is not being run, or during a planned key rotation:

```sh
openssl rand -hex 32 | bunx wrangler secret put HMAC_PEPPER --env production \
  --config wrangler.production.local.toml
```

Rotating production `HMAC_PEPPER` invalidates existing sessions, invite codes,
and form tokens.

Set `SESSION_COOKIE_DOMAIN` in `wrangler.production.local.toml` under
`[env.production]` (same as staging — it is a var, not a secret):

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

Confirm output shows all migrations applied (`0001` through `0009`).
Then verify the D1-backed form-token table exists:

```sh
bunx wrangler d1 migrations list zinnias-ciao-staging --remote --env staging \
  --config wrangler.staging.local.toml

bunx wrangler d1 execute zinnias-ciao-staging --remote --env staging --command \
  "SELECT name FROM sqlite_master WHERE type='table' AND name='form_tokens'" \
  --config wrangler.staging.local.toml
```

- [ ] Done.

### 3.2 Apply to production

```sh
bun run migrate:prod
```

Confirm all migrations applied (`0001` through `0009`).

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
bunx wrangler deploy --env staging --config wrangler.staging.local.toml
```

- [ ] Done.

### 4.3 Bootstrap the first staging community and admin invite

`wrangler deploy` only publishes the Worker. It does not seed D1 and does not
print an admin invite code. Bootstrap staging explicitly:

```sh
bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"
```

This command applies remote staging migrations, rotates staging `HMAC_PEPPER`,
inserts one staging community and seed admin, and prints the admin invite code
for `/join`. Keep the printed code private; it is a staging login credential.

- [ ] Staging community and admin invite seeded.

### 4.4 Smoke-test staging

```sh
STAGING_URL="https://zinnias-ciao-ssr-stg.<account>.workers.dev"

curl "$STAGING_URL/healthz"
# Expected: {"ok":true,"service":"ciao.zinnias"}

curl "$STAGING_URL/version"
# Expected: {"ok":true,"version":"staging"}
```

Open `$STAGING_URL/join` in a browser. Confirm the join form loads.

The RFC-050 prototype smoke can collect repeatable route/header and browser
evidence against the hosted staging Worker. Use the staging URL printed by
Wrangler after deploy:

```sh
bun run smoke:runtime -- "$STAGING_URL"
```

See `docs/src/tester/staging-runtime-prototype.md` for the output files and the manual
evidence that still remains outside the prototype.

- [ ] Health check passes.
- [ ] Version check passes.
- [ ] Join form loads without error.
- [ ] RFC-050 prototype smoke passes.

### 4.5 Run the full QA checklist against staging

Work through all `[~]` items in `docs/src/tester/release-checklist.md`:

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
bunx wrangler deploy --env production --config wrangler.production.local.toml
```

- [ ] Done.

### 4.7 Bootstrap the first production community and admin invite

`wrangler deploy --env production --config wrangler.production.local.toml` only
publishes the Worker. It does not seed D1 and does not print an admin invite
code. Bootstrap production explicitly:

```sh
bun run bootstrap:production -- --community "Production Community" --admin "Admin"
```

This command applies remote production migrations, rotates production
`HMAC_PEPPER`, inserts one production community and seed admin, and prints the
admin invite code for `/join`. Use it for initial production release setup only,
or for a planned production credential rotation. Keep the printed code private;
it is a production login credential.

- [ ] Production community and admin invite seeded.

### 4.8 Smoke-test production

```sh
PROD_URL="https://your-production-domain.com"

curl "$PROD_URL/healthz"
curl "$PROD_URL/version"
```

- [ ] Health check passes.
- [ ] Version check passes.
- [ ] Join form loads in a browser.

---

## Phase 5 — Verify production first admin

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

- [ ] Production secrets are not in source control, notes, shell history, or chat.
- [ ] Tracked `wrangler.toml` still contains placeholder D1/KV IDs; real IDs are
  only in ignored `wrangler.staging.local.toml` / `wrangler.production.local.toml`.
- [ ] Hosted deployment uses ignored local Wrangler config files plus
  Wrangler-managed remote secrets.
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
