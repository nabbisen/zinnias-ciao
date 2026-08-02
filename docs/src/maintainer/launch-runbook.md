# Launch Runbook

This document is the step-by-step operator guide for taking ciao.zinnias from a
clean tarball to a running production deployment. It is intended to be followed
exactly, in order, by one operator. Tick each step as you complete it.

**Version this runbook was written for:** v0.61.0
**Estimated time:** 60–90 minutes for a first deployment.

---

## Prerequisites

Before starting, confirm you have:

- [ ] A Cloudflare account with Workers, D1, and KV enabled.
- [ ] `wrangler` CLI authenticated: `bunx wrangler whoami` shows your account.
- [ ] Rust stable + `wasm32-unknown-unknown` target + `worker-build` installed
  (see `docs/src/developer/quick-start.md`).
- [ ] `bun` installed.
- [ ] The v0.61.0 source tarball extracted to a working directory.
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

For hosted staging, replace the placeholder in `[[env.staging.d1_databases]]`:

```toml
[[env.staging.d1_databases]]
binding       = "DB"
database_name = "zinnias-ciao-staging"
database_id   = "PASTE_STAGING_D1_DATABASE_ID_HERE"
```

The `[[env.staging.durable_objects.bindings]]` block for `ABUSE_LIMITER`
(RFC-078) needs no ID to paste — it references the inherited top-level
`[exports.AbuseLimiter]` class export and is provisioned automatically on
first deploy. Do not introduce another hosted deployment configuration layer.

**If `wrangler.staging.local.toml` or `wrangler.production.local.toml` already
exists from before RFC-078**, it still has the retired shape (a
`[[env.*.kv_namespaces]]` block bound to `RATE_LIMIT`, no
`durable_objects.bindings` for `ABUSE_LIMITER`) and will not work with this
version's compiled Worker. Before the next deploy to that environment:

- add `[[env.staging.durable_objects.bindings]]` / `[[env.production.durable_objects.bindings]]`
  with `name = "ABUSE_LIMITER"` and `class_name = "AbuseLimiter"` (copy from
  the tracked `wrangler.toml`);
