# Staging Runtime Verification Prototype

This page describes the testing-only prototype for RFC-045/RFC-050 runtime
evidence. It is useful for checking that an already-running Worker is reachable
and rendering basic public pages, but it does **not** complete the
production-pilot RFC-050 gate by itself.

## Scope

Wrangler owns runtime and deployment. Use local `wrangler dev` or hosted
`wrangler deploy --env staging --config wrangler.staging.local.toml` first, then
pass the resulting URL to the runtime evidence collector. The collector does not
start, deploy, seed, or mutate D1.

It verifies:

- `/healthz` returns JSON with `ok: true`;
- `/version` returns the expected staging build label;
- public HTML routes such as `/join` and `/offline` return security headers and
  no-store caching;
- PWA/static routes expose their expected cache behavior;
- public pages render in sandboxed/incognito Chromium at mobile width;
- `/join` renders at 200% text size and with JavaScript disabled;
- evidence JSON and screenshots are written under `.git-exclude/evidence/`.

It does not verify:

- seeded authenticated admin/member workflows;
- D1 mutations;
- invite or form-token race behavior;
- Asia/Tokyo event create/edit/ICS round-trip;
- real-phone 200% text scaling;
- Logpush delivery;
- Cloudflare dashboard CPU/runtime metrics.

Those remain RFC-050 operator evidence items.

## Staging Exposure Policy

### Public Exposure

Hosted Cloudflare staging is internet-reachable while it is published. A
`workers.dev` URL or custom staging domain must not be treated as private merely
because it is not widely shared.

### Publish Window

Use local `wrangler dev` for routine development and evidence that does not need
Cloudflare hosting. Publish hosted staging only for evidence that requires a real
Cloudflare deployment, such as edge reachability, hosted D1/KV bindings,
headers through Cloudflare, external phone/browser testing, or pre-release
RFC-050 artifacts.

### Required Safeguards

When hosted staging is published:

- use only non-production data;
- use separate staging D1, KV, and secrets;
- keep production secrets and production community data out of staging;
- prefer Cloudflare Access or an equivalent access control for custom staging
  domains when available;
- keep the public test window short.

Unknown or hard-to-guess URLs are not an access-control policy.

## Runtime Requirements

Prerequisites:

- Node.js 22 or newer. The script uses the global `WebSocket` implementation.
- Chromium installed at `/usr/bin/chromium`, or set `CHROMIUM` to the local
  binary path.

Optional environment variables:

- `CHROMIUM`: Chromium binary path. Default: `/usr/bin/chromium`.
- `CHROME_REMOTE_PORT`: remote debugging port. Default: `9250`.
- `EVIDENCE_DIR`: evidence output directory. Default:
  `.git-exclude/evidence/rfc050-prototype`.

Chromium is launched with `--incognito` and without `--no-sandbox`.

## Version Check

By default the script expects `/version` to return `"dev"` for localhost URLs
and `"staging"` for hosted URLs, matching the current Wrangler environment
defaults. Override it when testing a custom staging label or a
candidate-specific build:

```sh
EXPECTED_VERSION="custom-label" \
  bun run smoke:runtime -- https://<deployed-worker-url>
```

The default `"staging"` value confirms that a staging Worker is responding, but
it does not prove that a specific release candidate is deployed. For release
candidate evidence, set staging `BUILD_VERSION` to the candidate tag during
deploy and run with a matching value, for example:

```sh
EXPECTED_VERSION="v0.54.0" \
  bun run smoke:runtime -- https://<deployed-worker-url>
```

## Local Development Smoke

For local development, keep `wrangler dev` running in one terminal:

```sh
bunx wrangler dev --env dev --local
```

Then run the evidence collector from another terminal while the dev server is
still running:

```sh
bun run smoke:runtime -- http://127.0.0.1:8787
```

## Hosted Staging Smoke

Hosted staging has three operator-owned phases: deploy the staging Worker,
bootstrap remote staging data, then run the smoke collector against the deployed
URL.

### Deploy Staging

Deploy `[env.staging]` as an explicit operator action:

