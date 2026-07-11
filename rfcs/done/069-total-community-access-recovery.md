# RFC 069 - Total Community Access Recovery

**Status.** Done
**Target release.** v0.59.0
**Tracks.** Operations, account recovery, community administration, security.
**Touches.** Operational runbooks, admin recovery policy, D1 repair tools,
audit logging, possibly future user-facing recovery flows.

## Summary

Define a safe recovery path for the case where every active community admin and
member loses browser/session access to the application.

The initial direction is an operator-only recovery path, not public
self-service recovery. The first implementation should create a short-lived
one-time relink code for an existing active admin membership through a
disabled-by-default operator endpoint and a maintained operator script.

## Background

Current recovery paths are intentionally scoped:

- an unused valid invite code can let a person re-enter through `/join`;
- an active admin can generate invite codes or help-signin codes;
- admin-mediated relinking can recover a member while an admin still has
  access;
- bootstrap scripts can seed an initial admin/invite for a new environment.

However, if all active admins and members lose their sessions and no valid
invite/relink code exists, normal app workflows cannot restore access. That is
a real operational gap. It should be handled explicitly instead of being
discovered during an incident.

One important platform constraint shapes this RFC: Wrangler secrets cannot be
read back from Cloudflare. A D1-only operator script cannot mint normal invite,
relink, or session bearer credentials unless the operator already has the
current `HMAC_PEPPER` in a separate secret store. Rotating `HMAC_PEPPER` during
routine access recovery would invalidate all existing sessions, invite codes,
relink codes, and form tokens for the environment. That is too broad for the
first recovery design.

## Problem

When a community has no reachable signed-in admin and no valid entry code,
normal app workflows cannot restore access. The Worker can still mint a
relink-style recovery code because it can access `HMAC_PEPPER`, but no
operator-authenticated route, policy, runbook, or maintained tool exists for
that purpose.

## Goals

- Define who may authorize total community access recovery.
- Prefer a safe operator/system-admin procedure before adding public
  self-service recovery.
- Recover an existing active admin membership first, rather than creating a new
  admin identity as the default path.
- Avoid creating an account-takeover route that bypasses community
  authorization.
- Keep recovery actions auditable.
- Preserve community isolation and avoid cross-community disclosure.
- Define the minimum data needed to identify the target community and recovered
  admin.
- Document operational steps for staging and production.
- Avoid rotating `HMAC_PEPPER` for routine access recovery.

## Non-Goals

This RFC does not immediately require:

- public password-style account recovery;
- email-based recovery;
- SMS or external identity verification;
- automatic recovery for every lost session;
- weakening invite-code or help-signin security;
- exposing operator-only controls to ordinary community members.
- recovering a community that has no active admin membership in the database;
- restoring removed memberships;
- merging old and new user identities;
- changing member roles as part of recovery.

Email/contact-based recovery may be considered later after RFC-031 consentful
contact channels defines contact data, verification, and consent boundaries.

## Decision

Use a maintained operator-assisted relink flow:

1. A disabled-by-default Worker endpoint is enabled only for an operator
   recovery window.
2. A maintained script calls that endpoint with an operator bearer token.
3. The endpoint validates the target community and target active admin
   membership.
4. The endpoint creates a normal short-lived relink code for that admin
   membership.
5. The script prints the plaintext relink code once.
6. The operator gives the code to the intended admin, who signs in through the
   existing `/relink` flow.

This intentionally reuses the existing RFC-024 redemption behavior: relink code
redemption targets a membership, re-checks that the membership is active and in
the same community, mints a new session for the membership's user, and revokes
other active sessions for that user.

## Rejected or Deferred Alternatives

### Manual D1 invite insertion

Rejected as the first path. The operator would need the current `HMAC_PEPPER`
to compute `invite_codes.code_hmac`, but Cloudflare secrets cannot be read back
from Wrangler. Rotating `HMAC_PEPPER` just to recover one community is too
broad.

### New admin invite as the default

