-- RFC-072: membership-scoped UI language preference (Slice A, the locale seam).
--
-- Additive only. Nullable, no backfill: every existing membership row keeps
-- rendering Japanese with no write. NULL means Japanese fallback in the
-- first slice. Rollback is dropping an unread column.

ALTER TABLE community_memberships
ADD COLUMN ui_language TEXT
CHECK(ui_language IN ('ja', 'en') OR ui_language IS NULL);
