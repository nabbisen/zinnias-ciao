# RFC 081 - Account Recovery and Membership Continuity

**Status.** Implemented — design owner-accepted 2026-08-09; delivered alongside
RFC-080 across the same seven packages, each architecture-reviewed and Approved:

| Slice | Commit | This RFC's part |
|---|---|---|
| 1 — session provenance and community binding | `5d1ad94` | §2 / §2.1a — the live gap closed |
| 3 — schema re-baseline | `e3a51e8` | §1 membership continuity |
| 5a — the account surface | `6694c4a` | §6 no-membership surface |
| 5b — linking and re-authentication | `a31856f` | §4 linking |
| 5c — recovery credential and unlink prevention | `9b121f1` | §3, §3.1, §3.2, §3.3 |

**§2 was a live gap, not a precaution.** §2.1a records that a single community's
admin could already mint a session reaching every community a member belonged to,
reachable at `9d280b4` without RFC-080 — found while reviewing this RFC's own
design, and closed by Slice 1.

**AD-2 holds.** No member is walled behind an external account: the
provider-independent recovery credential exists, is visible before it is needed,
cannot be removed by any admin, and cannot be removed at all while it is the
member's last usable method — enforced in the same SQL statement as the unlink,
so the concurrent-unlink race cannot leave a member with none.

Two dated corrections: **§1.2a, 2026-08-10** — §1's invariant is unreachable by
migration under D1, and was instead reached by a one-time pre-deployment schema
re-baseline; **§2.1a, 2026-08-09** — §2's gap was already reachable.

Owner decision **§11.4** (expiring pre-cutover sessions) was answered in Slice 1
while it was still free: no real community had used the service, so all
pre-cutover sessions were expired and no legacy-assurance class exists.

**Stage 3 — choosing a provider — is outside this RFC**, and unchanged.

**This RFC chooses no provider.** It does assume more than one will eventually
exist, because the owner's recorded expectation is **Google Account and LINE, at
least**, which makes linking a day-one concern rather than a later one.

**Target release.** None. Design only.

**Tracks.** Account recovery authority, session capability, membership
continuity, schema. Amends **RFC-024** (admin-mediated relink), **RFC-063**
(removal and re-add), and AD-2. Depends on RFC-080's identity and provenance
model.

**Touches.** `community_memberships` uniqueness, `sessions` capability scope,
relink and help-signin authorization, a recovery-credential table, the audit
inventory, member-facing recovery copy.

---

## Summary

RFC-080 gives the application a stable principal. **This RFC makes that principal
safe to have.**

A stable `users.id` that spans communities breaks three things that are correct
today only because identity is disposable: a community admin can mint a session
that reaches every community the person belongs to; a removed member cannot come
back as the same principal; and losing your only credential means losing the
account with no path back that does not run through an admin.

Each is a consequence of the same change, and none can be deferred past
implementation.

## Background — why today is fine and tomorrow is not

**Today identity is disposable.** `workers/ssr/src/handlers/join.rs:202` mints a
fresh `user_id` from `crypto::random_token()` on **every** invite redemption. The
same human redeeming two invites becomes two principals with no relationship.

That single fact is why the current design is coherent:

- `UNIQUE(community_id, user_id)` at `migrations/0001_initial.sql:29` is never
  violated by a removed-then-returning member, because the returning member is a
  new `user_id`.
- A relink session minted by a community admin authorizes "every membership of
  this user" — which today is, in practice, one.
- Losing a credential loses one community's membership, not an account.

**RFC-080 removes that fact.** Once a returning person resolves to the *same*
`users.id`, all three become live problems simultaneously. This RFC settles them
before, not after.

RFC-024 anticipated this in its own text: *"Any future multi-community identity
or OIDC RFC must revisit this recovery flow before a single `user_id`…"* — this
is that revisit.

## 1. Membership continuity — amending RFC-063

### 1.1 The constraint

