-- Migration 0008: active-member help-signin codes (RFC-024)
--
-- Codes are HMACs at rest and target a community membership. The redundant
-- community_id is intentional: redemption re-checks membership community before
-- minting a session for the target user_id.

CREATE TABLE IF NOT EXISTS membership_relink_codes (
    id                       TEXT NOT NULL PRIMARY KEY,
    code_hmac                TEXT NOT NULL UNIQUE,
    community_id             TEXT NOT NULL REFERENCES communities(id),
    membership_id            TEXT NOT NULL REFERENCES community_memberships(id),
    created_by_membership_id TEXT NOT NULL REFERENCES community_memberships(id),
    created_at               TEXT NOT NULL,
    expires_at               TEXT NOT NULL,
    used_at                  TEXT,
    revoked_at               TEXT
);

CREATE INDEX IF NOT EXISTS idx_membership_relink_codes_membership_active
    ON membership_relink_codes (membership_id, used_at, revoked_at, expires_at);

CREATE INDEX IF NOT EXISTS idx_membership_relink_codes_community_created
    ON membership_relink_codes (community_id, created_at);
