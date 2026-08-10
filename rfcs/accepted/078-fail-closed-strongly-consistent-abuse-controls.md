# RFC 078 — Fail-Closed Strongly Consistent Abuse Controls

**Status.** Accepted — architecture-approved and owner-accepted 2026-07-23;
**implemented and committed at `c991b82` on 2026-07-28**, including the required
I-B1 concurrency-burst evidence. RFC-050 exact-candidate hosted evidence remains
pending, so **B3 and all staging/public/production holds remain open** and this
RFC stays in `accepted/` rather than moving to `done/`.

*Corrected 2026-08-09.* This line previously read "implementation is authorized
but has not started," which was false from 2026-07-28 onward. `rfcs/README.md`
recorded the implementation correctly throughout; only this header was stale.
Found during the Handoff 044 dead-code audit, which traced live enforcement on
three request paths (`join.rs`, `relink.rs`, `community_create.rs`) through the
`ABUSE_LIMITER` Durable Object binding — a security RFC reading "not started"
while its controls were live is the kind of record that misleads exactly when it
matters.  
**Priority.** Architect-review remediation; blocks controlled hosted staging unless
risk-accepted and blocks any public or production pilot  
**Source finding.** 2026-07-14 architecture preparation review B3  
**Tracks.** RFC-002, RFC-003, RFC-012, RFC-024, RFC-041, RFC-045, RFC-050,
RFC-057, RFC-063, RFC-071, RFC-077  
**Depends on.** Implemented RFC-077 fail-closed pepper resolution at `901855b`

**Touches.** `workers/ssr/src/rate_limit.rs`, join/relink/community-creation
handlers, a new Rust Durable Object class, Worker exports/entrypoint,
`wrangler.toml` bindings and class-lifecycle declaration,
deployment/teardown documentation, contracts and hosted concurrency evidence

## Summary

Replace the fail-open, read-then-write KV counters with fail-closed,
strongly-consistent, per-subject abuse-control coordinators.

The current rate limiter:

- treats a missing `RATE_LIMIT` binding as “allowed”;
- treats KV read, parse, write, and delete failures as success/no-op;
- reads a counter and later writes `current + 1`, losing concurrent increments;
- relies on Workers KV for a write-heavy atomic-counter workload that KV does
  not guarantee;
- accepts spoofable `X-Forwarded-For` fallback input when
  `CF-Connecting-IP` is absent; and
- does not distinguish direct edge ingress from Worker subrequests before
  trusting `CF-Connecting-IP`, or detect Pseudo IPv4 Overwrite mode before
  claiming native IPv6 `/64` grouping.

The affected surfaces are anonymous invite redemption, anonymous relink-code
redemption, and authenticated community creation.

RFC-078 uses a SQLite-backed Durable Object named `AbuseLimiter`, sharded by a
domain-separated HMAC of the canonical subject. Each object serializes one
counter window. A protected request reserves capacity atomically before the
credential lookup or state mutation. Missing bindings, storage failures,
malformed coordinator responses, and overload fail closed.

No runtime fail-open mode is provided. A transient abuse-control outage makes
the protected operation temporarily unavailable; it does not make low-entropy
credentials publicly guessable.

This is a remediation design, not implementation approval. It may move to
`rfcs/accepted/` only after architecture review and explicit owner acceptance.

## Problem and Security Invariant

Invite and relink codes use six characters from a 32-symbol alphabet—about 30
bits before expiry and one-time-use constraints. Rate limiting is therefore a
required part of the credential design, not an optional performance feature.

Workers KV is eventually consistent. Concurrent writes to the same key can
overwrite each other, and the current implementation performs an unguarded
read-modify-write. An attacker can submit requests concurrently while the
counter remains below its intended value. Binding or KV failure removes the
control completely.

The invariant introduced by this RFC is:

> No protected credential lookup or protected state mutation may begin unless
> the request has received an affirmative, strongly-consistent capacity
> reservation for every required abuse-control dimension. Any inability to
> resolve or update that control denies the operation safely.

For anonymous credential endpoints, “protected lookup” includes the D1 HMAC
lookup itself. A rate-limiter error must occur before the application reveals
whether a submitted value could match a credential. The purpose-bound anonymous
form token must be authenticated and atomically consumed before the request may
mutate limiter state. Form-token D1 access is request authentication, not a
credential lookup; token replay, invalidity, or storage failure performs no
reservation and no credential lookup.

## Goals

- Enforce bounded attempt windows under concurrent requests.
- Fail closed when the binding, coordinator, storage, or trusted client
  identity is unavailable.
- Keep generic credential errors and no-JavaScript form behavior.
- Preserve the existing invite and relink limit intent: ten attempts per five
  minutes per client network subject.
- Preserve the community-creation quota intent: three attempts per 24 hours by
  authenticated user, session, and client network subject.
- Avoid raw IP addresses, session IDs, user IDs, and submitted codes in Durable
  Object names, storage, logs, metrics, or retained evidence.
- Make denied and unavailable outcomes observable without creating a credential
  oracle.
- Prove exact hosted concurrency behavior and fail-closed binding behavior.

## Non-Goals

- No WAF, Bot Management, Turnstile, CAPTCHA, proof-of-work, or device
  fingerprinting requirement.
- No longer invite/relink code format in this slice; that remains valid
  defense-in-depth but does not repair fail-open controls.
