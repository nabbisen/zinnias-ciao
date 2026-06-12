-- RFC-032: event templates for admin quick-create.
-- Templates are community-scoped and admin-only.
-- Creating an event from a template copies fields; later template edits
-- do not mutate existing events.

CREATE TABLE IF NOT EXISTS event_templates (
    id                       TEXT PRIMARY KEY,
    community_id             TEXT NOT NULL REFERENCES communities(id),
    created_by_membership_id TEXT NOT NULL REFERENCES community_memberships(id),
    title                    TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 80),
    location                 TEXT CHECK(location IS NULL OR length(location) <= 120),
    description              TEXT CHECK(description IS NULL OR length(description) <= 500),
    -- Default duration in minutes (e.g. 90). NULL = admin must set on each use.
    duration_minutes         INTEGER CHECK(duration_minutes IS NULL OR duration_minutes > 0),
    is_active                INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)),
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_event_templates_community
    ON event_templates(community_id)
    WHERE is_active = 1;
