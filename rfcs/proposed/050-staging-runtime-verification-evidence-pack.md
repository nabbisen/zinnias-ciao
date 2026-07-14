# RFC 050 — Exact-Candidate Hosted Staging Evidence and Pilot Gate

**Status.** Proposed — remediation revision requires architecture review;
implementation and hosted execution are not authorized

**Priority.** Architect-review remediation B4; blocks every public or production
pilot

**Source finding.** 2026-07-14 architecture preparation review B4

**Phase.** F8 / pre-pilot hardening

**Revises.** The June 12, 2026 RFC-050 prototype design

**Tracks.** RFC-015, RFC-016, RFC-044, RFC-045, RFC-054, RFC-069, RFC-071,
RFC-076, RFC-077, RFC-078 and the B5 audit remediation

**Touches.** Worker version/readiness metadata, Wrangler configuration and
deployment commands, the runtime evidence collector, new authenticated/race
evidence tooling, staging bootstrap and teardown, release checklist, operations
documentation, candidate-specific evidence records and final security review

## Summary

RFC-050 becomes the executable contract for proving production-specific
behavior against one exact Cloudflare staging candidate.

The existing prototype is useful but insufficient. It verifies reachability,
a mutable `BUILD_VERSION` label, representative public headers, and simulated
mobile rendering. It does not establish which immutable Worker version served
the requests, which remote D1 state was used, whether authenticated and
concurrent mutations behave correctly, whether persistent logs arrive, or
whether a real phone remains usable.

The revised evidence campaign anchors every hosted artifact to:

- one clean repository commit;
- one candidate label;
- one Cloudflare Worker version ID and version tag;
- one deployment serving that version at 100 percent;
- one privacy-safe fingerprint of the ignored staging configuration;
- one remote D1 database fingerprint and exact migration ledger;
- the required binding/readiness probes; and
- one bounded start/end exposure window.

Cloudflare Worker versions capture code, static assets, bindings, and
compatibility settings, but do not snapshot D1, KV, Durable Object, or other
storage state. The evidence manifest therefore records Worker identity and
storage identity separately.

The campaign must exercise hosted D1 concurrency, cookies, cache/header
behavior, B1–B3 negative configuration, no-JavaScript workflows, real-device
200 percent text scaling, recovery closure, persistent logging, and runtime
resource behavior. A mutable checklist entry, local test, source inspection,
Miniflare result, or generic `BUILD_VERSION=staging` response cannot close B4.

This revision is a design checkpoint, not permission to deploy. It may move to
`rfcs/accepted/` only after architecture review and explicit owner acceptance.
Hosted execution is a later, separately approved operator action because it
publishes a Worker and mutates isolated Cloudflare resources.

## Problem and Evidence Invariant

The current runtime smoke script accepts an arbitrary URL and normally expects
`/version` to return `staging`. Any deployment with that mutable value can pass.
The JSON report does not capture a Cloudflare version ID, deployment allocation,
repository commit, configuration fingerprint, remote migration ledger, or
resource identity. Its authenticated, race, Logpush, CPU, recovery, and real-
phone items are a prose list rather than executable or signed evidence.

The release checklist also mixes source assertions, one-time project facts, and
candidate-specific observations in one mutable document. Marking an item there
does not prove which candidate was observed or whether it was redeployed later.

The invariant introduced by this RFC is:

> A hosted/runtime gate may pass only when its evidence identifies the exact
> immutable Worker version that served the test, identifies the external state
> on which the behavior depended, records the observed result and time, and
> remains attributable to a candidate-specific review. Any candidate,
> deployment, binding, schema, secret state, or relevant configuration change
> invalidates the affected evidence until it is repeated.

## Goals

- Make exact-candidate identity machine-verifiable from inside the Worker and
  independently through Wrangler deployment metadata.
- Separate immutable Worker-version evidence from mutable storage and account
  configuration evidence.
- Convert all B4 hosted checks into explicit procedures with pass/fail criteria.
- Exercise B1–B3 fail-closed behavior and B5 audit behavior on the integrated
  candidate rather than accepting source claims.
