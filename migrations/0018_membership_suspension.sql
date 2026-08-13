-- Migration 0018: membership suspension (RFC-082, Handoff 058)
--
-- A reversible state beside the terminal `removed_at`. Both columns are
-- additive and nullable — no table rebuild, matching migration 0017's own
-- reasoning and RFC-081 §1.2a: a rebuild is impossible under D1 for a table
-- with dependent rows.
--
-- `suspended_by_membership_id` records the acting admin at suspension time
-- and is cleared alongside `suspended_at` on unsuspend — it is not a
-- history of every suspend/unsuspend cycle, only the current one, matching
-- the audit log (not this column) as the place that history actually lives.
--
-- `idx_memberships_one_active_per_user` (migration 0001) is
-- `WHERE removed_at IS NULL`, unchanged here: a suspended row is not
-- removed, so it still occupies the (community_id, user_id) pair — nobody
-- may hold a suspended and an active membership in one community. That
-- migration's own comment already anticipated this.

ALTER TABLE community_memberships ADD COLUMN suspended_at TEXT;
ALTER TABLE community_memberships ADD COLUMN suspended_by_membership_id TEXT
    REFERENCES community_memberships(id);
