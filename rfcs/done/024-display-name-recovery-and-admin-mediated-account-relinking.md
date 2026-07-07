# RFC 024 — Display Name Recovery and Admin-Mediated Account Relinking

**Status.** Implemented (v0.51.0)
**Phase:** F3 / Identity and Recovery  
**Project:** ciao.zinnias  
**Date:** 2026-07-07
**Relationship:** Continuation RFC; follows RFC-003 session auth, RFC-004
community isolation, RFC-010 admin invite/member management, RFC-061 member
management navigation, RFC-062 admin role transfer, and RFC-063 member removal
policy.
**Design review:** `.git-exclude/reviewed/zinnias-ciao-v0.51.0-rfc024-account-relinking-design-review.md`

---

## 1. Summary

This RFC defines the invite-era lost-session recovery flow for ciao.zinnias.

The accepted v0.51.0 design direction is narrow: an active community admin can
help an active member sign in again after that member loses their browser,
phone, or session cookie. The admin issues a short-lived one-time code for a
specific active membership. The returning member redeems the code on a new
browser and receives a new session for the existing user identity behind that
membership.

The user-facing framing is "help sign in again", not "restore account",
"reactivate", "relink account", or "recover account". This is deliberate. The
feature does not prove real-world identity, does not restore removed members,
does not merge memberships, and does not infer identity from display names.

This RFC keeps RFC-063 intact: removed members still return through a normal new
invite and a new membership unless a later RFC explicitly accepts
removed-membership reactivation.

## 2. Problem Statement

The MVP intentionally avoids email/password login and does not yet use OIDC.
Invite redemption creates a browser session. If a member loses that session,
the product currently has no way to attach the new browser to the old
membership identity.

The support problem is real:

- a member can lose a phone, clear browser data, or lose their session cookie;
- the app cannot cryptographically know that a returning browser belongs to the
  same person;
- a community admin may personally know the member and want to restore access;
- asking the person to join again creates a duplicate membership and splits
  history;
- automatic recovery based on display name would enable impersonation.

The product needs a conservative admin-mediated recovery path that is honest
about its trust model and limited enough for small trusted communities.

## 3. Current Identity Model

The current invite-era model has these load-bearing facts:

- invite redemption creates a fresh random `users.id`;
- invite redemption creates one `community_memberships` row for that user;
- sessions bind to `users.id`;
- authorization is checked per community by looking for an active membership
  for the current session user;
- `community_memberships.removed_at IS NULL` is required for active access;
- display names are stored on memberships but are not unique;
- the join flow does not look up old memberships by display name.

In practice today, each invite-era `user_id` corresponds to one community
membership. Community isolation for the new-session recovery flow depends on
that invariant.

The schema does not guarantee the invariant forever. `UNIQUE(community_id,
user_id)` allows one user to hold memberships in multiple communities, and
`users.idp_subject` anticipates later stronger identity work. Therefore this RFC
must target and audit against `membership_id`, and redemption must re-check the
membership's community before minting the session. Any future multi-community
identity or OIDC RFC must revisit this recovery flow before a single `user_id`
can safely span communities.

## 4. Goals

- Let admins help an active member sign in again after session loss.
- Keep the flow admin-mediated; no self-service identity recovery.
- Target a specific active membership, not a display name.
- Store only HMAC-hashed recovery codes.
- Keep codes short-lived and single-use.
- Revoke old sessions after successful redemption.
- Preserve community isolation and generic denial behavior.
- Record auditable successful admin and member actions.
- Keep copy plain, calm, and non-technical.
- Preserve RFC-063 removal/re-add behavior.

## 5. Non-Goals

- No email/password accounts.
- No OIDC implementation.
- No biometric or legal identity proof.
- No automatic merge by display name.
- No removed-member reactivation.
- No former-member list.
- No "undo removal" flow.
- No support/operator impersonation.
- No cross-community recovery.
- No persistent failed-redemption audit trail.
- No QR-code flow in the first slice.

## 6. External Behavior

### 6.1 Admin Creates a Help-Signin Code

On `/c/:cid/admin/members`, an active admin sees a row action for each eligible
active member:

```text
サインインを手伝う
```

English equivalent:

```text
Help sign in again
```

The action opens a confirmation page:

```text
サインインし直すお手伝いをしますか？

このコードを使うと、このメンバーとしてサインインできます。
本人にだけ渡してください。
コードは15分で使えなくなり、1回だけ使えます。

[コードを作成]
[やめる]
```

The generated code is shown once. The UI may provide a copy button. QR-code
support is deferred.

### 6.2 Member Redeems the Code

The member opens a dedicated redemption page:

```text
GET  /relink
POST /relink
```

The route is intentionally separate from `/join`. Joining creates a new
membership. Helping sign in again restores browser access to an existing active
membership identity.

On successful redemption:

- the app creates a new session for the target membership's existing `user_id`;
- other active sessions for that `user_id` are revoked;
- the code is marked used;
- the member is redirected to `/`.

### 6.3 Invalid, Used, Expired, or Wrong Codes

All failed redemption cases must show the same generic error:

```text
このコードは無効か、有効期限が切れています。
```

English equivalent:

```text
This code is invalid or has expired.
```

The UI must not reveal whether a code existed, was already used, was revoked,
targeted another community, or targeted a removed membership.

## 7. Data Model

Add `membership_relink_codes`:

```sql
CREATE TABLE membership_relink_codes (
  id TEXT PRIMARY KEY,
  code_hmac TEXT NOT NULL UNIQUE,
  community_id TEXT NOT NULL REFERENCES communities(id),
  membership_id TEXT NOT NULL REFERENCES community_memberships(id),
  created_by_membership_id TEXT NOT NULL REFERENCES community_memberships(id),
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  used_at TEXT,
  revoked_at TEXT
);
```

Required indexes:

```sql
CREATE INDEX idx_membership_relink_codes_membership_active
  ON membership_relink_codes(membership_id, used_at, revoked_at, expires_at);

CREATE INDEX idx_membership_relink_codes_community_created
  ON membership_relink_codes(community_id, created_at);
```

Data rules:

- `code_hmac` stores an HMAC of the raw code using the same pepper discipline as
  invite codes, sessions, and form tokens.
- The raw code is shown once and never stored.
- `community_id` is redundant by design; it supports community-scoped
  authorization and the defensive redemption re-check.
- `membership_id` is the target. Redemption resolves the target membership to
  its current `user_id`.
- Do not snapshot display name into the code record.
- Do not store removal reasons or free-text notes.
- Creating a new unused code for the same membership revokes prior unused codes
  for that membership by setting `revoked_at`.
- Codes expire after 15 minutes.
- Codes are single-use through `used_at`.

## 8. Authorization and Domain Rules

Code creation:

- requires `require_admin` for the URL community;
- uses a confirmation page and form token;
- targets only an active membership in the same community;
- denies removed, absent, and cross-community memberships using the same safe
  denial style as existing admin member routes;
- is allowed for the last remaining admin because it does not change roles or
  membership state.

Code redemption:

- rate-limits attempts by client IP using the existing rate-limit module;
- looks up the HMAC-hashed code;
- requires `used_at IS NULL`, `revoked_at IS NULL`, and `expires_at > now`;
- resolves `membership_id` to an active membership;
- re-checks that the membership's `community_id` matches the code's
  `community_id`;
- creates a new session for the membership's existing `user_id`;
- revokes all other active sessions for the same `user_id`;
- marks the code used.

Old-session policy is fixed:

- never revoke sessions when a code is created;
- always revoke other sessions for the target `user_id` after successful
  redemption.

This avoids locking out a member when an admin creates a code that is never
used, while still closing access on the presumed lost browser/device after the
member signs in again.

## 9. Security, Privacy, and Safety

### 9.1 Trust Model

This feature is a social-trust recovery flow. It does not prove legal identity.
The admin is responsible for confirming out-of-band that the person receiving
the code is the intended member.

The confirmation page must make the risk clear: whoever receives the code can
sign in as that member.

### 9.2 No Display-Name Recovery

The implementation must not look up, merge, or restore memberships by display
name. Display name is only a label shown to help an admin choose the intended
active member row.

