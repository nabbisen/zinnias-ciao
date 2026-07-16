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

CREATE TRIGGER reject_proof_audit
BEFORE INSERT ON audit_log
WHEN NEW.request_id = 'proof-audit-failure'
BEGIN
    SELECT RAISE(ABORT, 'synthetic required audit rejection');
END;