- Preserve only synthetic non-production data in staging.
- Keep credentials, cookies, form tokens, peppers, recovery tokens, raw client
  identity, subject digests, Durable Object identities, and sensitive content
  out of evidence.
- Produce a durable, sanitized, candidate-specific attestation in the
  repository while retaining raw screenshots/log extracts in ignored storage.
- Bound staging exposure and prove teardown or access closure.
- Make rerun and invalidation rules unambiguous.

## Non-Goals

- No production deployment or production data access.
- No use of staging evidence as a substitute for backup/restore policy,
  incident response, accessibility expertise, or native-Japanese copy review.
- No claim that a single evidence run proves future candidates.
- No automatic creation of Cloudflare accounts, billing plans, Access policy,
  Logpush destinations, or production resources.
- No secrets in command arguments, tracked configuration, reports, screenshots,
  shell history, or review packages.
- No broad load, denial-of-service, penetration, or cost-amplification test.
- No execution of destructive negative-binding tests against the canonical
  staging candidate or production.
- No lifecycle shortcut: the existing prototype implementation does not make
  this revised RFC Accepted or Implemented.

## Prerequisites and Ordering

The design and local tooling may be reviewed before the other remediation RFCs
are implemented. The final B4 evidence campaign must not begin until:

1. RFC-076, RFC-077, and RFC-078 are Accepted and implemented in the candidate;
2. the B5 audit durability/redaction remediation is implemented if its behavior
   is included in the final pilot claim;
3. local release gates for the exact commit pass;
4. an isolated staging D1 and every other required staging resource exist;
5. a non-production staging pepper and any temporary test-only secrets are
   provisioned without exposing their values;
6. persistent logging capability and retention have been decided and
   provisioned;
7. the owner explicitly authorizes the bounded hosted mutation/exposure window;
8. a reviewer is named; and
9. the candidate commit is frozen for the duration of the campaign.

Controlled infrastructure preparation may occur earlier, but it cannot be
reported as completed B4 evidence.

## Decision

### Candidate identity tuple

Every evidence artifact is associated with one tuple:

```text
candidate_label
repository_commit_sha
repository_tree_state = clean
build_artifact_manifest_sha256
cloudflare_worker_name
cloudflare_version_id
cloudflare_version_tag
cloudflare_deployment_id_or_created_time
deployment_allocation = { version_id: 100% }
wrangler_version
compatibility_date
staging_config_sha256
d1_database_id_sha256
d1_migration_ledger_sha256
campaign_started_at
campaign_ended_at
```

The candidate label is human-readable, for example `v0.60.0-rc.1`; it is not
the immutable anchor. The Cloudflare version ID plus repository commit is the
runtime/source anchor. The build-artifact manifest hashes the release Worker
entrypoint, Wasm, and static assets produced from that clean commit before
upload. The preflight records clean-tree state both immediately before and
after the controlled build/deploy command. Resource IDs are fingerprinted with
SHA-256 for review artifacts and never exposed through the public application
endpoint.

The staging configuration fingerprint covers the exact ignored Wrangler file
used for the deployment. The evidence preflight also emits a redacted shape
containing environment name, binding names, resource kinds, compatibility
settings, and feature-flag values. It must omit secret values, raw resource IDs,
routes/account identifiers when unnecessary, and local filesystem usernames.

### Cloudflare version metadata binding

Add a Worker Version Metadata binding named `CF_VERSION_METADATA` to the root
and every named Wrangler environment:

```toml
[version_metadata]
binding = "CF_VERSION_METADATA"

[env.staging.version_metadata]
binding = "CF_VERSION_METADATA"
```

The exact environment inheritance behavior and TOML shape must be validated
against the installed Wrangler schema during implementation.

worker-rs 0.8.x exposes `WorkerVersionMetadata` with `id`, `tag`, and
`timestamp`. Extend `/version` to return a fixed JSON schema:

