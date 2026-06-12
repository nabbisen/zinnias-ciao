# RFC 002 — Data Model and D1 Migrations

**Status.** Implemented (v0.1.0)
**Phase:** M1 / Trust Boundary Foundation
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** AD-2 (invite + deferred OIDC), AD-4 (form token), and the confirmed **Event → EventDay → Attendance** grain.

---

## 1. Summary

Defines the MVP database model and migration discipline. `community_memberships` is first-class so authorization, roles, display names, member lists, and null attendance are safe. The activity grain is **Event → EventDay → Attendance**: an event spans one or more dated days, attendance status is per day, and the single short note is per event. Idempotency/CSRF use a server-issued `form_tokens` table (AD-4). `users.idp_subject` is reserved (nullable) so deferred OIDC (AD-2) lands without a migration break.

Design stance (per "small start, not small end"): the MVP common case is a one-day event (exactly one `event_days` row), but the schema natively supports multi-day events (camps, trips, multi-session workshops) without later restructuring.

## 2. Goals

- Tables for communities, users, memberships, invite codes, sessions, events, event days, attendances, event notes, form tokens, and audit.
- Preserve *no attendance row* vs explicit `No answer`.
- Per-day attendance; one note per member per event.
- Multiple communities per user; admin/member roles per community.
- Soft deletion/cancellation, auditability, and an OIDC seam.

## 3. Non-Goals

- No RBAC beyond `admin`/`member`; no recurring-series tables (RFC-022 future); no media/password/analytics tables; no client mutation queue (AD-1).

## 4. External Behavior

The UI shows communities, display names, events with their day(s), per-day status, one note per event, and admin tools. The model must render all active members for a day even with no attendance row, and must not leak implementation IDs.

## 5. Internal Design

```sql
communities (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  timezone TEXT NOT NULL,                 -- IANA name; per-day labels computed in this tz
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

users (
  id TEXT PRIMARY KEY,
  idp_subject TEXT UNIQUE,                -- reserved for deferred OIDC (AD-2); NULL for invite-only members
  created_at TEXT NOT NULL
);

community_memberships (
  id TEXT PRIMARY KEY,
  community_id TEXT NOT NULL REFERENCES communities(id),
  user_id TEXT NOT NULL REFERENCES users(id),
  role TEXT NOT NULL CHECK(role IN ('admin','member')),
  display_name TEXT NOT NULL,
  joined_at TEXT NOT NULL,
  removed_at TEXT,
  UNIQUE(community_id, user_id)
);

invite_codes (
  id TEXT PRIMARY KEY,
  community_id TEXT NOT NULL REFERENCES communities(id),
  code_hmac TEXT NOT NULL UNIQUE,         -- HMAC-SHA256(pepper, normalized_code); never plaintext (AD-3)
  created_by_membership_id TEXT NOT NULL REFERENCES community_memberships(id),
  used_by_membership_id TEXT REFERENCES community_memberships(id),
  expires_at TEXT NOT NULL,
  used_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL
);

sessions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  session_hmac TEXT NOT NULL UNIQUE,      -- HMAC-SHA256(pepper, high-entropy secret); fast (AD-3)
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,               -- now + SESSION_TTL_SECONDS; decoupled from any token exp (RFC-003)
  revoked_at TEXT,
  last_seen_at TEXT
);

events (
  id TEXT PRIMARY KEY,
  community_id TEXT NOT NULL REFERENCES communities(id),
  created_by_membership_id TEXT NOT NULL REFERENCES community_memberships(id),
  title TEXT NOT NULL,
  description TEXT,
  location TEXT,
  status TEXT NOT NULL DEFAULT 'scheduled' CHECK(status IN ('scheduled','cancelled')),
  cancelled_at TEXT,
  cancelled_by_membership_id TEXT REFERENCES community_memberships(id),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
  -- times intentionally NOT here; they live on event_days
);

event_days (
  id TEXT PRIMARY KEY,
  event_id TEXT NOT NULL REFERENCES events(id),
  community_id TEXT NOT NULL REFERENCES communities(id),  -- denormalized (immutable == events.community_id) for scoped time-window queries
  seq INTEGER NOT NULL,                   -- 1..N ordering within the event
  day_date TEXT NOT NULL,                 -- local calendar date in community tz (grouping/labels)
  starts_at_utc TEXT NOT NULL,
  ends_at_utc TEXT NOT NULL,
  created_at TEXT NOT NULL,
  CHECK(ends_at_utc > starts_at_utc),
  UNIQUE(event_id, seq)
);

attendances (
  id TEXT PRIMARY KEY,
  event_day_id TEXT NOT NULL REFERENCES event_days(id),
  membership_id TEXT NOT NULL REFERENCES community_memberships(id),
  status TEXT CHECK(status IN ('going','not_going','attended')),  -- NULL = No answer
  status_updated_at TEXT,
  updated_at TEXT NOT NULL,
  UNIQUE(event_day_id, membership_id)
);

event_notes (
  id TEXT PRIMARY KEY,
  event_id TEXT NOT NULL REFERENCES events(id),
  membership_id TEXT NOT NULL REFERENCES community_memberships(id),
  note TEXT NOT NULL CHECK(length(note) <= 200),  -- plain text; escaped at display
  note_updated_at TEXT NOT NULL,
  note_deleted_at TEXT,
  hidden_by_admin_at TEXT,                -- moderation soft-hide (RFC-025 future); audit, not content copy
  UNIQUE(event_id, membership_id)
);

-- AD-4: server-issued single-use token = CSRF + idempotency. No client-generated mutation_id.
form_tokens (
  token_hmac TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  purpose TEXT NOT NULL,                  -- 'set_status' | 'save_note' | 'delete_note' | 'create_event' | ...
  bound_resource TEXT,                    -- optional event_day_id / event_id / membership_id
  issued_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  consumed_at TEXT,                       -- first success; replay -> prior result / benign no-op
  result_ref TEXT
);

audit_log (
  id TEXT PRIMARY KEY,
  community_id TEXT,
  actor_membership_id TEXT,
  target_kind TEXT NOT NULL,
  target_id TEXT,
  action TEXT NOT NULL,
  metadata_json TEXT,                     -- structured + redacted; never full note bodies (RFC-014)
  created_at TEXT NOT NULL
);
```