RFC-063's accepted direction is **Option A: removal only; re-add creates a new
membership.** With a stable principal, `UNIQUE(community_id, user_id)` makes that
impossible: the removed row still occupies the pair.

The three ways out, and why two are rejected:

| Option | Verdict |
|---|---|
| Reactivate the removed membership | **Rejected.** Restores history and role silently; RFC-063 explicitly excluded reactivation, and removal must mean removal. |
| Give the returning person a new `user_id` | **Rejected.** Defeats RFC-080 entirely and splits one human's identity per removal. |
| Allow historical rows; require at most one *active* membership per pair | **Accepted.** |

### 1.2 The replacement

Drop the table constraint; add a partial unique index:

```sql
CREATE UNIQUE INDEX idx_memberships_one_active_per_user
    ON community_memberships(community_id, user_id)
    WHERE removed_at IS NULL;
```

The invariant becomes **at most one active membership per (community, user)**,
with any number of removed historical rows. SQLite enforces partial unique
indexes natively, so this is an index swap, not application-level checking.

### 1.2a Correction, 2026-08-10 — "an index swap" was wrong; the method changed

**The invariant above is unchanged and remains the accepted design.** How it is
reached changed, because the stated method turned out to be impossible under D1.

`UNIQUE(community_id, user_id)` is a **table-level constraint**
(`migrations/0001_initial.sql:29`), not a droppable index, so replacing it
requires rebuilding `community_memberships` — which has **8 dependent tables
across 11 foreign-key columns**. Under D1 that rebuild cannot be performed.
Handoff 051 escalated it as a stop condition, and the following were each tested
and ruled out rather than assumed:

| Mechanism | Result |
|---|---|
| `PRAGMA foreign_keys = OFF` | D1 runs every statement in an implicit transaction; the pragma is a documented no-op there and does not persist across calls |
| `PRAGMA defer_foreign_keys = ON` | Cloudflare's documented substitute. Fails at commit — **reproduced in plain SQLite 3.53**, so this is SQLite semantics, not a D1 quirk: dropping the parent leaves a deferred violation that re-creating the name does not clear |
| `PRAGMA legacy_alter_table = ON` | Verified set (`=1`); SQLite 3.53 still rewrites dependants' FK text on rename, so the old table cannot be orphaned |
| `DROP INDEX sqlite_autoindex_…` | *"index associated with UNIQUE or PRIMARY KEY constraint cannot be dropped"* |
| Explicit `BEGIN`/`COMMIT` | D1 refuses raw SQL transaction control outright |

The conclusion is general: **under D1, a table with dependent rows cannot be
dropped.** This equally blocks RFC-080 §3.4's `users.idp_subject` drop, since
`users` has three dependants with rows.

**Accepted method, owner decision 2026-08-10: re-baseline the initial schema.**
Because the service has never been deployed — no database outside a developer's
machine has ever applied these migrations, confirmed by the owner — the initial
migration is corrected at source rather than migrated forward:
`community_memberships` is created without the table-level `UNIQUE` and with the
partial index; `users` is created without `idp_subject`. Developers reset their
local database. No `0014` exists.

**This is a one-time exception with a hard boundary**, recorded in `ROADMAP.md`:
**migration immutability begins at first deployment.** It must not be cited as
precedent afterwards, when the same edit would silently diverge every deployed
database from the migration history.

Two options were rejected. **Reactivation** — making `removed_at` reversible —
was rejected as permanently lossy: once a row has been un-removed, nothing can
reconstruct which stint an attendance belonged to, or whether someone was a
member on a given date. **Rebuilding the dependent graph** was rejected as
disproportionate: the recursion reaches most of the schema with the authorization
table at its centre. The reversibility that made reactivation tempting is
supplied instead by RFC-082's suspension state, which is additive and needs none
of this.

### 1.3 The policy that goes with it

- A returning recognized person receives a **new membership row under the same
  `users.id`**.
- The removed membership stays removed and **keeps its historical attendance and
  notes**. Nothing is merged, reassigned, or recomputed.
