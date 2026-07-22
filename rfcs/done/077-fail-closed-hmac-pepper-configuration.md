# RFC 077 — Fail-Closed HMAC Pepper Configuration

**Status.** Implemented — local implementation committed at `901855b`; hosted
criteria 8–9 and architecture finding B2 were architecture-reviewed and
owner-accepted on 2026-07-22
**Priority.** Completed architect-review remediation for B2; unrelated
architecture and public/production pilot holds remain  
**Source finding.** 2026-07-14 architecture preparation review B2  
**Tracks.** RFC-003, RFC-012, RFC-016, RFC-038, RFC-045, RFC-050, RFC-069  
**Touches.** `workers/ssr/src/crypto.rs`, pepper callers and error propagation,
Worker entrypoint/readiness handling, Wrangler configuration, local-development
setup, bootstrap/deployment scripts, contracts/SSR tests, release checklist,
hosted negative evidence

## Summary

RFC-077 removed the fixed public HMAC-pepper fallback and requires valid secret material
before any secret-dependent application route can execute.

Before RFC-077, the centralized accessor returned
`dev-pepper-change-in-production` whenever neither a secret nor a plain variable
is bound. That fallback is reachable in hosted staging and production after a
configuration mistake. The same pepper protects invite codes, relink codes,
sessions, form tokens, calendar tokens, join tickets, and operator recovery
codes, so a silent fallback compromises several trust boundaries at once.

RFC-077 introduces two independent enforcement layers:

1. Wrangler configuration declares `HMAC_PEPPER` required for every deployable
   environment, blocking an ordinary upload/deploy when it is absent.
2. Runtime resolution returns a typed `Result`; dynamic routes and readiness
   fail closed with a non-mutating `503` if the secret is absent or invalid.

Local development uses a developer-specific ignored `.dev.vars.dev` secret.
There is no built-in, committed, deterministic, or plain-variable fallback.

This remediation design, implementation, and hosted evidence are accepted. Its
handoff was architecture-reviewed, the owner authorized the bounded local
implementation at checkpoint `91f3a39`, and the implementation was committed
as `901855b`. Corrected exact-candidate hosted evidence satisfied criteria 8–9
and closed source architecture finding B2 on 2026-07-22.

## Problem and Security Invariant

Before this implementation patch, `crypto::pepper(env)` checked:

1. a non-empty secret binding;
2. a non-empty plain variable binding;
3. a fixed public sentinel.

The release checklist asks an operator to configure the secret, but the Worker
still starts and issues credentials if that step is missed. A checklist is not
an enforcement boundary.

The invariant introduced by this RFC is:

> A request must not validate, mint, rotate, persist, or revoke any
> pepper-derived credential unless one valid `HMAC_PEPPER` secret binding has
> been resolved for that request. Missing, empty, malformed, legacy-sentinel,
> or plain-variable-only configuration is unavailable configuration, never a
> development default.

The secret value must never be logged, rendered, returned in an error, placed
in evidence, or copied into tracked configuration.

## Goals

- Make a missing or invalid pepper prevent all credential-dependent work.
- Make ordinary Wrangler deployment fail before publication when the required
  secret is absent.
- Preserve a practical local `wrangler dev --env dev --local` workflow without
  a repository-known pepper.
- Give operators and health checks an unambiguous not-ready signal distinct
  from session expiry, invalid invite codes, or transient internal errors.
- Force all pepper callers to handle resolution failure explicitly.
- Prove both deployment-time and runtime enforcement on isolated hosted
  infrastructure.
- Preserve existing credentials when the same valid secret is restored.

## Non-Goals

- No pepper rotation protocol, dual-key verification window, KMS, or Secrets
  Store migration.
- No change to HMAC-SHA256, code normalization, token format, or credential
  lifetimes.
- No recovery of credentials created with the public fallback; hosted evidence
  must determine whether any such rows exist before pilot approval.
- No B1 invite response transport, B3 rate-limit availability, or B5 audit
  durability/redaction implementation.
- No claim that successful RFC-077 implementation alone permits hosted staging
  or a public/production pilot.
