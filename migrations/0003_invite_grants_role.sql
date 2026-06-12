-- Migration 0003: add grants_role to invite_codes (RFC-003 / v0.6.2 fix)
--
-- Purpose: the join handler was hardcoding 'member' for every redeemed invite.
-- Admin bootstrapping requires the setup-seeded invite to grant 'admin' role.
-- All existing invite codes receive DEFAULT 'member' — no behaviour change.

ALTER TABLE invite_codes
  ADD COLUMN grants_role TEXT NOT NULL DEFAULT 'member'
    CHECK(grants_role IN ('admin', 'member'));
