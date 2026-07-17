# RFC 076 — One-Time Invite Code Response Isolation

**Status.** Accepted — architecture-approved and owner-accepted 2026-07-17;
implementation must follow a reviewed handoff
**Priority.** Architect-review remediation; blocks controlled hosted staging unless
risk-accepted and blocks any public or production pilot  
**Source finding.** 2026-07-14 architecture preparation review B1  
**Tracks.** RFC-003, RFC-010, RFC-041, RFC-048, RFC-071, RFC-079
**Touches.** `workers/ssr/src/handlers/admin/members.rs`, shared response/render
helpers if needed, contracts release gates, SSR tests, release checklist, hosted
staging evidence

## Summary

Generated invite codes must be revealed only in the body of the authenticated
POST response that created them. They must never be copied into a redirect
target, query parameter, fragment, cookie, server-side flash record, analytics
event, audit metadata, or application log.

The current handler redirects to:

```text
/c/:community_id/admin/invites?code=:plaintext_code
```

That places a bearer credential in a request URL, browser history, and
potential access-log and referrer surfaces. This RFC replaces that handoff with
a direct, non-cacheable HTML response to the consumed POST.

This remediation design was architecture-approved and explicitly
owner-accepted on 2026-07-17. Implementation must follow a separately reviewed
handoff.

## Problem and Security Invariant

RFC-010 requires the plaintext code to be shown once and never stored or
logged. The current POST/Redirect/GET implementation preserves one-time UI
display but violates the stronger secrecy boundary by interpolating the code
into the redirect URL.

An invite code is a short-lived bearer credential. Possession is sufficient to
start the join flow, so URL exposure is credential exposure even when the
route is authenticated and same-origin.

The invariant introduced by this RFC is:

> From generation until deliberate copying by the admin, plaintext invite-code
> bytes may exist only in the generation handler's bounded in-memory value and
> the final authenticated HTML response body. The code must not cross any
> request-target, header, persistence, logging, audit, analytics, or referrer
> boundary.

The intended one-time browser display, deliberate admin copy/share action, and
the recipient's later manual entry at `/join` are allowed surfaces.

## Goals

- Remove generated plaintext invite codes from all application-created URLs.
- Preserve a simple no-JavaScript admin workflow.
- Preserve HMAC-only persistence and one-time display semantics.
- Make refresh, replay, double-click, and Back behavior safe and predictable.
- Add a durable release gate that prevents reintroduction of a plaintext-code
  query handoff.
- Define hosted evidence that demonstrates the request URL remains code-free.

## Non-Goals

- No change to invite alphabet, entropy, expiry, role, redemption, or
  revocation semantics.
- No email, SMS, QR-code, or public join-link delivery.
- No server-side plaintext flash store, KV handoff, encrypted invite archive,
  or recoverable display after leaving the response.
- No broad refactor of member management.
- No resolution of B2 pepper configuration, B3 abuse-control availability, or
  the remaining B5 hosted/persistent-incident evidence. RFC-079 already governs
  local audit atomicity and redaction; this RFC preserves that accepted
  contract.
- No claim that this RFC alone permits controlled staging or a public pilot.

## Decision

### Successful generation returns HTML directly

`POST /c/:community_id/admin/invites` keeps its existing authentication,
community-admin authorization, purpose-bound form-token consumption, random
generation, HMAC persistence, expiry, and safe audit identity.

Invite generation and the required `invite_code.generated` audit row remain
one RFC-079 Class A batch. Only after that batch succeeds does the handler
return the invite administration page directly:

```text
HTTP/1.1 200 OK
Cache-Control: no-store, private
Referrer-Policy: no-referrer
Content-Type: text/html; charset=utf-8
```

The response:

- has no `Location` header;
- contains the escaped plaintext code once in the intended reveal panel;
- contains no code in links, form actions, hidden inputs, data attributes,
  scripts, comments, headers, or structured metadata;
- is available without JavaScript;
- uses only same-origin static assets already used by normal authenticated
  pages;
- gives the admin clear Japanese copy equivalent to: “Copy this code now. It
  will not be shown again after you leave or reload this page.”

The code is selectable plain text. This remediation adds no copy-button script:
no application script reads or receives the reveal value, and the app does not
write it to the clipboard. An admin may use the browser or operating system's
normal text-selection and copy interaction.

### Rendering boundary

The implementation should extract a private invite-page renderer or page-data
builder used by both GET and POST:

