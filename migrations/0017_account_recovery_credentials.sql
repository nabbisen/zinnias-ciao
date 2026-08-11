-- Migration 0017: account recovery credentials (RFC-081 §3, Handoff 057)
--
-- A member-held, provider-independent way back into their account. Follows
-- `membership_relink_codes` (migration 0008) rather than inventing a new
-- shape: HMAC at rest, never plaintext (AD-3), used_at/revoked_at, an
-- expiry column, and an index on the active-lookup shape.
--
-- Unlike a relink code, this credential is meant to be held indefinitely —
-- it is a member's *only* provider-independent way in, potentially for as
-- long as they hold the account, not a short-lived bearer code generated
-- and redeemed within minutes. `expires_at` is therefore nullable and left
-- NULL at issuance: the column exists for shape-consistency with every
-- other credential table in this schema and to leave room for a future
-- expiry policy, but nothing in this package ever sets it. The "usable
-- method" check (`db/recovery.rs`) treats NULL as never-expired, not as
-- already-expired — the opposite of every other NULL-is-fail-closed
-- convention in this codebase, because here a member with no other method
-- must be able to reach this credential indefinitely (RFC-081 §3.1: "must
-- not mean hidden until an emergency and then unusable").
--
-- No CHECK constraint and no FOREIGN KEY beyond `users(id)`, matching this
-- project's established D1/SQLite posture elsewhere in this schema.

CREATE TABLE IF NOT EXISTS account_recovery_credentials (
    id          TEXT NOT NULL PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id),
    code_hmac   TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL,
    expires_at  TEXT,
    consumed_at TEXT,
    revoked_at  TEXT
);

CREATE INDEX IF NOT EXISTS idx_account_recovery_credentials_user_active
    ON account_recovery_credentials (user_id, consumed_at, revoked_at, expires_at);