Deferred. An admin invite creates a new user and membership. For total session
loss, recovering an existing active admin membership is a narrower and more
auditable first step.

### Direct session minting

Deferred. A direct session mint would require the operator or tool to handle a
session cookie secret and cookie attributes. Reusing `/relink` keeps the
browser/session behavior inside the existing app path.

### Pepper rotation plus bootstrap

Rejected for routine recovery. This is suitable only for planned credential
rotation or fresh environment bootstrap because it invalidates existing
sessions, invite codes, relink codes, and form tokens for the whole
environment.

### Pre-generated emergency recovery codes

Deferred. This is a user-facing admin feature and requires careful UX around
offline storage, rotation, expiration, and sharing.

### Contact-based recovery

Deferred. This depends on RFC-031 because the app does not yet have consentful
verified contact channels.

## Operator Endpoint Contract

Add a POST-only operator route:

```text
POST /operator/recovery/community-access
```

The route must be unavailable unless all of these are true:

- `COMMUNITY_RECOVERY_ENABLED` is set to `"true"` in the active environment;
- `COMMUNITY_RECOVERY_TOKEN` is configured as a Worker secret, generated with
  high entropy such as `openssl rand -hex 32`;
- the request includes `Authorization: Bearer <token>`;
- the presented token matches the configured secret using constant-time
  comparison.

When unavailable or unauthorized, the route must return the generic not-found
response. It must not reveal whether operator recovery is configured, whether a
community exists, or whether a membership id is valid.

Request body:

```json
{
  "community_id": "com_...",
  "admin_membership_id": "mem_...",
  "operator_label": "incident-ticket-or-operator-label"
}
```

Validation:

- `community_id` must identify an active community;
- `admin_membership_id` must identify an active membership in that community;
- the membership role must be `admin`;
- the target membership must not be removed;
- `operator_label` is required and must be short, bounded plain text, such as
  an incident id, ticket id, or operator handle;
- `operator_label` must be free of secrets, notes, contact data, and arbitrary
  private text.

Mutation:

- revoke previous unused relink codes for the target admin membership;
- create one new `membership_relink_codes` row using the existing relink TTL;
- store only `code_hmac`, never plaintext code;
- use the target admin membership id as `created_by_membership_id` only because
  the existing table requires a membership FK; the audit action must make clear
  that this was operator-initiated;
- write an audit row with action
  `operator_recovery.admin_relink_created`;
- audit target should be the recovered admin membership;
- audit metadata must include `operator_label`, `relink_code_id`,
  `membership_id`, and `community_id`;
- audit metadata must not include plaintext code, code HMAC, bearer token,
  pepper, session data, invite data, note bodies, or arbitrary free-form
  private text.

Response:

```json
{
  "ok": true,
  "community_id": "com_...",
  "admin_membership_id": "mem_...",
  "expires_at": "2026-07-11T00:00:00.000Z",
  "relink_code": "ABC123"
}
```

The plaintext `relink_code` is returned once. The Worker must not log it.

## Maintained Operator Tool Contract

Add a maintained script:

```text
scripts/recover-community-access.mjs
```

Expected invocation:

```sh
COMMUNITY_RECOVERY_TOKEN="<operator-token>" \
node scripts/recover-community-access.mjs \
  --target staging \
  --url https://<staging-worker-url> \
  --community-id com_... \
  --admin-membership-id mem_... \
  --operator-label "INC-1234"
```

Production must require an explicit confirmation flag and interactive prompt:

```sh
COMMUNITY_RECOVERY_TOKEN="<operator-token>" \
node scripts/recover-community-access.mjs \
  --target production \
  --url https://<production-worker-url> \
  --community-id com_... \
  --admin-membership-id mem_... \
  --operator-label "INC-1234" \
  --confirm-production
```

Tool rules:

- require explicit target environment;
- require explicit Worker URL;
- require explicit community id and admin membership id;
- refuse production unless `--confirm-production` is present and the operator
  confirms interactively, unless a later reviewed CI mode is added;