```sh
bunx wrangler deploy --env staging --config wrangler.staging.local.toml
```

### Bootstrap Remote Resources

Before hosted smoke, create or refresh staging bootstrap data. This step applies
remote staging migrations, rotates staging `HMAC_PEPPER`, inserts one staging
community and seed admin, and prints the admin invite code for `/join`:

```sh
bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"
```

Keep the printed invite code private. It is a staging login credential. The
command targets only `[env.staging]`; production has a separate bootstrap
command.

After bootstrap, confirm staging uses remote Cloudflare resources. A local or
preview D1 check is not evidence for the deployed Worker:

```sh
bunx wrangler d1 migrations list zinnias-ciao-staging --remote --env staging \
  --config wrangler.staging.local.toml

bunx wrangler d1 execute zinnias-ciao-staging --remote --env staging --command \
  "SELECT name FROM sqlite_master WHERE type='table' AND name='form_tokens'" \
  --config wrangler.staging.local.toml
```

Confirm that `[env.staging]` in `wrangler.staging.local.toml` has the same staging D1
`database_id` that the hosted Worker should use.

### Run Smoke

Run the evidence collector against the URL reported by Wrangler:

```sh
bun run smoke:runtime -- https://<deployed-worker-url>
```

Use the invite code printed by `bun run bootstrap:staging` to log in through
`/join` for manual authenticated staging checks.

### Diagnose Join Failures

If `/healthz`, `/version`, `/offline`, `/manifest.webmanifest`, and `/sw.js`
pass but `/join` returns 500, the deployed Worker is reachable but the first
D1-backed public form failed. `/join` issues an anonymous form token and writes
to `form_tokens`. If the hosted staging preparation checks above pass but
`/join` still returns 500, capture the runtime exception from the deployed
Worker:

```sh
bunx wrangler tail --env staging

# In another terminal:
curl "https://zinnias-ciao-ssr-stg.<account>.workers.dev/join"
```

### Close Hosted Staging

After hosted evidence collection and manual checks finish, close the temporary
public staging deployment. For the usual case, delete only the staging Worker and
keep staging D1/KV for later reuse:

```sh
bunx wrangler delete --env staging --config wrangler.staging.local.toml --dry-run
bunx wrangler delete --env staging --config wrangler.staging.local.toml
```

For full staging disposal, including D1, KV, and secret deletion, follow
[Staging Teardown](../shared/deployment.md#staging-teardown).

## Evidence

The script writes:

- `.git-exclude/evidence/rfc050-prototype/rfc050-runtime-smoke-results.json`;
- `.git-exclude/evidence/rfc050-prototype/join-mobile-200-percent.png`;
- `.git-exclude/evidence/rfc050-prototype/join-no-js-mobile.png`;
- `.git-exclude/evidence/rfc050-prototype/offline-mobile-200-percent.png`.

Attach the JSON and screenshots to the release checklist or review request when
using the prototype as staging evidence. The report includes an explicit list of
manual RFC-050 evidence items that remain open.

## Manual RFC-050 Evidence Template

Use this template after the prototype smoke passes.

- [ ] Staging D1 migrations applied.
- [ ] `HMAC_PEPPER` secret set for staging.
- [ ] `RATE_LIMIT` KV namespace bound for staging.
- [ ] First staging community and admin seeded with non-production data.
- [ ] Join flow succeeds with a staging invite code.
- [ ] Asia/Tokyo 09:00 event creation displays as 09:00 after round-trip.
- [ ] Event edit from 09:00 to 13:00 updates the existing day row.
- [ ] ICS download has correct DTSTART/DTEND for JST.
- [ ] Concurrent invite redemption creates exactly one membership/session.
- [ ] Concurrent form-token double-submit creates exactly one mutation.
- [ ] No-JS destructive confirmations work in a browser.
- [ ] Real phone 200% text scaling screenshots captured.
- [ ] Admin action is visible in audit log.
- [ ] Logpush delivers Worker logs to the configured sink.
- [ ] Cloudflare dashboard shows no consistent Free-plan CPU limit errors.
