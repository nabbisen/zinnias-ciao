CREATE TABLE proof_boundary_audits (
  id TEXT PRIMARY KEY,
  request_id TEXT NOT NULL CHECK (request_id != 'force_failure'),
  action TEXT NOT NULL
);

CREATE TABLE proof_boundary_payloads (
  kind TEXT PRIMARY KEY,
  protected_value TEXT NOT NULL
);

CREATE TABLE proof_boundary_sessions (
  id TEXT PRIMARY KEY,
  revoked INTEGER NOT NULL DEFAULT 0 CHECK (revoked IN (0, 1))
);

INSERT INTO proof_boundary_payloads (kind, protected_value)
VALUES ('community', 'protected-community-json');

INSERT INTO proof_boundary_sessions (id, revoked)
VALUES ('session_proof', 0);
