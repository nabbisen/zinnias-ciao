# RFC 069 - Total Community Access Recovery

**Status.** Proposed  
**Target release.** Unscheduled  
**Tracks.** Operations, account recovery, community administration, security.  
**Touches.** Operational runbooks, admin recovery policy, D1 repair tools,
audit logging, possibly future user-facing recovery flows.

## Summary

Define a safe recovery path for the case where every active community admin and
member loses access to the application.

The initial direction is not to force a self-service user feature. A
system-admin or system-operator recovery procedure is acceptable and may be the
safer first implementation, because total community access recovery is a high
risk account-takeover surface.

## Background

Current recovery paths are intentionally scoped:

- an unused valid invite code can let a person re-enter through `/join`;
- an active admin can generate invite codes or help-signin codes;
- admin-mediated relinking can recover a member while an admin still has
  access;
- bootstrap scripts can seed an initial admin/invite for a new environment.

However, if all active admins and members lose their sessions and no valid
invite/relink code exists, there is no in-app self-service recovery path. That
is a real operational gap. It should be recorded explicitly instead of being
discovered during an incident.

## Problem

When a community has no reachable signed-in admin and no valid entry code,
normal app workflows cannot restore access. The operator may still be able to
repair access through D1 and the environment `HMAC_PEPPER`, but this is not yet
formalized as a policy, runbook, tool, or auditable workflow.

## Goals

- Define who may authorize total community access recovery.
- Prefer a safe operator/system-admin procedure before adding public
  self-service recovery.
- Avoid creating an account-takeover route that bypasses community
  authorization.
- Keep recovery actions auditable.
- Preserve community isolation and avoid cross-community disclosure.
- Define the minimum data needed to identify the target community and recovered
  admin.
- Document operational steps for staging and production.

## Non-Goals

This RFC does not immediately require:

- public password-style account recovery;
- email-based recovery;
- SMS or external identity verification;
- automatic recovery for every lost session;
- weakening invite-code or help-signin security;
- exposing operator-only controls to ordinary community members.

Email/contact-based recovery may be considered later after RFC-031 consentful
contact channels defines contact data, verification, and consent boundaries.

## Candidate Approaches

### Option A - Operator-Only Recovery Runbook

Define a manual procedure for a trusted system operator to create a new admin
invite or help-signin-equivalent recovery code for an existing community.

Expected properties:

- requires production operator access;
- uses the environment's current `HMAC_PEPPER`;
- writes only the minimum D1 rows needed for recovery;
- records an audit row describing the operator recovery event;
- does not expose a public route.

### Option B - Maintained Operator Tool

Create a script similar in spirit to `scripts/bootstrap-cloudflare.mjs`, but
targeting an existing community rather than creating the first community.

Expected properties:

- requires explicit target environment and ignored Wrangler config;
- prints a one-time recovery invite/code only once;
- refuses production unless explicitly confirmed;
- uses remote D1 and Wrangler-managed secrets;
- writes audit metadata that does not include the plaintext code.

### Option C - Pre-Generated Emergency Recovery Codes

Allow community admins to generate offline emergency codes while they still
have access.

Expected properties:

- user-facing admin feature;
- codes must be single-use, expiring or rotatable, and safely stored by admins;
- requires careful UX to prevent casual sharing or insecure storage.

This may be useful later but is not the safest first step.

### Option D - Contact-Based Recovery

Recover through verified email or another consentful contact channel.

Expected properties:

- depends on RFC-031;
- requires verified contact ownership;
- requires abuse prevention and privacy review.

This should not be implemented before contact-channel policy exists.

## Initial Preference

Start with Option A or B: operator/system-admin recovery.

Reasoning:

- total access loss is expected to be rare;
- operator-controlled recovery avoids publishing a new takeover-sensitive
  endpoint;
- existing bootstrap and deployment runbooks already establish an operator
  trust boundary;
- user-facing recovery can be considered later with more identity and contact
  infrastructure.

## Security Considerations

- Recovery must not reveal whether arbitrary users or communities exist to
  unauthenticated callers.
- Plaintext recovery codes must not be logged, committed, or printed except as
  the intended one-time operator output.
- Audit logs should record recovery action, target community, operator context
  where available, and safe metadata only.
- Production recovery should require explicit operator confirmation.
- Staging recovery must use staging resources and non-production data only.
- Existing sessions, invite codes, and relink codes should not be weakened.

## Open Questions

- Should the first implementation be a documented manual runbook, a maintained
  script, or both?
- How should an operator select the target community safely: community id,
  community name plus confirmation, or another identifier?
- Should recovery create an admin invite code, a one-time relink-style code for
  an existing admin membership, or a new admin membership?
- What audit action name should be used?
- How should production operator identity be represented in audit metadata when
  the action is performed outside the Worker request path?
- Should the runbook require a backup/export before recovery mutation?

## Acceptance Criteria

RFC-069 can be considered complete when:

- a reviewed recovery policy exists for total community access loss;
- staging and production runbooks describe the recovery path;
- any maintained tool is explicit about environment, target community, and
  production confirmation;
- recovery actions are auditable without logging plaintext secrets;
- release gates or smoke tests cover the tool/policy where practical;
- user-facing self-service recovery is either explicitly deferred or separately
  designed.
