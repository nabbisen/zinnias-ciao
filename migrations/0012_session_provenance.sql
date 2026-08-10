-- RFC-081 §2 / §2.1a (Handoff 048, external-identity Slice 1): give every
-- session a provenance and, for community-admin-mediated sessions, a
-- binding to the community that granted them.
--
-- Closes a live gap: authorization previously resolved from
-- (auth.user_id, community_id) alone, so a relink- or help-signin-derived
-- session — grantable by a single community's admin — authorized every
-- community that user_id belonged to, not just the granting one.
--
-- `provenance` is nullable in the schema, not `NOT NULL DEFAULT '...'`. A
-- default would stamp legacy rows with a provenance they never had; a
-- `'legacy'` value would be a value with no producer. Authorization
-- (workers/ssr/src/authz.rs) refuses NULL provenance instead, so the
-- guarantee comes from behaviour, not from an invented fact in the table.
--
-- `scope_community_id` is NULL for first-class (invite-redemption)
-- sessions and set to the granting community for relink-derived ones.
--
-- RFC-081 §11.4: as of this migration no real community has used the
-- service (deployment remains No-Go), so every pre-existing session is
-- revoked outright rather than carried forward through a migration
-- ceremony — there is no legacy-assurance session class to preserve.

ALTER TABLE sessions ADD COLUMN provenance TEXT;
ALTER TABLE sessions ADD COLUMN scope_community_id TEXT REFERENCES communities(id);

UPDATE sessions
SET revoked_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE revoked_at IS NULL;
