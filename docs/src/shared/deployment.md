# Deployment

This page is shared by developers, maintainers, and testers. It separates the
common deployment tasks by environment and puts the short operating summary
before the detailed commands.

## Task Map

### Local (`dev`)

| Task | Use when | Main command |
|------|----------|--------------|
| Local development | You need a local Worker for coding or smoke checks. | `bunx wrangler dev --env dev --local` |

### Staging

| Task | Use when | Main command |
|------|----------|--------------|
| Hosted staging deploy | You need a public Cloudflare-hosted staging Worker for evidence. | `bunx wrangler deploy --env staging --config wrangler.staging.local.toml` |
| Staging bootstrap | You need a staging admin invite code for `/join`. | `bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"` |
| Hosted staging smoke | You need RFC-050 runtime evidence from the deployed URL. | `bun run smoke:runtime -- https://<deployed-worker-url>` |
| Close staging | You are done with temporary public staging exposure. | `bunx wrangler delete --env staging --config wrangler.staging.local.toml` |

### Production

| Task | Use when | Main command |
|------|----------|--------------|
| Production deploy | You are publishing the production Worker. | `bunx wrangler deploy --env production --config wrangler.production.local.toml` |
| Production bootstrap | You need the first production admin invite code. | `bun run bootstrap:production -- --community "Production Community" --admin "Admin"` |

Important distinction: `wrangler deploy` only publishes a Worker. It does not
seed D1 and does not print an admin invite code. Bootstrap commands seed the
first community/admin data and print the invite code.

## Environments

| Environment | Purpose | Config file |
|-------------|---------|-------------|
| `dev` | Local developer testing via `wrangler dev --local`. | tracked `wrangler.toml` |
| `staging` | Temporary hosted evidence with non-production data. | ignored `wrangler.staging.local.toml` |
| `production` | Real communities. | ignored `wrangler.production.local.toml` |

Hosted staging is public while published. Use local `wrangler dev` unless a real
Cloudflare deployment is needed for evidence.

## Configuration

### Overview

Hosted deployment configuration has three places:

| Config kind | Where it lives | Commit it? |
|-------------|----------------|------------|
| Config shape and placeholder IDs | `wrangler.toml` | Yes |
| Real non-secret D1/KV binding IDs | ignored `wrangler.*.local.toml` files | No |
| Secrets such as `HMAC_PEPPER` | Cloudflare secrets via Wrangler | No |

Use explicit `--config` and `--env` for every hosted command.

### Steps

Create ignored local config files:

```sh
cp wrangler.toml wrangler.staging.local.toml
cp wrangler.toml wrangler.production.local.toml
git check-ignore -v wrangler.staging.local.toml wrangler.production.local.toml
```

Edit only the relevant hosted environment blocks in each local config. For
hosted staging:

```toml
[env.staging]
name = "zinnias-ciao-ssr-stg"

[env.staging.vars]
LOG_LEVEL                  = "info"
BUILD_VERSION              = "staging"
COMMUNITY_CREATION_ENABLED = "true"

[[env.staging.d1_databases]]
binding       = "DB"
database_name = "zinnias-ciao-staging"
database_id   = "PASTE_STAGING_D1_DATABASE_ID_HERE"

[[env.staging.kv_namespaces]]
binding = "RATE_LIMIT"
id      = "PASTE_STAGING_RATE_LIMIT_KV_ID_HERE"
```

For hosted production:

```toml
[env.production]
name = "zinnias-ciao-ssr-prd"

[env.production.vars]
LOG_LEVEL                  = "warn"
BUILD_VERSION              = "production"
COMMUNITY_CREATION_ENABLED = "false"

[[env.production.d1_databases]]
binding       = "DB"
database_name = "zinnias-ciao"
database_id   = "PASTE_PRODUCTION_D1_DATABASE_ID_HERE"

[[env.production.kv_namespaces]]
binding = "RATE_LIMIT"
id      = "PASTE_PRODUCTION_RATE_LIMIT_KV_ID_HERE"
```

## Hosted Staging Deployment

### Overview