- No global accounting, billing quota, or general-purpose distributed lock.
- No analytics dashboard before the existing Logpush/observability work is
  available.
- No B1 invite response, B2 pepper, B4 hosted evidence, or B5 audit
  durability/redaction implementation, except for explicit dependencies and
  evidence coordination.
- No runtime owner override that converts coordinator failure into allow.
- No migration of unrelated KV use if another feature later adds it.

## Protected Surface Contract

| Surface | Subject dimensions | Window | Reservation point | Exhausted | Control unavailable |
|---|---|---:|---|---|---|
| `POST /join` invite submission | canonical client network subject | 10 / 300 s | After successful purpose-token consumption; before credential HMAC/D1 lookup | Generic `429` retry-later response with a replacement form token | Generic `503`; no credential lookup, with a replacement form token when rendering remains available |
| `POST /relink` code submission | canonical client network subject | 10 / 300 s | After successful purpose-token consumption; before credential HMAC/D1 lookup | Generic `429` retry-later response with a replacement form token | Generic `503`; no credential lookup, with a replacement form token when rendering remains available |
| `POST /communities/new` | authenticated user, session, and canonical client network subject | 3 / 86,400 s for each | After auth/authorization, field validation, and form-token consumption; before community/audit mutation | Generic `429`; no community created | Generic `503`; no community created |

GET form rendering is not counted. Static assets and unrelated authenticated
routes are not gated by RFC-078.

The first ten authenticated form submissions in a live window are permitted to
reach credential validation; the eleventh and later submissions are denied
until expiry. Tokenless, invalid-token, and replay submissions do not consume
capacity. This settles the current checklist ambiguity around “attempt 11/12.”

## Decision

### SQLite-backed Durable Object

Add one same-Worker Durable Object class and binding:

```text
class:   AbuseLimiter
binding: ABUSE_LIMITER
backend: SQLite-backed Durable Object storage
```

The project currently uses worker-rs 0.8.x, whose stable Durable Object surface
uses an internal `fetch` contract. The implementation may use that private
binding request/response protocol rather than inventing unsupported typed Rust
RPC. The Durable Object is not exposed as a public application route.

Wrangler configuration uses Cloudflare's current declarative Durable Object
class lifecycle plus an explicit binding:

```toml
[exports.AbuseLimiter]
type = "durable-object"
storage = "sqlite"

[[durable_objects.bindings]]
name = "ABUSE_LIMITER"
class_name = "AbuseLimiter"
```

`durable_objects` is non-inheritable, so the binding must also be declared in
`env.dev`, `env.staging`, and `env.production`. The project deliberately keeps
one top-level `exports` declaration, which named environments inherit. Each
environment still provisions and owns a distinct namespace. Cloudflare permits
an environment-specific `exports` override, but RFC-078 does not need one and
must not introduce divergent class lifecycle truth. `exports` is mutually
exclusive with the legacy `migrations` array; this Worker has no existing
Durable Object migration history, so RFC-078 starts with `exports` as the
lifecycle source of truth.

The currently locked Wrangler 4.106.0 schema predates `exports`. Implementation
must first update and lock Wrangler to a version whose bundled schema accepts
the declaration, then validate the exact TOML locally and with a deploy dry-run.
It must not silently fall back to `new_sqlite_classes` merely to avoid that
tooling update.

Cloudflare requires new Durable Object namespaces to use SQLite storage. This
design does so explicitly and does not use the legacy KV-backed Durable Object
backend.

### Sharding and subject derivation

Never route all traffic through one global object.

Create one object per policy scope and privacy-preserving subject digest:

```text
v1:invite:<digest>
v1:relink:<digest>
v1:community-user:<digest>
v1:community-session:<digest>
v1:community-network:<digest>
```

`digest` is a domain-separated HMAC using RFC-077's validated pepper:

```text
HMAC-SHA256(HMAC_PEPPER, "abuse-control:v1:" + scope + ":" + subject)
```

This design depends on RFC-077 so the HMAC key cannot silently fall back to a
public value. Pepper rotation intentionally starts fresh limiter identities;
rotation already invalidates the protected credentials and sessions, so
preserving old counter windows has no security value.

The raw subject exists only long enough to canonicalize and derive the digest.
It is never placed in the Durable Object name, object storage, structured logs,
audit rows, or evidence.

### Direct-edge ingress and trusted client network subject

Hosted requests use only `CF-Connecting-IP`, which Cloudflare supplies as a
single client address for normal incoming edge requests. Remove the
`X-Forwarded-For` fallback from the production path because clients and
upstream proxies can influence that chain.

That header is trusted only when the SSR Worker is the direct public ingress for
the protected route and the target does not use Pseudo IPv4 Overwrite Headers.
Cloudflare documents that a same-zone Worker subrequest can derive
`CF-Connecting-IP` from caller-controlled `x-real-ip`, while a cross-zone Worker
subrequest receives a fixed Worker client address. Cloudflare adds `CF-Worker`
to Worker `fetch()` subrequests.

Cloudflare Pseudo IPv4 has Off, Add Header, and Overwrite Headers modes. In
Overwrite mode, a native IPv6 visitor's `CF-Connecting-IP` becomes a Class E
pseudo IPv4 derived from the full IPv6 address and the native value moves to
`CF-Connecting-IPv6`. That shape would let privacy-address rotation evade this
RFC's `/64` grouping. RFC-078 therefore supports only Off or Add Header and does
not trust `CF-Connecting-IPv6` as a second identity input.

