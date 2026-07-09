-- RFC-065: recurrence v2 and occurrence exceptions.
-- Adds a recurrence-series source of truth while preserving event_days as the
-- stable attendance anchor.

CREATE TABLE IF NOT EXISTS event_series (
    id                            TEXT PRIMARY KEY,
    event_id                      TEXT NOT NULL UNIQUE REFERENCES events(id),
    community_id                  TEXT NOT NULL REFERENCES communities(id),
    frequency                     TEXT NOT NULL CHECK(frequency IN ('weekly','biweekly','monthly')),
    start_day_date                TEXT NOT NULL,
    -- NULL for legacy migrated series. Existing event_days remain usable, but
    -- future materialization is disabled unless local form times are known.
    starts_at_local               TEXT,
    ends_at_local                 TEXT,
    timezone                      TEXT NOT NULL,
    end_mode                      TEXT NOT NULL CHECK(end_mode IN ('after_count','until_date','open_ended')),
    occurrence_count              INTEGER,
    until_day_date                TEXT,
    materialized_through_day_date TEXT,
    created_at                    TEXT NOT NULL,
    updated_at                    TEXT NOT NULL
);

ALTER TABLE event_days
    ADD COLUMN occurrence_status TEXT NOT NULL DEFAULT 'scheduled'
        CHECK(occurrence_status IN ('scheduled','cancelled'));

ALTER TABLE event_days ADD COLUMN series_id TEXT REFERENCES event_series(id);
ALTER TABLE event_days ADD COLUMN series_occurrence_date TEXT;

INSERT INTO event_series (
    id,
    event_id,
    community_id,
    frequency,
    start_day_date,
    starts_at_local,
    ends_at_local,
    timezone,
    end_mode,
    occurrence_count,
    until_day_date,
    materialized_through_day_date,
    created_at,
    updated_at
)
SELECT
    'ser_' || e.id,
    e.id,
    e.community_id,
    e.repeat_rule,
    first_day.day_date,
    NULL,
    NULL,
    c.timezone,
    'after_count',
    e.repeat_count,
    NULL,
    max_day.max_day_date,
    e.created_at,
    e.updated_at
FROM events e
JOIN communities c ON c.id = e.community_id
JOIN event_days first_day ON first_day.event_id = e.id AND first_day.seq = 1
JOIN (
    SELECT event_id, MAX(day_date) AS max_day_date
    FROM event_days
    GROUP BY event_id
) max_day ON max_day.event_id = e.id
WHERE e.repeat_rule != 'none' OR e.repeat_count IS NOT NULL;

UPDATE event_days
SET
    series_id = 'ser_' || event_id,
    series_occurrence_date = day_date
WHERE event_id IN (
    SELECT event_id FROM event_series
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_event_days_series_occurrence
    ON event_days(series_id, series_occurrence_date)
    WHERE series_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_event_series_community
    ON event_series(community_id, event_id);

CREATE TABLE IF NOT EXISTS event_series_exceptions (
    id                       TEXT PRIMARY KEY,
    series_id                TEXT NOT NULL REFERENCES event_series(id),
    community_id             TEXT NOT NULL REFERENCES communities(id),
    exception_day_date       TEXT NOT NULL,
    action                   TEXT NOT NULL CHECK(action IN ('skip','cancel')),
    event_day_id             TEXT REFERENCES event_days(id),
    created_by_membership_id TEXT NOT NULL REFERENCES community_memberships(id),
    created_at               TEXT NOT NULL,
    CHECK (
        (action = 'skip' AND event_day_id IS NULL)
        OR (action = 'cancel' AND event_day_id IS NOT NULL)
    ),
    UNIQUE(series_id, exception_day_date)
);

CREATE INDEX IF NOT EXISTS idx_event_series_exceptions_community
    ON event_series_exceptions(community_id, series_id);