- No migration from the existing Wrangler TOML format in this security patch.

## Decision

### Secret-only pepper resolver

Replace `pepper(env) -> String` with a fallible secret-only resolver. The exact
Rust names may vary, but the boundary should resemble:

```rust
pub struct HmacPepper(String);

pub enum PepperConfigError {
    Missing,
    Empty,
    InvalidLength,
    SurroundingWhitespace,
    LegacySentinel,
}

pub fn pepper(env: &worker::Env) -> Result<HmacPepper, PepperConfigError>;
```

`HmacPepper` must not implement `Debug`, `Display`, serialization, or conversion
into log fields. It may expose a narrowly scoped `as_bytes`/`as_str` borrow to
the HMAC helper. `PepperConfigError` may be formatted only as a stable category;
it must never retain or format the rejected value.

Resolution rules:

1. read only `env.secret("HMAC_PEPPER")`;
2. reject a missing binding;
3. reject empty or all-whitespace text;
4. reject leading/trailing whitespace rather than silently trimming and
   changing key identity;
5. reject values shorter than 32 bytes;
6. reject the known legacy sentinels `dev-pepper-change-in-production` and
   `dev-pepper` regardless of length rules;
7. return the exact accepted bytes without normalization.

The standard operational value remains 32 random bytes encoded as 64 lowercase
hex characters. The runtime need not enforce hexadecimal encoding because an
existing strong non-hex secret is still valid key material; it enforces a
minimum byte length and obvious configuration mistakes.

There is no `env.var("HMAC_PEPPER")` compatibility path. A plain Wrangler var is
not an acceptable secret binding in any environment.

### Wrangler required-secret declaration

The installed Wrangler 4.106 configuration schema supports the
`secrets.required` property. Declare the requirement separately at the root
and in every named environment because Wrangler environment bindings are not
inherited:

```toml
[secrets]
required = ["HMAC_PEPPER"]

[env.dev.secrets]
required = ["HMAC_PEPPER"]

[env.staging.secrets]
required = ["HMAC_PEPPER"]

[env.production.secrets]
required = ["HMAC_PEPPER"]
```

The root declaration protects accidental `wrangler deploy` without `--env`.
Named declarations protect the intended commands. The implementation must
validate the exact TOML syntax against the repository's installed Wrangler
schema and a dry-run/negative command before relying on it.

This CLI layer is defense in depth, not a substitute for runtime validation.
Deploys can occur through other tooling, secrets can be deleted, and a test
must be able to exercise the runtime failure path deliberately.

`COMMUNITY_RECOVERY_TOKEN` remains an optional, short-lived RFC-069 incident
secret and must not be added to the required list. Current Wrangler also uses
`secrets.required` as the allowlist for keys loaded from local `.dev.vars` or
`.env` files. Implementation validation must therefore prove that normal hosted
operator recovery still sees an attached optional recovery secret and must
document a dedicated ignored test configuration if local recovery testing needs
that optional key. RFC-077 must not make the recovery token permanently
required merely to simplify local loading.

### Local development

Local development must provide a real developer-specific secret through:

```text
.dev.vars.dev
```

That filename matches `wrangler dev --env dev --local`. Cloudflare documents
`.dev.vars.<environment-name>` as local-development secret input; it is not a
deployed secret source.

Implementation requirements:

- add `.dev.vars*` and `.env*` to `.gitignore` without unignoring any populated
  secret file;
- document a setup command or small helper that generates at least 32 random
  bytes, writes the ignored file with restrictive permissions, and never prints
  the value;
- do not ship a populated example, shared sample key, default literal, or
  `LOCAL_DEV_MODE` bypass;
- verify that worker-rs resolves the local value through `env.secret`, not by
  reintroducing a production-reachable `env.var` path;
- make missing local configuration fail visibly with the same safe `503`, plus
  a developer-facing setup instruction in terminal documentation.

Tests that do not instantiate a Worker environment should pass explicit test
keys directly to pure HMAC helpers. They must not depend on a global fallback.

### Runtime preflight and route behavior

