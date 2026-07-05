# RFC 057 — Community Creation and Bootstrap Flow

**Status.** Implemented (v0.41.0)
**Phase:** F8 / Community workflow improvement
**Project:** ciao.zinnias
**Date:** 2026-07-02
**Shipped in:** v0.41.0
**Relationship:** Extends RFC-002 (multi-community data model), RFC-003
(invite-only onboarding), RFC-004 (community isolation), RFC-010 (admin invite
and member management), RFC-030 (first-run admin onboarding), and RFC-056
(multi-community Home).
**Review:** `.git-exclude/reviewed/zinnias-ciao-v0.41.0-rfc057-community-creation-review.md`

---

## 1. Summary

v0.41.0 adds in-app creation for additional communities, but only as a narrow
trusted-admin workflow.

An authenticated user who is already an active admin in at least one community
may create a new private community when the operator feature flag is enabled.
The creator becomes the first admin of the new community. The app then redirects
to the new community Home, where existing first-run admin actions guide the user
to create the first event or invite members.

Anonymous users cannot create communities. First production and staging
community bootstrap remains operator-controlled through the launch runbook.

## 2. Goals

- Let trusted in-app admins create an additional private community.
- Preserve the invite-only, no-password, no-public-registration product model.
- Keep first community bootstrap operator-controlled.
- Make the creator the first admin of the new community.
- Do not copy members, events, templates, invite codes, calendar tokens, notes,
  attendance, or other data from any existing community.
- Keep the flow server-rendered and no-JS compatible.
- Keep creation online-only.
- Add explicit audit events and release gates for the new trust-boundary action.
- Add rate limiting/quota and an operator feature flag before production use.
- Keep Japanese copy simple for non-technical community organizers.

## 3. Non-Goals

- No anonymous public self-service signup.
- No public request-to-create workflow.
- No email verification, password, passkey, OIDC, or billing flow.
- No public community discovery.
- No cloning of an existing community.
- No automatic invite-code generation.
- No community deletion/archive self-service.
- No role transfer redesign.
- No subgroup or event visibility changes.
- No DST timezone expansion.

## 4. Policy Decision

RFC-057 uses the reviewed **Option C-prime** policy:

```text
Authenticated active-admin creation, guarded by an operator feature flag, with
rate limiting and audit.
```

Eligibility for v0.41.0:

- user is authenticated;
- user has at least one active membership;
- user is an admin in at least one active community;
- feature flag permits community creation;
- user/session/IP passes rate limiting and pilot quota.

This intentionally rejects:

- any-active-member creation for v0.41.0;
- anonymous first-community bootstrap;
- public self-service creation.

## 5. First-Community Bootstrap Policy

First production and staging community bootstrap remains operator-controlled.

The operator continues to use the documented runbook/setup path to create the
first community and first admin. RFC-057 covers only additional community
creation by already-trusted in-app admins.

The first-community bootstrap path must not become reachable through
`/communities/new` in v0.41.0.

## 6. Anonymous Visitor Policy

Anonymous visitors cannot open or submit the create-community flow.

Route behavior:

- anonymous `GET /communities/new`: redirect to `/join` or return the existing
  safe not-found/session-expired response;
- anonymous `POST /communities/new`: no mutation, generic safe response;
- `/join` remains the anonymous public entry point.

Do not add a public "start now" call to action in v0.41.0. If quiet help text
is later desired on `/join`, use a separate product decision. Candidate copy:

```text
コミュニティを始めたい場合は、運営者にお問い合わせください。
```

## 7. Routes and Auth Chain

Preferred route shape:

```text
GET  /communities/new
POST /communities/new
```

The route is not community-scoped because the target community does not exist
yet.

Required auth chain:

```text
require_auth
  -> require_active_admin_somewhere
  -> feature flag check
  -> rate limit / quota check
  -> form token issue or consume
```

The authorization check is server-side. UI hiding is not sufficient.

## 8. Entry Point

Primary entry point:

- Me page.

Eligible active admins see a quiet action:

```text
新しいコミュニティを作る
```

Ineligible users should not see the action. Do not show a disabled control
unless there is a clear human-readable reason.

Do not place community creation on Home or Calendar in v0.41.0. Those pages are
member workflow surfaces after RFC-056.

An additional quiet link from an admin tools area is acceptable if it does not
compete with existing event/member management actions.

## 9. Create Community Screen

The form asks only for:

1. community name;
2. time setting, fixed to Japan time or a restricted supported choice;
3. user's display name in the new community.

Proposed Japanese form shape:

```text
新しいコミュニティを作る

コミュニティ名
[________]

予定の時間帯
日本時間

このコミュニティでのあなたの名前
[________]

作成すると、あなたは予定の作成や招待ができる人になります。
ほかのコミュニティのメンバーや予定はコピーされません。

[作成する]
[やめる]
```

The copy explains admin power through concrete actions instead of abstract role
language.

## 10. Field Validation

Community name:

- required;
- trimmed;
- plain text;
- suggested maximum: 80 characters;
- escaped on display;
- no global uniqueness requirement;
- duplicate names are allowed because communities are private and ID-scoped.

Timezone:

- required server-side;
- production UI offers Japan time only for v0.41.0, or only zones fully
  supported by the current timezone policy;
- unknown or unsupported zones are rejected server-side;
- DST-observing zones are not offered until timezone support is expanded.

Suggested unsupported timezone error:

```text
この時間帯はまだ選べません。
```

Display name:

- required;
- default from current membership display name when available;
- same validation as join/profile display name;
- scoped to the new community membership.

Validation errors must be field-local, plain language, and free of SQL/token/ID
terms.

## 11. Success Behavior

On successful creation:

