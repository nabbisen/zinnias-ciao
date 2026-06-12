# Deployment

## Environments

| Environment | Purpose |
|------------|---------|
| `dev` | Local developer testing via `wrangler dev --local` |
| `staging` | Release-candidate testing with non-production data |
| `production` | Real communities |

## Secrets (set via `wrangler secret put`)

| Secret | Purpose |
|--------|---------|
| `HMAC_PEPPER` | Server pepper for invite-code and session HMAC (AD-3) |
| `SESSION_COOKIE_DOMAIN` | Cookie `Domain` attribute for the session cookie |

Never commit secrets. Never use production secrets in dev/staging.

## Migrations

Apply locally:
```sh
bun run migrate:dev
```

Apply to production (run in staging first):
```sh
bun run migrate:prod
```

Destructive migration changes require a backup/export and operator approval before applying to production.

## Deploy

```sh
# Staging
bunx wrangler deploy --env staging

# Production
bunx wrangler deploy --env production
```

## Rollback

Cloudflare Workers supports rollback via the dashboard or:
```sh
bunx wrangler rollback --env production
```

Database migrations are not automatically rolled back; if a migration must be reversed, write a new forward migration and apply it.

## Log persistence

V8 isolates have no filesystem. Use Cloudflare Logpush to R2 or an external S3-compatible store for log persistence. Configure in the Cloudflare dashboard under Workers → Logpush.