```json
{
  "ok": true,
  "build": "v0.60.0-rc.1",
  "worker_version_id": "<Cloudflare UUID>",
  "worker_version_tag": "v0.60.0-rc.1"
}
```

The endpoint remains non-secret, `Cache-Control: no-store`, and protected by
the normal security headers. It returns `503` in hosted staging/production if
version metadata is missing or malformed. Local development may return an
explicit `local` metadata state that the hosted collector rejects.

The collector requires exact expected build, version ID, and tag values. It
must not default hosted checks to `staging` or silently accept any deployed
version.

### Upload and deployment record

Complete resource creation, migrations, staging bootstrap, secret changes, and
feature-flag decisions before the canonical upload. A later `secret put`,
binding change, or configuration change can create or deploy a different Worker
version and therefore invalidates the candidate identity. The current runbook's
deploy-then-bootstrap order must be reversed or split so the exact candidate is
uploaded only after bootstrap has finished and no secret value changes again.

The operator deploys the frozen candidate with an exact tag and a message that
contains the candidate label and commit SHA:

```text
wrangler deploy --env staging --config <ignored-staging-config>
  --tag <candidate-label>
  --message <candidate-label-and-commit>
  --strict
```

The command is shown as a wrapped design example; the implemented runbook must
use safe shell quoting and must not interpolate untrusted values.

Immediately after deploy, capture machine-readable output from:

```text
wrangler deployments list --env staging --config <ignored-config> --json
wrangler versions view <version-id> --env staging
  --config <ignored-config> --json
```

The gate requires one version serving 100 percent of staging traffic. Gradual
deployment, two-version allocation, preview routing, or a version override is
not acceptable for canonical evidence. Negative tests use a separately named
Worker and do not deploy a broken version over the canonical Worker. If the
canonical deployment changes for any reason, the operator must restore or
redeploy the exact candidate at 100 percent and repeat all evidence whose
behavior it could affect.

Deployment output and Wrangler JSON must be sanitized before retention. Account
tokens, headers, raw resource identifiers, routes not needed for review, and
operator-local paths are excluded.

### External-state manifest

Cloudflare Worker versions do not snapshot storage. Before the first
application request and again at campaign end, capture:

- the SHA-256 fingerprint of the staging D1 database ID from the ignored config;
- `wrangler d1 migrations list ... --remote` output;
- a bounded query of `d1_migrations` containing migration names only;
- a bounded schema-presence query for required tables/indexes, not row data;
- readiness results for D1, the RFC-078 Durable Object binding, and the
  RFC-077 secret-dependent routes;
- feature flags that affect tested behavior;
- logging/observability enabled state and Logpush job health evidence; and
- the staging hostname and exposure-control mode, redacted where the review
  package does not need a public URL.

The start/end migration ledger and resource fingerprints must match. If they do
not, the campaign is invalid unless the changed state was itself the reviewed
test and all dependent checks were repeated afterward.

Secret presence is recorded only as `present_and_runtime_validated` or
`missing_for_negative_test`; secret names may be recorded, values and value
hashes may not. A secret hash would create an offline oracle and is forbidden.

### Evidence package layout

Raw evidence remains ignored:

```text
.git-exclude/evidence/rfc050/<candidate-label>/
  00-manifest.json
  01-local-gates.json
  02-deployment.json
  03-external-state-before.json
  10-public-runtime.json
  20-authenticated-flows.json
  30-concurrency.json
  40-negative-controls.json
  50-browser-and-device.md
  60-observability-and-runtime.md
  70-recovery-and-restore.md
  80-external-state-after.json
  90-teardown.json
  99-review-verdict.md
  screenshots/
```

Each machine-readable record contains schema version, candidate tuple,
collection time, tool/version, test ID, observed result, pass/fail, and artifact
hash. It must not merely say “checked.” Manual evidence records device/browser
versions, steps, expected result, actual result, reviewer, and time.

Create one sanitized tracked attestation at:

```text
docs/src/tester/release-candidates/<candidate-label>.md
```

