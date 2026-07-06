# Deployment

## Environments

| Environment | Purpose |
|------------|---------|
| `dev` | Local developer testing via `wrangler dev --local` |
| `staging` | Release-candidate testing with non-production data |
| `production` | Real communities |

The tracked `wrangler.toml` is the canonical Wrangler config shape and must keep
placeholder Cloudflare resource IDs. Do not commit real D1 database IDs or KV
namespace IDs. For hosted staging or production operations, copy it to ignored
local config files and put real IDs there:

```sh
cp wrangler.toml wrangler.staging.local.toml
cp wrangler.toml wrangler.production.local.toml
# Edit wrangler.staging.local.toml: replace staging D1 database_id and KV id.
# Edit wrangler.production.local.toml: replace production D1 database_id and KV id.
git check-ignore -v wrangler.staging.local.toml wrangler.production.local.toml
```

For hosted staging, the edited local blocks should look like this:

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

For hosted production, use the same production blocks in
`wrangler.production.local.toml`:

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

Use explicit `--config` and `--env` for every hosted command. Hosted deployment
configuration has three places:

- normal vars and config shape in tracked `wrangler.toml`;
- real non-secret Cloudflare binding IDs in ignored local TOML files;
- secrets in Cloudflare via `wrangler secret`.

Hosted staging is public while published. Use local `wrangler dev` unless a
real Cloudflare deployment is needed for evidence. Staging must use separate
D1/KV resources, separate secrets, and non-production data. Keep hosted staging
exposure short; protect custom staging domains with Cloudflare Access or an
equivalent control where available, and remove/disable the public staging route
or delete the staging Worker after evidence collection when it is not needed.

### Staging Exposure

Treat every hosted staging route as public. Unknown `workers.dev` URLs and custom
staging domains are not private by default. Use hosted staging only for evidence
that requires Cloudflare hosting, and keep the public window short.

### Staging Data

Use only non-production data in staging. Keep staging D1, KV, and secrets
separate from production.

### Staging Teardown

After evidence collection, either close only the public staging Worker or fully
discard staging resources.

#### Close public staging only

To stop the published `workers.dev` staging Worker while keeping staging D1/KV
resources for later reuse:

```sh
bunx wrangler delete --env staging --config wrangler.staging.local.toml --dry-run
bunx wrangler delete --env staging --config wrangler.staging.local.toml
```

Run the dry run first and confirm it targets the staging Worker. Deleting the
Worker stops the public staging URL from serving. It does not delete staging D1
or KV resources; delete those separately only when intentionally discarding
staging data.

#### Fully discard staging resources

Only run these when staging data is no longer needed. Confirm every identifier is
for staging, not production. If the staging database may contain useful evidence,
export it first:

```sh
bunx wrangler d1 export zinnias-ciao-staging --remote --output staging-d1-backup.sql
```

Delete the Worker secret before deleting the Worker, while the Worker still
exists:

```sh
bunx wrangler secret delete HMAC_PEPPER --env staging --config wrangler.staging.local.toml
```

Then delete the public staging Worker:

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

## Secrets (set via `wrangler secret put`)

| Secret | Purpose |
|--------|---------|
| `HMAC_PEPPER` | Server pepper for invite-code and session HMAC (AD-3) |

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

Apply locally:
```sh
bun run migrate:dev
```

Apply to production (run in staging first):
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

Destructive migration changes require a backup/export and operator approval before applying to production.

## Deploy

```sh
# Staging
bunx wrangler deploy --env staging --config wrangler.staging.local.toml

# Production
bunx wrangler deploy --env production --config wrangler.production.local.toml
```

## Initial production bootstrap

`wrangler deploy --env production --config wrangler.production.local.toml`
publishes the Worker, but does not seed D1 or print an admin invite code. For
initial production release setup, bootstrap the first community and admin invite
explicitly:

```sh
bun run bootstrap:production -- --community "Production Community" --admin "Admin"
```

This command applies remote production migrations, rotates production
`HMAC_PEPPER`, inserts one production community and seed admin, and prints the
admin invite code for `/join`. Keep the printed code private; it is a production
login credential. Do not run it on an active production database unless a
planned credential rotation is approved.

## Runtime smoke

Wrangler owns start/deploy. After starting local dev or deploying staging, pass
the running URL to the RFC-050 runtime evidence collector:

### Local Development

```sh
# Terminal 1: keep local dev running.
bunx wrangler dev --env dev --local

# Terminal 2: collect evidence from the running local Worker.
bun run smoke:runtime -- http://127.0.0.1:8787
```

### Hosted Staging

Deploy completes first, then collect evidence from the URL reported by Wrangler:

```sh
bunx wrangler deploy --env staging --config wrangler.staging.local.toml
bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"
bun run smoke:runtime -- https://<deployed-worker-url>
```

`wrangler deploy` does not seed D1 or print an admin invite code. The staging
bootstrap command creates staging-only seed data and prints the invite code for
authenticated checks. The runtime collector itself does not start, deploy, seed,
or mutate D1. It writes evidence under `.git-exclude/evidence/rfc050-prototype/`
and does not replace the full RFC-050 manual evidence pack.

## Rollback

Cloudflare Workers supports rollback via the dashboard or:
```sh
bunx wrangler rollback --env production
```

Database migrations are not automatically rolled back; if a migration must be reversed, write a new forward migration and apply it.

## Log persistence

V8 isolates have no filesystem. Use Cloudflare Logpush to R2 or an external S3-compatible store for log persistence. Configure in the Cloudflare dashboard under Workers → Logpush.
