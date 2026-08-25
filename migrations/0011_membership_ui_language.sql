-- RFC-072: membership-scoped UI language preference (Slice A, the locale seam).
--
-- Additive only. Nullable, no backfill: every existing membership row keeps
-- rendering whatever the caller resolves for a NULL value, with no write to
-- this row. RFC-085 separated that resolution into two named answers in
-- packages/contracts/src/locale.rs; a NULL here always means the product's
-- own current-preference answer (English, since ROADMAP.md's decision was
-- taken by Handoff 079; Japanese in the first slice this migration shipped
-- in) — never the separate, unmoving fail-closed answer a corrupt value
-- resolves to. This schema and its CHECK constraint are unaffected by that
-- decision — this comment is the only thing that changed. Rollback is
-- dropping an unread column.

ALTER TABLE community_memberships
ADD COLUMN ui_language TEXT
CHECK(ui_language IN ('ja', 'en') OR ui_language IS NULL);
