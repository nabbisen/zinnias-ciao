# RFC 080 - External Identity Foundation

**Status.** Accepted — owner-accepted 2026-08-09. Stage 1 of the four-stage
external-identity track recorded in the 2026-07-17 pre-RFC consultation and
unblocked by the 2026-08-09 hold-lift (`ROADMAP.md`, Post-Hold Feature
Candidates §1). **Acceptance confers no implementation authority** — Stage 2 must
also be accepted first; see §12.

**This RFC chooses no provider.** Provider selection is Stage 3 and depends on
user research that does not yet exist. Every mechanism here is
provider-independent by construction, and the RFC is not implementable against a
real provider until Stage 2 (recovery and membership continuity) is also
accepted.

**Target release.** None. Design only; no implementation authority is conferred
by acceptance of this RFC alone.

**Tracks.** Identity, session provenance, account recovery boundaries, audit
inventory, schema. Follows RFC-003 (invite redemption and session auth), RFC-024
(relink), RFC-038 (session and secret binding), RFC-071 (threat model), RFC-079
(atomic audits). Amends AD-2.

**Touches.** New `user_identities` and authentication-transaction tables, a new
migration, `users.idp_subject` deprecation, `sessions` provenance, the audit
action inventory, SSR routes, release gates.

**Depends on.** Stage 2 (recovery and membership-continuity policy) must be
accepted before any provider rollout. See §12.

---

## Summary

Give the application a stable internal principal that an external identity
provider can *authenticate*, without letting any provider fact *authorize*
anything.

Today a member is authenticated by possessing a session cookie, which was minted
by redeeming a one-time invite code. There is no way to prove "I am the same
person" on a new device without an admin. This RFC builds the seam at which an
OIDC provider could supply that proof — and nothing more.

**Read the scope precisely: this is a foundation, not a feature.** Nothing in
this RFC becomes visible to a member. No provider is chosen, registered, or
called. The deliverable is a set of contracts and a schema that make Stage 3
possible and make it safe.

## Background

`migrations/0001_initial.sql:17` has carried this since the first migration:

```sql
-- Reserved for deferred OIDC (AD-2). NULL for invite-only members.
idp_subject TEXT UNIQUE,
```

It is referenced by no code. It is also the wrong shape, and §3 replaces it.

The 2026-07-17 consultation settled the load-bearing questions and its
conclusions are inputs here, not open items:

1. `users.id` is the principal; identities and sessions authenticate it; active
   memberships authorize it.
2. An invite is one-time community-join authorization — never an identity
   credential for an existing user.
3. `(issuer, subject)` is the OIDC conceptual key but is **not** a sufficient
   database key.
4. Email and display name may never link accounts.
5. Community-admin help-signin cannot remain account-wide recovery authority once
   one `users.id` spans communities.

Stage 0 provider due diligence
(`.git-exclude/research/2026-08-09-stage0-provider-due-diligence.md`) added one
fact that changes a contract in this RFC: **LINE signs web-login ID tokens with
HS256 keyed on the channel secret, while signing native/LIFF tokens with ES256**.
A verifier that trusts the token header's `alg` would be exploitable against a
provider that genuinely issues both. §4 makes algorithm pinning part of the
adapter contract rather than a provider-RFC detail.

## Goals

- A stable application principal, unchanged by provider changes.
- An identity model that survives provider registration changes, environment
  separation, and Apple-style team transfers without mis-identifying anyone.
- A verified-identity boundary that no provider-specific value crosses.
- A server-side, single-use authentication transaction resistant to replay,
  mix-up, fixation, and open redirect.
- Session provenance, so a later high-risk operation can refuse a weakly
  authenticated session.
- Audit coverage for identity operations within RFC-079's contract.
- A local fake-issuer harness, so all of the above is testable with no provider
  account and no network.

## Non-goals

Reproduced from the consultation's first-foundation exclusions, and binding:

provider selection or registration; provider-specific production code; automatic
link or merge by email, name, avatar, phone, contact list, or provider group;
account merge or history reassignment; removed-membership reactivation;
provider-derived community membership or role; email delivery, profile
synchronization, contact import, or provider API access; routine storage of
provider access or refresh tokens; replacement of codlet-backed sessions or form
tokens; mandatory external-provider adoption; JavaScript-only authentication;
passwords, passkeys, or biometrics; real provider identities, tokens, secrets, or
subjects in tests or documentation; hosted deployment or pilot.

