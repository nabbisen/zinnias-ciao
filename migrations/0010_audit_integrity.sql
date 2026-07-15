-- RFC-079: closed audit integrity schema and destructive legacy metadata reset.
--
-- This migration intentionally never reads legacy metadata_json. It preserves
-- the core audit chronology byte-for-byte, assigns request_id = 'legacy', and
-- replaces every historical metadata value with the empty object.

PRAGMA defer_foreign_keys = ON;

CREATE TABLE audit_log_v2 (
    id                   TEXT PRIMARY KEY,
    request_id           TEXT NOT NULL,
    community_id         TEXT,
    actor_membership_id  TEXT,
    target_kind          TEXT NOT NULL,
    target_id            TEXT,
    action               TEXT NOT NULL,
    metadata_json        TEXT NOT NULL DEFAULT '{}'
        CHECK (
            json_valid(metadata_json)
            AND json_type(metadata_json) = 'object'
            AND length(CAST(metadata_json AS BLOB)) <= 2048
        ),
    created_at           TEXT NOT NULL,
    CHECK (length(id) BETWEEN 8 AND 64),
    CHECK (length(request_id) BETWEEN 1 AND 96),
    CHECK (length(target_kind) BETWEEN 1 AND 64),
    CHECK (length(action) BETWEEN 3 AND 96)
) STRICT;

INSERT INTO audit_log_v2 (
    id,
    request_id,
    community_id,
    actor_membership_id,
    target_kind,
    target_id,
    action,
    metadata_json,
    created_at
)
SELECT
    id,
    'legacy',
    community_id,
    actor_membership_id,
    target_kind,
    target_id,
    action,
    '{}',
    created_at
FROM audit_log;

-- Constraint-backed verification makes any mismatch abort the migration before
-- the old table is dropped. Core rows are compared losslessly in both
-- directions; legacy metadata is deliberately excluded from every query.
CREATE TABLE audit_migration_0010_guard (
    check_name TEXT PRIMARY KEY,
    passed     INTEGER NOT NULL CHECK (passed = 1)
) STRICT;

INSERT INTO audit_migration_0010_guard (check_name, passed)
SELECT
    'row_count',
    CASE
        WHEN (SELECT COUNT(*) FROM audit_log)
           = (SELECT COUNT(*) FROM audit_log_v2)
        THEN 1 ELSE 0
    END;

INSERT INTO audit_migration_0010_guard (check_name, passed)
SELECT
    'core_rows_forward',
    CASE WHEN NOT EXISTS (
        SELECT
            id,
            community_id,
            actor_membership_id,
            target_kind,
            target_id,
            action,
            created_at
        FROM audit_log
        EXCEPT
        SELECT
            id,
            community_id,
            actor_membership_id,
            target_kind,
            target_id,
            action,
            created_at
        FROM audit_log_v2
    ) THEN 1 ELSE 0 END;

INSERT INTO audit_migration_0010_guard (check_name, passed)
SELECT
    'core_rows_reverse',
    CASE WHEN NOT EXISTS (
        SELECT
            id,
            community_id,
            actor_membership_id,
            target_kind,
            target_id,
            action,
            created_at
        FROM audit_log_v2
        EXCEPT
        SELECT
            id,
            community_id,
            actor_membership_id,
            target_kind,
            target_id,
            action,
            created_at
        FROM audit_log
    ) THEN 1 ELSE 0 END;

INSERT INTO audit_migration_0010_guard (check_name, passed)
SELECT
    'legacy_reset',
    CASE WHEN NOT EXISTS (
        SELECT 1
        FROM audit_log_v2
        WHERE request_id <> 'legacy' OR metadata_json <> '{}'
    ) THEN 1 ELSE 0 END;

ALTER TABLE audit_log RENAME TO audit_log_legacy_0010;
ALTER TABLE audit_log_v2 RENAME TO audit_log;

CREATE INDEX idx_audit_log_community_created_at
    ON audit_log(community_id, created_at);

CREATE INDEX idx_audit_log_action_created_at
    ON audit_log(action, created_at);

DROP TABLE audit_log_legacy_0010;
DROP TABLE audit_migration_0010_guard;

CREATE TABLE audit_change_assertions (
    operation_id  TEXT PRIMARY KEY
        CHECK (
            length(operation_id) = 26
            AND substr(operation_id, 1, 4) = 'ast_'
            AND substr(operation_id, 5) NOT GLOB '*[^A-Za-z0-9_-]*'
        ),
    changed_count INTEGER NOT NULL CHECK (changed_count = 1)
) STRICT;