Before form-token, session, limiter, or application D1 work, every RFC-078
protected POST must:

1. reject any request containing `CF-Worker` as control unavailable (`503`);
2. reject any request containing `CF-Connecting-IPv6` as an unsupported
   overwrite/drift shape (`503`);
3. require exactly one `CF-Connecting-IP` and reject a Class E
   `240.0.0.0/4` value (`503`); and
4. canonicalize the accepted address and proceed only when all checks establish
   the supported direct-edge shape.

A client-supplied `CF-Worker` value may deny that client's own request but can
never make it pass; fail-closed rejection is safe. No upstream Worker, service
binding, proxy Worker route, or Worker-to-Worker forwarding may front these
routes. Adding one later requires a separately authenticated client-identity
contract and architecture review.

Parsing rules:

- require one syntactically valid IPv4 or IPv6 address;
- canonicalize IPv4 as the full address;
- canonicalize IPv4-mapped IPv6 consistently as IPv4;
- group native IPv6 by `/64` to limit trivial privacy-address rotation;
- reject Class E IPv4 `240.0.0.0/4`, which Cloudflare reserves for Pseudo IPv4;
- reject any `CF-Connecting-IPv6` value rather than attempting a two-header
  trust or matching protocol;
- ignore `CF-Pseudo-IPv4` in supported Add Header mode; it is never parsed,
  logged, persisted, or used as a limiter subject;
- reject missing, multiple, malformed, or unsupported values as control
  unavailable;
- reject any `CF-Worker` header, including same-zone and cross-zone subrequest
  shapes, before trusting the address;
- never use a shared literal such as `unknown`, which would let one request
  exhaust every unidentified user.

Local `wrangler dev` must demonstrate what `CF-Connecting-IP` value Miniflare
provides. Focused pure tests may pass explicit canonical addresses. If local
browser development lacks the platform header, a dev-only request harness may
inject it at the Wrangler boundary; runtime code must not regain an
`X-Forwarded-For` trust path or a deployable bypass flag.

Hosted evidence must inventory routes, custom domains, service bindings, and
upstream Workers for the exact candidate. The current direct `workers.dev`
isolated-staging topology is acceptable only after runtime header evidence shows
native IPv6 remains in `CF-Connecting-IP` and neither Class E replacement nor
`CF-Connecting-IPv6` is present. For every custom-domain target, the operator
must query and record the zone's `pseudo_ipv4` setting as `off` or `add_header`
before deployment and repeat the runtime header-shape check; a default is not
inferred. `overwrite_header`, an unreadable setting, or unexpected runtime
shape is No-Go.

A disposable upstream Worker negative test must show that the resulting
`CF-Worker` request receives `503` before token consumption, limiter mutation,
or application D1 access, followed by predefined teardown of that disposable
Worker. A controlled IPv6 harness with two source addresses in the same owned
`/64` must prove they share one counter on the exact target configuration.
Evidence records only outcomes and setting classification, never the source
addresses or prefix.

IP/network limiting can affect legitimate users behind mobile carrier NAT,
enterprise NAT, or privacy proxies. Before identity exists, however, the
project has no stronger stable subject for broad code guessing. The limit is
short, error copy is plain, successful validation resets the credential
counter, and admins can retry from another trusted network if necessary.

### Coordinator state and atomic reservation

Each Durable Object stores one bounded row, conceptually:

```sql
CREATE TABLE limiter_state (
    singleton          INTEGER PRIMARY KEY CHECK (singleton = 1),
    policy             TEXT NOT NULL,
    window_started_ms  INTEGER NOT NULL,
    count              INTEGER NOT NULL CHECK (count >= 0)
);
```

The object owns the policy constants. The caller requests an operation and
policy identifier; it does not supply arbitrary limits, windows, counts, or
timestamps. On first use, the stored policy is fixed. For an existing row, the
object checks logical expiry first: an expired row is replaced using the current
policy even if alarm cleanup was delayed; an unexpired row with a different
policy is a coordinator error and fails closed.

`reserve` runs as one serialized SQLite operation:

1. read the current Worker/DO time;
2. if no state exists or the window expired, begin a new window with count 1;
3. otherwise increment count atomically, saturating at `limit + 1`;
4. return `Allowed` when the new count is within the policy limit;
5. return `Blocked` with bounded remaining-window seconds otherwise.

The state write occurs for the first blocked request, so concurrent requests
cannot all observe an old count and pass. Later blocked requests retain the
saturated value; attacker traffic cannot overflow the stored counter. No
external I/O occurs inside the state transition.

The private response is a small fixed schema such as:

```text
Allowed { retry_after_seconds: 0 }
Blocked { retry_after_seconds: 1..window }
```

Unexpected status, content type, body, enum, or bounds are control failure and
therefore `503`, never “allowed.”

The object should schedule its one alarm for the end of the current window.
The alarm deletes limiter state. Alarm cleanup is a storage-retention control,
not part of correctness: a later reservation always checks the timestamp and
starts a new window even if an alarm was delayed or exhausted its retries.

### Authenticate the form, then reserve before credential lookup

`POST /join` and `POST /relink` use this strict order:

1. validate direct-edge ingress and parse a bounded request body;
2. authenticate and atomically consume the route's issued purpose-bound form
   token;
3. call `reserve` for the canonical client network subject; and
4. only on `Allowed`, normalize and validate the submitted credential against
   HMAC/D1 state.