**Also excluded here and deferred to Stage 2:** provider-independent account
recovery, retirement of community-admin account-wide authority, final-credential
unlink prevention, and the RFC-063 continuity amendment. This RFC must not be
implemented ahead of them — see §12.

## 1. Vocabulary

Used consistently throughout, and required in implementation naming:

| Term | Meaning |
|---|---|
| **application principal** | `users.id` |
| **external identity proof** | a verified `(identity namespace, subject)` pair |
| **join authorization** | an unconsumed community invite |
| **community authorization** | an active membership and its role |
| **application session** | the revocable local cookie-backed session |
| **recovery credential** | an account-level, provider-independent mechanism (Stage 2) |
| **form protection** | existing purpose- and resource-bound single-use form tokens |

No provider token, provider group, email domain, contact list, display name, or
invite possession may directly grant a community role.

## 2. The principal invariant

`users.id` is the principal and does not change when an identity is added,
removed, or replaced.

- **Authentication** answers *which principal is present*: an application
  session, or a verified external identity that resolves to one.
- **Authorization** answers *what that principal may do here*: an active
  `community_memberships` row and its role, exactly as today.

External authentication changes only how a session is first obtained. It does not
change the session, CSRF, or idempotency model — codlet-backed sessions and form
tokens keep their current roles.

## 3. Identity namespaces and `user_identities`

### 3.1 Why `(issuer, subject)` is not the key

OIDC defines `sub` as locally unique within an issuer and permits pairwise
subjects that differ per client. Concretely, from Stage 0 due diligence:

- **Apple** scopes both the subject and the private-relay address to the
  *developer team*. An app transfer produces a different value for the same
  person, recoverable only through a `transfer_sub` exchanged within a 60-day
  window.
- **LINE** issues tokens whose `aud` is the Channel ID; whether `sub` is stable
  across channels or provider groupings is **not documented** and is an open
  Stage 3 question.
- **Google** states plainly that `sub` is the account key and `email` must not be.

A key of `(issuer, subject)` would therefore silently mis-identify users across
an Apple team transfer, and possibly across a LINE channel change.

### 3.2 The namespace

An `identity_namespace` is an immutable internal record of a **reviewed provider
registration**, capturing at minimum:

```text
provider kind
canonical issuer
expected audience / client registration
subject scope (public, pairwise sector, channel, team or app group)
environment (production, staging, local-fake)
```

Namespaces are created by migration or reviewed configuration, never at runtime
from a token. Production, staging, and the local fake issuer use **separate
namespaces**; a test identity can never collide with a production identity.

### 3.3 The table

```sql
CREATE TABLE user_identities (
    id                    TEXT PRIMARY KEY,
    user_id               TEXT NOT NULL REFERENCES users(id),
    identity_namespace_id TEXT NOT NULL,
    subject_lookup        TEXT NOT NULL,   -- keyed digest, never the raw subject
    linked_at             TEXT NOT NULL,
    last_authenticated_at TEXT,
    status                TEXT NOT NULL CHECK(status IN ('active','revoked')),
    UNIQUE(identity_namespace_id, subject_lookup)
);
```

Uniqueness is `(identity_namespace_id, subject_lookup)` — **not** email, not a
provider label, not the subject alone. Two different namespaces never identify
the same person, and the implementation must not infer that they do.

**`subject_lookup` is a keyed digest** of the provider subject, using the existing
HMAC pepper discipline (AD-3, RFC-077). Rationale: a D1 export must not become a
cross-system correlation list keyed on provider identifiers. Treat the subject as
opaque and case-sensitive before digesting.

If a future provider lifecycle operation genuinely requires the raw subject —
Apple's `transfer_sub` exchange is the known candidate — the **provider RFC** must
justify encrypted recoverable storage with key versioning, retention, and
deletion. This RFC does not grant that.

### 3.4 `users.idp_subject`

Deprecated. It is `UNIQUE`, singleton, and namespace-free — every property this
model rejects. It is unreferenced by code and NULL in every row.

The migration drops it. **Do not populate it, and do not treat it as a fallback.**

## 4. The verified-identity boundary

Provider adapters terminate every provider-specific protocol and claim
difference. Only this crosses into identity logic:

```rust
VerifiedExternalIdentity {
    identity_namespace_id,
    subject,              // raw, in memory only; digested at the boundary
    authenticated_at,
    provider_authentication_context: Option<...>,  // only if separately reviewed
}
```