The Worker entrypoint must resolve security configuration before dispatching
any dynamic application route. This central preflight is necessary because
many handlers currently map all `require_auth` errors to “session expired”; a
missing pepper must not masquerade as an invalid session.

When resolution fails:

- return `503 Service Unavailable`;
- perform no D1 or KV read/write and no credential comparison or generation;
- set no session or credential cookie;
- return a fixed, generic Japanese temporary-unavailability page for browser
  routes;
- include normal security headers, `Cache-Control: no-store`, and the request
  ID;
- emit a structured operator diagnostic containing request ID and a bounded
  error category, but no secret value or derived HMAC;
- do not redirect to `/join`, `/relink`, or a session-expired page.

The preflight applies to `/join`, `/relink`, `/operator/*`, `/logout`, `/`,
`/c`, `/switch`, `/communities/*`, and every `/c/*` member/admin route. Unknown
dynamic routes may also return `503` while configuration is unavailable; safe
unavailability is preferable to routing behavior that accidentally bypasses
the guard.

Static assets, `/offline`, and `/version` may remain available because they do
not consume credential material. They must not claim readiness. `/healthz`
must run the same resolver and return:

```text
200 {"ok":true,"ready":true,"service":"ciao.zinnias"}
```

when valid, or a generic:

```text
503 {"ok":false,"ready":false,"service":"ciao.zinnias"}
```

when unavailable. The response must not name the missing binding publicly.

After central preflight, individual callers still use the fallible accessor and
propagate `?`; they must not `unwrap`, `expect`, substitute an empty string, or
map configuration failure to an authentication outcome.

### Caller propagation

The compiler-guided change covers every current pepper consumer, including:

- session authentication and session issuance;
- form-token issue/consume compatibility helpers;
- join tickets and invite redemption;
- relink and help-signin codes;
- admin invite generation;
- calendar-token generation/revocation;
- community creation and operator recovery.

`codlet::issue_token` must become fallible rather than returning an empty token
when pepper resolution fails. All render callers must propagate the error; an
empty hidden token is not a valid fail-closed result.

A release gate must continue to require one centralized `HMAC_PEPPER` accessor
and forbid direct secret/var reads elsewhere. It must additionally reject the
legacy sentinel literals, `unwrap`/`expect` on pepper resolution, empty-string
substitution, and any plain-variable compatibility path.

## Deployment and Bootstrap Contract

Required-secret validation changes first-deploy ordering.

Current documentation deploys staging before bootstrap, while bootstrap later
generates and sets the pepper. After RFC-077, the secret must be provisioned as
part of or before the first publish. The accepted implementation must update
scripts and documentation to one coherent sequence.

The preferred fresh-environment sequence is:

1. create isolated D1/KV resources and ignored config;
2. run the explicitly confirmed initial bootstrap/provision operation, which
   generates one pepper without printing it, sets the Wrangler secret, applies
   migrations, and seeds rows HMACed with that same pepper;
3. deploy the exact candidate with required-secret validation active;
4. verify `/healthz` is ready before any join flow.

Because `wrangler secret put` creates and deploys a Worker version, the script
and runbook must state that provisioning can publish code. The operation must
remain explicit about target environment and destructive rotation effects.

Routine bootstrap must not silently rotate an active environment. A rerun on
an existing database requires explicit operator confirmation that existing
sessions, invites, relink codes, form tokens, calendar tokens, and recovery
codes will become invalid. A non-rotating seed path is deferred unless needed;
the project must not pretend it can read back an existing secret.

Restoring the same deleted/missing secret restores validation of existing HMAC
rows. Replacing it is a credential rotation and follows the existing launch and
recovery warnings.

## Threat Analysis

