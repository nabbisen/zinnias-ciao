-- Migration 0001 — initial schema
-- ciao.zinnias / RFC-002
-- Event -> EventDay -> Attendance grain (confirmed).
-- All secrets stored as HMAC-SHA256(pepper, value) — never plaintext (AD-3).

CREATE TABLE IF NOT EXISTS communities (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    timezone   TEXT NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY,
    -- Reserved for deferred OIDC (AD-2). NULL for invite-only members.
    idp_subject TEXT UNIQUE,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS community_memberships (
    id           TEXT PRIMARY KEY,
    community_id TEXT NOT NULL REFERENCES communities(id),
    user_id      TEXT NOT NULL REFERENCES users(id),
    role         TEXT NOT NULL CHECK(role IN ('admin','member')),
    display_name TEXT NOT NULL,
    joined_at    TEXT NOT NULL,
    removed_at   TEXT,
    UNIQUE(community_id, user_id)
);

CREATE TABLE IF NOT EXISTS invite_codes (
    id                      TEXT PRIMARY KEY,
    community_id            TEXT NOT NULL REFERENCES communities(id),
    -- HMAC-SHA256(pepper, normalize(code)). Never store plaintext (AD-3).
    code_hmac               TEXT NOT NULL UNIQUE,
    created_by_membership_id TEXT NOT NULL REFERENCES community_memberships(id),
    used_by_membership_id   TEXT REFERENCES community_memberships(id),
    expires_at              TEXT NOT NULL,
    used_at                 TEXT,
    revoked_at              TEXT,
    created_at              TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES users(id),
    -- HMAC-SHA256(pepper, high-entropy random secret). Never store plaintext (AD-3).
    session_hmac  TEXT NOT NULL UNIQUE,
    created_at    TEXT NOT NULL,
    -- expires_at = created_at + SESSION_TTL_SECONDS.
    -- MUST NOT be derived from any upstream token exp (RFC-003 regression rule).
    expires_at    TEXT NOT NULL,
    revoked_at    TEXT,
    last_seen_at  TEXT
);

CREATE TABLE IF NOT EXISTS events (
    id                         TEXT PRIMARY KEY,
    community_id               TEXT NOT NULL REFERENCES communities(id),
    created_by_membership_id   TEXT NOT NULL REFERENCES community_memberships(id),
    title                      TEXT NOT NULL,
    description                TEXT,
    location                   TEXT,
    -- Times live on event_days, not here (Event->EventDay->Attendance grain).
    status                     TEXT NOT NULL DEFAULT 'scheduled'
                               CHECK(status IN ('scheduled','cancelled')),
    cancelled_at               TEXT,
    cancelled_by_membership_id TEXT REFERENCES community_memberships(id),
    created_at                 TEXT NOT NULL,
    updated_at                 TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS event_days (
    id           TEXT PRIMARY KEY,
    event_id     TEXT NOT NULL REFERENCES events(id),
    -- Denormalized from events.community_id (immutable) for scoped time-window queries.
    community_id TEXT NOT NULL REFERENCES communities(id),
    seq          INTEGER NOT NULL,
    -- Local calendar date in community timezone, e.g. "2026-06-14" (grouping/labels).
    day_date     TEXT NOT NULL,
    starts_at_utc TEXT NOT NULL,
    ends_at_utc   TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    CHECK(ends_at_utc > starts_at_utc),
    UNIQUE(event_id, seq)
);

CREATE TABLE IF NOT EXISTS attendances (
    id                TEXT PRIMARY KEY,
    event_day_id      TEXT NOT NULL REFERENCES event_days(id),
    membership_id     TEXT NOT NULL REFERENCES community_memberships(id),
    -- NULL = No answer (distinct from any explicit value — RFC-002 / requirements §6.5).
    status            TEXT CHECK(status IN ('going','not_going','attended')),
    status_updated_at TEXT,
    updated_at        TEXT NOT NULL,
    UNIQUE(event_day_id, membership_id)
);

CREATE TABLE IF NOT EXISTS event_notes (
    id                 TEXT PRIMARY KEY,
    event_id           TEXT NOT NULL REFERENCES events(id),
    membership_id      TEXT NOT NULL REFERENCES community_memberships(id),
    note               TEXT NOT NULL CHECK(length(note) <= 200),
    note_updated_at    TEXT NOT NULL,
    -- Soft-delete fields — preserve auditability without hard deletion.
    note_deleted_at    TEXT,
    -- Admin moderation soft-hide (RFC-025 future). Audit, not content copy.
    hidden_by_admin_at TEXT,
    UNIQUE(event_id, membership_id)
);

-- AD-4: server-issued single-use tokens = CSRF protection + idempotency.
-- Replaces any client-generated mutation_id. No client_mutations table.
CREATE TABLE IF NOT EXISTS form_tokens (
    token_hmac     TEXT PRIMARY KEY,
    user_id        TEXT,  -- NULL or sentinel for pre-auth tokens (no FK: short-lived operational table)
    -- Purpose values are defined in contracts::auth::token_purpose.
    purpose        TEXT NOT NULL,
    -- Optional: event_day_id / event_id / membership_id the token is scoped to.
    bound_resource TEXT,
    issued_at      TEXT NOT NULL,
    expires_at     TEXT NOT NULL,
    -- Set on first successful POST. A replay returns the prior result / benign no-op.
    consumed_at    TEXT,
    -- Opaque reference to the result (e.g. the new status, or "deleted").
    result_ref     TEXT
);

CREATE TABLE IF NOT EXISTS audit_log (
    id                   TEXT PRIMARY KEY,
    community_id         TEXT,
    actor_membership_id  TEXT,
    target_kind          TEXT NOT NULL,
    target_id            TEXT,
    action               TEXT NOT NULL,
    -- Structured + redacted. NEVER store full note bodies (RFC-014).
    metadata_json        TEXT,
    created_at           TEXT NOT NULL
);

-- ── Indexes ───────────────────────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_memberships_user
    ON community_memberships(user_id);

CREATE INDEX IF NOT EXISTS idx_memberships_community_active
    ON community_memberships(community_id, removed_at);

-- Home window query: bounded date scan for one community (RFC-002 §6).
CREATE INDEX IF NOT EXISTS idx_event_days_community_time
    ON event_days(community_id, starts_at_utc);

CREATE INDEX IF NOT EXISTS idx_event_days_event
    ON event_days(event_id, seq);

CREATE INDEX IF NOT EXISTS idx_attendances_day
    ON attendances(event_day_id);

CREATE INDEX IF NOT EXISTS idx_attendances_membership
    ON attendances(membership_id);

CREATE INDEX IF NOT EXISTS idx_event_notes_event
    ON event_notes(event_id);

CREATE INDEX IF NOT EXISTS idx_sessions_hmac
    ON sessions(session_hmac);

CREATE INDEX IF NOT EXISTS idx_invite_codes_hmac
    ON invite_codes(code_hmac);

CREATE INDEX IF NOT EXISTS idx_form_tokens_user
    ON form_tokens(user_id);