It records the candidate tuple, evidence-package digest, per-gate verdicts,
open exceptions, reviewer decision, and staging closure result. It contains no
credentials, cookies, sensitive screenshots, subject identifiers, raw resource
IDs, or user data. A failed candidate may be recorded as failed; it must never
be edited into a pass after a different version is deployed. A new candidate
gets a new record.

### Data and privacy rules

Staging uses only synthetic communities, users, events, notes, and attendance.
Do not use real names, phone numbers, email addresses, community names, or
production exports.

Evidence tooling must redact or omit:

- invite/relink/recovery codes and their HMACs;
- session cookies and cookie values;
- form tokens, peppers, recovery tokens, authorization headers, and CSRF-like
  values;
- raw/HMAC IP addresses or network prefixes;
- user, membership, session, Durable Object, and credential identifiers unless
  a synthetic identifier is strictly needed, in which case retain only a
  campaign-local alias;
- note/event content beyond fixed synthetic fixtures;
- full URLs containing any query or fragment; and
- raw D1 row dumps.

Browser screenshots must be reviewed before retention. Developer Tools network
exports and HAR files are prohibited by default because they commonly contain
cookies and form bodies. If a HAR is indispensable, a purpose-built sanitizer
and manual review are required before it enters the evidence directory.

The collector fails if it detects a known credential, cookie value, token,
pepper, raw resource ID, or unredacted submitted form body in a report.

## Required Evidence Matrix

### E0 — local candidate freeze

Pass requires observed results from the exact commit:

- clean worktree and recorded commit SHA;
- formatting, native tests including SSR, full-workspace clippy, wasm check,
  mdBook build, dependency/security gates, and release-contract tests;
- production-equivalent release build;
- Wrangler dry-run with the ignored staging config;
- candidate label aligned across Cargo/package/static cache metadata; and
- no unreviewed dependency or generated-file drift after the build.

The evidence record stores commands, exit status, bounded summaries, and tool
versions. It does not claim success from an earlier review thread.

### E1 — identity, deployment, bindings, and migrations

Pass requires:

- `/version` matches the expected build, Cloudflare version ID, and tag;
- Wrangler independently reports the same version at 100 percent allocation;
- `/healthz` reports only the readiness it actually probes;
- the intended D1 fingerprint and complete migration ledger are recorded;
- required D1 tables/indexes exist;
- all required bindings for RFC-076–RFC-078 and the audit remediation are
  present and operational;
- staging feature flags match the manifest; and
- start/end external-state manifests agree.

A successful `/healthz` alone is not sufficient.

### E2 — public routes, headers, cookies, cache, and offline behavior

Against the exact candidate:

- representative public, authenticated, error, redirect, static, manifest,
  service-worker, export, `429`, and `503` responses have expected status,
  content type, CSP, referrer, framing, MIME, permissions, and cache headers;
- invite, relink, session, and form-token cookies have the reviewed `Secure`,
  `HttpOnly` where applicable, `SameSite`, path, expiry, and host/domain shape;
- no cookie value is retained;
- authenticated HTML is not served from service-worker or browser cache after
  logout;
- B1 invite plaintext never appears in a URL, redirect `Location`, browser
  history, referrer, retained log event, or evidence artifact; and
- error bodies contain no SQL, stack, binding, secret, code-validity, or
  platform detail.

### E3 — authenticated core and timezone/export flows

Using synthetic data and a fresh browser context:

- join, logout, relink/help-signin, attendance, note, community switch, and
  authorized admin flows complete;
- authorization-denied cross-community/direct-route checks remain generic;
- an `Asia/Tokyo` 09:00–10:30 event round-trips at the same local time;
- editing it to 13:00 updates the intended occurrence without duplication;
- ICS `DTSTART`/`DTEND` represent the reviewed local time correctly;
- community creation behavior matches its enabled/disabled staging flag; and
- required audit rows exist with reviewed action and redacted metadata.

The operator records outcome counts and campaign-local aliases, not database
row dumps.

### E4 — concurrency and fail-closed controls