| Step | Command | Expected result |
|------|---------|-----------------|
| 1. Deploy staging Worker | `bunx wrangler deploy --env staging --config wrangler.staging.local.toml` | Public staging URL is published. |
| 2. Bootstrap staging data | `bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"` | Remote staging D1 is migrated and an admin invite code is printed. |
| 3. Run runtime smoke | `bun run smoke:runtime -- https://<deployed-worker-url>` | Public runtime evidence is written under `.git-exclude/evidence/`. |
| 4. Close staging when done | `bunx wrangler delete --env staging --config wrangler.staging.local.toml` | Public staging Worker stops serving. |

The admin invite code appears in step 2, not step 1.

### Steps

Deploy `[env.staging]`:

```sh
bunx wrangler deploy --env staging --config wrangler.staging.local.toml
```

Bootstrap staging login data:

```sh
bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"
```

This command applies remote staging migrations, rotates staging `HMAC_PEPPER`,
inserts one staging community and seed admin, and prints the admin invite code
for `/join`. Keep the printed code private; it is a staging login credential.

Run runtime smoke against the URL reported by Wrangler:

```sh
bun run smoke:runtime -- https://<deployed-worker-url>
```

Use the invite code printed by `bun run bootstrap:staging` for manual
authenticated staging checks.

## Staging Exposure

Treat every hosted staging route as public. Unknown `workers.dev` URLs and
custom staging domains are not private by default.

Use hosted staging only for evidence that requires Cloudflare hosting. Keep the
public window short. Staging must use separate D1/KV resources, separate
secrets, and non-production data. Protect custom staging domains with
Cloudflare Access or an equivalent control where available.

## Staging Data

Use only non-production data in staging. Keep staging D1, KV, and secrets
separate from production.

## Staging Teardown

### Overview

| Goal | What it removes | What it keeps |
|------|-----------------|---------------|
| Close public staging only | The published staging Worker URL. | Staging D1, KV, and data for reuse. |
| Fully discard staging resources | Worker, secret, D1, and KV. | Nothing, unless you export first. |

Most testing sessions should close only the public staging Worker. Fully discard
staging resources only when the staging data is no longer needed.

### Close Public Staging Only

Run the dry run first and confirm it targets the staging Worker:

```sh
bunx wrangler delete --env staging --config wrangler.staging.local.toml --dry-run
bunx wrangler delete --env staging --config wrangler.staging.local.toml
```

Deleting the Worker stops the public staging URL from serving. It does not
delete staging D1 or KV resources.

### Fully Discard Staging Resources

Only run these when staging data is no longer needed. Confirm every identifier
is for staging, not production. If the staging database may contain useful
evidence, export it first:

```sh
bunx wrangler d1 export zinnias-ciao-staging --remote --output staging-d1-backup.sql
```

Delete the Worker secret before deleting the Worker, while the Worker still
exists:

```sh
bunx wrangler secret delete HMAC_PEPPER --env staging --config wrangler.staging.local.toml
```

Delete the public staging Worker:

```sh
bunx wrangler delete --env staging --config wrangler.staging.local.toml --dry-run
bunx wrangler delete --env staging --config wrangler.staging.local.toml
```

Delete the remote staging D1 database:

```sh
bunx wrangler d1 delete zinnias-ciao-staging
```

Delete the staging KV namespace. Copy the staging `RATE_LIMIT` namespace ID from
`wrangler.staging.local.toml` before running this:

```sh
bunx wrangler kv namespace delete --namespace-id <staging-rate-limit-kv-id>
```

After deleting D1 or KV, remove or replace the deleted staging IDs in
`wrangler.staging.local.toml` before the next staging deployment.

## Production Deployment

### Overview

| Step | Command | Expected result |
|------|---------|-----------------|
| 1. Apply production migrations | `bun run migrate:prod` | Remote production D1 schema is current. |
| 2. Deploy production Worker | `bunx wrangler deploy --env production --config wrangler.production.local.toml` | Production Worker is published. |
| 3. Bootstrap first admin, initial release only | `bun run bootstrap:production -- --community "Production Community" --admin "Admin"` | First production community/admin is seeded and an admin invite code is printed. |

`wrangler deploy --env production --config wrangler.production.local.toml`
publishes the Worker, but does not seed D1 or print an admin invite code.

### Steps

Apply production migrations after staging has been verified:

```sh
bun run migrate:prod
```

Deploy production:

