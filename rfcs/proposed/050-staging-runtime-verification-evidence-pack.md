# RFC 050 — Staging Runtime Verification Evidence Pack

**Status.** Proposed (runtime evidence collector prototype implemented in v0.47.0; full evidence pack still pending)
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Completes RFC-045 §6 (staging-runtime verification). Supersedes the "staging pending" items in the release checklist. Blocks production-pilot approval.

## 1. Summary

RFC-045 discharged the source-verification half of the architect's checklist. This RFC is the staging-runtime half: deploy to Cloudflare staging and collect evidence artifacts that the handoff claims are true in a live environment.

## 2. Deliverables

- **Route/header matrix:** curl or browser-DevTools screenshots showing `Cache-Control: no-store`, CSP, and other security headers on Home, Event Detail, Join, and export.
- **Asia/Tokyo timezone round-trip:** create event at 09:00 JST, view detail, edit to 13:00, confirm display, download ICS, check DTSTART/DTEND.
- **Concurrent invite redemption:** two simultaneous POSTs against the same invite code; confirm exactly one membership created.
- **Concurrent form-token double-submit:** two POSTs with the same `SET_STATUS` or `SAVE_NOTE` token; confirm exactly one mutation.
- **No-JS confirmations:** cancel event, remove member, delete note, admin hide note with browser scripting disabled.
- **200% text scaling:** screenshots on real mobile browser for join, home, event detail, note edit, admin create event, destructive confirmation.
- **Logpush/observability:** admin action appears in audit log; Logpush delivers to R2/S3.
- **Workers CPU:** no consistent Free-plan CPU limit errors on standard routes; if present, document plan to move to Standard/Paid.

## 3. Blocker

Requires Cloudflare staging deployment (`worker-build` + `wrangler deploy --env
staging --config wrangler.staging.local.toml`, staging D1, KV, and
`HMAC_PEPPER` secret set). Cannot be done in-repo.

## 4. Gate

Production pilot not approved until this RFC's deliverables are attached to the release checklist.

## 5. Prototype Implementation — v0.47.0

v0.47.0 adds a testing-only runtime evidence collector:

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
bun run smoke:runtime -- https://<deployed-worker-url>
```

Wrangler remains the only tool that starts or deploys the Worker and the owner
of environment/binding configuration. The collector takes one input, a URL to an
already-running Worker, and produces JSON/screenshots under
`.git-exclude/evidence/rfc050-prototype/`. It does not start, deploy, seed, or
mutate D1.

The collector checks `/healthz`, `/version`, `/join`, `/offline`,
`/manifest.webmanifest`, and `/sw.js`; verifies representative security/cache
headers; launches sandboxed incognito Chromium without `--no-sandbox`; and
captures mobile screenshots for `/join` and `/offline`.

The prototype defaults to expecting `/version` to return `"dev"` for localhost
URLs and `"staging"` for hosted URLs, matching the current Wrangler environment
defaults. The hosted default confirms that a staging Worker is up, but it is not
a strong release-candidate freshness check. Release-candidate evidence should
deploy with a candidate-specific `BUILD_VERSION` and run the smoke with a
matching `EXPECTED_VERSION`, such as `v0.47.0`.

This prototype helps operators collect early runtime evidence. It does not
complete the production-pilot gate. Seeded authenticated flows, race tests,
event timezone/ICS round-trip, real phone 200% screenshots, Logpush delivery,
and Cloudflare CPU/runtime review remain manual RFC-050 evidence items and/or
future RFC-044/049 live-D1 harness scope.

## 6. Staging Exposure Policy

### Public Exposure

Hosted Cloudflare staging is public while it is published. A `workers.dev` URL
or custom staging domain is not private just because it is obscure. Staging must
therefore use only non-production data, separate D1/KV resources, and separate
secrets.

### Publish Window

Operators should prefer local `wrangler dev` for routine checks and publish
hosted staging only when RFC-050 evidence requires a real Cloudflare deployment.
If a custom staging domain is used, protect it with Cloudflare Access or an
equivalent access-control layer where available. Keep the public test window
short; after evidence collection, remove or disable the public staging route, or
delete the staging Worker when it is no longer needed.

### Access-Control Baseline

Unknown URLs are not an access-control policy.