Use bounded bursts against synthetic one-use credentials/tokens:

- two or more concurrent redemptions of one invite produce exactly one winning
  membership/session and one used invite;
- concurrent submissions of one form token produce exactly one protected
  mutation for attendance, note, and one representative destructive admin
  action;
- RFC-078 invite and relink limits allow exactly the reviewed capacity under a
  concurrent burst and block the remainder;
- community creation enforces every approved user/session/network dimension;
- reset/expiry behavior matches RFC-078; and
- D1 postconditions and audit cardinality match the winning operations.

The test harness has hard request/concurrency ceilings and aborts on unexpected
status, cost, or mutation. This is correctness evidence, not a load test.

### E5 — isolated negative configuration

Negative tests use separate ignored Wrangler configurations, separately named
Workers, and separate mutable D1/Durable Object resources. They use the same
synthetic-data classification but do not share the canonical candidate's D1 or
coordinator state. They must never target production. An isolated test may
reuse a non-mutating account-level logging destination only when canary identity
keeps its events distinguishable.

Prove at minimum:

- missing/invalid `HMAC_PEPPER` causes RFC-077 `503` behavior before protected
  D1 work;
- missing/unavailable RFC-078 coordinator causes `503`, not credential lookup
  or community mutation;
- an exhausted coordinator returns generic `429`, not `503` or credential
  validity information;
- wrong/missing D1 binding is not reported healthy and cannot partially mutate;
- malformed version metadata cannot pass the exact-candidate collector; and
- audit failure behavior matches the accepted B5 policy.

After negative testing, confirm the canonical Worker still serves its expected
version ID at 100 percent and that its E1 identity/readiness checks still pass.
If any canonical deployment or resource changed, redeploy/restore it and repeat
E1 plus every affected smoke/postcondition. A negative Worker/version is never
the candidate of record.

### E6 — no-JavaScript and real-device accessibility

Automated Chromium emulation remains useful regression evidence but does not
replace the following manual checks:

- one real phone/browser at operating-system or browser 200 percent text size;
- join, home, event detail, note edit, event creation, member removal
  confirmation, and recovery/help-signin surfaces;
- no hidden control, clipped critical copy, two-dimensional page scrolling, or
  unusable target;
- JavaScript-disabled join, attendance, community switch, note workflows, and
  representative destructive confirmations; and
- keyboard/focus and visible error-summary checks on a desktop browser.

Screenshots use synthetic data and are manually inspected for tokens/codes
before retention. Native-Japanese copy acceptance remains RFC-054; B4 records
whether that independent gate is complete rather than self-approving it.

### E7 — persistent logs, audit, and incident visibility

Pass requires:

- Workers logging enabled for the exact staging Worker;
- the intended persistent sink receives a unique synthetic canary event from
  the exact version;
- the canary is retrievable after the documented delivery interval;
- request ID, Worker version, outcome, and bounded event category correlate
  without credentials or subject data;
- a security-control unavailable event and representative audit event arrive;
- Logpush job health/alerting is observed; and
- retention and access roles match the reviewed operations policy.

Cloudflare currently documents Workers Trace Events Logpush as a Workers Paid
capability. If the selected plan cannot provide the required Logpush evidence,
this gate remains open and the public/production pilot remains No-Go unless a
separate architecture decision explicitly replaces that requirement. Real-time
`wrangler tail` is diagnostic and is not persistent-delivery evidence.

### E8 — CPU, query, error, and plan behavior

For each representative standard route, record bounded Cloudflare dashboard or
API observations for:

- invocation count and outcome;
- uncaught exceptions and CPU-limit outcomes;
- CPU time distribution/max available from the selected plan;
- D1 read/write/row behavior where observable; and
- Durable Object error/overload behavior introduced by RFC-078.

Pass requires zero unexplained exceptions, CPU-limit terminations, or storage
overload errors during the bounded campaign. The final reviewer must see useful
headroom against the selected plan rather than merely “no consistent errors.”
If available metrics cannot establish headroom, record the uncertainty and keep
the pilot gate open or move to a suitable plan before approval.

