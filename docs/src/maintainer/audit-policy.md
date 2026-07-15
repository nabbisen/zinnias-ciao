# Audit Retention and Access Policy (RFC-052 / RFC-079)

**Applies to:** ciao.zinnias pilot and beta  
**Source:** `workers/ssr/src/audit.rs`, `workers/ssr/src/handlers/`,
`migrations/0010_audit_integrity.sql`

## Overview

Audit events are operator-only security and moderation records. RFC-079 makes
required audit writes atomic with their business mutations, closes action and
metadata inputs, and recursively sanitizes the typed metadata before storage.
The migration resets all historical metadata to `{}` because the old arbitrary
JSON writer and direct inserts could not prove that legacy values were safe.

Package 2 defines the final schema and policy, but it is not itself deployable.
Do not apply migration 0010 to a hosted database until the later implementation
packages and architecture review establish the earliest deployable boundary.

## Access policy

Members and admins cannot read audit records through the application. Only an
authorized operator may query them directly in D1. Prefer explicit core columns
and bounded result sets; do not use `SELECT *` in evidence collection.

```sql
SELECT id, request_id, community_id, actor_membership_id,
       target_kind, target_id, action, created_at
FROM audit_log
WHERE community_id = '<community_id>'
ORDER BY created_at DESC
LIMIT 50;
```

Metadata is not needed for ordinary chronology checks. When an approved
incident investigation requires typed metadata, select it deliberately and do
not copy it into review evidence.

## Raw-history compatibility query

Migration 0010 preserves every historical `target_kind` and `action` byte-for-
byte. It does not rename legacy values. Use this query to derive a logical
action for mixed legacy/new history while retaining both raw columns:

```sql
SELECT
    id,
    request_id,
    community_id,
    actor_membership_id,
    target_kind AS raw_target_kind,
    target_id,
    action AS raw_action,
    CASE
        WHEN target_kind = 'event_day' AND action = 'occurrence_cancelled'
            THEN 'event.occurrence_cancelled'
        WHEN target_kind = 'calendar_feed' AND action = 'calendar_token_generated'
            THEN 'calendar_feed.token_generated'
        WHEN target_kind = 'calendar_feed' AND action = 'calendar_token_revoked'
            THEN 'calendar_feed.token_revoked'
        WHEN instr(action, '.') > 0 THEN action
        ELSE target_kind || '.' || action
    END AS logical_action,
    created_at
FROM audit_log
ORDER BY created_at DESC
LIMIT 100;
```

The three aliases must precede the generic rules. Historical `community` +
`exported` therefore remains `community.exported` and is distinct from new
pre-disclosure evidence named `community.export_authorized`. Unknown historical
values remain visible through the generic `target_kind || '.' || action` rule;
operators must not guess or rewrite their meaning.

## Retention policy

**Pilot and beta:** indefinite. No TTL cleanup runs.

**Future production:** a follow-up RFC may add TTL cleanup after volume and
incident needs are understood. The current minimum recommendation is 90 days.

Backups created before migration 0010 are a separate sensitive class because
they may contain unsafe legacy `metadata_json`. Follow
[Backup and Recovery](backup-recovery.md) without opening or copying that
metadata into evidence.

## Closed metadata allowlist

Identifiers already represented by `community_id`, `actor_membership_id`, or
`target_id` are never duplicated in metadata.

| Action family | Allowed metadata |
|---|---|
| Invite generation/revocation, membership removal/role, event cancellation, templates, calendar tokens, community export, logout | `{}` |
| Invite redemption | Optional closed `role_granted` value only when required |
| Relink creation/redemption | `relink_code_id` only |
| Operator recovery | Lowercase bounded `operator_label`, `relink_code_id` |
| Event creation | `creation_mode`, optional `source_event_id` |
| Event edit | `edit_scope` or fixed `changed_fields` |
| Occurrence cancellation | `series_id`, ISO local `day_date` |
| Attendance override | Bounded integer `changed_count` |
| Admin note moderation | Target membership ID only when no target column can represent it |
| Display-name update | `changed_fields: ["display_name"]` |
| Matrix export request | Validated `YYYY-MM` month |

Event titles/descriptions, display names, note content, exported content,
credentials, codes, HMACs, sessions, cookies, and arbitrary JSON are forbidden.
The typed model is the primary boundary. The recursive sanitizer is defense in
depth and enforces an object root, depth 8, 128 visited nodes, and 2,048
serialized UTF-8 bytes.

## Canonical action inventory

Class A contains exactly 23 actions:

```text
community.created
membership.created_first_admin
membership.display_name_updated
invite_code.generated
invite_code.revoked
invite_code.redeemed
membership.relink_code_created
membership.relink_redeemed
operator_recovery.admin_relink_created
membership.removed
membership.promoted_to_admin
membership.demoted_to_member
event.created
event.edited
event.cancelled
event.occurrence_cancelled
attendance.admin_override
attendance.admin_set_attended
event_note.admin_hidden
calendar_feed.token_generated
calendar_feed.token_revoked
event_template.created
event_template.deleted
```

Class B contains `community.export_authorized` and
`calendar_matrix_csv.export_requested`. Class C contains only `session.logout`
and carries no session, actor, community, or target identifier.

## Schema reference

Migration 0010 rebuilds `audit_log` as a STRICT table with:

- `request_id TEXT NOT NULL` bounded to 1–96 bytes;
- required object JSON metadata bounded to 2,048 bytes;
- bounded ID, target-kind, and action fields;
- `(community_id, created_at)` and `(action, created_at)` indexes; and
- a shared STRICT `audit_change_assertions` table for the reviewed one-row
  assertion primitive.

All legacy rows receive `request_id = 'legacy'` and `metadata_json = '{}'`.
Migration guard checks compare the row count and every core column in both
directions before the old table is dropped.

## Incident response queries

```sql
-- Invite chronology without metadata
SELECT id, request_id, community_id, actor_membership_id,
       target_kind, target_id, action, created_at
FROM audit_log
WHERE target_kind = 'invite_code'
  AND created_at >= datetime('now', '-7 days')
ORDER BY created_at DESC
LIMIT 100;

-- Membership chronology
SELECT id, request_id, community_id, actor_membership_id,
       target_kind, target_id, action, created_at
FROM audit_log
WHERE target_kind = 'membership'
  AND target_id = '<membership_id>'
ORDER BY created_at DESC
LIMIT 100;
```

Member self-service attendance, a member's own note writes, reads, and invalid
credential guesses remain intentionally unaudited at subject level.

## Related

- RFC-014 — original audit implementation
- RFC-052 — retention and access policy
- RFC-079 — atomic required audits and recursive redaction
- `docs/src/maintainer/operations.md` — operator procedures
- `docs/src/maintainer/backup-recovery.md` — sensitive backup and recovery policy
