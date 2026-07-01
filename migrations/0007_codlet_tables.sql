-- Migration 0007: codlet auth tables (RFC-014, codlet v0.15.x Option A)
--
-- Creates codlet's three auth tables alongside the existing service tables.
-- Existing invite_codes / sessions / form_tokens are NOT renamed or altered;
-- they continue to serve all non-join handlers during the grace period.
--
-- New invite codes, sessions, and form tokens from the join flow are written
-- here under codlet's domain-separated HMAC scheme. Legacy session lookup
-- stays active for 30 days (SESSION_TTL_SECONDS) to avoid forcing re-login.

CREATE TABLE IF NOT EXISTS codlet_codes (
    id              TEXT    NOT NULL PRIMARY KEY,
    lookup_key      TEXT    NOT NULL UNIQUE,
    key_version     TEXT    NOT NULL,
    purpose         TEXT,
    scope           TEXT,
    grant_payload   TEXT,
    created_at      INTEGER NOT NULL,
    expires_at      INTEGER NOT NULL,
    used_at         INTEGER,
    used_by_subject TEXT,
    revoked_at      INTEGER
);

CREATE INDEX IF NOT EXISTS idx_codlet_codes_lookup
    ON codlet_codes (lookup_key, used_at, revoked_at, expires_at);
CREATE INDEX IF NOT EXISTS idx_codlet_codes_scope
    ON codlet_codes (scope, used_at, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS codlet_sessions (
    id          TEXT    NOT NULL PRIMARY KEY,
    lookup_key  TEXT    NOT NULL UNIQUE,
    key_version TEXT    NOT NULL,
    subject     TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    expires_at  INTEGER NOT NULL,
    revoked_at  INTEGER
);

CREATE INDEX IF NOT EXISTS idx_codlet_sessions_lookup
    ON codlet_sessions (lookup_key, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS codlet_form_tokens (
    lookup_key      TEXT    NOT NULL PRIMARY KEY,
    key_version     TEXT    NOT NULL,
    subject_kind    TEXT    NOT NULL,
    purpose         TEXT    NOT NULL,
    bound_resource  TEXT,
    issued_at       INTEGER NOT NULL,
    expires_at      INTEGER NOT NULL,
    consumed_at     INTEGER,
    result_ref      TEXT
);

CREATE INDEX IF NOT EXISTS idx_codlet_form_tokens_lookup
    ON codlet_form_tokens (lookup_key, consumed_at, expires_at);