Route handlers, membership code, and render code must **never** see unverified
claims, JWTs, access tokens, email addresses, avatars, or provider user objects.

### 4.1 Algorithm pinning is part of this contract

Each namespace declares its **expected signature algorithm and key source**, and
the adapter rejects any token whose algorithm differs — regardless of what the
token header says.

This is a foundation concern, not a provider detail, because a single provider can
issue different algorithms for different client types. LINE is the concrete case:
HS256 keyed on the channel secret for web login, ES256 via JWKS for native and
LIFF. An adapter that selected its verification strategy from the token header
would be exploitable; one that selects it from the namespace cannot be.

Where the key is a shared secret (LINE web login), the namespace must record that
**the client secret is also the verification key**, so rotation is understood as
one indivisible operation rather than two.

## 5. The authentication transaction

Every provider round trip is bound to a short-lived, single-use, server-side
transaction created before the redirect. It binds:

opaque transaction lookup; action (`sign_in` | `join` | `link`); identity
namespace; expected `state`; OIDC `nonce`; PKCE verifier or protected derivation;
initiating session provenance (for `link`); a **non-secret internal invite
reference** (for `join`); the exact callback URI; an allowlisted post-login
destination; creation, expiry, and consumption; and attempt/replay status.

**Never** placed in an authorization URL: invite plaintext, internal user or
membership IDs, an arbitrary return URL, a provider token, or sensitive
transaction state. An invite is resolved to a non-secret internal reference
before the redirect and **claimed only in the final D1 transaction**.

### 5.1 Callback contract

1. Select only the namespace the transaction expects.
2. Atomically consume or reserve the transaction against replay.
3. Exchange the code server-to-server with the exact redirect URI and PKCE.
4. Verify the signature using the namespace's pinned algorithm and key source.
5. Validate exact issuer, audience or authorized party, `exp`, `iat`, the
   transaction `nonce`, and provider-required claims, with bounded clock skew.
6. Reject mix-up, unknown key, discovery mismatch, stale state, reused code,
   unsafe return route, and claim-type mismatch.
7. Pass only `VerifiedExternalIdentity` into identity logic.
8. Atomically create or link the identity, claim any invite, create the
   membership, issue the session, and write required audits as one transaction.
9. Redirect to a fixed clean local route, with provider response parameters
   removed from history and referrer propagation.

Authorization Code flow, **PKCE S256 even though the Worker is a confidential
client** (RFC 9700), independent `state`, and OIDC `nonce` are all mandatory. A
provider that cannot support a required control is blocked at Stage 3.

### 5.2 Failure posture

Provider discovery, JWKS, or token-endpoint outages **fail closed for new
sign-ins and links**, while **existing application sessions remain valid**. Local
session expiry and revocation are never derived from provider token expiry.

User-facing failures say, in plain Japanese, that sign-in could not be completed
and offer retry, cancel, or recovery. They must not confirm account existence, a
linked provider, invite validity, a subject, an email, or any internal ID.

## 6. Session provenance

`sessions` today records no information about *how* it was authenticated. That is
sufficient while every session comes from an invite redemption; it is not
sufficient once a session can permanently bind an external identity.

Add provenance and authentication time to `sessions`, with values covering at
least: invite redemption, relink or help-signin recovery, and external identity.

**Do not treat every existing 30-day cookie as sufficient step-up for a permanent
external link.** A stolen session or an admin-mediated recovery session must not
be able to bind an attacker's provider account.

First-link therefore requires: an authenticated account-level surface; acceptable
session provenance and freshness; a purpose-bound, user-bound, single-use link
token; explicit confirmation; fresh provider authentication in a fresh OIDC
transaction; uniqueness rejection; a required audit; and session rotation with
revocation of other sessions.

**A help-signin-derived session is not sufficient by itself.** A separately
reviewed independent confirmation, defined in Stage 2, is required.

Legacy sessions predating provenance get a bounded migration ceremony, also
Stage 2.

## 7. Collision, orphan, and merge policy

- **No orphan users.** An unrecognized identity with no claimable invite creates
  nothing. User, identity, membership, and session are created only when a valid
  invite can be claimed atomically.
- **A known identity may authenticate without a new invite**, but community
  access still requires an active membership. A known principal with no active
  memberships reaches a minimal account surface, never community data.