- read the bearer token only from `COMMUNITY_RECOVERY_TOKEN`;
- never print the bearer token;
- print the recovery relink code once;
- print the expiration time and `/relink` URL;
- avoid writing evidence files containing the plaintext code;
- exit non-zero for generic failures without echoing secrets.

## Operator Runbook

Staging and production runbooks must document:

1. Confirm the recovery request through an out-of-band operator policy.
2. Identify the target community by id, not only by display name.
3. Identify the intended existing active admin membership.
4. Review recent `audit_log` rows for suspicious activity before mutation.
5. Enable `COMMUNITY_RECOVERY_ENABLED = "true"` in the ignored local Wrangler
   config for the target environment.
6. Set `COMMUNITY_RECOVERY_TOKEN` as a Worker secret for the target environment.
7. Deploy the temporary recovery-enabled Worker.
8. Run `scripts/recover-community-access.mjs`.
9. Give the printed relink code only to the intended admin.
10. After successful recovery, close the temporary window:
    - set `COMMUNITY_RECOVERY_ENABLED = "false"` in the ignored local Wrangler
      config;
    - delete or rotate `COMMUNITY_RECOVERY_TOKEN`;
    - redeploy the environment;
    - verify `POST /operator/recovery/community-access` returns the same
      generic not-found response.
11. Review `audit_log` for
    `operator_recovery.admin_relink_created` and subsequent relink redemption.

Runbooks must also state that `scripts/bootstrap-cloudflare.mjs` is not a
routine community recovery tool because it rotates `HMAC_PEPPER`.

## Security Considerations

- Recovery must not reveal whether arbitrary users or communities exist to
  unauthenticated callers.
- Plaintext recovery codes must not be logged, committed, or printed except as
  the intended one-time operator output.
- Audit logs should record recovery action, target community, target membership,
  relink code id, bounded operator label, and safe metadata only.
- Production recovery should require explicit operator confirmation.
- Staging recovery must use staging resources and non-production data only.
- Existing sessions, invite codes, and relink codes should not be weakened.
- The endpoint must be disabled by default and removed from public reachability
  after the recovery window by configuration and redeploy.
- The operator bearer token is separate from `HMAC_PEPPER`; it authorizes only
  the temporary recovery route and must not be reused as any other credential.
- Recovery should prefer an existing active admin membership. If no active
  admin membership exists, this RFC does not authorize creating a new admin as
  a routine path.

## Reviewed Implementation Constraints

- The first implementation is JSON-only.
- `operator_label` is required for both staging and production. The production
  tool must also refuse to run without `--confirm-production` and interactive
  confirmation.
- Production recovery does not require a special pre-mutation export in this
  slice because the mutation is limited to one relink code. Operators must still
  review recent audit rows and follow the normal D1 backup policy.
- Tracked `wrangler.toml` must keep `COMMUNITY_RECOVERY_ENABLED = "false"` by
  default, including production. Temporary staging or production enablement
  belongs only in ignored local Wrangler config for the recovery window.
- The endpoint must read `COMMUNITY_RECOVERY_TOKEN` only via `env.secret`, not
  as a plain var.
- Missing/disabled config, missing secret, invalid bearer token, wrong method,
  invalid community id, invalid membership id, non-admin membership, and removed
  membership must all converge on the generic not-found response where
  practical.

## Acceptance Criteria

RFC-069 can be considered complete when:

- a reviewed recovery policy exists for total community access loss;
- staging and production runbooks describe the recovery path;
- a maintained operator recovery script is explicit about environment, target
  Worker URL, target community, target admin membership, and production
  confirmation;
- a disabled-by-default operator endpoint can create one short-lived relink
  code for an existing active admin membership;
- recovery actions are auditable without logging plaintext secrets;
- release gates cover disabled-by-default behavior, authorization, active-admin
  targeting, audit action name, and absence of plaintext secrets from metadata;
- tool checks cover production confirmation and secret-handling behavior;
- user-facing self-service recovery is either explicitly deferred or separately
  designed.
