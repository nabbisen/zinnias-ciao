# Staging Runtime Verification Prototype

This page describes one piece of RFC-045/RFC-050 runtime evidence: the
`smoke:runtime` browser/route prototype. It is useful for checking that an
already-running Worker is reachable and rendering basic public pages, but it
does **not** complete the production-pilot RFC-050 gate by itself, and it
predates most of the local tooling that now exists alongside it.

**Reconciled 2026-07-29 (RFC-050 Tooling Slice 9).** RFC-050 Tooling Slices
1–8 (all local-only; see `.git-exclude/tasks/018-rfc050-local-evidence-tooling-handoff.md`)
have since landed a much larger local evidence suite than this one prototype
script: version-metadata/manifest utilities, exact-identity smoke mode,
authenticated/browser flow collection (E3), concurrency/postcondition
tooling (E4), negative-configuration fixtures (E5), manual evidence
templates, artifact hashing, leakage scanning, and the tracked attestation.
See [RFC-050 Manual Evidence Templates](evidence-templates/index.md) and
[RFC-050 Release Candidate Attestations](release-candidates/index.md) for
the current tooling and the one artifact that is ever committed per
candidate. This page's own scope (below) is unchanged and still accurate for
what `smoke:runtime` itself does.

## Scope

Wrangler owns runtime and deployment. Use local `wrangler dev` or hosted
`wrangler deploy --env staging --config wrangler.staging.local.toml` first, then
pass the resulting URL to the runtime evidence collector. The collector does not
start, deploy, seed, or mutate D1.

It verifies:

- `/healthz` returns JSON with `ok: true` and `ready: true`;
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

Those remain RFC-050 operator evidence items. As of Tooling Slice 9, three of
them have a **local** (non-hosted, non-authoritative) collector — this is
progress on the tooling, not a substitute for the hosted evidence a real
candidate still needs:

- seeded authenticated workflows and the Asia/Tokyo round-trip:
  `scripts/collect-evidence-e3-flows.mjs` (RFC-050 E3, local);
- invite/form-token race behavior: `scripts/collect-evidence-e4-concurrency.mjs`
  (RFC-050 E4, local; E4a direct-ingress topology is hosted-only and gates it);
- negative configuration (missing pepper, missing/misnamed `ABUSE_LIMITER`,
  exhausted coordinator, malformed version metadata):
  `scripts/collect-evidence-e5-negative-config.mjs` (RFC-050 E5, local).

Real-phone 200% scaling, Logpush delivery, and Cloudflare dashboard CPU/runtime
metrics have no local equivalent — they require the deployed edge and remain
purely hosted-operator items (RFC-050 E6/E7/E8).

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
EXPECTED_VERSION="v0.59.0" \
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

### Refresh Staging Deployment

Redeploy staging after changing source, release version variables, static asset
cache-busters, `wrangler.staging.local.toml`, or any D1/KV binding IDs:

```sh
bunx wrangler deploy --env staging --config wrangler.staging.local.toml
```

The deployed Worker keeps using the bindings from the deployment that published
it. If a staging D1 or KV resource is recreated and
`wrangler.staging.local.toml` is updated, hosted staging will still use the old
resource ID until this deploy command is run again.

If staging D1 was recreated, use the fresh-target bootstrap procedure after
updating `wrangler.staging.local.toml` and while the target remains dark:

```sh
bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"
```

### Bootstrap Remote Resources

Before hosted smoke on a new target, bootstrap while it is dark. This step
applies remote migrations, proves the exact recognized application tables are
empty, creates one staging `HMAC_PEPPER`, inserts the matching community/admin
seed, prints the private admin invite, and ends `provisioned-not-ready`:

```sh
bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"
```

Keep the printed invite code private and unused until the exact candidate is
deployed and both `/version` identity and ready `/healthz` are verified. The
command targets only `[env.staging]`; production has a separate bootstrap
command. Do not use default bootstrap to refresh a non-fresh target: normal
candidate redeploy preserves its valid pepper. Destructive rotation requires
the explicit rotation flag and exact target-bound confirmation described in
the shared deployment guide.

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
bunx wrangler tail --env staging --config wrangler.staging.local.toml

# In another terminal:
curl "https://zinnias-ciao-ssr-stg.<account>.workers.dev/join"
```

Readable and machine-readable log formats are available:

```sh
bunx wrangler tail --env staging --config wrangler.staging.local.toml --format pretty
bunx wrangler tail --env staging --config wrangler.staging.local.toml --format json
```

If the log says a D1 database ID has been deleted, the deployed Worker is bound
to a stale D1 ID. Find the current staging D1 ID, update
`wrangler.staging.local.toml`, redeploy, then rerun migrations/bootstrap if the
database was recreated:

```sh
bunx wrangler d1 list
bunx wrangler deploy --env staging --config wrangler.staging.local.toml
bun run migrate:staging
bun run bootstrap:staging -- --community "Staging Community" --admin "Admin"
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

## Manual RFC-050 Evidence Template — superseded

**Reconciled 2026-07-29.** The flat checklist that used to live in this
section is superseded by the tracked, sanitized tooling built in RFC-050
Tooling Slices 7–8. Use those instead of this page for a real evidence
campaign:

- [RFC-050 Manual Evidence Templates](evidence-templates/index.md) — the
  fill-in-the-blank per-check templates (browser/device, observability/
  runtime, recovery/restore), covering the same ground as the old checklist
  above item-for-item plus named fields the flat list never had (e.g. E7's
  delivery-interval/retrieval-timestamp pair).
- [RFC-050 Release Candidate Attestations](release-candidates/index.md) —
  the per-candidate, per-gate (E0–E9) tracked record with mechanically
  enforced gate rules, which this checklist had no equivalent of at all.

This section is kept only so an old link to it still resolves to something;
do not fill in real values here — this page is not evidence.
