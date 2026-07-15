-- RFC-079 Package 0A local proof fixture.
-- This is intentionally outside migrations/ and must never enter the D1
-- migration ledger. It contains synthetic structure and no production data.

DROP TABLE IF EXISTS proof_audits;
DROP TABLE IF EXISTS proof_dependents;
DROP TABLE IF EXISTS audit_change_assertions;
DROP TABLE IF EXISTS proof_items;

CREATE TABLE proof_items (
    id         TEXT PRIMARY KEY,
    case_name  TEXT NOT NULL,
    eligible   INTEGER NOT NULL CHECK (eligible IN (0, 1)),
    winner     TEXT
) STRICT;

CREATE TABLE audit_change_assertions (
    operation_id TEXT PRIMARY KEY
      CHECK (
        length(operation_id) = 26
        AND substr(operation_id, 1, 4) = 'ast_'
        AND substr(operation_id, 5) NOT GLOB '*[^A-Za-z0-9_-]*'
      ),
    changed_count INTEGER NOT NULL CHECK (changed_count = 1)
) STRICT;

CREATE TABLE proof_dependents (
    id         TEXT PRIMARY KEY,
    case_name  TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE proof_audits (
    id         TEXT PRIMARY KEY,
    case_name  TEXT NOT NULL UNIQUE,
    outcome    TEXT NOT NULL CHECK (outcome = 'ok')
) STRICT;
