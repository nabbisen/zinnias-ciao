# RFC 070 - Self Display Name Editing

**Status.** Proposed  
**Target release.** v0.60.0 candidate  
**Tracks.** Member profile, community UX, identity display, audit safety.  
**Touches.** Me/profile page, community route dispatcher, membership write path,
display-name validation, form tokens, i18n, audit logging, release gates,
browser smoke.

## Summary

Add a user-facing UI for an active signed-in member to change their own display
name inside a community.

The current invite-era model stores display names on
`community_memberships`. A member can choose a display name while joining, and
a first admin can choose their display name while creating a community, but
there is no normal self-service way to correct a typo, update a preferred name,
or adjust how they appear to other community members later.

This RFC makes display-name editing a small community-scoped profile
maintenance workflow. It is not identity recovery, account merging, or a global
profile system.

## Background

Current facts from the codebase:

- `community_memberships.display_name` is the visible name used in member lists,
  attendance/matrix rows, notes, and profile surfaces.
- `packages/domain/src/display_name.rs` already defines
  `validate_display_name`, with trimming, non-empty validation, control
  character rejection, and a 40-character limit.
- `workers/ssr/src/handlers/me.rs` shows the current active membership display
  name on the Me page but does not expose an edit action.
- `workers/ssr/src/handlers/community.rs` dispatches community-scoped routes
  and already has `/c/:cid/me` and `/c/:cid/me/calendar`.
- RFC-024 deliberately rejects recovery or identity merge by display name.
- RFC-063 keeps removed-member return behavior separate from active membership
  edits.

Those facts imply that the safest first design is to update only the current
active membership row that belongs to the authenticated user and selected
community.

## Problem

Members can make mistakes or have legitimate name changes after joining. Today,
the app has no friendly in-app path for them to update the label other people
see.

The current alternatives are poor:

- ask an admin/operator to repair D1 manually;
- remove and re-invite the person, splitting history;
- leave a typo or obsolete name visible in community workflows;
- treat display-name correction as account recovery, which would confuse the
  trust model.

## Goals

- Let an active signed-in member change their own display name.
- Scope the edit to one active community membership.
- Reuse the existing domain display-name validation.
- Keep the entry point easy to find from the Me page.
- Preserve community isolation and generic denial behavior.
- Keep the no-JS form flow fully functional.
- Use normal single-use form-token protection.
- Audit successful changes.
- Avoid side effects to roles, sessions, invites, relink/recovery codes, notes,
  attendance, events, and historical rows.
- Keep Japanese UI copy short, calm, and non-technical.

## Non-Goals

- No global account profile.
- No legal-name or verified identity feature.
- No display-name uniqueness guarantee.
- No admin editing another member's display name in this RFC.
- No removed-member reactivation or identity merge.
- No automatic merge by old display name.
- No historical rewrite of old audit rows, notes, attendance records, event
  responses, or exported data.
- No avatar/photo/profile-bio feature.
- No contact-channel or notification preference changes.
- No rate-limited public recovery surface.

## Decision

Add a community-scoped self-edit flow:

```text
GET  /c/:cid/me/display-name
POST /c/:cid/me/display-name
```

The route is available only to the signed-in active member for `:cid`.

The Me page should show a quiet link near the displayed name, for example:

```text
表示名を変更
```

English equivalent:

```text
Change display name
```

The edit form should:

- use the normal app header with community switcher;
- make the active community context visible;
- prefill the current display name;
- show the same maximum-length expectation as join/profile;
- submit through a POST form with a scoped token;
- preserve no-JS behavior;
- redirect back to `/c/:cid/me?flash=display_name_updated` after an actual
  successful change;
- render a fixed-code success banner on Me for `flash=display_name_updated`.

## Route and Authorization Contract

### GET `/c/:cid/me/display-name`

1. Require a valid session.
2. Require active membership in `:cid` through the same authorization path as
   the Me page.
3. Render the edit form with current `membership.display_name`.
4. Issue a single-use form token with purpose `CHANGE_DISPLAY_NAME` bound to the
   active `membership_id`.

Denied cases:

- no session: session-expired response, consistent with current member pages;
- non-member or removed member: generic not-found response;
- inactive community: generic not-found response through existing membership
  authorization behavior.

### POST `/c/:cid/me/display-name`

1. Require a valid session.
2. Require active membership in `:cid`.
3. Validate and normalize `display_name` using `validate_display_name`.
4. Compare the normalized value with the current value.
5. Consume the `CHANGE_DISPLAY_NAME` token bound to the active `membership_id`.
6. Store a replay result for every consumed outcome before returning:
   - same-value no-op stores `display_name_unchanged`;
   - successful change stores `display_name_updated`;
   - validation failures do not consume the token because validation happens
     before consume.
7. If the normalized value matches the current value, redirect back to Me
   without writing a database row or audit event.
8. Update only:

```sql
community_memberships.display_name
```

for:

```sql
id = <active membership id>
community_id = <cid>
user_id = <authenticated user id>
removed_at IS NULL
```

9. Write audit action:

```text
membership.display_name_updated
```

10. Redirect to `/c/:cid/me?flash=display_name_updated`.

Invalid form input should re-render the form with a plain validation message and
the submitted value escaped. Token replay or invalid token must not perform the
mutation. A replay that returns `display_name_updated` should redirect to the
same success destination without re-updating the row or writing another audit
event. A replay that returns `display_name_unchanged` should redirect quietly to
Me without audit. Invalid tokens should use a generic safe response consistent
with other mutating member forms; the implementation should prefer redirecting
back to Me or showing the generic error rather than revealing token state.

### Replay-Safe Token Contract

`CHANGE_DISPLAY_NAME` must not follow handler patterns that treat
`consume(...).await? == Ok(None)` as safe after a token has already been
consumed without a stored result. The implementation must call
`form_token::set_result` for every consumed `CHANGE_DISPLAY_NAME` outcome.

Required behavior:

- validation happens before token consumption, so invalid names do not burn the
  token;
- same-value no-op consumes the token and immediately stores
  `display_name_unchanged`;
- actual change consumes the token, performs the update and audit, then stores
  `display_name_updated` before returning success;
- altered-body replay of the same consumed token must not update
  `community_memberships.display_name`;
- altered-body replay of the same consumed token must not write a second
  `membership.display_name_updated` audit row.

Actual changes must batch the membership update, audit insert, and
`result_ref = 'display_name_updated'` write together. This keeps the returned
success state, audit evidence, and replay guard aligned. If the batch fails, the
handler must not return success.

## Data and Audit Contract

No schema migration is required.

Add a small membership data-access function, for example:

```rust
update_display_name_for_active_user(db, membership_id, community_id, user_id, display_name)
```

The write must be scoped by membership id, community id, authenticated user id,
and `removed_at IS NULL`. The implementation should verify that exactly one row
changed before treating the update as successful.

The display-name update and audit insert must not use best-effort audit. The
handler must avoid `let _ = audit::write(...)`.

The required implementation shape for actual changes is one D1 batch:

1. update `community_memberships.display_name`;
2. insert `audit_log` action `membership.display_name_updated`;
3. set the consumed token's `result_ref` to `display_name_updated`.

For same-value no-op, set `result_ref` to `display_name_unchanged` and do not
write membership or audit rows.

Audit metadata should be minimal:

```json
{
  "changed": ["display_name"]
}
```

The audit row columns are the canonical location for `community_id`,
`actor_membership_id`, and `target_id`; do not duplicate those IDs in
`metadata_json` for this slice. Do not store old/new display-name values in
audit metadata. They are visible elsewhere in normal app context, but storing
both old and new values creates unnecessary long-lived personal-data copies.
The audit action itself is sufficient to show that a member changed the visible
label.

Audit metadata must not include sessions, invite codes, recovery codes, form
tokens, contact data, note bodies, arbitrary private text, or the submitted raw
unvalidated value.

## UI Copy

Candidate Japanese copy:

| Surface | Japanese | English reference |
|---------|----------|-------------------|
| Me link | `表示名を変更` | Change display name |
| Page title | `表示名を変更` | Change display name |
| Field label | `このコミュニティでの表示名` | Display name in this community |
| Help text | `この名前は、このコミュニティのメンバーに表示されます。` | This name is shown to members of this community. |
| Submit | `保存` | Save |
| Cancel/back | `戻る` | Back |
| Success | `表示名を変更しました。` | Display name updated. |
| Invalid empty | `表示名を入力してください。` | Enter a display name. |
| Invalid long | `表示名は40文字以内にしてください。` | Display name must be 40 characters or fewer. |
| Invalid chars | `表示名に使えない文字が含まれています。` | Display name contains invalid characters. |

Final Japanese copy can be adjusted during implementation review or a later
RFC-054 copy pass. The first slice should avoid technical words such as
"identity", "account", "merge", "relink", or "recovery".

## Rejected or Deferred Alternatives

### Admin edits another member's display name

Deferred. Admin editing creates a different consent and moderation surface. It
also raises questions about whether admins can rewrite another person's visible
label. This RFC is self-service only.

### Global display name

Deferred. The current identity model is membership-scoped. A global name would
need stronger multi-community identity semantics and would affect all
communities at once.

### Display-name uniqueness

Rejected for the first slice. Display names are not identifiers and cannot be
used for recovery or authorization.

### Audit old and new display names

Rejected for the first slice. The audit action should show that a change
occurred, but long-lived old/new personal labels in audit metadata are not
necessary for the main safety goal.

### Reuse `/join/profile`

Rejected. Joining is an unauthenticated invite-redemption profile step and has
different cookies, tickets, and trust boundaries. Self-edit is an authenticated
community-scoped member workflow.

## Release Gates

Add source gates covering:

- `CHANGE_DISPLAY_NAME` token purpose exists and is included in the token
  purpose completeness gate;
- route dispatcher includes only `/c/:cid/me/display-name` GET/POST for this
  feature;
- handler requires active membership and does not use admin-only authorization;
- update SQL is scoped by membership id, community id, authenticated user id,
  and `removed_at IS NULL`;
- validation uses `validate_display_name`;
- audit action is `membership.display_name_updated`;
- handler calls `form_token::set_result` or an equivalent D1 statement for both
  `display_name_updated` and `display_name_unchanged` consumed-token outcomes;
- altered-body replay of a consumed `CHANGE_DISPLAY_NAME` token cannot reach
  the membership update path;
- handler does not use `let _ = audit::write` or best-effort audit for the
  actual-change path;
- actual-change path batches membership update, audit insert, and replay-result
  storage;
- audit metadata does not include old/new display-name values, raw submitted
  values, tokens, invite data, recovery data, sessions, or note bodies;
- no code path references removed-member reactivation, identity merge, or
  display-name lookup for recovery.

## Browser Smoke

Add a reusable smoke script for the self-display-name workflow. It should verify:

- Me page shows current display name and the edit link;
- valid edit updates the Me page and a member-visible surface that uses the
  display name;
- actual successful edit shows fixed success feedback on Me;
- same-value submit does not create an extra audit row;
- replaying a consumed token with a different `display_name` does not update the
  display name and does not write a second audit row;
- invalid empty/too-long/control-character input is rejected with plain copy;
- direct route access by a non-member receives generic denial;
- changing one community's display name does not change another community's
  membership display name;
- mobile 390px viewport with 200% text scaling keeps label, input, buttons, and
  validation copy usable without horizontal overflow.

## Acceptance Criteria

RFC-070 can be considered complete when:

- active members can update their own community-scoped display name through a
  reviewed UI;
- non-members and removed members cannot update names through direct routes;
- validation matches existing display-name rules;
- same-value submission is idempotent and does not write unnecessary audit rows;
- consumed-token replay, including altered-body replay, cannot update the
  display name or write another audit row;
- actual successful changes show fixed success feedback on Me;
- no role, session, invite, recovery, attendance, note, event, or identity-merge
  side effects are introduced;
- successful changes are auditable with minimal metadata;
- release gates cover route authorization, scoped mutation, validation reuse,
  audit action, and absence of recovery/merge behavior;
- browser smoke covers normal edit, invalid input, direct-route denial,
  cross-community scoping, and mobile/200% text usability.