```text
GET  -> render_invites_page(reveal = none)
POST -> generate + persist HMAC -> render_invites_page(reveal = plaintext)
```

The reveal value should use a narrow private type that does not implement
`Debug`, `Display`, serialization, URL encoding, or logging traits. It should
be borrowed or moved directly into escaped HTML construction and dropped after
the response is built. It must not be added to a generic flash-message type.

The shared renderer may query the active-invite list and issue a fresh generate
form token so the returned page remains usable. The active list continues to
show metadata only, never plaintext codes.

An authorization or guarded-insert `false` result reveals no code. Construction,
D1, or required-audit failure follows RFC-079: return the accepted generic
`503 Service Unavailable` response with normal security headers and disclose no
code, success redirect, or success page. A failed Class A batch persists
neither the invite row nor its audit row.

### Replay, refresh, and double submit

The generation form token remains single-use.

If the POST is replayed—through reload confirmation, double-click, browser
retry, or a copied request—the consumed-token branch must:

1. create no second invite;
2. disclose no previously generated plaintext;
3. return `303 See Other` to the canonical clean GET route;
4. include no secret or user-controlled text in `Location`.

This is an intentional exception to RFC-071's general “303 after successful
POST” baseline. A redirect cannot carry this one-time secret safely, while a
server-side plaintext handoff would add persistence and cleanup risks. The
first successful POST therefore returns `200`; replay returns `303`.

The one-time reveal may disappear on reload or navigation. That loss is safer
than making the code recoverable. The admin can revoke the unused invite by
its non-secret identifier/metadata and generate a replacement.

### GET and legacy query behavior

At handler entry, before authentication, authorization, D1/KV or other binding
access, page/form-token issuance, rendering, or any other application
subrequest, `GET /c/:community_id/admin/invites` must detect whether the query
contains a `code` parameter. An empty or repeated parameter also matches. When
it matches, the handler must immediately return:

```text
HTTP/1.1 303 See Other
Location: /c/:community_id/admin/invites
Referrer-Policy: no-referrer
```

`community_id` at this point is route-matched request input, not an already
authorized value. The handler must construct `Location` through the route's
bounded community-identifier grammar or an equivalent safe canonical-path
builder. Apart from that validated/encoded path segment, `Location` is fixed
and contains no query-derived data. The branch must not authenticate, authorize,
access application bindings, issue a page/form token, render an invite page,
or initiate an application subrequest. It must never reflect the query value
into HTML, headers, logs added by application code, or audit metadata.

Canonicalization is containment, not erasure: the incoming unsafe URL may
already have reached browser or platform records. The fixed redirect prevents
the application from retaining it as a rendered document or propagating it as
a same-origin referrer. The release guarantee also remains that current
application code never creates such a URL. Retained evidence must use synthetic
non-credential query values, not real or legacy invite codes.

Unrelated fixed-code flash behavior may remain if it does not accept or reflect
arbitrary text. It must not become an alternate invite-code channel.

## Threat Analysis

| Threat | Required control |
|---|---|
| Worker/access logs capture query strings | Generated code never enters a request URL. |
| Browser history stores a bearer URL | Successful POST URL is the canonical admin route without a query. |
| Same-origin referrer forwards a bearer URL | Reveal response explicitly sends `Referrer-Policy: no-referrer`; code is absent from the URL regardless. |
| Browser/intermediary caches reveal HTML | Reveal response explicitly sends `Cache-Control: no-store, private`. |
| Replay generates additional invites | Single-use generation token; replay redirects without mutation. |
| Generic flash/session store retains plaintext | No server-side handoff or plaintext persistence is introduced. |
| Required audit fails after invite mutation | RFC-079 Class A batch rolls back the invite and audit together; generic `503` reveals no code. |
| Audit or diagnostic metadata captures plaintext | Typed RFC-079 audit identifies the invite row only; metadata contains neither code nor HMAC. |
| HTML or attribute injection | Generated code is escaped at the render boundary and appears only as text. |
| JavaScript/third-party asset exfiltration | No third-party resources or copy-button enhancement; no script receives the reveal value. |
| Legacy query propagates through a same-origin referrer | Handler-entry detection returns a safe canonical-path `303` with `no-referrer` before authentication, authorization, bindings, rendering, or token issuance. |
| Reviewer mistakes source gates for hosted proof | Hosted request inspection remains a distinct acceptance item. |