- **No auto-link.** A verified identity that resolves to no `user_id` is never
  attached to an existing user by email, name, avatar, phone, or provider group.
- **Collisions fail closed** with generic UX that does not disclose account
  existence.
- **No merge.** Out of scope entirely; a future merge RFC must handle membership
  conflicts, history ownership, revocation, rollback, audit, and consent.

## 8. Audit

`audit_log_v2.community_id` and `actor_membership_id` are already **nullable**, so
account-level identity actions fit the existing table without a schema change —
confirmed against `migrations/0010_audit_integrity.sql`.

`AuditAction` is a closed inventory of 26, pinned by `AuditAction::ALL: [Self; 26]`
so additions are compiler-gated. This RFC extends it with Class A candidates:
external session issued, identity linked, identity unlinked, identity replaced,
and recovery-method changed.

Session revocations caused by the same identity operation share that atomic
action rather than emitting ambiguous secondary successes. Anonymous failures and
collisions use bounded, rate-limited operational events — **not**
attacker-amplifiable audit rows.

**Never in audit metadata or telemetry:** raw subject, issuer URL, email, token,
authorization code, `nonce`, `state`, PKCE material, or session ID.

## 9. No-JS and SSR

Every application-owned route — start, cancel, callback, confirmation, collision,
recovery — works through SSR links and forms with **no application JavaScript**,
per AD-1. Provider-owned pages have their own platform behaviour; the application
must never require a client-side SDK as its only path.

Post-login destinations come from a server-side allowlist. `form-action 'self'`
and the current CSP are unaffected: the authorization redirect is a top-level
navigation, not a form post or a fetch.

## 10. Testability without a provider

A **local fake issuer** is a required deliverable, not an optional convenience:
its own namespace, its own keys, and coverage for key rotation, discovery
failure, JWKS failure, token-endpoint outage, replayed code, stale `state`,
wrong `nonce`, wrong `aud`, wrong `iss`, and **wrong algorithm**.

The full contract must be exercisable with no provider account, no secrets, and
no network. A design that can only be tested against a real provider is not
accepted by this RFC.

## 11. Acceptance criteria

1. `users.id` is the principal; no provider value grants authorization anywhere.
2. `user_identities` exists with `UNIQUE(identity_namespace_id, subject_lookup)`;
   `subject_lookup` is a keyed digest; no raw subject is stored.
3. `users.idp_subject` is dropped, not populated.
4. Namespaces are immutable, reviewed, and separate per environment.
5. No provider-specific value crosses the `VerifiedExternalIdentity` boundary.
6. The verification algorithm comes from the namespace, never the token header,
   and a wrong-algorithm token is rejected by an executable test.
7. Authorization Code + PKCE S256 + `state` + `nonce`, single-use server-side
   transactions, exact redirect URIs; replay, mix-up, and unsafe-return rejected.
8. `sessions` carries provenance and authentication time; first-link enforces
   step-up, rotation, and revocation; help-signin provenance alone is refused.
9. No orphan users, no auto-link, no merge; collisions fail closed and disclose
   nothing.
10. Audit inventory extended; `AuditAction::ALL` count updated; no prohibited
    value reaches metadata.
11. Every route works with no application JavaScript.
12. The fake-issuer harness covers rotation, outage, replay, and every negative
    validation case, with no network.

## 12. What acceptance does *not* authorize

Acceptance of this RFC alone authorizes **no implementation**. It authorizes no
provider selection, registration, secret provisioning, hosted callback, external
data collection, or deployment. B1, B3, B4, and B5 remain open; production,
public-pilot, and first-real-community deployment remain **No-Go**.

**Stage 2 must be accepted before implementation begins.** This RFC deliberately
leaves provider-independent recovery, community-admin authority retirement,
final-credential unlink prevention, and the RFC-063 continuity amendment
unresolved — and an identity foundation implemented without them would create
accounts that can be locked out or recovered by the wrong authority.

## 13. Open questions for Stage 3, not for this RFC

- LINE `sub` scope across channels and provider groupings — undocumented; blocks
  LINE selection.
- Google PKCE support on the authorization endpoint — undocumented on the OIDC
  page; blocks Google selection under §5's baseline.
- Provider priority and choice-overload behaviour — requires user research.
- Whether the invite link's arrival inside LINE's in-app browser degrades any of
  this. Worth testing regardless of provider choice, since invites will travel by
  LINE message either way.
