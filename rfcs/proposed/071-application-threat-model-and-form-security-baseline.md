# RFC 071 - Application Threat Model and Form Security Baseline

**Status.** Proposed  
**Target release.** v0.61.0 candidate  
**Tracks.** Security, privacy, forms, release gates, operations.  
**Touches.** Developer security docs, tester release checklist, form handlers,
domain validators, render escaping tests, smoke scripts, review process.

## Summary

Create a durable application threat model and use it to define the minimum
security baseline for HTML forms and POST handlers.

The project already has many security controls spread across requirements,
RFCs, release gates, implementation reviews, and operations documents. That is
useful, but it is hard to reason about coverage because there is no single
document that maps assets, attackers, trust boundaries, threats, controls,
evidence, and known gaps.

This RFC does not add a new user feature. It creates the security design map
that future feature RFCs and release reviews must reference.

## Background

Current facts:

- The requirements document defines security acceptance criteria for
  cross-community isolation, direct URL privacy, HttpOnly sessions,
  invite-code brute-force protection, XSS prevention, SQL injection
  prevention, CSRF protection, and private-cache behavior.
- `docs/src/developer/architecture.md` has a short security model section.
- Done RFCs already cover many security slices, including authorization,
  sessions, form tokens, audit logging, security headers, ICS feed privacy,
  admin-mediated help sign-in, member lifecycle, CSV export privacy, and
  operator-only recovery.
- `docs/src/tester/release-checklist.md` records many safety gates.
- The app has many server-rendered forms. State changes are POST forms with
  server-issued form tokens and 303 redirects.
- Recent form-heavy work, including RFC-070, showed that form coverage needs a
  consistent test strategy across validation, rendering, token handling,
  authorization, audit, and smoke evidence.

These pieces are valuable, but they are not yet a threat model.

## Problem

Security reasoning is currently distributed. That creates several risks:

- reviewers must reconstruct the trust model from multiple RFCs;
- form reviews may focus on the new happy path and miss shared attack classes;
- XSS, CSRF, SQL injection, IDOR, replay, stale-session, cache, and audit-log
  leakage checks may be applied inconsistently;
- staging and production deployment risks may be considered separately from
  application threats;
- release checklists can say a control exists without pointing to the evidence
  that proves it for the current surface.

The project needs a compact, reviewable threat model that becomes the common
reference for future security-sensitive RFCs.

## Goals

- Define protected assets and privacy expectations.
- Define attacker and misuse actors in project-specific terms.
- Define trust boundaries across browser, Worker, D1, KV, service worker,
  staging, production, and operator tools.
- Map major threat categories to existing controls and required evidence.
- Establish a reusable form-security baseline for all HTML forms and POST
  handlers.
- Identify known gaps without forcing all of them into the same release.
- Make future RFC reviews cheaper and more consistent.
- Keep the document practical for this codebase, not a generic security essay.

## Non-Goals

- No new authentication method.
- No OAuth/OIDC implementation.
- No user-facing security dashboard.
- No penetration test claim.
- No formal certification, compliance framework, or legal privacy policy.
- No automated fuzzing framework in the first slice.
- No broad handler rewrite just to match a new document shape.
- No change to production deployment policy by itself.

## Decision

Add a threat model document as the canonical security design map.

The first durable location should be:

```text
docs/src/developer/security-threat-model.md
```

The document should be linked from:

```text
docs/src/SUMMARY.md
docs/src/developer/index.md
docs/src/maintainer/index.md
docs/src/maintainer/operations.md
docs/src/tester/release-checklist.md
ROADMAP.md
```

The threat model must be explicit that it is a living project artifact. It
should distinguish:

- accepted current controls;
- required evidence;
- known gaps;
- deferred hardening candidates.

## Protected Assets

The threat model must cover at least these assets:

| Asset | Protection expectation |
|-------|------------------------|
| Community membership | A user sees or mutates only communities where they are an active member. |
| Roles | Admin-only actions require active admin role in the relevant community unless an RFC explicitly defines a narrower operator path. |
| Event data | Event titles, dates, locations, attendance, notes, matrix data, and cancelled/recreated relationships remain community-private. |
| Attendance and notes | Member status and notes are not leaked across community boundaries or through export/feed channels. |
| Invite/relink codes | Plaintext codes are shown only at intended one-time surfaces and are never logged or stored in reusable plaintext. |
| Sessions | Session identifiers remain HttpOnly, Secure, SameSite-bound, and revocable. |
| Form tokens | Form tokens protect write operations against CSRF, replay, and accidental double submit. |
| Audit log | Security-relevant actions are recorded without storing unnecessary personal data or bearer secrets. |
| D1 data | Database writes are scoped, parameterized, and migration-compatible. |
| KV rate-limit state | Abuse counters support protection without becoming a source of sensitive user data. |
| Service-worker cache | Cache must not expose private authenticated content after logout, session loss, or offline navigation. |
| Deployment configuration | D1/KV identifiers, secrets, and environment-specific config must not be accidentally committed or exposed. |

## Actors and Misuse Cases

The threat model must include these actors:

| Actor | Capability to model |
|-------|---------------------|
| Anonymous internet user | Can visit public routes, submit invite/relink codes, and attempt brute force or malformed requests. |
| Signed-in active member | Can access their own communities and attempt direct URLs or forged forms for other communities. |
| Community admin | Can perform admin workflows in their community and may accidentally or intentionally misuse exports, invites, help-signin, and member lifecycle actions. |
| Removed member | May retain old URLs, stale browser state, old session cookies, or old invitation context. |
| User with a leaked bearer link | May access an ICS feed until revoked. |
| Malicious script payload author | Can enter hostile text into any user-controlled field that later renders in HTML, CSV, ICS, logs, or audit metadata. |
| Network observer | May see public URLs and response metadata, but not HTTPS payloads. |
| Operator | Can run deployment, D1, KV, and recovery commands; must avoid printing, committing, or logging secrets. |
| Reviewer/tester | Can run smoke scripts and inspect evidence, but should not need real production secrets. |

## Trust Boundaries

The threat model must diagram or tabulate these boundaries:

- Browser to Worker request boundary.
- Anonymous routes to authenticated routes.
- Session cookie to authenticated user identity.
- Authenticated user to active community membership.
- Member role to admin role.
- Admin role to operator-only tools.
- Worker to D1.
- Worker to KV.
- Worker to environment variables and secrets.
- Server-rendered HTML to browser DOM.
- Client-side enhancement JavaScript to no-JS form behavior.
- Service worker cache to private authenticated content.
- Local development to hosted staging.
- Hosted staging to production.

## Threat Categories and Required Controls

The threat model should use practical project categories rather than requiring
a heavyweight external framework. STRIDE-like terminology may be used when it
helps, but every item must map to project controls and evidence.

| Threat | Required control baseline |
|--------|---------------------------|
| Cross-community access / IDOR | Every community-scoped route requires active membership for `:cid`; admin routes require active admin role for that same community; denied resources use generic not-found behavior. |
| Removed-member access | Membership lookups and authorization queries exclude `removed_at IS NOT NULL`; stale sessions do not restore access. |
| CSRF | Every state-changing form uses a server-issued purpose-bound token. Authenticated tokens are subject-bound to the current user and bound to the resource where applicable. |
| Replay / double submit | Token consume is single-use and conditional. Mutations that redirect after success store deterministic replay results when duplicate submit would otherwise be ambiguous. |
| SQL injection | D1 access uses prepared statements and bound parameters for user-controlled input. Release gates should catch string-built SQL around new handlers. |
| XSS in HTML | User-controlled text is escaped at render boundaries. Validation may reject impossible/control values but must not be treated as the only XSS defense. |
| Attribute injection | Values inserted into HTML attributes must be escaped with the same render helper or a context-appropriate equivalent. Tests must include hostile quotes and event-handler payloads for new forms. |
| CSV formula injection | Client-generated CSV exports prefix dangerous formula-like values and avoid server-side CSV content generation unless specifically reviewed. |
| ICS/feed bearer leakage | Feed URLs are treated as bearer secrets, shown with privacy warnings, revocable, and excluded from logs/audit metadata. |
| Invite/relink brute force | Failed code redemption is rate-limited and returns generic errors. Plaintext codes are not logged. |
| Session theft impact | Cookies are HttpOnly, Secure, SameSite=Strict, host-only unless explicitly configured, and revocable by logout/session invalidation. |
| Cache privacy leak | Authenticated HTML is not stored in service-worker cache; offline fallback does not expose stale private content. |
| Audit leakage | Audit rows use action names and minimal metadata; no plaintext codes, bearer tokens, session IDs, CSV contents, or unnecessary old/new personal labels. |
| Deployment/config leakage | Environment-specific D1/KV IDs and secrets live in ignored local config or Cloudflare secrets, not committed shared config. |
| Staging exposure | Hosted staging is public while deployed; it must use non-production data, separate resources, and an explicit close-down procedure. |