```sh
bunx wrangler deploy --env production --config wrangler.production.local.toml
```

For initial production release setup, bootstrap the first community and admin
invite explicitly:

```sh
bun run bootstrap:production -- --community "Production Community" --admin "Admin"
```

This command applies remote production migrations, rotates production
`HMAC_PEPPER`, inserts one production community and seed admin, and prints the
admin invite code for `/join`. Keep the printed code private; it is a production
login credential. Do not run it on an active production database unless a
planned credential rotation is approved.

## Local Development Smoke

### Overview

Local development uses one terminal for Wrangler and one terminal for smoke
evidence. The smoke collector does not start the Worker.

### Steps

```sh
# Terminal 1: keep local dev running.
bunx wrangler dev --env dev --local

# Terminal 2: collect evidence from the running local Worker.
bun run smoke:runtime -- http://127.0.0.1:8787
```

## Secrets

| Secret | Purpose |
|--------|---------|
| `HMAC_PEPPER` | Server pepper for invite-code and session HMAC (AD-3). |

Never commit secrets. Never use production secrets in dev/staging.

Generate and set `HMAC_PEPPER` without printing it when not using bootstrap
seeding:

```sh
openssl rand -hex 32 | bunx wrangler secret put HMAC_PEPPER --env staging \
  --config wrangler.staging.local.toml
openssl rand -hex 32 | bunx wrangler secret put HMAC_PEPPER --env production \
  --config wrangler.production.local.toml
```

For hosted bootstrap, `bun run bootstrap:staging` and `bun run
bootstrap:production` generate and set a fresh environment-specific
`HMAC_PEPPER` as part of seeding. Use the standalone secret command only when
bootstrap seeding is not being run.

Use a different pepper for each environment. Rotating the pepper invalidates
existing sessions, invite codes, and form tokens for that environment.

## Vars

| Var | Purpose |
|-----|---------|
| `SESSION_COOKIE_DOMAIN` | Optional cookie `Domain` attribute for the session cookie. Leave unset/empty for host-only cookies. |
| `BUILD_VERSION` | Value returned by `/version`. |
| `LOG_LEVEL` | Runtime log verbosity. |

## Migrations

### Overview

| Environment | Command |
|-------------|---------|
| Local dev | `bun run migrate:dev` |
| Hosted staging | `bun run migrate:staging` |
| Hosted production | `bun run migrate:prod` |

### Steps

Apply locally:

```sh
bun run migrate:dev
```

Apply to hosted staging:

```sh
bun run migrate:staging
```

Apply to production after staging has been verified:

```sh
bun run migrate:prod
```

`migrate:staging` uses `wrangler.staging.local.toml`; `migrate:prod` uses
`wrangler.production.local.toml`. Both use Wrangler's `--remote` D1 flag because
they target hosted Cloudflare databases. Local development remains explicitly
`--local`.

For hosted staging, verify the remote migration ledger and schema before
debugging the browser path:

```sh
bunx wrangler d1 migrations list zinnias-ciao-staging --remote --env staging \
  --config wrangler.staging.local.toml

bunx wrangler d1 execute zinnias-ciao-staging --remote --env staging --command \
  "SELECT name FROM sqlite_master WHERE type='table' AND name='form_tokens'" \
  --config wrangler.staging.local.toml
```

Destructive migration changes require a backup/export and operator approval
before applying to production.

## Runtime Smoke

The runtime smoke collector checks an already-running Worker URL. It does not
start, deploy, seed, or mutate D1.

For local smoke, see [Local Development Smoke](#local-development-smoke). For
hosted staging smoke, see [Hosted Staging Deployment](#hosted-staging-deployment).

The smoke collector writes evidence under `.git-exclude/evidence/rfc050-prototype/`
and does not replace the full RFC-050 manual evidence pack.

## Rollback

Cloudflare Workers supports rollback via the dashboard or:

```sh
bunx wrangler rollback --env production
```

Database migrations are not automatically rolled back. If a migration must be
reversed, write a new forward migration and apply it.

## Log persistence

V8 isolates have no filesystem. Use Cloudflare Logpush to R2 or an external
S3-compatible store for log persistence. Configure in the Cloudflare dashboard
under Workers -> Logpush.