### E9 — migration, restore, recovery, and closure

Pass requires:

- remote staging migrations applied from a fresh or documented baseline;
- migration ledger and required schema verified;
- an export/restore rehearsal into a separate staging recovery database;
- representative sign-in/read verification against the restored database using
  an isolated recovery Worker/config;
- the RFC-069 total-community-access recovery drill with a synthetic community;
- recovery flag disabled, temporary secret deleted/rotated, and endpoint
  confirmed closed afterward;
- negative-test and recovery Workers/routes closed; and
- canonical staging Worker either removed or placed behind the reviewed access
  control when the evidence window ends.

Teardown evidence records resource kind, privacy-safe fingerprint, action,
result, time, and operator. Destructive deletion of retained staging D1 or
evidence sinks remains a separate explicit owner decision.

## Tooling Slices

After acceptance, implementation should proceed in reviewable slices:

1. Add version metadata binding, strict hosted `/version` schema, and tests.
2. Add candidate manifest/schema validation and redaction utilities.
3. Upgrade `runtime-smoke.mjs` to require exact identity for hosted mode while
   preserving an explicitly non-authoritative local mode.
4. Add bounded authenticated/browser flow collection using synthetic fixtures.
5. Add concurrency/postcondition tooling with hard safety ceilings.
6. Add negative-config fixtures and canonical-candidate restoration checks.
7. Add manual evidence templates, artifact hashing, and leakage scanning.
8. Add candidate-specific tracked attestation template and release-gate rules.
9. Reconcile deployment, staging prototype, recovery, backup, observability,
   threat-model, RFC-045, and release-checklist documentation.

The tooling must not deploy, create/delete resources, set/delete secrets,
bootstrap, migrate, restore, or tear down merely because an evidence collector
is invoked. Mutating operator commands remain explicit, individually described,
and confirmation-gated.

A developer handoff is appropriate only after this RFC is Accepted because the
tooling crosses Rust runtime, Wrangler configuration, Node evidence scripts,
Cloudflare operations, privacy, and manual QA.

## Gate Semantics and Invalidation

Each test is `Pass`, `Fail`, `Blocked`, or `Not run`. Only observed `Pass` closes
it. `Blocked`, missing, assumed, source-inspected, or carried from another
candidate is not pass.

Repeat at least the affected gate when any of these changes:

- Worker version ID, deployment allocation, code, static asset, binding,
  compatibility setting, feature flag, or relevant environment variable;
- D1 database identity, migration ledger, relevant seed shape, or schema;
- Durable Object class/migration/binding;
- secret presence or pepper rotation for credential-dependent tests;
- Logpush job, observability setting, destination, retention, or access policy;
- hostname, route, Access policy, cache rule, or service-worker version;
- browser/device version for a device-specific regression; or
- evidence tooling/schema.

A cosmetic documentation-only change may retain runtime evidence only if the
reviewer records why it cannot affect the tested artifact. The attestation still
identifies the exact deployed source commit.

## Review and Approval

The evidence author attests the manifest with name and time and records who
operated the hosted window. Cryptographic signing is not claimed unless a
separate repository policy defines its keys and verification. A reviewer
independently checks:

- candidate and deployment identity;
- external-state start/end agreement;
- failed/blocked items and exceptions;
- privacy scan and artifact hashes;
- teardown/closure; and
- final security-review dependencies.

The owner then records one decision:

```text
No-Go
Controlled staging only
Private named-participant pilot
Public/production pilot
```

Approval scope and expiry are explicit. An evidence pack never silently grants
production deployment authority.

## Acceptance Criteria

RFC-050 implementation and first-candidate execution are complete only when:

1. Hosted evidence is pinned to a clean commit, build-artifact manifest, and
   immutable Cloudflare Worker version, confirmed both in-runtime and through
   deployment metadata.
2. External D1/binding/logging state is separately identified and unchanged
   across the campaign or all affected checks are repeated.
