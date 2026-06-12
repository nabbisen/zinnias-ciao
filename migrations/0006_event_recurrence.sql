-- RFC-022: event recurrence (bounded materialization).
-- These columns are informational — the actual days are already in event_days.
-- repeat_rule: 'none' | 'weekly' | 'biweekly' | 'monthly'
-- repeat_count: how many occurrences were generated (NULL for non-recurring)
ALTER TABLE events ADD COLUMN repeat_rule  TEXT NOT NULL DEFAULT 'none'
    CHECK(repeat_rule IN ('none','weekly','biweekly','monthly'));
ALTER TABLE events ADD COLUMN repeat_count INTEGER;