- No removed membership is reactivated. Re-entry requires a **valid invite**,
  exactly as it does today — recognition is not admission.
- Attendance and note history stays attached to the membership that produced it,
  so a member who left and returned sees a fresh history in that community.

**Owner decision (1).** Does a returning member see their prior history in that
community? This RFC says **no** — history belongs to the membership, and a
removal was a real boundary. The alternative is a display-layer join across a
user's memberships, which is a product decision with privacy consequences: an
admin who removed someone would see the prior history reattach itself.

## 2. Community-admin authority — the load-bearing correction

### 2.1 What is wrong once identity is stable

`membership_relink_codes` (`migrations/0008`) is community-scoped: it carries
`community_id`, `membership_id`, and `created_by_membership_id`. But the session
it mints is **account-wide** — `sessions.user_id` is the only scope a session
has.

So with a stable principal, **any single community admin could mint a session
that authorizes every community that person belongs to.** A volunteer admin of
one small group would hold effective access to unrelated groups.

This is the finding that most justifies Stage 2 existing.

### 2.1a Correction, 2026-08-09 — this is live today, not only after RFC-080

The paragraph above was drafted on the assumption that multi-community
membership under one `users.id` arrives with RFC-080. **That is wrong, and the
gap is already reachable.**

- `workers/ssr/src/handlers/community_create.rs:195` passes **`&auth.user_id`** to
  `create_with_first_admin`. A signed-in member who creates a community gains a
  second membership under the **same** `users.id`.
- `workers/ssr/src/authz.rs:40` resolves authorization as
  `find_active(&auth.user_id, community_id)`. A session is scoped to a *user*,
  never to a community, so it authorizes **every** active membership that user
  holds.
- Invite redemption does not produce this, because
  `workers/ssr/src/handlers/join.rs:202` mints a fresh `user_id` per redemption.
  Community creation is the path that does.

**Preconditions for exploitation**, stated so severity is not overstated: a
victim holds active memberships in two communities under one `users.id` (reached
by creating a community while signed in to another); the attacker is an admin of
one of those communities but not the other (reached via RFC-062 promotion); the
attacker generates a relink or help-signin code and redeems it themselves rather
than delivering it. The result is a session that reaches the community the
attacker does not administer.

It is not anonymously exploitable and it needs a specific sequence, but it needs
no new code and no RFC-080. **§2.2's community binding is therefore a fix, not a
precaution**, and it is sequenced first in the implementation slice plan for that
reason.

### 2.2 The fix — capability, not just provenance

RFC-080 adds session provenance. Provenance alone records *how* a session was
made; it does not constrain what the session may reach. This RFC adds the
constraint.

**A relink- or help-signin-derived session is bound to the granting community.**
It carries that `community_id`, and authorization refuses any membership outside
it. Concretely such a session may not:

- reach any community other than the granting one;
- link, unlink, or replace an external identity;
- add, remove, or view recovery credentials; or
- elevate itself by any means short of a fresh first-class authentication.

An admin-mediated recovery therefore restores **access to that admin's
community** and nothing else — which is the authority a community admin actually
has, expressed in the session model rather than assumed by absence.

**Owner decision (2).** Two ways to express this: bind the session to the
community (recommended, and the smaller change given provenance already exists),
or split sessions into account-level and community-level kinds. The second is
cleaner in the abstract and a much larger change to shipped code. This RFC
proposes the first.

## 3. Provider-independent recovery

AD-2 requires no external-account hard wall, so a provider-independent path must
exist and must remain usable — **"recovery-only" must not mean "hidden until an
emergency and then unusable."**

### 3.1 The baseline: a member-held recovery credential

At first identity link, the member is issued a **single-use account recovery
code**, shown once, stored as an HMAC exactly like invite and relink codes
(AD-3). It authenticates the principal at account level, independent of any
provider.

Properties: single-use; regenerable from an authenticated first-class session;
its existence visible in account settings so the member knows it exists; and
never recoverable by an admin, since that would reintroduce §2's problem.