The response body necessarily exists in the intended admin browser and can be
copied, photographed, or captured by a compromised device. Those endpoint
risks are inherent to manual credential delivery and are not solved by moving
the code between server-side transports.

## Data, Audit, and Logging Contract

No schema change is required.

- D1 continues to store only `HMAC(pepper, normalize(code))` and non-secret
  invite metadata.
- Invite insertion and the typed `invite_code.generated` audit row remain one
  RFC-079 Class A required batch.
- An authorization/guard `false` result reveals no code.
- Construction, D1, or audit failure returns the accepted generic `503`,
  persists no invite, and reveals no code.
- The audit event continues to target the non-secret invite identifier.
- Audit metadata must not contain the plaintext code, normalized code, HMAC,
  form token, request body, or rendered HTML.
- New diagnostics may record bounded facts such as outcome and invite ID, but
  must never format the reveal value or request body.
- Error reports must not attach local variables or response bodies containing
  the code.

RFC-079's local audit atomicity and redaction contract is settled and is a
precondition of successful disclosure. Remaining B5 exact-candidate hosted and
persistent-incident evidence stays open for public/production pilot closure
and is not claimed by this RFC's local implementation evidence.

## Implementation Slices

Implementation should remain one reviewable security patch after acceptance:

1. Extract or adapt the invite-page rendering path so GET renders without a
   reveal and successful POST can render with one.
2. Replace query parsing/reflection with the mandatory fixed clean `303` and
   `no-referrer` canonicalization at handler entry, before authentication,
   authorization, binding access, rendering, token issuance, or application
   subrequests.
3. Replace the success redirect containing `?code=` with direct HTML.
4. Set reveal-response cache and referrer headers explicitly.
5. Add focused handler/render, legacy-query, and RFC-079 failure-path tests plus
   a source-level release regression gate.
6. Update the threat model and release checklist from “known B1 gap” to a
   precise local-evidence statement; do not mark hosted evidence complete yet.
7. Capture isolated hosted-staging evidence under RFC-050 after B2 and B3 make
   that environment eligible.

No developer handoff is required at Proposed status. If the RFC is Accepted
and implementation is delegated, a companion handoff should name exact test
locations and keep B2/B3 and remaining hosted/persistent-incident B5 work out
of the patch unless separately accepted.

## Test and Release Evidence

### Required local automated evidence

- A source release gate rejects application-created invite URLs containing a
  plaintext-code query handoff, including the prior `invites?code=` pattern.
- GET with an attacker-supplied, empty, or repeated `code` query immediately
  returns `303` to the fixed clean canonical route with `no-referrer`, does not
  render or reflect the value, and proves through focused spies/counters that
  authentication, authorization, D1/KV or other binding access, page-token
  issuance, rendering, and application subrequests are not reached.
- The canonical redirect path accepts only the route's bounded
  community-identifier grammar or uses an equivalent safe path builder; no
  query-derived value enters `Location`.
- Successful generation returns `200`, has no `Location`, displays the code in
  body text, and sends `no-store, private` plus `no-referrer`.
- A forced required-audit failure returns the generic `503`, persists neither
  invite nor audit row, and discloses no plaintext in its body, headers, or
  redirect.
- The stored invite value is an HMAC and the plaintext is absent from database
  parameters retained beyond the insert call, audit metadata, and response
  headers.
- Replaying the same form token returns the canonical `303`, creates no second
  invite, and does not reveal the first code.
- The returned page remains usable without JavaScript and contains a fresh
  generation token if another invite may be generated.
- Existing generation entropy/format, revocation, authorization, and
  cross-community denial gates remain green.

The source gate must be narrow enough to allow security documentation and a
legacy-query rejection test to mention `code`, while still failing if a
handler interpolates invite plaintext into `Location` or a generated href.

### Required manual browser evidence

- Generate an invite with DevTools Network open and confirm no request URL or
  redirect `Location` contains the displayed code.
- Confirm the code is visible and selectable with JavaScript disabled.
- Open a synthetic `?code=` URL and confirm the first response is the fixed
  clean `303` with `no-referrer`; confirm the following application request
  does not carry the unsafe query in `Referer`.
- Reload/resubmit and confirm no additional invite is created and the old code
  is not redisplayed.
- Navigate away/back and confirm the application does not intentionally
  reconstruct or fetch the plaintext code.
- Confirm the response is marked non-cacheable and no third-party request is
  emitted by the reveal page.

