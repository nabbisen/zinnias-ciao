# Application Threat Model

## Purpose and Status

This document is the durable security map for ciao.zinnias. It connects the
project's assets, actors, trust boundaries, threats, current controls, evidence,
and open gaps.

It is not a penetration-test report, compliance statement, or guarantee that
every possible attack has been eliminated. It is a review baseline. New
security-sensitive RFCs and every new or materially changed form must either
include a short threat-model impact note or explicitly cite the form-security
gate in the release checklist.

Status:

- **Current controls** are controls already present in the codebase or durable
  documentation.
- **Evidence** points to tests, release gates, smoke scripts, reviewed RFCs, or
  operational docs that support the control.
- **Gaps** are not accepted as solved; they are candidates for future hardening
  or release-review follow-up.

## Assets

| Asset | Protection expectation |
|-------|------------------------|
| Community membership | A user sees or mutates only communities where they are an active member. |
| Roles | Admin-only actions require active admin role in the relevant community unless a reviewed RFC defines an operator path. |
| Event data | Event titles, schedules, locations, recurrence state, cancellation state, notes, attendance, matrix data, and copy/recreate relationships remain community-private. |
| Attendance and notes | Member status and notes are not leaked across community boundaries, exports, feeds, logs, or audit metadata. |
| Invite and relink codes | Plaintext codes are shown only at intended one-time surfaces and are never logged or stored in reusable plaintext. |
| Sessions | Session identifiers remain HttpOnly, Secure, SameSite-bound, host-scoped by default, and revocable. |
| Form tokens | Write operations are protected from CSRF, replay, and accidental double submit by purpose-bound tokens. |
| Audit log | Security-relevant actions are recorded with minimal metadata and without bearer secrets or unnecessary personal data. |
| D1 data | Writes are scoped, parameterized, migration-compatible, and auditable where security relevant. |
| KV rate-limit state | Abuse counters support brute-force protection without becoming a sensitive user-data store. |
| Service-worker cache | Offline behavior must not expose private authenticated content after logout, session loss, or direct offline navigation. |
| Deployment configuration | D1/KV identifiers, local environment files, and secrets must not be accidentally committed or printed. |

## Actors

| Actor | Capability to model |
|-------|---------------------|
| Anonymous internet user | Can visit public routes, submit invite/relink codes, and send malformed or repeated requests. |
| Signed-in active member | Can access their own communities and attempt direct URLs, forged forms, stale links, or cross-community resource IDs. |
| Community admin | Can perform admin workflows in their community and may misuse exports, invite codes, help-signin, member lifecycle, role transfer, or event management. |
| Removed member | May retain stale URLs, browser state, old sessions, previous invite context, or screenshots. |
| Bearer-link holder | May access an ICS feed until the token is revoked or expires according to its policy. |
| Malicious payload author | Can enter hostile text into any user-controlled field that later renders in HTML, CSV, ICS, logs, or audit metadata. |
| Network observer | May see public URLs and response metadata, but not HTTPS payload bodies. |
| Operator | Can run deployment, D1, KV, recovery, and log commands; must avoid printing or committing secrets. |
| Reviewer/tester | Can run smoke scripts and inspect evidence without needing production secrets. |

## Trust Boundaries

| Boundary | Why it matters |
|----------|----------------|
| Browser to Worker | Every request can be forged; hidden fields and route parameters are attacker-controlled. |
| Anonymous route to authenticated route | Join, relink, static, and health routes have different exposure from community pages. |
| Session cookie to authenticated user | A valid session establishes the user identity used for authorization and token subject binding. |
| Authenticated user to active community membership | Membership is the main privacy boundary; stale or removed memberships must not grant access. |
| Member role to admin role | Admin forms must re-check admin status on POST, not only when rendering GET pages. |
| Admin role to operator-only tooling | Operator recovery and deployment tasks must not become ordinary app UI. |
| Worker to D1 | SQL must be parameterized and scoped to the current community/user/resource. |
| Worker to KV | KV rate-limit failures and namespace mistakes affect abuse controls. |
| Worker to secrets/config | Secrets and environment-specific IDs must remain outside shared commits and logs. |
| Server-rendered HTML to DOM | User-controlled values must be escaped before entering text or attribute contexts. |
| Client enhancement to no-JS behavior | JavaScript can improve UX, but state changes must work correctly as plain forms. |
| Service worker cache to private content | Offline support must not cache authenticated HTML or leak stale private state. |
| Local development to hosted staging | Hosted staging is public while deployed and must use isolated non-production resources. |
| Hosted staging to production | Production requires stricter data, secret, log, and exposure discipline. |

