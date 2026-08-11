-- RFC-080 §6 / RFC-081 §6 (Handoff 055, external-identity Slice 5a): give
-- every session its own authentication time, distinct from `created_at`.
--
-- Deferred deliberately at Handoff 053 §7.2: until this package, every
-- minting site sets both fields to the same value, so `created_at` *is*
-- the authentication time everywhere — only session rotation (Slice 5b)
-- makes them diverge. Adding the column here, ahead of its first real
-- consumer, keeps the schema change and the rotation logic in separate,
-- independently reviewable packages.
--
-- Nullable, for the same reason `provenance` is (migration 0012): a
-- default would stamp existing rows with a fact they never had. The
-- step-up predicate (`authz.rs`) treats NULL as "not fresh" — fail-closed,
-- consistent with how NULL provenance is already refused.

ALTER TABLE sessions ADD COLUMN authenticated_at TEXT;