Only `ConsumeResult::Proceed` reaches reservation. Missing, invalid, expired,
wrong-purpose, wrong-binding, and replayed tokens return the existing generic
form error without reserving capacity or performing credential lookup. This
prevents a cross-site tokenless form from spending the victim network's quota.

This intentionally counts every authenticated form submission that is eligible
to reach credential validation, not only invalid credentials. A check-then-
lookup-then-increment design still allows a concurrent burst to pass before
failures are recorded. Counting authenticated submissions is the concurrency-
safe control.

Because the token is single-use, a `Blocked` or `Unavailable` result after token
consumption must render or redirect to a form containing a newly issued token.
The consumed token is never restored or accepted on replay. If replacement-token
issuance itself fails, return generic `503`; do not reserve again and do not
perform credential lookup. Same-origin no-JavaScript submission and retry must
remain fully supported.

After a credential is proven valid:

- request a `reset` for that scope/subject;
- if reset succeeds, continue normally;
- if reset fails, record bounded degraded telemetry and continue the already
  authorized request because the earlier reservation succeeded and leaving a
  counter in place is fail-safe, not fail-open;
- never delay credential consumption/session issuance on repeated reset
  retries.

An invalid, expired, used, revoked, race-lost, or malformed credential does not
perform a second increment because capacity was already reserved.

Invite and relink scopes remain separate so ordinary help-signin recovery is
not blocked by unrelated invite-entry mistakes. A future combined anonymous
credential cap may be added only with usability evidence.

### Community-creation reservations

Community creation is authenticated and has a different threat model.

Perform reusable field validation first so normal correction does not consume
quota. Then consume the purpose-bound form token. Before any community,
membership, or audit write, reserve all three dimensions:

1. authenticated user;
2. current session;
3. canonical client network subject.

If any reservation is blocked or fails, perform no D1 mutation. Earlier
successful reservations in that sequence may remain charged. This conservative
partial charge is acceptable for a security quota and avoids a distributed
rollback protocol across three coordinators. The UI can issue a fresh form
token after an infrastructure failure.

Reservations are not reset after success: they count creation attempts allowed
to reach mutation. If D1 later fails, the slot remains consumed. This is
fail-safe and bounded to three per day; operator support can disable the feature
flag while diagnosing repeated failures.

### Failure outcomes

The limiter API must return a typed result such as:

```text
Allowed
Blocked { retry_after_seconds }
Unavailable { category }
```

Handlers may proceed only on `Allowed`.

`Blocked` returns `429 Too Many Requests` with:

- fixed Japanese retry-later copy that does not mention counters, IPs, code
  validity, Durable Objects, or internal state;
- `Cache-Control: no-store`;
- a bounded integer `Retry-After` no greater than the configured window;
- the normal request ID and security headers;
- no submitted code or subject identifier in URL/headers.

`Unavailable` returns `503 Service Unavailable` with similarly generic copy,
no D1/KV access for the protected operation, no credential/session cookie, and
no form success. It covers at least:

- missing `ABUSE_LIMITER` binding;
- object-name/HMAC derivation failure;
- Durable Object stub/fetch failure or overload;
- SQLite/storage failure;
- timeout if a bounded timeout is implementable in worker-rs without unsafe
  promise races;
- malformed private coordinator response;
- missing/malformed trusted client network address;
- a `CF-Worker` subrequest shape;
- any `CF-Connecting-IPv6` header; or
- a Class E `CF-Connecting-IP` indicating Pseudo IPv4 overwrite or unsupported
  address input.

The response must not fall back to the normal invalid-code result because
operators and tests must distinguish control outage from an attacker guessing
incorrectly. The user copy may remain generic while status and bounded logs
preserve that distinction.

A caller-side timeout can race with a coordinator that completed its
reservation. The request must fail closed, and a retry may consume another
slot. This conservative charge is bounded, recorded only as privacy-safe
unavailable telemetry, and covered by integration tests; the caller must not
attempt an unsafe compensating decrement.

## Binding Readiness and Configuration

RFC-077's readiness work should be extended so `/healthz` resolves the
`ABUSE_LIMITER` binding in hosted environments. Binding presence alone does not
prove storage health, so health must not overstate a full transaction probe.

Pseudo IPv4 is a zone setting rather than a Wrangler binding. Deployment
preflight for a custom-domain target must read it through the Cloudflare API,
accept only `off` or `add_header`, and stop without changing the setting when it
is `overwrite_header`, unreadable, or unexpected. The repository gate must pin
the runtime rejection of Class E and `CF-Connecting-IPv6`, plus the operator
inventory requirement. `workers.dev` has no project-owned custom-domain zone
setting to assert, so its gate is the exact runtime IPv6 header-shape evidence.

The release checklist and deployment docs must stop claiming that a KV
namespace makes abuse controls ready. `RATE_LIMIT` KV becomes unused after the
cutover:

- remove it from root/dev/staging/production Worker bindings;
- remove KV creation as a deployment prerequisite;
- retain existing remote namespaces temporarily during the recovery/evidence
  window but mark them unused;
- delete them only after exact-candidate evidence and explicit operator review;
- update staging teardown accordingly.

Removing the KV binding must not affect any non-abuse feature; a repository
search must prove `RATE_LIMIT` has no remaining runtime consumer.

## Observability and Alerting Contract