| Threat | Required control |
|---|---|
| Hosted deploy omits pepper | `secrets.required` blocks ordinary deploy. |
| Secret is deleted or alternate tooling bypasses CLI | Runtime preflight returns non-mutating `503`. |
| Public fallback makes D1 HMACs forgeable | All sentinel literals and fallbacks are removed and gated. |
| Plain var exposes key material | Resolver accepts secret binding only. |
| Missing pepper appears as expired session | Central preflight occurs before auth error mapping. |
| Empty token is rendered after helper failure | Token issuance returns `Result`; callers propagate. |
| Secret appears in logs/errors | Opaque pepper type and category-only configuration error. |
| Local convenience leaks into hosted config | Local key exists only in ignored `.dev.vars.dev`; no mode flag. |
| Readiness says healthy while auth is unusable | `/healthz` resolves and validates the pepper. |
| Test bypass becomes a production path | Negative runtime config is ignored, isolated, time-bounded, and torn down. |

## Hosted Negative-Test Design

Both enforcement layers require evidence.

### Deployment-time evidence

Against an isolated staging Worker with no configured secret, run the ordinary
candidate deploy using the normal environment config. Capture the bounded CLI
failure showing that required-secret validation prevented publication. Do not
capture or infer any secret value.

### Runtime evidence

CLI rejection means a normal deploy cannot reach the runtime guard. To test
that second layer:

1. create a separate ignored negative-test Wrangler config and isolated Worker
   name/resources;
2. deliberately omit only the `secrets.required` declaration and
   `HMAC_PEPPER`, keeping the exact candidate Worker artifact/code;
3. deploy for a short evidence window with no real users or production data;
4. verify `/healthz`, `/join`, `/relink`, one authenticated route, and one
   secret-dependent POST return safe `503` behavior;
5. verify no D1/KV rows changed and no `Set-Cookie` or secret-derived output was
   produced;
6. tear down the Worker and isolated resources immediately.

This bypass is test infrastructure, not a supported environment. Its config
must remain ignored and its use must be recorded in RFC-050 evidence.

For an empty-value case, first attempt the platform-supported secret mechanism.
If Cloudflare rejects empty secrets before publication, record that rejection
and pair it with local resolver tests for empty/all-whitespace values. Do not
weaken the hosted configuration merely to manufacture an empty secret.

Evidence records may contain Worker name, candidate version, timestamps,
response status/headers, bounded error category, and row counts. They must not
contain the pepper, HMACs, session cookies, invite/relink codes, form tokens, or
request bodies.

## Implementation Slices

Implementation should remain one reviewable B2 patch after acceptance:

1. Add opaque pepper validation and unit tests; remove all sentinels and the
   plain-var path.
2. Add entrypoint preflight, fixed `503` renderer, and readiness behavior.
3. Propagate fallibility through all pepper and form-token callers.
4. Declare required secrets for root/dev/staging/production and validate the
   config with installed Wrangler.
5. Add ignored local-secret rules and update developer setup.
6. Reconcile bootstrap/deploy ordering and rotation warnings.
7. Add source/SSR gates and update threat model/release checklist claims.
8. Capture isolated hosted deployment-time and runtime-negative evidence under
   RFC-050 after B1/B3 staging conditions are satisfied or the environment is
   explicitly approved solely for this short-lived negative test.

No developer handoff is required at Proposed status. If the RFC is Accepted
and implementation is delegated, a companion handoff should enumerate caller
groups and keep B1/B3/B5 changes out of the patch unless separately accepted.

## Test and Release Evidence

### Required local automated evidence

- Pure validation tests cover missing, empty, whitespace-only, surrounding
  whitespace, under-32-byte, both legacy sentinels, and valid 32+/64-byte keys.
- No repository source contains either legacy fallback literal outside a
  regression-test forbidden list or historical documentation.
- No runtime source reads `HMAC_PEPPER` as a plain var or outside the centralized
  resolver.
- Every pepper caller propagates failure; token issuance cannot return an empty
  token on configuration failure.
- Dynamic-route preflight returns `503` before D1/KV access when resolution
  fails.
- `/healthz` returns ready only with a valid secret and never exposes the value
  or binding name publicly.
- Static assets and `/version` behavior match the explicit allowlist.
- Valid-secret auth, invite, relink, calendar, form-token, and operator tests
  remain green.
- An attached optional `COMMUNITY_RECOVERY_TOKEN` remains available to the
  hosted operator-recovery path without becoming a normal deployment
  prerequisite.
