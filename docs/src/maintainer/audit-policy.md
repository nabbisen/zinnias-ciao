# Audit Retention and Access Policy (RFC-052)

**Applies to:** ciao.zinnias pilot and beta  
**Implemented in:** v0.36.0  
**Source:** `workers/ssr/src/audit.rs`, `workers/ssr/src/handlers/`, `migrations/0001_initial.sql`

---

## Overview

ciao.zinnias records structured audit events for security- and
moderation-relevant admin actions. This document defines who can read them,
how long they are kept, what metadata is allowed, and how the operator uses
them for incident response.

---

## Access policy

Audit events are **operator access only**. There is no audit UI in the
application. Members and admins cannot read audit records through the app.

The operator reads audit events directly from D1:

```sql
-- Most recent 50 events in a community
SELECT id, actor_membership_id, target_kind, target_id, action,
       metadata_json, created_at
FROM   audit_log
WHERE  community_id = '<community_id>'
ORDER  BY created_at DESC
LIMIT  50;

-- Events for a specific membership
SELECT * FROM audit_log
WHERE  actor_membership_id = '<membership_id>'
ORDER  BY created_at DESC
LIMIT  100;

-- Events for a specific action type
SELECT * FROM audit_log
WHERE  action = 'removed'
ORDER  BY created_at DESC
LIMIT  100;
```

---

## Retention policy

**Pilot and beta:** indefinite. Audit log volume is small (one row per admin
action). No TTL cleanup runs.

**Future production:** a TTL-based cleanup job may be added by a follow-up
RFC once volume is understood. The minimum recommended retention is 90 days
for incident investigation.

---

## Metadata allowlist

Each audit record stores structured JSON metadata. The following fields are
permitted:

| Field | Type | Example | Notes |
|---|---|---|---|
| `title` | string | `"お花見 2026"` | Event title at time of action |
| `role_granted` | string | `"admin"` or `"member"` | On invite redemption |
| `membership_id` | string | `"mem_xxx"` | Help-signin target membership |
| `created_by_membership_id` | string | `"mem_admin"` | Admin membership that created a help-signin code |
| `community_id` | string | `"com_xxx"` | Community scope for help-signin audit |

The following fields are **explicitly forbidden** from metadata:

- Note body text (`memo`, `note`, `body`)
- Invite code plaintexts or HMACs (`code`, `code_hmac`)
- Session token values (`session_id`, `token`)
- Display names or personal information beyond what is already captured in `actor_membership_id` / `target_id`

The `audit.rs` writer calls `redact_sensitive_keys()` before persisting any
metadata JSON. The redact list covers `password`, `token`, `secret`, `code`,
`hmac`, `session`, `note`, `memo`, `body`.

---

## Audit event inventory

All events currently written. Format: `target_kind.action`.

| Event | Trigger | Actor |
|---|---|---|
| `invite_code.generated` | Admin generates an invite code | Admin |
| `invite_code.redeemed` | User redeems a valid invite code (join succeeds) | New member |
| `invite_code.revoked` | Admin revokes an unused invite code | Admin |
| `membership.removed` | Admin removes a member | Admin |
| `membership.relink_code_created` | Admin creates an active-member help-signin code | Admin |
| `membership.relink_redeemed` | Member redeems a help-signin code | Member |
| `event.created` | Admin creates a new event | Admin |
| `event.edited` | Admin edits an event's fields | Admin |
| `event.cancelled` | Admin cancels an event | Admin |
| `attendance.admin_override` | Admin sets attendance for a member | Admin |
| `attendance.admin_set_attended` | Admin marks a member as attended after event end | Admin |
| `event_note.admin_hidden` | Admin hides an inappropriate member note | Admin |
| `session.logout` | Any user logs out | Member or Admin |
| `calendar_feed.calendar_token_generated` | Member generates an ICS feed URL | Member |
| `calendar_feed.calendar_token_revoked` | Member revokes their ICS feed URL | Member |
| `community.exported` | Admin downloads community JSON export | Admin |
| `event_template.created` | Admin saves an event template | Admin |
| `event_template.deleted` | Admin deletes an event template | Admin |

---

## Schema reference

```sql
CREATE TABLE IF NOT EXISTS audit_log (
    id                   TEXT PRIMARY KEY,
    community_id         TEXT,
    actor_membership_id  TEXT,
    target_kind          TEXT NOT NULL,
    target_id            TEXT,
    action               TEXT NOT NULL,
    metadata_json        TEXT,         -- structured JSON; sensitive keys redacted
    created_at           TEXT NOT NULL -- UTC ISO-8601
);
```

The `id` column is a 16-character random hex string generated at write time.
There is no auto-increment; UUIDs were avoided to keep the size small.

---

## Incident response procedure

### Investigate suspicious invite-code activity

```sql
SELECT id, actor_membership_id, target_id, action, metadata_json, created_at
FROM   audit_log
WHERE  target_kind = 'invite_code'
  AND  created_at >= datetime('now', '-7 days')
ORDER  BY created_at DESC;
```

If repeated `invite_code.redeemed` events appear with different
`actor_membership_id` values in a short window, investigate whether codes were
shared broadly. Revoke the relevant codes:

```sql
-- Identify the invite code IDs from the audit target_id
-- Then mark them revoked in the invite_codes table:
UPDATE invite_codes SET revoked_at = datetime('now') WHERE id = '<code_id>';
```

### Investigate removed member

```sql
SELECT * FROM audit_log
WHERE  target_kind  = 'membership'
  AND  target_id    = '<membership_id>'
ORDER  BY created_at DESC;
```

### Investigate note moderation

```sql
SELECT * FROM audit_log
WHERE  target_kind = 'event_note'
ORDER  BY created_at DESC
LIMIT  50;
```

---

## What audit does not cover

The following are **not** audit-logged and are by design:

- Member self-service attendance changes (`going`, `not_going`) — high-volume,
  low-risk; observable via the `attendances` table directly
- Member memo saves and deletes (own notes) — observable via `event_notes`
- Read access (GET requests) — no logging; D1 query logs via Logpush if needed
- Failed authentication attempts — rate-limit state is in KV, not D1

---

## Related

- RFC-014 — Audit system implementation
- RFC-052 — This policy RFC
- `docs/src/maintainer/operations.md` — D1 direct access commands
- `docs/src/maintainer/backup-recovery.md` — D1 backup and restore
