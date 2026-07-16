-- Local-only business-state fixture for the required-audit batch proof.
CREATE TABLE proof_mutations (
    case_name TEXT PRIMARY KEY,
    state INTEGER NOT NULL CHECK (state IN (0, 1)),
    allowed INTEGER NOT NULL CHECK (allowed IN (0, 1))
) STRICT;

CREATE TABLE proof_multi_writes (
    id TEXT PRIMARY KEY,
    case_name TEXT NOT NULL
) STRICT;

CREATE TABLE proof_event_headers (
    id TEXT PRIMARY KEY,
    allowed INTEGER NOT NULL CHECK (allowed IN (0, 1))
) STRICT;

CREATE TABLE proof_event_parts (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES proof_event_headers(id)
) STRICT;

CREATE TABLE proof_attendance (
    cell_id TEXT PRIMARY KEY,
    status TEXT
) STRICT;

CREATE TABLE proof_calendar_tokens (
    id TEXT PRIMARY KEY,
    active INTEGER NOT NULL CHECK (active IN (0, 1))
) STRICT;

CREATE TABLE proof_edit_events (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL
) STRICT;

CREATE TABLE proof_edit_days (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES proof_edit_events(id),
    day_date TEXT NOT NULL,
    occurrence_status TEXT NOT NULL
) STRICT;

CREATE TABLE proof_occurrence_days (
    id TEXT PRIMARY KEY,
    occurrence_status TEXT NOT NULL
) STRICT;

CREATE TABLE proof_occurrence_exceptions (
    day_id TEXT PRIMARY KEY REFERENCES proof_occurrence_days(id)
) STRICT;

CREATE TRIGGER reject_proof_audit
BEFORE INSERT ON audit_log
WHEN NEW.request_id = 'proof-audit-failure'
BEGIN
    SELECT RAISE(ABORT, 'synthetic required audit rejection');
END;