### 3.2 The natural second method

Once a member has linked **two** providers — the expected Google Account and LINE
case — each is a recovery path for the other, and directive 10's "at least one
other verified usable method" is satisfied without the member holding anything.

This is why linking is a day-one concern: with two expected providers, the good
recovery story is available immediately, but only if linking is designed now.

### 3.3 Final-credential unlink is prohibited

Unlink requires recent step-up, explicit confirmation, **at least one other
verified usable authentication or recovery method**, a required audit, and
revocation or rotation of affected sessions. A member may not remove their last
way in.

**Clarified 2026-08-15, owner-confirmed: unlink is permanent for that provider
identity.** This RFC did not say whether an unlinked identity could later be
re-linked. It cannot, and the behaviour is intended rather than incidental —
surfaced by the RFC-054 copy review, which asked whether
`JA_ACCOUNT_UNLINK_BODY`'s 「この操作は取り消せません」 was accurate.

It is accurate, and enforced at two independent levels:

- **Application** — `handlers/identity/mod.rs::link_outcome` calls
  `db::identity::find_by_subject_lookup`, which deliberately does **not** filter
  on `status` (Slice 2's decision: collision policy belongs to the authentication
  transaction, not the accessor). A revoked row is therefore returned, the link is
  treated as a collision, and it fails closed with generic copy.
- **Schema** — `UNIQUE(identity_namespace_id, subject_lookup)` still holds the
  pair, so the insert could not succeed even if the application check were
  bypassed.

The member-facing consequence is that unlinking is a one-way door for that
identity, which is why §3.3's other-usable-method requirement is the load-bearing
protection rather than a formality.

**Owner decision (3).** Is a one-time code shown once acceptable for this
audience? For low-technology-familiarity members it is a known failure point —
people lose it. The alternatives are worse for this product (email needs an
address we deliberately do not collect; admin-held recovery reintroduces §2), but
the copy and the moment of issuance need care, and it is a genuine product risk
rather than a technical one.

## 4. Linking, with two providers expected from the start

A verified identity that resolves to no `user_id` is **never** auto-attached to
an existing principal — not by email, name, avatar, phone, or provider group
(RFC-080 §7). So a member with both a Google Account and LINE becomes two
principals unless linking is explicit.

The flow: from an authenticated first-class session, in account settings, the
member starts a `link` transaction (RFC-080 §5); a fresh provider
authentication in a fresh OIDC transaction; a purpose-bound, user-bound,
single-use link token; uniqueness rejection if that identity is already attached
elsewhere; a required audit; session rotation and revocation of others.

A collision — the identity already belongs to another principal — **fails closed
with generic copy** that does not disclose account existence. No merge. A member
who has genuinely created two accounts needs a future merge RFC, which remains
out of scope.

## 5. Legacy sessions at cutover

Every session existing at implementation time predates provenance. RFC-080
forbids treating a 30-day cookie as sufficient step-up for a permanent link.

The ceremony: sessions without provenance are treated as **lowest assurance**;
they may continue ordinary community use, but any account-level operation —
first link, recovery-credential issuance, unlink — requires a fresh first-class
authentication. Since the only first-class authentication before providers exist
is invite redemption, in practice a member links from a session created after
cutover, or through a bounded, audited admin-assisted path that §2 already
constrains to one community.

**Owner decision (4).** Whether to expire all pre-cutover sessions at
implementation instead. That is cleaner and forces every member through one
re-authentication — a real cost for a volunteer community, and only worth it if a
pilot has begun. If no real community has used the service by then, this decision
is free, which is another argument for doing the identity track before a pilot.

## 6. Account with no active memberships

A recognized principal with no active membership reaches a **minimal account
surface only**: view and manage linked identities, manage recovery credentials,
delete the account. No community data, no member lists, no event data, no
disclosure of which communities exist or once existed.

## 7. Provider loss or compromise

- **Provider outage:** existing application sessions survive; new sign-ins and
  links fail closed (RFC-080 §5.2).