## Threats, Controls, and Evidence

| Threat | Current control | Evidence | Gap or target |
|--------|-----------------|----------|---------------|
| Cross-community access / IDOR | Community-scoped routes require active membership for `:cid`; admin routes require active admin role for that community; denied resources use generic not-found behavior. | `authz.rs`, release checklist safety gates, RFC-004, RFC-061, RFC-067. | Keep adding route-level release gates for new community-scoped surfaces. |
| Removed-member access | Active membership queries exclude removed memberships. | RFC-063, release gates for member lifecycle, checklist safety gate. | Staging smoke should continue to verify stale-session behavior on sensitive flows. |
| CSRF | State-changing forms use server-issued form tokens plus `SameSite=Strict` session cookies. | Architecture AD-4, `form_token.rs`, RFC-037, release checklist. | New forms must state token purpose and resource binding in implementation review. |
| Replay / double submit | Form-token consume is conditional; workflows that need stable duplicate-submit behavior store replay results. | RFC-037, RFC-041, RFC-070 implementation tests. | Older forms should be audited for whether replay UX is sufficient, especially destructive/admin forms. |
| SQL injection | D1 access uses prepared statements and bound parameters for user-controlled values. | Requirements security acceptance, release gates that inspect scoped SQL for sensitive RFCs. | No broad automated SQL construction scanner exists. Add static gates where a new handler has non-trivial SQL. |
| XSS in HTML | User-controlled text is escaped at render boundaries. Validators reject control characters but are not the only defense. | `contracts::html` tests, release checklist i18n/XSS gate, RFC-070 form rendering tests. | Existing forms should gradually gain targeted hostile-input render tests. |
| Attribute injection | Values inserted into attributes are escaped with the same render helper or a context-appropriate equivalent. | `escape_html` tests include attribute vectors; RFC-070 form tests cover hostile quotes. | New form tests should include quoted event-handler payloads for prefilled values and hidden tokens. |
| CSV formula injection | Matrix CSV export is generated client-side and hardens formula-like values; server receives metadata-only audit. | RFC-068, smoke script, release checklist CSV gates. | Re-review if server-side CSV generation is ever introduced. |
| ICS/feed bearer leakage | Feed URLs are treated as bearer links, shown with privacy warning, revocable, and excluded from audit metadata. | RFC-053, release checklist calendar-feed gates. | Feed URL exposure through browser history and copy/paste remains a user education issue. |
| Invite/relink brute force | Failed code redemption is rate-limited and returns generic errors; plaintext codes are not logged. | RFC-012, RFC-024, RFC-041, RFC-063, smoke scripts and release gates. | KV failure behavior should be reviewed before production pilot. |
| Generated invite-code transport leakage | Invite generation returns a direct non-cacheable `200` with the plaintext once in escaped body text, a clean canonical URL, and `no-referrer`; replay redirects without mutation or redisplay. Legacy `code` query keys are canonicalized before authentication or binding access. | RFC-076, `admin/members.rs`, contracts gate, focused native tests, and local invite-redemption browser/D1 smoke. | RFC-050 exact-candidate hosted and manual no-JS evidence remain required before public/production B1 closure. |
| Session theft impact | Session cookies are HttpOnly, Secure, SameSite=Strict, host-only by default, and revocable. | RFC-038, `session.rs`, release checklist. | Account-wide session management UI is not implemented. |
| Cache privacy leak | Authenticated HTML is not stored in service-worker cache; offline fallback does not expose stale private content. | RFC-042, RFC-055, release checklist offline gates. | Browser-specific cache behavior should remain part of staging smoke. |
| Audit loss or leakage | RFC-079 uses a closed 26-action model, typed action-specific metadata, recursive sanitation, atomic Class A batches, bounded exactly-once Class A failure events, fail-before-disclosure Class B writes, and identifier-free safety-first logout audit. Bearer secrets, plaintext codes/HMACs, session IDs, export contents, raw subject IDs in Worker events, and unnecessary personal labels are excluded. | RFC-014, RFC-052, RFC-079, audit source gates, local compiled-SSR/D1 rollback, concurrency, and boundary proofs. | Exact-candidate hosted negative tests and persistent incident delivery remain required by RFC-050; new actions must be classified and added to the closed inventory. |
| Deployment/config leakage | Environment-specific D1/KV IDs and secrets live in ignored local config or Cloudflare secrets, not committed shared config. | Deployment docs, staging runtime prototype docs, `.gitignore` policy. | Developers must still avoid committing copied local config files. |
| Staging exposure | Hosted staging is public while deployed; it uses non-production data, separate resources, and explicit close-down steps. | Staging runtime prototype docs and deployment docs. | Staging close-down and resource cleanup remain operator tasks. |