Emit structured, bounded events for:

```text
abuse_control.blocked
abuse_control.unavailable
abuse_control.reset_failed
```

Allowed fields:

- request ID;
- route class (`invite`, `relink`, `community_create`);
- outcome/error category;
- build/environment label;
- bounded retry-after bucket, if blocked.

Forbidden fields:

- raw or hashed submitted code;
- raw/HMAC client IP or network prefix;
- raw/HMAC user ID or session ID;
- Durable Object name/ID or subject digest;
- cookies, form tokens, HMAC pepper, or coordinator response body.

Any `abuse_control.unavailable` event in hosted staging or production is a
security-control incident. Until Logpush/alert delivery is proven under RFC-050,
the environment cannot claim unattended public-pilot readiness. The operator
must inspect Workers logs during controlled evidence windows.

Blocked events should be aggregated/sampled for alerting to avoid attacker-
driven log volume, while unavailable events should not be silently sampled
away. Exact sampling and alert thresholds may be set with the Logpush evidence
work, but lack of a dashboard does not permit fail-open behavior.

## Risk Acceptance Policy

There is no application configuration, environment variable, header, query
parameter, or owner token that makes coordinator failure return `Allowed`.

If an isolated short-lived staging exercise must proceed while the coordinator
is unavailable, the acceptable choices are:

- do not exercise invite/relink/community-creation routes;
- disable community creation with its existing feature flag;
- repair the binding/deployment;
- tear down and recreate the isolated environment.

Explicit owner risk acceptance may allow the isolated environment to exist for
unrelated read-only checks, but it does not authorize fail-open credential
validation. Public/production pilot remains No-Go.

### Dated owner risk acceptances

**2026-07-28 — IPv6 client support is not confirmed/implemented for this
deployment.** State this plainly in every document that discusses it: this
service must be treated as **IPv4-only in practice** until a hosted campaign
proves otherwise. No IPv6 client request has ever been observed or verified
against a real deployed candidate. Do not describe IPv6 as "supported,"
"working," or "implemented" in any tracked document, release checklist, or
evidence record — describe it as **not yet confirmed / not yet implemented in
a verified sense**, full stop, regardless of what the source code contains.

The owner explicitly risk-accepts deferring the controlled-external-IPv6-
harness hosted evidence in acceptance criterion 6 (two addresses in one owned
native IPv6 `/64` proven to share capacity) and the corresponding RFC-050
E4a bullet. No such harness will be pursued for the current campaign. This is
a decision about hosted *evidence acquisition and production readiness*, not
a promise about what the code does; the distinction matters because the code
facts below must not be read as "IPv6 is supported":

- native IPv6 addresses are grouped by `/64` in
  `abuse_control::canonicalize` (`zero_low_64_bits`) as designed, but this has
  only ever run against synthetic native-test inputs, never a real client;
- any `CF-Connecting-IPv6` header is rejected outright as an unsupported
  Pseudo IPv4 overwrite/drift shape, independently of this deferral;
- the `/64` grouping logic is natively tested
  (`abuse_control::tests::canonicalizes_native_ipv6_to_first_64_bits`,
  `same_64_prefix_yields_the_same_canonical_subject`) — native unit tests are
  evidence that the *arithmetic* is correct, not that a real IPv6 client is
  handled correctly end to end on hosted Cloudflare infrastructure.

Until hosted proof exists (or this deferral is revisited), treat all of
criterion 6's IPv6 handling as **risk-accepted-open and operationally
unconfirmed**, not merely "the hosted sharing proof is missing." Do not
represent any part of IPv6 handling as hosted-proven, working, or supported
in any release checklist, roadmap entry, or evidence record. Revisit this
decision if the deployment's actual client population makes native IPv6
material enough to warrant the harness.

## Data Protection and Retention

- Durable Object state contains only policy, timestamp, and count.
- Subject digests are used only for object routing and are not copied into
  object storage.
- No D1 schema migration is required.
- Expired rows are cleared by alarms and logically ignored even if physical
  cleanup is delayed.
- Evidence records only aggregate outcomes and redacted request identifiers.
- Operator tools must not enumerate or export limiter object state as routine
  analytics.

## Implementation Slices

Implementation should remain one reviewable B3 patch after RFC-077 is complete:

1. Add pure policy, subject canonicalization, and domain-separated digest
   helpers with native tests.
2. Add/export the `AbuseLimiter` Rust Durable Object, SQLite state, private
   protocol, alarm cleanup, and focused tests.
3. Update and lock Wrangler for `exports`, then add the top-level SQLite class
   lifecycle declaration and the binding for root and every named environment.
4. Add direct-edge ingress validation and replace join/relink
   check/record/clear calls with token-then-reserve/reset flows.
5. Replace community-creation KV quota calls with three fail-closed
   reservations.
6. Remove all runtime `RATE_LIMIT` KV reads/writes and fail-open branches.
7. Add fixed `429`/`503` rendering and privacy-safe structured events.
8. Add focused RFC-078 unit/integration/contract tests in dedicated test
   modules rather than extending the monolithic
   `packages/contracts/tests/release_gates.rs` file.
9. Strengthen the standard CI path to run native SSR tests and
   workspace/all-target Clippy with warnings denied; align the repository
   `lint` script with that boundary and retain the SSR WASM type-check.
10. Update threat model, architecture map, deployment/teardown docs, runbook,
   RFC-045/RFC-050 evidence expectations, and release checklist.