- **Verified provider compromise, account disable, or invalid Apple transfer
  state:** revoke all local sessions for the affected principal.
- **Provider logout alone** does not revoke local sessions unless a
  provider-specific contract requires it.
- **A member losing access to their provider account** uses the §3 recovery
  credential or a second linked identity. There is no admin path to account-level
  recovery — by design, per §2.

## 8. Audit

Extending RFC-080's additions, all Class A: recovery credential issued,
regenerated, or consumed; identity link rejected for collision; admin-derived
session refused an account-level operation.

The last one is deliberately audited on *refusal*. A community admin attempting an
account-level operation is the exact misuse §2 exists to prevent, and it should be
visible rather than merely blocked.

Standard prohibitions carry over: no raw subject, issuer, email, token, code,
`nonce`, `state`, PKCE material, or session ID in metadata.

## 9. Member-facing copy

Recovery, collision, and cancellation copy is plain Japanese and must not
disclose account existence, which provider is linked, invite validity, or any
internal identifier. It must not present the provider-independent path as
second-class — no dark pattern favouring provider adoption (consultation §UX
direction).

Copy quality here overlaps RFC-054's scope; the two should be reviewed together
if RFC-054 runs first.

## 10. Acceptance criteria

1. `UNIQUE(community_id, user_id)` replaced by a partial unique index on
   `removed_at IS NULL`; historical removed rows permitted.
2. A returning recognized person gets a new membership under the same `users.id`;
   no reactivation; prior history stays on the removed membership; a valid invite
   is still required.
3. Relink- and help-signin-derived sessions are bound to the granting community
   and refused every account-level operation, enforced and tested.
4. A provider-independent recovery credential exists, is member-held, is visible
   in account settings, and is never admin-recoverable.
5. Final-credential unlink is impossible; unlink requires step-up, another usable
   method, audit, and revocation.
6. Linking is explicit, step-up-gated, collision-safe, and never automatic.
7. Legacy sessions are lowest-assurance and cannot perform account-level
   operations.
8. A principal with no active membership reaches only the minimal account
   surface.
9. Audit inventory extended, including the refusal case; `AuditAction::ALL`
   updated.
10. All routes work with no application JavaScript.

## 11. Owner decisions carried in this RFC

**Resolved by acceptance, 2026-08-09** — each was drafted with a stated position,
and acceptance adopts it:

1. **§1.3** — returning members do **not** see prior community history; history
   stays attached to the removed membership.
2. **§2.2** — relink- and help-signin-derived sessions are **bound to the
   granting community**, rather than sessions being split into account-level and
   community-level kinds.
3. **§3.3** — a **one-time member-held recovery code** is the provider-independent
   baseline. Recorded as carrying real product risk for
   low-technology-familiarity members: people lose one-time codes. The
   alternatives were rejected as worse for this product, not as risk-free.

**Still open — no default was drafted:**

4. **§5 — whether to expire all pre-cutover sessions at implementation.** This is
   deliberately deferred to implementation time, because its answer depends on a
   fact not yet known: **whether any real community has used the service by
   then.**

   The decision trigger, for whoever writes the implementation handoff:

   - **No real community has used the service** → expire all pre-cutover
     sessions. The cost is zero and it removes the legacy-assurance class
     entirely, so §5's ceremony never has to run.
   - **A real community has used the service** → keep §5's ceremony; forcing every
     member of a volunteer community through re-authentication is a real cost that
     needs owner sign-off at that moment.

   This must be answered explicitly in the implementation handoff, not assumed.

## 12. What acceptance does not authorize

No implementation, provider selection, registration, secret provisioning, hosted
callback, or deployment. B1, B3, B4, and B5 remain open; production,
public-pilot, and first-real-community deployment remain **No-Go**.

Acceptance of **both** this RFC and RFC-080 is the precondition for an
implementation handoff — and even then, provider rollout is Stage 3 and requires
the user research that does not yet exist.
