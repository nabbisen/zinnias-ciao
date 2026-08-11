-- RFC-080 §5 (Handoff 053, external-identity Slice 4a): the server-side
-- authentication transaction. Additive only — no ALTER TABLE on any
-- existing table, no callers yet (4b wires this into SSR routes).
--
-- Binds one in-flight authorization-code + PKCE + state/nonce exchange
-- across the redirect-out and callback-in requests. Single-use, enforced
-- the same way as invite_codes/membership_relink_codes: a nullable
-- consumed_at, checked with `WHERE consumed_at IS NULL` at consumption
-- time so a second attempt against an already-consumed row affects zero
-- rows rather than racing.
--
-- lookup_key_hmac and the OIDC `state` parameter are the same concern,
-- deliberately unified rather than kept as two columns: the raw `state`
-- value this row was created for is never stored — only its HMAC. On
-- callback, the presented `state` is HMACed and matched against
-- lookup_key_hmac; successfully finding the row *is* the state check, so
-- there is no separate "expected state" column to keep in sync or drift
-- from it. Same AD-3 discipline as session_hmac/code_hmac: the row is
-- found by a digest, never by a value a browser carries in the clear.
--
-- nonce_hmac is the same idea applied to the OIDC nonce: the raw nonce
-- sent in the authorize request is never stored; the verified ID token's
-- own `nonce` claim is HMACed and compared against this column.
--
-- pkce_verifier is stored raw, not digested — unlike lookup_key_hmac and
-- nonce_hmac, the code_verifier must be recoverable in full to complete
-- the token exchange (RFC 7636's exchange step sends the raw verifier;
-- a one-way digest cannot be reversed for that). This is not a departure
-- from AD-3's "digest what can be a digest" principle so much as a case
-- outside it: the verifier alone is not a bearer credential — it is
-- worthless without the also-required, single-use, provider-issued
-- authorization code, which this table never stores at all (it is
-- consumed immediately, in-memory, during the token exchange 4b performs).
--
-- invite_reference (action = 'join' only) is a non-secret internal
-- reference — the referenced invite_codes row's own id, never the invite
-- plaintext or its HMAC, which live only in invite_codes itself.
CREATE TABLE auth_transactions (
    id                             TEXT PRIMARY KEY,
    lookup_key_hmac                TEXT NOT NULL UNIQUE,
    action                         TEXT NOT NULL CHECK(action IN ('sign_in', 'join', 'link')),
    identity_namespace_id          TEXT NOT NULL REFERENCES identity_namespaces(id),
    nonce_hmac                     TEXT NOT NULL,
    pkce_verifier                  TEXT NOT NULL,
    -- 'link' only: the provenance of the session that initiated the link,
    -- so the callback can confirm it is still the same kind of session.
    initiating_session_provenance  TEXT,
    -- 'join' only: see comment above.
    invite_reference                TEXT,
    callback_uri                   TEXT NOT NULL,
    -- Allowlisted post-login destination; 4b validates against its own
    -- allowlist at consumption time, not at creation time.
    return_to                      TEXT,
    created_at                     TEXT NOT NULL,
    expires_at                     TEXT NOT NULL,
    consumed_at                    TEXT
);