### Required hosted staging evidence

Against the exact candidate build in isolated staging:

- inspect Worker/Cloudflare request evidence for the generation action and the
  following navigation;
- record only redacted URLs and identifiers in the evidence pack;
- demonstrate that the request target and redirect chain contain no plaintext
  invite code;
- redeem the code once to prove the response-only transport did not alter
  normal invite semantics.

The evidence pack must never copy the plaintext code into Markdown, filenames,
screenshots intended for retention, shell history, or review comments. A
reviewer may compare a transiently observed value in the browser with redacted
request records, then discard it.

## Acceptance Criteria

RFC-076 implementation is complete only when:

1. No application-generated URL, header, cookie, persisted record, audit row,
   analytics event, or log contains a generated plaintext invite code.
2. The first successful POST reveals the code only in non-cacheable HTML and
   returns no redirect.
3. A query-bearing GET is immediately canonicalized through a fixed clean
   `303` with `no-referrer` at handler entry, before authentication,
   authorization, binding access, rendering, token issuance, reflection, or an
   application subrequest. Its route-matched community identifier is handled
   through a bounded grammar or equivalent safe canonical-path builder.
4. Replay creates no invite and reveals no prior code.
5. Authorization/guard rejection reveals no code, and an RFC-079 required-audit
   failure persists no invite and returns generic `503` without disclosure.
6. Admin authorization, form-token, HMAC-only storage, expiry, revocation, and
   redemption behavior remain unchanged.
7. The automated regression gate and focused tests pass.
8. Manual no-JS and browser-network evidence passes.
9. Hosted staging evidence is recorded under RFC-050 before B1 is closed for a
   public or production pilot.

Architecture approval may accept this design and move it to `accepted/` before
hosted evidence exists. Moving an implementation to `done/` establishes the
local code change; it does not by itself close the hosted evidence item or lift
the roadmap remediation hold.

## Rollout and Rollback

There is no migration and no compatibility dependency. Existing HMAC invite
rows remain valid.

The patch can roll out independently at source level, but hosted staging should
evaluate it with B2 and B3 because invite generation and redemption depend on
the pepper and abuse-control bindings.

Rollback to the query-string handoff reintroduces the blocking vulnerability
and is not an acceptable production rollback. If direct rendering fails, the
safe operational response is to disable invite generation or roll forward;
do not restore plaintext URL transport.

## Alternatives Rejected

### Redirect with the code in a fragment

Rejected. Fragments remain in browser history, are exposed to same-page script,
and turn a server-rendered no-JS workflow into client-side secret transport.

### Redirect with an opaque receipt token in the URL

Rejected for this slice. The receipt becomes another bearer credential in
logs/history and requires a server-side plaintext or decryptable handoff,
expiry, consume semantics, cleanup, and abuse analysis.

### Plaintext flash value in a cookie

Rejected. Cookies are request headers on later requests, can enter diagnostics,
and expand credential exposure. `HttpOnly` would not make plaintext cookie
transport appropriate.

### Plaintext flash value in KV or D1

Rejected. It contradicts HMAC-only persistence and creates retention, cleanup,
concurrency, binding-failure, and operator-access risks.

### Encrypt the code for a redirect

Rejected. Ciphertext would still be a redeemable display bearer, would require
key/nonce/lifetime design, and adds complexity without improving on a direct
POST response.

### Keep PRG and rely on `same-origin`

Rejected. Same-origin referrer policy does not keep the URL out of browser or
platform request records and can forward the full URL to same-origin routes.

## Resolved Architecture Review Decisions

The initial architecture review retained the central direct POST-response
design and resolved these supporting decisions:

1. Normal same-origin navigation may remain because the reveal response has a
   code-free URL, explicit `no-referrer`, no third-party resources, and no
   code-bearing links or attributes. Copy warns that leaving or reloading makes
   the plaintext unrecoverable.
2. `Cache-Control: no-store, private` remains explicit at the handler boundary
   even though the global response hook currently supplies `no-store`.
3. Legacy query-bearing GETs require immediate fixed clean canonicalization
   with `no-referrer` at handler entry, before authentication, authorization,
   binding access, token issuance, rendering, or application subrequests.
   Ignoring a query at the unsafe URL is insufficient.
4. The remediation uses selectable text only and adds no copy-button script.
5. RFC-079 required-audit atomicity/redaction is settled locally; remaining
   hosted/persistent-incident evidence stays distinct and open.