## Form-Security Baseline

Every new or changed form must be reviewed against this baseline.

### Route and authorization

- GET render route must require the same or weaker access than POST only when
  the rendered form does not leak private data.
- POST route must repeat authorization checks; it must not rely on the GET
  route having been rendered earlier.
- Community-scoped forms must bind all writes to the route `:cid` and the
  authenticated active membership/admin role as appropriate.
- Hidden fields must be treated as attacker-controlled input.

### Validation and normalization

- Use domain validators for reusable business rules.
- Normalize before comparison or persistence where the validator defines
  normalization.
- Reject control characters and impossible values at the domain boundary.
- Do not rely on validation alone to prevent HTML, SQL, CSV, ICS, or log
  injection.

### Token and replay behavior

- Use purpose-specific form-token constants.
- Bind tokens to the authenticated user for authenticated forms.
- Bind tokens to the most specific resource where practical, for example event,
  membership, month, feed token, or join ticket.
- Consume tokens before mutation unless the RFC explicitly requires validation
  to happen first to avoid burning tokens on normal correction.
- For idempotent or replay-visible workflows, store a replay result so browser
  retry/double-click behavior returns a stable outcome.

### Persistence and audit

- Use prepared statements and bound parameters.
- Scope updates and deletes by the narrowest stable identifiers:
  `community_id`, authenticated `user_id`, membership id, role, and active
  flags as applicable.
- Security-relevant writes must be audited unless the RFC explicitly explains
  why audit is unnecessary.
- RFC-079 Class A writes require one atomic business/audit D1 batch. Class B
  requires durable audit evidence before protected disclosure or authorization
  acknowledgement and returns generic `503` on audit failure.
- `session.logout` is the sole Class C exception: require revocation first,
  await an identifier-free audit attempt, emit bounded failure telemetry, and
  clear the cookie even if that audit fails. No other action may discard or
  background required audit work.
- Audit metadata must exclude raw submitted secrets, plaintext codes, bearer
  links, session IDs, full CSV/export contents, and unnecessary personal data.
  Production inputs are closed typed variants; recursive sanitation is defense
  in depth, not permission for arbitrary JSON.

### Rendering and response

- Escape all user-controlled values in HTML text and attribute contexts.
- Use fixed flash/error codes, not raw query strings or raw submitted messages.
- Use generic denial messages for resource enumeration boundaries.
- Preserve no-JS operation for state changes.
- Use 303 redirects after successful POST.

### Tests and evidence

For each new or materially changed form, the implementation review should
identify the expected evidence from this list:

- domain validator tests for accepted/rejected boundaries;
- render tests for hostile text and attribute payloads;
- release gate static tests for route authorization, token purpose/resource
  binding, scoped SQL, audit action, and absence of unsafe metadata;
- browser smoke for the intended user workflow, including sandboxed/incognito
  browser mode when browser smoke is required;
- hosted staging smoke when Cloudflare bindings, D1/KV behavior, or deployment
  configuration are part of the risk.

Not every form requires every evidence type, but the RFC or implementation
review must say which are required and why.

## Initial Application Mapping

The first threat-model document should map at least these existing surfaces:

