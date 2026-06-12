-- RFC-023: per-membership calendar export tokens.
-- One active token per (membership_id, community_id) pair.
-- Revoked by setting revoked_at; regeneration inserts a new row.
-- token_hmac stores HMAC-SHA256(pepper, plaintext_token) — never the plaintext.

CREATE TABLE IF NOT EXISTS calendar_tokens (
    id            TEXT PRIMARY KEY,
    community_id  TEXT NOT NULL REFERENCES communities(id),
    membership_id TEXT NOT NULL REFERENCES community_memberships(id),
    token_hmac    TEXT NOT NULL UNIQUE,
    created_at    TEXT NOT NULL,
    revoked_at    TEXT
);

CREATE INDEX IF NOT EXISTS idx_calendar_tokens_membership
    ON calendar_tokens(membership_id, community_id)
    WHERE revoked_at IS NULL;