11. Capture isolated hosted binding-negative, concurrency, expiry, reset, and
   valid-flow evidence.

No developer handoff is required at Proposed status. If the RFC is Accepted
and implementation is delegated, a companion handoff should pin the worker-rs
Durable Object protocol, Wrangler version/configuration boundary, and exact test
locations while keeping B1/B5 changes out unless separately accepted. It must
also identify the CI commands and dedicated RFC-078 test modules required by
slices 8–9.

## Test and Release Evidence

### Required pure/native tests

- IPv4, IPv4-mapped IPv6, native IPv6 `/64`, malformed, multiple, and missing
  client-address cases.
- Class E IPv4, present/empty/malformed/valid `CF-Connecting-IPv6`, and
  Off/Add-Header/Overwrite-shaped header combinations; only the supported
  Off/Add Header shapes may reach canonicalization.
- Domain separation produces different subject digests across invite, relink,
  user, session, and network scopes.
- No raw subject appears in object names returned by helper tests.
- Fixed-window boundary: attempts 1–10 allowed, attempt 11 blocked, expiry
  starts a new window at count 1.
- Community policy: attempts 1–3 allowed, attempt 4 blocked.
- Retry-after values remain within `1..=window`.
- A sustained blocked burst leaves the stored count saturated at `limit + 1`
  rather than overflowing or extending the fixed window.
- Unknown policy and malformed coordinator result fail closed.

### Required local Worker/Durable Object tests

- Concurrent reservation burst allows exactly the configured maximum and
  blocks the remainder; no lost increments.
- Separate subject digests and scopes do not share state.
- Successful credential reset starts a fresh window.
- Alarm cleanup removes stored state; delayed/missing alarm does not break
  logical expiry.
- Missing binding, object failure, storage failure simulation, overload-like
  error, and malformed private response produce `503` before protected D1 work.
- Tokenless, invalid, expired, wrong-purpose, and replayed anonymous form tokens
  perform no reservation and no credential lookup.
- Join/relink blocked and unavailable cases occur after one successful token
  consumption, perform no credential lookup, and provide a fresh token for a
  same-origin no-JavaScript retry.
- A cross-origin tokenless form POST neither consumes limiter capacity nor
  reaches credential lookup; the equivalent same-origin no-JavaScript form does.
- Any `CF-Worker` request shape returns `503` before form-token consumption,
  limiter mutation, or application D1 access.
- Class E `CF-Connecting-IP` and every `CF-Connecting-IPv6` shape return `503`
  before form-token consumption, limiter mutation, or application D1 access.
- Community creation performs no mutation when any required dimension blocks
  or fails.
- Community reservations remain ordered user, session, network; injected block
  or failure at each position leaves earlier charges in place, skips later
  reservations, and performs no community/audit mutation.
- Source gates reject `return false`/no-op error fallbacks, KV read-modify-write,
  `X-Forwarded-For`/`CF-Connecting-IPv6` trust, acceptance of Class E client
  subjects, raw subject logging, and a global singleton object name.
- Existing valid invite/relink, one-winner redemption, generic invalid-code,
  form-token, authorization, and community idempotency tests remain green.
- An unexpired policy mismatch fails closed; an expired mismatched row starts a
  new window under the current policy.
- A simulated caller timeout after a completed reservation may retain one charge
  and never performs a compensating decrement.

Native `cargo test` alone cannot prove Durable Object serialization/storage.
The implementation must add a workerd/Wrangler-compatible integration path
appropriate for the Rust Worker rather than claiming source-string assertions
prove concurrency.

### Required local runtime evidence

- `wrangler dev --env dev --local` loads the declaratively exported SQLite
  Durable Object class and exercises real binding calls.
- Eleven sequential and a larger concurrent burst match the exact policy.
- Removing/misnaming the local binding yields `503`, not normal credential
  validation.
- Browser/no-JS forms display fixed retry/unavailable copy without leaking
  internals.
- Browser evidence proves cross-origin tokenless submissions do not spend
  capacity and same-origin no-JavaScript `429`/`503` retries receive fresh form
  tokens.
- Local header-matrix evidence proves Pseudo IPv4 overwrite shapes fail closed;
  local simulation does not substitute for target setting inventory or hosted
  IPv6 `/64` evidence.

### Required hosted evidence

Against the exact isolated staging candidate:

- verify the SQLite Durable Object class/binding exists in the intended named
  environment;
- capture the `exports` reconciliation output for each first environment
  deployment and confirm each environment owns a distinct namespace;
- inventory the exact candidate's routes, custom domains, service bindings, and
  upstream Workers to prove the SSR Worker is direct public ingress;
- for `workers.dev`, capture bounded runtime evidence that an IPv6 request keeps
  native IPv6 in `CF-Connecting-IP` with no `CF-Connecting-IPv6`; for every
  custom-domain target, query `pseudo_ipv4`, require `off` or `add_header`, and
  repeat the runtime shape check;
- from a controlled external IPv6 harness with two addresses in one owned `/64`,
  prove both addresses share the exact anonymous capacity without retaining the
  addresses or prefix;
- route one request through a disposable upstream Worker, observe `503` with no
  token/limiter/application-D1 mutation, then execute and verify its predefined
  teardown;
- send sequential and concurrent invite/relink submissions from a controlled
  client and observe exactly the approved capacity;