1. Create the new community.
2. Create the creator's admin membership in that community.
3. Write audit event(s).
4. Redirect to `/c/:new_community_id/home`.

The new Home should use existing first-run admin behavior:

- create first event;
- invite members.

Do not create a dedicated success wizard in v0.41.0.

Do not auto-generate an invite code. Invite codes are one-time sensitive values;
the new admin should intentionally generate them from the existing invite screen.

## 12. Duplicate Submit and Idempotency

Community creation writes at least `communities`, `community_memberships`, and
audit rows. Duplicate form submits must not create multiple communities.

Required rule:

- use the existing form-token discipline;
- consume the token for the create operation;
- replay/double-submit must not create a second community.

Preferred replay behavior:

```text
redirect to /c/:created_community_id/home
```

If the current form-token model cannot store a replay result, implementation
must use another idempotency or recovery strategy before release.

Use a single D1 batch for community insert, membership insert, and audit writes
if the Worker binding supports it. If batching is unavailable, the write
sequence must be explicitly resumable or compensating.

## 13. Offline Behavior

Community creation is online-only.

Offline behavior:

- no local placeholder community;
- no queued create operation;
- submit is disabled by the existing offline read-only contract where possible;
- if a direct submit happens while offline, it fails safely and does not show
  false success.

## 14. Abuse Controls

Required for v0.41.0:

- active-admin eligibility;
- operator feature flag;
- rate limit by user/session and IP;
- small pilot quota, such as three communities per creator per 24 hours;
- audit events.

Turnstile is not required while creation is authenticated-only. If anonymous
request-to-create is added later, bot protection should be reconsidered.

The UI must not expose rate-limit internals. Use plain retry-later copy.

## 15. Operator Feature Flag

Community creation must be controlled by an operator setting such as:

```text
COMMUNITY_CREATION_ENABLED
```

Required behavior:

- default disabled in production until explicitly enabled;
- staging may enable it for evidence collection;
- disabled state does not expose a broken link;
- direct route access while disabled is denied safely.

Candidate disabled copy:

```text
新しいコミュニティの作成は、現在準備中です。
```

## 16. Audit Contract

Required audit events:

- `community.created`
- `membership.created_first_admin`

One combined action is acceptable only if the existing audit model is
intentionally action-level rather than row-level.

The audit metadata should identify the new community and actor without storing
secrets or excessive personal data.

`community.created` may be included in the new community's admin export because
it describes that community itself. Operator-wide analytics remain separate.

## 17. Data Writes

Minimum writes:

`communities`:

- id;
- name;
- timezone;
- is_active = true;
- created_at.

`community_memberships`:

- id;
- new community id;
- current user id;
- role = admin;
- display name;
- joined_at.

`audit_log`:

- community-created event;
- first-admin membership event, or one combined equivalent.

No migration is expected if existing tables and audit action storage accept the
new actions. A migration is needed only if audit constraints, idempotency result
tracking, or D1 quota persistence require schema changes.

## 18. No Data Copy Rule

The create flow must not copy:

- members;
- events;
- event days;
- attendance;
- notes;
- invite codes;
- templates;
- calendar tokens;
- exports;
- audit rows from other communities.

This must have a release gate.

## 19. Removed and Edge Users

If a session exists but the user has no active memberships, creation is denied.

Community creation must not become account recovery from removal. The user
should be redirected to `/join` or shown safe help consistent with current
session-expired behavior.

## 20. Accessibility and Mobile Requirements

The flow must be:

- server-rendered;
- no-JS compatible;
- usable at 360px width;
- usable at 200% text scaling;
- free of horizontal scrolling;
- clear at large text sizes;
- field-error accessible;
- built with controls at least 44px target size;
- independent of hover/tooltips.

The timezone field must not overflow. If production only supports Japan time,
prefer fixed readable text over a long dropdown.

## 21. Release Gates

Required source/tests/gates:

- anonymous GET `/communities/new` cannot open the create form;
- anonymous POST `/communities/new` cannot mutate data;
- non-admin active member cannot create in v0.41.0;
- eligible active admin can create;
- feature flag disabled state blocks the route safely;
- rate limit/quota path is tested;
- form token is required and consumed;
- duplicate/double-submit does not create two communities;
- creator becomes admin only in the new community;
- no source community data is copied;
- audit event is written;
- unsupported timezone is rejected plainly;
- i18n parity includes all new labels and errors;
- existing community isolation gates still pass;
- release checklist documents the feature flag and smoke evidence.

## 22. Browser and Runtime Evidence

Before production promotion, attach release-candidate evidence for:

- eligible admin sees the Me page entry point;
- ordinary member does not see the entry point;
- direct anonymous GET/POST is denied;
- 360px Create Community form screenshot;
- 360px plus 200% text screenshot;
- no horizontal scroll;
- no-JS GET and POST flow;
- field-local validation errors;
- offline submit behavior;
- success redirect to new Home;
- first-run admin actions visible on new Home.

Use sandboxed incognito Chromium for local smoke evidence unless a broader
device QA pass is explicitly requested. The v0.41.0 release checklist tracks
this as a runtime `[~]` item separate from the source-level release gates.

## 23. Operations Documentation Updates

Update operations documentation to distinguish:

- first community bootstrap remains operator/runbook-controlled;
- additional community creation may be enabled for trusted active admins through
  the feature flag;
- production flag default and rollout policy;
- rate-limit/quota expectations;
- incident response for accidental/test communities.

## 24. Future Policy Options

Future RFCs may revisit:

- allowing any active member to create a community;
- public request-to-create with operator approval;
- anonymous/public self-service creation;
- community archival/deletion self-service;
- ownership transfer or first-admin transfer;
- broader timezone choices after DST support;
- Turnstile or other bot protection for any public creation surface.

Those are intentionally out of scope for v0.41.0.
