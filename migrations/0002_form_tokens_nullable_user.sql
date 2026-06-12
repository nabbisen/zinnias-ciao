-- Migration 0002 — make form_tokens.user_id nullable (no FK).
--
-- Reason: the join flow issues tokens before a user row exists
-- (GET /join and POST /join are pre-authentication).  The original
-- REFERENCES users(id) FK caused a FOREIGN KEY constraint failure.
-- form_tokens is a short-lived operational table; the HMAC scopes
-- each token independently of the user FK.
--
-- SQLite does not support DROP CONSTRAINT, so we recreate the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE IF NOT EXISTS form_tokens_new (
    token_hmac     TEXT PRIMARY KEY,
    user_id        TEXT,   -- NULL / empty for pre-auth tokens; no FK
    purpose        TEXT NOT NULL,
    bound_resource TEXT,
    issued_at      TEXT NOT NULL,
    expires_at     TEXT NOT NULL,
    consumed_at    TEXT,
    result_ref     TEXT
);

INSERT OR IGNORE INTO form_tokens_new
    SELECT token_hmac, user_id, purpose, bound_resource,
           issued_at, expires_at, consumed_at, result_ref
    FROM form_tokens;

DROP TABLE form_tokens;
ALTER TABLE form_tokens_new RENAME TO form_tokens;

CREATE INDEX IF NOT EXISTS idx_form_tokens_user
    ON form_tokens(user_id);

PRAGMA foreign_keys = ON;