| Surface | Key threats |
|---------|-------------|
| `/join` | invite brute force, single-use claim race, user-controlled display name, session issuance, wrong-code confusion with relink codes. |
| `/relink` | code brute force, active-member-only return, generic failure, session issuance. |
| `/c/:cid` and calendar pages | community isolation, direct URL access, stale community switch state. |
| Event create/edit/cancel/copy/recreate | admin authorization, scoped writes, date/time normalization, recurrence exceptions, replay, audit. |
| Attendance and notes | member ownership, status semantics, note XSS, no-JS POST behavior. |
| Member management | active-admin-only actions, last-admin guard, invite display, removed-member denial, audit. |
| Admin role transfer | admin authorization, last-admin guard, direction-specific audit, replay. |
| Member lifecycle | removal/re-add policy, stale sessions, active-only queries. |
| Help-signin | active-only target, code display once, relink-code secrecy, cross-community denial. |
| Total community access recovery | disabled-by-default operator path, explicit operator label, fail-closed audit, no plaintext secret logging. |
| Calendar feed | bearer-link warning, revocation, scoped ICS body, no-store response. |
| Monthly matrix and CSV export | member-visible matrix, admin-only CSV, client-side CSV generation, formula hardening, metadata-only audit. |
| Self display-name editing | active-member-only write, validator reuse, render escaping, token replay, minimal audit metadata. |

## Documentation Shape

The developer document should use this chapter outline:

```text
# Application Threat Model

## Purpose and Status
## Assets
## Actors
## Trust Boundaries
## Threats, Controls, and Evidence
## Form-Security Baseline
## Surface Map
## Known Gaps and Deferred Work
## Review Checklist
```

The tester release checklist should link to the threat model and add a compact
form-security review gate rather than duplicating the full model.

Maintainer docs should link to the threat model as the operations-facing entry
point for deployment/config leakage, staging exposure, D1/KV binding changes,
secret handling, log access, audit retention, and operator-only recovery. A
separate shorter maintainer copy is deferred unless operators find the developer
document too hard to use.

The roadmap should include threat-model maintenance before broad public pilot
deployment.

Future security-sensitive RFCs and every new or materially changed form must
include a short threat-model impact note or explicitly cite the release
checklist's form-security baseline gate. The note can be brief, but it should
name the affected assets, actors, trust boundaries, controls, and evidence.

## Acceptance Criteria

- Threat model document exists under `docs/src/developer/`.
- `docs/src/SUMMARY.md` links the new document.
- `docs/src/developer/index.md` links the new document.
- Maintainer docs link to the new document as the operations-facing entry
  point, or explicitly defer a maintainer summary.
- `docs/src/tester/release-checklist.md` links the threat model and adds a
  form-security baseline gate.
- The future-review hook for security-sensitive RFCs and new/changed forms is
  documented.
- `ROADMAP.md` includes RFC-071 as a near-term security hardening candidate.
- `rfcs/README.md` indexes RFC-071.
- The threat model maps assets, actors, trust boundaries, threats, controls,
  evidence, and known gaps.
- The form-security baseline is explicit enough for reviewers to apply to new
  forms.
- The document avoids real secrets, production identifiers, and environment
  values.

## Review Questions

- Are the actors and trust boundaries complete enough for current app scope?
- Does the form-security baseline reflect how the server-rendered app is
  actually built?
- Are any controls overstated compared with current tests and smoke evidence?
- Which gaps should block the next release, and which should remain deferred?
- Should the threat model live under Developer docs only, or should Maintainer
  docs have a shorter operations-facing copy?
- Should future feature RFCs include a mandatory "Threat Model Impact" section,
  or is a release-review checklist enough?

## Rollout Plan

1. Review and accept this RFC.
2. Add the developer threat-model document and checklist links.
3. Add or adjust release gates only where the document reveals missing
   evidence for already-active forms.
4. Use the threat model in the next security-sensitive RFC review.
5. Revisit the document before first pilot deployment and after any major auth,
   notification, export, or visibility-boundary feature.

## Open Gaps to Track

This RFC should not hide known gaps. Initial candidates:

- No single threat-model document exists yet.
- Some existing forms have workflow smoke but limited render-level hostile-input
  tests.
- Some security assertions live in review notes or `.git-exclude` evidence
  rather than durable docs.
- Full hosted staging evidence is still operator-dependent.
- No automated scanner or fuzzing harness is required today.
- No formal incident-response playbook exists beyond operational recovery and
  audit notes.

These gaps should be reviewed and prioritized, not silently treated as solved.