- prove blocked requests do not query/consume credentials or mutate D1;
- prove valid redemption resets its own credential window without affecting a
  different subject/scope;
- remove or deliberately misconfigure the binding in an isolated negative-test
  config and observe `503` plus zero protected mutations;
- verify community creation stops at all three dimensions and remains disabled
  or unavailable when the coordinator fails;
- inspect structured blocked/unavailable events without retaining subject or
  credential material;
- confirm old `RATE_LIMIT` KV is not read by the candidate;
- tear down negative-test resources and record bounded evidence under RFC-050.

Evidence must not contain submitted invite/relink values, session cookies, raw
or hashed IP/network subjects, Durable Object IDs/names, form tokens, or the
pepper.

## Acceptance Criteria

RFC-078 implementation is complete only when:

1. No protected endpoint treats a missing binding or coordinator/storage error
   as allowed.
2. Anonymous credential capacity is reserved atomically only after successful
   purpose-token consumption and before credential lookup, with exactly ten
   authenticated submissions per five-minute subject/scope window; tokenless,
   invalid-token, and replay requests spend no capacity.
3. Community creation reserves user/session/network capacity before mutation,
   with exactly three attempts per 24-hour dimension.
4. Concurrent requests cannot lose increments or exceed the configured
   capacity except for explicitly tested platform-level failure.
5. `429` and `503` outcomes are distinct, generic, non-cacheable, and leak no
   credential or subject material.
6. Only canonical `CF-Connecting-IP` from a verified direct-edge Off/Add-Header
   Pseudo IPv4 configuration is trusted in hosted runtime. Missing/malformed
   identity, any `CF-Worker`, any `CF-Connecting-IPv6`, or Class E
   `CF-Connecting-IP` fails closed before token, limiter, or application D1
   work; exact-target evidence proves two addresses in one native IPv6 `/64`
   share capacity. **IPv6 client support is not confirmed/implemented for
   this deployment as of the 2026-07-28 owner risk acceptance — see § Dated
   owner risk acceptances. Treat this criterion's IPv6 sub-clause as
   risk-accepted-open and operationally unconfirmed, never as hosted-proven
   or supported, regardless of the native `/64`-grouping test coverage.
   Every other part of this criterion is unaffected.**
7. The Durable Object is sharded per HMAC-derived subject/scope, uses SQLite,
   has correct root/named-environment bindings, and uses one inherited top-level
   declarative lifecycle entry whose per-environment reconciliation is proven.
8. `RATE_LIMIT` KV and every fail-open branch are absent from runtime code.
9. Privacy-safe blocked/unavailable telemetry is observable for hosted review.
10. CI runs the native SSR tests and workspace/all-target Clippy with warnings
    denied, retains the SSR WASM check, and the repository `lint` command
    enforces the same Clippy boundary. Focused RFC-078 contract gates live in
    dedicated test modules rather than enlarging the general release-gate file.
11. Local integration and exact-candidate hosted concurrency, cross-origin,
    direct-ingress, lifecycle, and negative evidence pass before B3 is closed
    for controlled staging or public/production pilot.

Architecture approval may accept this design and move it to `accepted/` before
hosted evidence exists. Moving implementation to `done/` establishes the local
source/config change; it does not by itself close hosted B3 evidence or lift
the roadmap remediation hold.

## Rollout and Forward Recovery

There is no D1 migration. The new Durable Object namespace is provisioned from
the inherited top-level Wrangler `exports` declaration with SQLite storage.
Each named environment provisions a distinct namespace. The first lifecycle
deployment is a forward-only boundary: Cloudflare does not permit version
rollback across an `exports` lifecycle change, and subsequent deployment must
not return to legacy `migrations` or omit the live export.

Rollout order:

1. confirm RFC-077 at the exact pepper dependency checkpoint;
2. update and lock Wrangler, then validate `exports` and bindings for root,
   dev, staging, and production configurations;
3. inventory the direct-ingress topology and Pseudo IPv4 behavior: verify the
   actual `workers.dev` header shape and require every custom-domain zone's
   `pseudo_ipv4` setting to be `off` or `add_header`;
4. build, review, and retain a reproducible forward-recovery artifact/config
   from the accepted candidate; it must keep `AbuseLimiter` exported and bound,
   preserve fail-closed handlers, and never reactivate KV counters;
5. deploy the new class/binding to isolated staging while retaining the old KV
   namespace unused, and capture the lifecycle reconciliation output;
6. verify class provisioning, direct-ingress/header topology, native IPv6 `/64`
   sharing, sequential/concurrent behavior, failure outcomes, and the
   forward-recovery dry-run;
7. run valid invite/relink/community smoke;
8. remove old KV provisioning requirements only after evidence;
9. deploy production only after final security review and all other blockers.

Counter state starts empty at cutover. That one planned window reset is
acceptable before a public pilot and must be noted in deployment evidence.

Rollback to a pre-lifecycle Worker version or to KV fail-open behavior is
prohibited and is not an executable recovery plan. Recovery is a new forward
deployment of the retained, reviewed artifact/config (or a reviewed successor)
that keeps the live `AbuseLimiter` export, binding, namespace, and fail-closed
handlers. Before hosted evidence, the operator must prove by dry-run that the
recovery config neither omits the live export nor uses legacy `migrations`, and
must record the artifact digest and exact command without secrets. If
coordination fails, protected operations continue returning `503` while that
forward deployment is prepared and applied.