Note grain decision: status is per `event_day`, the note is per `event` (matches "one note per member per event"). Separating `event_notes` from `attendances` keeps the two grains clean and leaves an obvious seam to make notes per-day later (add `event_day_id`) without reshaping attendance.

## 6. Data and API Design

```sql
CREATE INDEX idx_memberships_user ON community_memberships(user_id);
CREATE INDEX idx_memberships_community_active ON community_memberships(community_id, removed_at);
CREATE INDEX idx_event_days_community_time ON event_days(community_id, starts_at_utc);  -- Home window query
CREATE INDEX idx_event_days_event ON event_days(event_id, seq);
CREATE INDEX idx_attendances_day ON attendances(event_day_id);
CREATE INDEX idx_attendances_membership ON attendances(membership_id);
CREATE INDEX idx_event_notes_event ON event_notes(event_id);
CREATE INDEX idx_sessions_hmac ON sessions(session_hmac);
CREATE INDEX idx_invite_codes_hmac ON invite_codes(code_hmac);
CREATE INDEX idx_form_tokens_user ON form_tokens(user_id);
```

Home query is a bounded date-window scan of `event_days` by `(community_id, starts_at_utc)`, joined to `events` (filter `scheduled`), grouped by event. Per-day status counts are an aggregate over `attendances` for the day; `No answer` = active members minus members with a row.

DTOs (in `packages/contracts`) never expose session/invite/token HMACs or audit internals; IDs are opaque and re-authorized server-side.

## 7. Security, Privacy, and Safety

- Isolation enforced by joining through active `community_memberships` (`removed_at IS NULL`); `event_days.community_id` lets day-grain queries enforce scope without an extra hop.
- Invite codes and session secrets stored only as peppered HMACs (AD-3): DB leak alone cannot recover them; verification stays in budget.
- Notes plain text, escaped at display (RFC-007/012); soft-delete/hide fields for auditability without content duplication.
- 6-char invite codes are low-entropy; pepper + 24 h expiry + one-time use + redemption rate-limiting (RFC-003/012) is the accepted tradeoff; raise length rather than adopt a slow KDF.

## 8. Acceptance Criteria

- Migration builds all tables from empty.
- A one-day event has exactly one `event_days` row; a multi-day event has N, ordered by `seq`.
- Attendance is per `event_day`; member list renders for a day with no attendance rows.
- One note per member per event; >200 chars rejected at the DB.
- Cancelled events / removed members representable without hard deletion.
- `No answer` stays distinct from the three explicit statuses.
- A consumed `form_token` cannot drive a second write.

## 9. Test Plan

- Empty-DB migration; constraint tests (role, status, note length, `ends_at > starts_at`, unique `(event_day, membership)` and `(event, membership)` note).
- Multi-day event: 3 days, per-day attendance independent.
- Active-members left-joined-with-attendance (per day) query test.
- Removed-member access-denied at day grain.
- `form_tokens` single-use / replay test.
- HMAC round-trip (no plaintext at rest).

## 10. Open Questions / Decisions

Decisions: text status values (readability); status per day, note per event; `event_days.community_id` denormalized for scoped time-window queries. No open items — grain confirmed.