## Form-Security Baseline

Every new or materially changed form should be reviewed against this baseline.

### Route and Authorization

- GET render routes must not leak private data to users who cannot submit the
  corresponding POST.
- POST routes must repeat authorization checks. They must not rely on the GET
  page having been rendered earlier.
- Community-scoped forms must bind writes to the route `:cid` and the
  authenticated active membership or admin role as appropriate.
- Hidden fields are attacker-controlled input.

### Validation and Normalization

- Use domain validators for reusable business rules.
- Normalize before comparison or persistence when the validator defines
  normalization.
- Reject control characters and impossible values at the domain boundary.
- Do not rely on validation alone to prevent HTML, SQL, CSV, ICS, or log
  injection.

### Token and Replay Behavior

- Use purpose-specific form-token constants.
- Bind authenticated form tokens to the authenticated user.
- Bind tokens to the most specific resource that makes sense: event,
  membership, month, feed token, join ticket, or equivalent.
- Decide whether validation happens before token consumption. If validation
  happens first, explain why normal correction should not burn the token.
- Store a replay result for workflows where browser retry or double-click would
  otherwise create an ambiguous response.

### Persistence and Audit

- Use prepared statements and bound parameters.
- Scope updates and deletes by narrow stable identifiers such as
  `community_id`, authenticated `user_id`, membership id, role, and active
  flags.
- Audit security-relevant writes unless the RFC explicitly explains why audit
  is unnecessary.
- Class A business mutations and required audit evidence must commit atomically.
- Class A construction, D1, or impossible post-batch failure must emit exactly
  one bounded `audit.required_batch_failed` event from the central audit module;
  rejected request IDs are replaced wholesale with `invalid_request_id`.
- Class B must persist audit evidence before protected disclosure or
  acknowledgement and return generic `503` on failure.
- Logout is the only secondary-audit exception: revoke first, await the audit
  attempt without session/subject identifiers, emit bounded failure telemetry,
  and clear the credential even when auditing fails.
- No required audit may be ignored, floated, queued, or placed in
  `waitUntil()`.
- Audit metadata must exclude raw submitted secrets, plaintext codes, bearer
  links, session IDs, full CSV/export contents, and unnecessary personal data;
  callers cannot supply arbitrary actions or JSON.

### Rendering and Response

- Escape all user-controlled values in HTML text and attribute contexts.
- Use fixed flash/error codes, not raw query strings or raw submitted messages.
- Use generic denial messages at resource-enumeration boundaries.
- Preserve no-JS operation for state changes.
- Use 303 redirects after successful POST.

### Tests and Evidence

For each new or materially changed form, implementation review should identify
which evidence applies:

- domain validator tests for accepted/rejected boundaries;
- render tests for hostile text and attribute payloads;
- release gate tests for route authorization, token purpose/resource binding,
  scoped SQL, audit action, and absence of unsafe metadata;
- browser smoke for intended user workflow and mobile/large-text behavior;
- hosted staging smoke when Cloudflare D1/KV/binding behavior is part of the
  risk.

Not every form requires every evidence type. The RFC or implementation review
must say what is required and why.

## Surface Map