Remote KV namespaces should not be deleted until the evidence/recovery window
ends, but they must never be reactivated as authoritative counters without a
new reviewed design.

## Alternatives Rejected

### Keep Workers KV and add error propagation

Rejected. Fail-closed errors would repair availability semantics but not lost
concurrent increments or cross-location eventual consistency. KV explicitly
does not provide atomic read-modify-write.

### Use the native Workers Rate Limiting binding alone

Rejected as the authoritative control. Cloudflare documents it as location-
local, permissive, eventually consistent, and intentionally inaccurate; its
supported periods are 10 or 60 seconds, not the existing five-minute and
24-hour policies. It may be reconsidered later as an outer volumetric shield,
but cannot discharge B3.

### Store counters in primary D1

Rejected for this slice. Atomic SQL could improve counter correctness, but
anonymous hostile traffic would write into the primary community data plane,
couple credential availability to D1 write contention, require cleanup/index
work, and make isolation from business data weaker. Durable Objects provide a
sharded coordination primitive designed for per-entity strong consistency.

### One global Durable Object

Rejected. It creates a global bottleneck and attack target. Per-subject objects
preserve serialization where needed without routing unrelated users through one
coordinator.

### Use only WAF rate-limiting rules

Rejected as the repository-owned contract. WAF configuration may be useful
defense-in-depth, but plan/account configuration is external, route-centric,
and cannot express the application success-reset and authenticated
user/session quota semantics alone.

### Add Turnstile instead of a counter

Rejected. It adds user friction and accessibility/availability dependencies and
does not replace bounded server-side credential attempts. A later layered bot
control requires its own UX/security review.

### Increase code length and remove rate limiting

Rejected. Longer codes reduce guessing probability but do not meet the existing
unconditional rate-limit requirement or protect infrastructure from automated
submission volume.

### Allow transient fail-open with owner acceptance

Rejected for runtime code. A deployable bypass is likely to survive its
incident and recreates the architect's blocker. Owner acceptance may narrow an
isolated staging test's route scope, not authorize unprotected credential
validation.

## Review Questions

1. Is a per-scope, HMAC-sharded SQLite Durable Object the appropriate
   coordination atom for these low-volume exact counters?
2. Should IPv6 use `/64`, a different prefix, or exact addresses given privacy
   rotation and shared-network false positives?
3. Is consuming a purpose-bound token before counting every authenticated
   anonymous submission, then resetting after valid credential proof, the
   correct CSRF-safe and concurrency-safe interpretation of the existing
   failed-attempt policy?
4. Should invite and relink share an additional combined per-network cap, or is
   scope separation the better recovery UX?
5. Is conservative partial charging across community user/session/network
   reservations acceptable without a distributed rollback protocol?
6. Are `429` for exhaustion and `503` for control failure sufficiently distinct
   while preserving generic credential validity messages?
7. Should the native Workers Rate Limiting binding be added as an optional
   coarse shield in the same implementation, or deferred to keep the blocking
   fix auditable?
8. Is the alarm cleanup plus logical timestamp expiry sufficient for limiter
   storage retention?
9. Is rejecting every `CF-Worker` request and requiring direct public ingress a
   sufficient enforceable boundary for trusting `CF-Connecting-IP`?
10. Is inherited top-level `exports` plus a retained, reviewed forward-recovery
    artifact the correct lifecycle contract for all environments?
11. Is requiring Pseudo IPv4 Off/Add Header, rejecting Class E and every
    `CF-Connecting-IPv6` shape, and proving native `/64` sharing on each exact
    target sufficient to close configuration drift and privacy-address bypass?

## Current Platform References

- [Workers KV consistency](https://developers.cloudflare.com/kv/concepts/how-kv-works/)
  — eventual consistency and lack of atomic read-modify-write.
- [Workers KV writes](https://developers.cloudflare.com/kv/api/write-key-value-pairs/)
  — concurrent writes can overwrite each other.
- [Workers Rate Limiting binding](https://developers.cloudflare.com/workers/runtime-apis/bindings/rate-limit/)
  — locality, supported periods, permissive accuracy, and monitoring limits.
- [Durable Objects](https://developers.cloudflare.com/durable-objects/)
  — globally addressable per-object coordination with strongly consistent
  storage.
- [Durable Object rules](https://developers.cloudflare.com/durable-objects/best-practices/rules-of-durable-objects/)
  — sharding around coordination atoms and avoiding a global singleton.
- [Durable Object class exports](https://developers.cloudflare.com/durable-objects/reference/durable-objects-migrations/)
  — declarative lifecycle, environment inheritance, SQLite storage selection,
  and forward-only boundary.
- [Workers rollbacks](https://developers.cloudflare.com/workers/versions-and-deployments/rollbacks/)
  — lifecycle changes prevent rollback to an earlier Worker version.
- [Wrangler environments](https://developers.cloudflare.com/workers/wrangler/environments/)
  — bindings are non-inheritable across named environments.
- [Cloudflare request headers](https://developers.cloudflare.com/fundamentals/reference/http-headers/)
  — `CF-Connecting-IP`, `CF-Worker`, and Worker-subrequest semantics.
- [Cloudflare Pseudo IPv4](https://developers.cloudflare.com/network/pseudo-ipv4/)
  — Off/Add Header/Overwrite behavior, Class E pseudo addresses, and
  `CF-Connecting-IPv6` preservation under overwrite.