- Wrangler config validation proves all four deployable scopes declare the
  required secret.
- `.dev.vars*` and `.env*` secret files are ignored.

### Required local runtime evidence

- `wrangler dev --env dev --local` without `.dev.vars.dev` starts only in a
  visibly not-ready/fail-closed state; `/join` and `/healthz` return `503`.
- With a generated ignored `.dev.vars.dev`, health is ready and normal join and
  authenticated flows work.
- Replacing the local secret invalidates old local credentials as documented;
  restoring the original value restores them when their rows remain valid.

### Required hosted evidence

- Ordinary deploy without the secret is rejected before publication.
- Isolated runtime-negative deployment behaves as specified above.
- Exact candidate with a valid staging secret reports ready and completes join,
  authenticated form-token, calendar token, relink/help-signin, and invite
  generation checks required by RFC-050.
- Deleting/unbinding the secret, if the platform permits this operation despite
  required-secret metadata, changes readiness and dynamic routes to `503`
  without mutating credential data.

## Acceptance Criteria

RFC-077 implementation is complete only when:

1. The Worker contains no deterministic pepper fallback and accepts no plain
   variable as `HMAC_PEPPER`.
2. Root, dev, staging, and production Wrangler scopes declare the secret
   required.
3. Missing or invalid configuration produces a generic, non-mutating `503` on
   every dynamic route and not-ready `/healthz`.
4. Configuration failure cannot be presented as session expiry, invalid code,
   empty form token, or a successful health response.
5. Local development works with a generated ignored environment-specific
   secret and no shared key.
6. Bootstrap/deploy ordering is executable and rotation consequences remain
   explicit.
7. Automated caller, config, validation, and regression gates pass.
8. Deployment-time and hosted runtime-negative evidence is recorded without
   secrets.
9. Exact-candidate valid-secret hosted smoke passes before B2 is closed for a
   public or production pilot.

Architecture approval may accept this design and move it to `accepted/` before
hosted evidence exists. Moving implementation to `done/` establishes the local
source/config change; it does not by itself close hosted B2 evidence or lift
the roadmap remediation hold. For RFC-077, the separately reviewed hosted
evidence now closes B2; the overall roadmap hold remains because unrelated
findings and release gates are still open.

## Rollout and Rollback

There is no database migration.

Roll out first to isolated staging with the existing correct pepper preserved.
Confirm readiness before exercising credentials. Production rollout requires
confirming the current production secret is attached to the candidate version;
do not rotate merely to adopt the fallible accessor.

Rollback to any fixed fallback or plain-variable acceptance is prohibited. If
configuration resolution fails after rollout, restore the same secret or roll
forward. Serving a `503` is the safe failure mode.

## Alternatives Rejected

### Keep the public sentinel only for `[env.dev]`

Rejected. A named Wrangler environment can be deployed, so its name or tracked
vars do not prove local execution. A committed sentinel remains publishable
key material.

### Add `LOCAL_DEV_MODE=true`

Rejected. A non-secret mode flag can be copied or deployed and turns the
fallback back into an operator-controlled bypass. An ignored local secret is
equally convenient and safer.

### Continue accepting `HMAC_PEPPER` as a plain var

Rejected. It permits tracked configuration or visible dashboard variables to
hold key material and weakens the required-secret deployment gate.

### Infer environment from `LOG_LEVEL` or `BUILD_VERSION`

Rejected. Those are presentation/observability conventions, not trustworthy
execution provenance, and can be missing or copied.

### Panic or `expect` when the secret is missing

Rejected. A trap produces generic runtime failure without controlled status,
readiness, mutation ordering, or bounded diagnostics.

### Rely only on the launch checklist

Rejected. The current vulnerability exists precisely because documentation
cannot enforce runtime configuration.

### Rely only on Wrangler required-secret validation

Rejected. Alternate deployment paths, later secret deletion, and configuration
drift still require a runtime security boundary.

## Local Implementation Evidence

