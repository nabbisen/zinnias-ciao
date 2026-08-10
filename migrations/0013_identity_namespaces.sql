-- RFC-080 §3.2/§3.3 (Handoff 050, external-identity Slice 2): the identity
-- namespace and user_identities tables. Additive only — no ALTER TABLE on
-- any existing table, no behaviour change, no callers yet (Slice 4 wires
-- these into a route).
--
-- identity_namespaces is an immutable record of a reviewed provider
-- registration (RFC-080 §3.2). Namespaces are created by migration or
-- reviewed configuration, never at runtime from a token — the release
-- gate `rfc080_identity_namespaces_are_never_created_outside_a_migration`
-- keeps that true after code that touches this table exists. Only the
-- local-fake namespace is seeded here; no production or staging namespace
-- exists until a provider RFC creates one.
CREATE TABLE identity_namespaces (
    id            TEXT PRIMARY KEY,
    provider_kind TEXT NOT NULL,
    issuer        TEXT NOT NULL,
    audience      TEXT NOT NULL,
    subject_scope TEXT NOT NULL CHECK(subject_scope IN ('public','pairwise','channel','team')),
    environment   TEXT NOT NULL CHECK(environment IN ('production','staging','local_fake')),
    created_at    TEXT NOT NULL
);

-- user_identities (RFC-080 §3.3). Uniqueness is (identity_namespace_id,
-- subject_lookup) — not the subject alone, not email, not a provider
-- label: two namespaces never identify the same person. subject_lookup is
-- a keyed digest (crypto::subject_lookup, HMAC-SHA256 with the existing
-- pepper, AD-3/RFC-077) — the raw provider subject is never stored.
CREATE TABLE user_identities (
    id                    TEXT PRIMARY KEY,
    user_id               TEXT NOT NULL REFERENCES users(id),
    identity_namespace_id TEXT NOT NULL REFERENCES identity_namespaces(id),
    subject_lookup        TEXT NOT NULL,
    linked_at             TEXT NOT NULL,
    last_authenticated_at TEXT,
    status                TEXT NOT NULL CHECK(status IN ('active','revoked')),
    UNIQUE(identity_namespace_id, subject_lookup)
);

-- The one seeded namespace: the local fake issuer (RFC-080 §10), the only
-- environment this slice, or any slice before the fake-issuer harness
-- lands, may reference. issuer/audience are placeholders naming what the
-- harness will register, not a live registration.
INSERT INTO identity_namespaces
    (id, provider_kind, issuer, audience, subject_scope, environment, created_at)
VALUES
    ('idns_local_fake', 'local_fake', 'local-fake-issuer', 'local-fake-client',
     'public', 'local_fake', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