3. E0–E9 have observed results against the exact candidate; every public-pilot
   blocker is `Pass` with no unreviewed exception.
4. B1–B3 fail-closed and concurrency behavior is demonstrated on hosted
   Cloudflare infrastructure.
5. Accepted B5 audit durability/redaction behavior is demonstrated, including
   failure and hostile nested-metadata cases where required by that design.
6. Real-phone, 200 percent text, no-JavaScript, recovery, restore, persistent
   logging, CPU/runtime, cookies/cache, and security-header evidence is present.
7. No credential, secret, cookie, client identity, raw resource ID, or sensitive
   user data is retained in evidence or the tracked attestation.
8. Negative/recovery infrastructure is closed and canonical staging exposure is
   closed or explicitly access-controlled after the bounded window.
9. A candidate-specific tracked attestation and ignored evidence package have
   matching hashes and an independent review verdict.
10. Final owner approval states the permitted scope; absent that approval the
    public/production pilot remains No-Go.

Moving this RFC to `done/` records that the evidence system and its first
accepted candidate campaign were completed. Every later release candidate must
still produce its own attestation; RFC lifecycle completion does not make
runtime evidence permanent.

## Rollback and Failed Campaigns

Evidence collection must not make application behavior less secure. If staging
fails:

1. stop the affected test and preserve only sanitized diagnostics;
2. close public/negative routes when they are not needed for diagnosis;
3. mark the candidate `Fail` or `Blocked`; never edit evidence into a pass;
4. repair through a new commit/version or reviewed external-state change;
5. create a new candidate record or repeat every invalidated gate; and
6. re-run teardown verification.

Do not roll back to B1 URL handoffs, the public pepper fallback, fail-open KV
rate limiting, or best-effort required audits. Infrastructure failure results
in a No-Go and roll-forward repair.

## Alternatives Rejected

### Keep `BUILD_VERSION=staging` as the identity check

Rejected. It is mutable, reusable across uploads, and cannot identify the code,
bindings, compatibility settings, or deployment allocation that served a test.

### Trust deploy output without runtime metadata

Rejected. It does not prove which version handled a later request, especially
after another deploy or a multi-version allocation. Runtime metadata and
Wrangler deployment metadata must agree.

### Treat the Worker version as the whole candidate

Rejected. Cloudflare versions do not snapshot D1, KV, Durable Object, or other
storage state. External state requires its own manifest and postconditions.

### Commit all raw evidence

Rejected. Screenshots, logs, browser data, and database observations create
credential, personal-data, and operational-metadata risk. The repository keeps
a sanitized attestation and hashes; raw evidence remains access-controlled and
ignored.

### Accept local Wrangler/Miniflare evidence

Rejected for B4. Local evidence is valuable before deployment but does not
establish Cloudflare edge, hosted binding, remote D1 concurrency, cookie/cache,
logging delivery, plan, or device behavior.

### Use `wrangler tail` as Logpush proof

Rejected. Tail is a real-time diagnostic stream, not proof of persistent
delivery, retention, access control, or job health.

## Current Platform References

- [Cloudflare Worker Version Metadata binding documentation](https://developers.cloudflare.com/workers/runtime-apis/bindings/version-metadata/).
- [Cloudflare Workers Versions and Deployments documentation](https://developers.cloudflare.com/workers/versions-and-deployments/).
- [Cloudflare Wrangler environments documentation](https://developers.cloudflare.com/workers/wrangler/environments/).
- [Cloudflare D1 migrations documentation](https://developers.cloudflare.com/d1/reference/migrations/).
- [Cloudflare D1 import/export documentation](https://developers.cloudflare.com/d1/best-practices/import-export-data/).
- [Cloudflare Workers Logs documentation](https://developers.cloudflare.com/workers/observability/logs/workers-logs/).
- [Cloudflare Workers Logpush documentation](https://developers.cloudflare.com/workers/observability/logs/logpush/).
- Installed Wrangler configuration schema and command help.
- Installed worker-rs `WorkerVersionMetadata` API.