The authorized local patch now contains the secret-only opaque resolver,
validated route preflight and readiness behavior, four required-secret scopes,
safe developer secret-file setup, fresh-only hosted bootstrap with a distinct
destructive-rotation contract, and a shared isolated Worker harness for every
affected executable smoke/audit gate.

The complete local command set required by the reviewed implementation handoff
was observed passing on 2026-07-21: Rust formatting/tests/Clippy/wasm check,
release Worker build, local secret/bootstrap/configuration behavioral tests,
all listed browser/domain smokes, the compiled Class A audit-failure proof,
the documentation build, and `git diff --check`. Two initially parallelized
browser runs collided on shared legacy ports and passed when rerun
sequentially; only the uncontested reruns are treated as gate evidence.

The local implementation and correction reviews were accepted on 2026-07-22.
The local evidence does not by itself establish hosted behavior; the separately
reviewed evidence below supplies that boundary.

## Hosted Evidence and B2 Closure

The owner-authorized disposable Cloudflare run against exact candidate
`901855bfa584a373a66bc240a54fba9c78eefc84` observed:

- ordinary deployment rejected specifically because required
  `HMAC_PEPPER` was absent, with strict non-publication inventory;
- exact-artifact runtime-negative readiness and fixed dynamic `503` behavior,
  no redirect or cookie, and bounded D1/KV non-mutation;
- valid-secret ready health plus invite generation, calendar-token generation,
  join/authenticated session, authenticated form-token, help-signin, and
  relink flows;
- permitted secret deletion followed by the same fail-closed matrix; and
- strict deletion and zero final disposable D1, KV, and Worker inventory.

The provenance chain binds clean tracked `HEAD`, an in-run release build,
JS/Wasm SHA-256 manifest, isolated matching snapshot, Wrangler version IDs,
and matching remote version annotations. The corrected schema-2 evidence and
architecture acceptance are retained in ignored project records:

- `.git-exclude/evidence/rfc077/hosted-evidence-corrected-rerun.md`;
- `.git-exclude/tmp/rfc077-hosted-evidence-901855b.json`; and
- `.git-exclude/reviewed/zinnias-ciao-main-2026-07-22-rfc-077-hosted-evidence-correction-rereview.md`.

Acceptance criteria 8–9 are satisfied and architecture finding B2 is closed.
This closure does not authorize production deployment, public traffic or
pilot activity, release, or closure of unrelated RFC-050, B1, B3, B5,
real-device, performance, or persistent-observability gates.

## Review Questions

1. Should RFC-077 require 32 bytes minimum while accepting both hex and other
   strong textual encodings, or require the current 64-character lowercase-hex
   operational format exactly?
2. Is allowing static assets, `/offline`, and `/version` during configuration
   failure preferable to making the entire Worker return `503`?
3. Is central dynamic-route preflight sufficient to prevent widespread
   `require_auth` error collapsing, provided individual callers also propagate
   the fallible accessor?
4. Is the bootstrap-before-explicit-deploy sequence acceptable given that
   `wrangler secret put` itself creates and deploys a version?
5. Is the isolated required-secret bypass appropriate and sufficiently bounded
   for proving the runtime guard on hosted Cloudflare infrastructure?
6. Should missing-configuration diagnostics be sampled to avoid health-probe
   log volume while retaining a clear operator signal?
7. Is a dedicated ignored Wrangler configuration acceptable for rare local
   RFC-069 recovery testing, given that `secrets.required` restricts normal
   local secret-file loading to required keys?

## Current Platform References

- [Cloudflare Workers secrets](https://developers.cloudflare.com/workers/configuration/secrets/)
  — local `.dev.vars.<environment>` files and deployed required-secret validation.
- [Wrangler configuration](https://developers.cloudflare.com/workers/wrangler/configuration/#secrets-configuration-property)
  — `secrets.required` schema and per-environment declarations.
- [Cloudflare Workers local development](https://developers.cloudflare.com/workers/local-development/)
  — local execution and binding behavior.
- [Cloudflare Workers best practices](https://developers.cloudflare.com/workers/best-practices/workers-best-practices/)
  — store secrets with Wrangler rather than source or tracked vars.