### 9.3 No Removed-Member Reactivation

Removed memberships cannot receive help-signin codes in the first slice. This
feature never clears `removed_at` and never exposes a former-member list.

If a removed person should return, RFC-063 still applies: an admin sends a
normal invite and the person joins as a new membership.

### 9.4 No Existence Leaks

Redemption errors must be generic. The app must not reveal whether a code was
real, expired, already used, revoked, tied to another community, or tied to a
membership that is no longer active.

### 9.5 Logging and Audit

Audit logs must not contain raw codes, hashed codes, or unnecessary personal
data.

Record two successful actions:

- `membership.relink_code_created`
- `membership.relink_redeemed`

Metadata should contain ids only:

- `membership_id`;
- `created_by_membership_id`;
- `community_id`.

Failed redemption attempts should not create audit rows. Use rate limiting and
operational observability instead, to avoid creating enumeration-prone records.

## 10. UI and Copy Contract

Avoid these user-facing terms in the new surface:

- restore;
- reactivate;
- relink account;
- recover account;
- suspend.

Use:

- `サインインを手伝う`;
- `サインインし直すお手伝い`;
- `Help sign in again`.

The member-management row is the first-slice admin entry point:

```text
GET  /c/:cid/admin/members/:mid/help-signin
POST /c/:cid/admin/members/:mid/help-signin
```

The member redemption page is:

```text
GET  /relink
POST /relink
```

No link from `/join` is required in the first slice. The join and help-signin
flows should remain visually and conceptually separate.

## 11. Release Gates

The v0.51.0 implementation must add gates for:

- JA and EN "help sign in again" strings exist;
- new relink/help-signin handlers and routes do not use the forbidden
  user-facing terms `restore`, `reactivate`, or `suspend`;
- redemption resolves `membership_id` to `user_id` and re-checks `community_id`;
- redemption contains no membership lookup by `display_name`;
- code creation cannot target removed memberships;
- relink TTL is 15 minutes and defined as a named constant;
- single-use behavior is enforced through `used_at`;
- redemption revokes other sessions for the target `user_id`;
- the RFC-063 no-restore/no-reactivate/no-suspend policy remains intact for
  removed members.

## 12. Test Plan

Automated coverage should include:

- admin can create a code for an active member in the same community;
- admin cannot create a code for a removed member;
- admin cannot create a code for a member of another community;
- non-admin cannot create a code;
- new code creation revokes prior unused code for the same membership;
- expired code cannot be redeemed;
- used code cannot be redeemed twice;
- revoked code cannot be redeemed;
- redemption creates a session for the target membership's existing `user_id`;
- redemption revokes other active sessions for that `user_id`;
- redemption re-checks membership activity and community;
- redemption does not look up by display name;
- failed redemption responses are generic;
- audit rows are written for successful creation and redemption only.

Browser smoke should cover local no-JS SSR behavior at 390px width and 200%
text:

1. Admin creates a help-signin code from an active member row; confirmation copy
   fits and warns that the code grants that member's access.
2. The generated code is shown once.
3. A fresh browser context redeems the code and lands signed in at `/`.
4. Reused or expired code shows the generic error.
5. A removed member row exposes no help-signin action.
6. If practical, a code does not authorize a different community.

## 13. Documentation

Add or update an operations/admin document explaining:

- this flow is for active members who lost browser/session access;
- the code lets someone sign in as that member;
- the code is single-use and expires after 15 minutes;
- the code should only be shared with the intended member;
- removed members are not restored through this flow;
- returning removed members still join through a new invite and a new
  membership under RFC-063.

## 14. Rollout Plan

1. Add schema migration and domain logic.
2. Add admin creation route and dedicated redemption route.
3. Add i18n copy and release gates.
4. Add automated tests for authorization, lifecycle, audit, and denial behavior.
5. Add browser smoke coverage.
6. Run release gates before moving this RFC to `rfcs/done/`.

## 15. Deferred Work

- Removed-member reactivation.
- Former-member visibility.
- QR-code sharing.
- Stronger identity/OIDC recovery.
- Multi-community identity semantics.
- Admin-configurable recovery policy.
- Support/operator recovery tooling.