| Surface | Primary threats | Current evidence |
|---------|-----------------|------------------|
| `/join` | Invite brute force, single-use claim race, display-name rendering, session issuance, wrong-code confusion with relink codes. | RFC-003, RFC-041, invite smoke script, release checklist. |
| `/relink` | Relink-code brute force, active-member-only return, generic failure, session issuance. | RFC-024, help-signin smoke script, release gates. |
| `/c/:cid` and calendar pages | Community isolation, direct URL access, stale community switch state. | RFC-056 through RFC-059, RFC-067, and release-checklist gates that reference observed smoke evidence for specific shipped slices. |
| Event create/edit/cancel/copy/recreate | Admin authorization, scoped writes, date/time normalization, recurrence exceptions, replay, audit. | RFC-009, RFC-051, RFC-060, RFC-065, RFC-066. |
| Attendance and notes | Member ownership, status semantics, note XSS, no-JS POST behavior. | RFC-006, RFC-007, RFC-046, release checklist. |
| Member management | Active-admin-only actions, last-admin guard, invite display, removed-member denial, audit. | RFC-010, RFC-061, RFC-063. |
| Admin role transfer | Admin authorization, last-admin guard, direction-specific audit, replay. | RFC-062. |
| Help-signin | Active-only target, code display once, relink-code secrecy, cross-community denial. | RFC-024, help-signin smoke. |
| Total community access recovery | Disabled-by-default operator path, explicit operator label, fail-closed audit, no plaintext secret logging. | RFC-069. |
| Calendar feed | Bearer-link warning, revocation, scoped ICS body, no-store response. | RFC-053. |
| Monthly matrix and CSV export | Member-visible matrix, admin-only CSV, client-side CSV generation, formula hardening, metadata-only audit. | RFC-067, RFC-068. |
| Self display-name editing | Active-member-only write, validator reuse, render escaping, token replay, minimal audit metadata. | RFC-070, SSR form tests, and local hosted-staging checklist evidence under `.git-exclude/evidence/rfc070/`. |

## Known Gaps and Deferred Work

- Several existing forms have workflow smoke coverage but limited render-level
  hostile-input tests.
- Some security evidence still lives in `.git-exclude` review/evidence files
  rather than durable shared docs.
- Full hosted staging evidence remains operator-dependent.
- No automated scanner or fuzzing harness is required today.
- Persistent `audit.*_failed` incident delivery has not yet been demonstrated
  against an owner-approved sink; `console`/tail observation is diagnostic only.
- KV rate-limit failure behavior should be reviewed before first production
  pilot.
- Older destructive/admin forms should be checked against the replay-result
  baseline.

These gaps should be prioritized during release review. They should not be read
as already solved by this document.

## Review Checklist

Use this checklist when reviewing a security-sensitive RFC, implementation, or
release candidate:

- Does the change introduce a new form, route, export, feed, recovery path, or
  deployment step?
- Which assets does it read, write, export, cache, or log?
- Which actor can trigger it, and which actor should be denied?
- Which trust boundary does it cross?
- Does POST repeat authorization?
- Is the form token purpose/resource binding explicit?
- Are hidden fields treated as attacker-controlled?
- Are validators and render escaping both present where needed?
- Are SQL writes parameterized and scoped by community/user/resource?
- Is replay/double-submit behavior defined?
- Is audit required, and is audit metadata minimal?
- Are denial messages generic where enumeration matters?
- Is no-JS behavior still correct?
- Which test, release gate, smoke script, or manual evidence proves the
  important controls?

## Maintainer Entry Point

Maintainers should use this document together with:

- [Operations](../maintainer/operations.md)
- [Launch Runbook](../maintainer/launch-runbook.md)
- [Deployment](../shared/deployment.md)
- [Staging Runtime Prototype](../tester/staging-runtime-prototype.md)
- [Audit Policy](../maintainer/audit-policy.md)

The most relevant maintainer concerns are deployment/config leakage, staging
exposure, D1/KV binding correctness, secret handling, log access, audit
retention, and operator-only recovery. This first RFC-071 slice keeps the
canonical model in Developer docs and links it from Maintainer docs rather than
duplicating a shorter operations copy.