- remove the `[[env.*.kv_namespaces]]` block bound to `RATE_LIMIT`;
- do **not** delete the hosted `RATE_LIMIT` KV namespace itself yet — see
  [Deployment: Staging Teardown](../shared/deployment.md#staging-teardown)
  for the separately authorized retirement procedure.

Deploying an exact candidate that exports `AbuseLimiter` against a local
config that omits this binding will make every protected `POST` route
(`/join`, `/relink`, `/communities/new`) fail closed with a generic `503`,
since the Worker cannot resolve the binding at runtime.

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

### 1.3 Confirm the Durable Object class/binding for each environment

`workers/ssr` exports the `AbuseLimiter` Durable Object class (RFC-078)
declaratively — there is no `wrangler kv namespace create` equivalent step.
Confirm `wrangler.production.local.toml` and `wrangler.staging.local.toml`
each carry the binding (inherited unchanged from the tracked template):

```toml
[[env.production.durable_objects.bindings]]
name       = "ABUSE_LIMITER"
class_name = "AbuseLimiter"
```

The namespace is provisioned automatically the first time the exact
candidate is deployed to that environment. A retired hosted `RATE_LIMIT` KV
namespace from a pre-RFC-078 deployment may still exist; do not delete it as
part of this bootstrap — see
[Deployment: Staging Teardown](../shared/deployment.md#staging-teardown) for
the separately authorized retirement procedure.

- [ ] Done.

---

## Phase 2 — Set secrets

Secrets are never committed to source. Set them once per environment.

Do not set `COMMUNITY_RECOVERY_TOKEN` during normal launch. That secret is only
created for a temporary RFC-069 total-community-access recovery window; follow
`docs/src/maintainer/operations.md` when that incident procedure is approved.

### 2.1 Generate the HMAC pepper

The pepper is a cryptographically random 32-byte value. Generate a different
pepper per environment. Do not reuse staging's pepper in production.

The commands below generate the pepper and send it directly to Wrangler without
printing it or storing it in shell history.

### 2.2 Set secrets for staging

For hosted staging with bootstrap login testing, `bun run bootstrap:staging`
will generate and set a fresh staging `HMAC_PEPPER` later in §4.2. Use the
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
later in §4.6. Use the standalone command below only when production bootstrap
seeding is not being run, or during a planned key rotation:

```sh
openssl rand -hex 32 | bunx wrangler secret put HMAC_PEPPER --env production \
  --config wrangler.production.local.toml
```

Rotating production `HMAC_PEPPER` invalidates existing sessions, invite codes,
relink/help-signin codes, form tokens, calendar tokens, and outstanding
recovery codes. Restoring the same missing value is recovery; generating a
replacement on a non-fresh environment is destructive rotation.

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

Confirm output shows all migrations applied (`0001` through `0010`) only for an
RFC-079 exact candidate authorized for hosted staging. Migration 0010 resets
legacy audit metadata; complete the sensitive-backup and bounded-verification
steps in the shared deployment guide without displaying metadata.
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

Confirm all migrations applied (`0001` through `0010`). Production use of 0010
requires the separately approved RFC-079 exact candidate and the staged
destructive-migration evidence.

- [ ] Done.

---

## Phase 4 — Build and deploy

### 4.1 Install dependencies

```sh
bun install
```

- [ ] Done.

### 4.2 Bootstrap fresh staging while it is dark

`wrangler deploy` only publishes the Worker. It does not seed D1 and does not
print an admin invite code. Bootstrap staging explicitly:

```sh
bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"
```

The command proves the recognized application tables are empty before creating
random material. It then sets one staging `HMAC_PEPPER`, inserts the matching
community/admin seed, prints the private admin invite, and ends
`provisioned-not-ready`. `--yes` can skip only the fresh-target prompt; it
cannot authorize rotation. Wrangler secret provisioning can publish a Worker
version, so keep the target dark: no custom route, public test traffic, or user
data.

- [ ] Staging community and admin invite seeded.

### 4.3 Deploy the exact candidate and verify readiness

Immediately deploy the reviewed exact candidate:

```sh
bunx wrangler deploy --env staging --config wrangler.staging.local.toml
```

Confirm candidate identity with `/version`, then require ready `/healthz`
before using the invite, `/join`, or any public test traffic.

- [ ] Exact candidate deployed without replacing the provisioned pepper.

### 4.4 Smoke-test staging

```sh
STAGING_URL="https://zinnias-ciao-ssr-stg.<account>.workers.dev"

curl "$STAGING_URL/healthz"
# Expected: {"ok":true,"ready":true,"service":"ciao.zinnias"}

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
evidence that still remains outside the prototype — that page now points to
the full local evidence tooling (`docs/src/tester/evidence-templates/`) and
the tracked per-candidate attestation (`docs/src/tester/release-candidates/`)
where this candidate's gate verdicts get recorded.

- [ ] Health check passes.
- [ ] Version check passes.
- [ ] Join form loads without error.
- [ ] RFC-050 prototype smoke passes.
- [ ] This candidate's release attestation
      (`docs/src/tester/release-candidates/<candidate-label>.md`) exists and
      passes `bun run evidence:check-attestation <path>`.

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

### 4.6 Bootstrap fresh production while it is dark

Only after staging QA passes, keep the new production target dark and bootstrap
production explicitly:

```sh
bun run bootstrap:production -- --community "Production Community" --admin "Admin"
```

This is for a proven-fresh first release. It creates one pepper and matching
seed and ends `provisioned-not-ready`; it is not a routine deploy command. Keep
the printed invite private and unused.

- [ ] Production community and admin invite seeded.

### 4.7 Deploy the exact candidate and smoke-test production

Immediately deploy the reviewed candidate without replacing the provisioned
pepper:

```sh
bunx wrangler deploy --env production --config wrangler.production.local.toml
```

```sh
PROD_URL="https://your-production-domain.com"

curl "$PROD_URL/healthz"
curl "$PROD_URL/version"
```

- [ ] Health check passes.
- [ ] Version check passes.
- [ ] Join form loads in a browser.

On any non-fresh target, default bootstrap stops. A separately approved
interactive rotation requires `--rotate-hmac-pepper` and typing exactly
`ROTATE staging` or `ROTATE production`. Non-interactive rotation also requires
`--yes --confirm-rotation "ROTATE <target>"`. Rotation invalidates sessions,
invites, relink/help-signin codes, form tokens, calendar tokens, and outstanding
recovery codes.

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
After migration 0010, never roll back to code that accepts arbitrary audit
metadata, restores the removed RFC-079 compatibility adapter, omits required
request IDs, or treats required audits as best effort.

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
