// RFC-050 local evidence tooling (Tooling Slice 2): candidate manifest, schema
// validation, and redaction utilities shared by every local evidence
// collector. Nothing in this module performs a hosted call, deploy, or
// resource mutation — it is pure data validation and serialization.
//
// A record produced here is local evidence only. It becomes RFC-050 B4
// evidence solely when the same tooling runs against a frozen hosted
// candidate (RFC-050 Tooling Slice 3 onward).

import { createHash } from 'node:crypto';

export const MANIFEST_SCHEMA_VERSION = 1;
export const MANIFEST_FILENAME = '00-manifest.json';
export const LOCAL_GATES_FILENAME = '01-local-gates.json';

export class EvidenceRedactionError extends Error {
  constructor(category, path, message) {
    super(message);
    this.name = 'EvidenceRedactionError';
    this.category = category;
    this.path = path;
  }
}

export class EvidenceSchemaError extends Error {}

// Fields whose value is required to be hex-shaped for a legitimate reason
// (a git commit sha, a namespaced artifact hash). Every other bare hex string
// of a length matching a common secret/digest/token/Durable-Object-id length
// is rejected wherever it appears, since this project's own tokens, session
// secrets, subject digests, and Durable Object ids are all bare lowercase hex.
const ARTIFACT_HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;
const COMMIT_PATTERN = /^[0-9a-f]{7,40}$/i;

// Digest/token/secret/Durable-Object-id lengths this project actually
// produces: sha1/DO-id-adjacent (40), sha256/HMAC token (64, this project's
// `random_token()` length), sha512 (128), plus the shorter md5 (32) and
// sha224 (56) lengths some tooling in the wild still emits.
const HEX_SECRET_LENGTHS = new Set([32, 40, 56, 64, 96, 128]);
const HEX_ONLY_PATTERN = /^[0-9a-f]+$/i;

// This project's real, persisted resource-id prefixes (see
// `workers/ssr/src/handlers/community_create.rs`'s `format!("com_{}", ...)`
// and siblings). Evidence tooling must alias resource identities instead of
// recording them raw; a value shaped like one of these is a leak, not a
// campaign-local alias.
const RAW_RESOURCE_ID_PREFIXES = [
  'com_', 'mem_', 'att_', 'aud_', 'ast_', 'day_', 'event_', 'safe_', 'ser_',
  'usr_', 'sess_', 'inv_', 'note_',
];

const FORBIDDEN_KEY_PATTERN = /(password|credential|secret|pepper|csrf|api[_-]?key|cookie|form[_-]?token|durable[_-]?object|do[_-]?name|do[_-]?id|subject[_-]?(id|digest)|^binds?$|bindings?$)/i;

// Requires the syntactic companion of each keyword (not a bare English verb
// like "select" or "update" appearing in a human-written `observed`
// description) so ordinary prose does not false-positive as SQL.
const SQL_KEYWORD_PATTERN = /\b(SELECT\b[\s\S]{0,200}?\bFROM\b|INSERT\s+INTO\b|UPDATE\b[\s\S]{0,200}?\bSET\b|DELETE\s+FROM\b|CREATE\s+TABLE\b|ALTER\s+TABLE\b|DROP\s+TABLE\b)/i;
const D1_ERROR_PATTERN = /D1_ERROR/;
const COOKIE_SHAPED_VALUE_PATTERN = /ciao_sid\s*=/i;

// Path-scoped, not name-scoped (S-N2 from the Slices 1+2 review): a nested
// field that merely happens to be named "commit" elsewhere does not inherit
// the exemption, only the real candidate-tuple/record positions do.
const HEX_FIELD_PATH_EXEMPTIONS = [
  { pattern: /\.candidate\.commit$/, valuePattern: COMMIT_PATTERN, name: 'commit' },
  { pattern: /\.artifactHash$/, valuePattern: ARTIFACT_HASH_PATTERN, name: 'artifactHash' },
];

// Run-scoped value registry (S-N1 from the Slices 1+2 review). A synthetic
// invite code, display name, or note body is shape-indistinguishable from
// ordinary prose, so no pattern rule can catch it inside a free-text field
// like `observed` without false-positiving on legitimate descriptions. A
// harness that generates such a value registers it for the run; any string
// containing a registered value is rejected by exact-match containment,
// which is mechanical and has no false-positive mode.
const registeredRunSecrets = new Set();

// S3-N2 from the Slices 3+4 review: a registered value shorter than this
// would reject unrelated prose (e.g. registering "ok" breaks any record
// containing that word). Failing closed here is deliberate — the caller
// must pick a longer, specific fixture value rather than have the registry
// silently skip or truncate a short one.
const MIN_REGISTERED_SECRET_LENGTH = 4;

export function registerRunSecrets(values) {
  for (const value of values) {
    if (typeof value !== 'string' || value.length === 0) {
      throw new EvidenceSchemaError('registerRunSecrets values must be non-empty strings');
    }
    if (value.length < MIN_REGISTERED_SECRET_LENGTH) {
      throw new EvidenceSchemaError(
        `registerRunSecrets value "${value}" is shorter than ${MIN_REGISTERED_SECRET_LENGTH} characters; `
        + 'a short value would match unrelated prose and reject unrelated records — use a longer, specific fixture value',
      );
    }
    registeredRunSecrets.add(value);
  }
}

export function clearRegisteredRunSecrets() {
  registeredRunSecrets.clear();
}

// Handoff 065: every category a single string value could violate, collected
// rather than returned on the first hit — the shared core both
// `checkStringValue` (throws the first one, for the existing fail-fast
// construction/parse callers below) and the exhaustive scanner (collects all
// of them) are built from, so the two can never drift on what counts as a
// violation or in what order.
function stringViolations(value, path) {
  const violations = [];
  // Case-insensitive (S3-N1 from the Slices 3+4 review): the application
  // itself accepts invite codes case-insensitively, so a collector may
  // legitimately hold or record a code in a different case than it was
  // generated in. Matching case-sensitively would let that case variant
  // silently defeat the registry.
  const lowerValue = value.toLowerCase();
  for (const secret of registeredRunSecrets) {
    if (lowerValue.includes(secret.toLowerCase())) {
      violations.push(new EvidenceRedactionError(
        'registered_run_secret',
        path,
        `value at ${path} contains a registered run-scoped value (never the value itself, only its presence)`,
      ));
    }
  }
  const hexExemption = HEX_FIELD_PATH_EXEMPTIONS.find((entry) => entry.pattern.test(path));
  if (hexExemption) {
    if (!hexExemption.valuePattern.test(value)) {
      violations.push(new EvidenceRedactionError(
        'malformed_exempt_field',
        path,
        `"${hexExemption.name}" at ${path} does not match its required shape`,
      ));
    }
  } else if (HEX_ONLY_PATTERN.test(value) && HEX_SECRET_LENGTHS.has(value.length)) {
    violations.push(new EvidenceRedactionError(
      'raw_or_hashed_secret',
      path,
      `bare ${value.length}-hex-char value at ${path} (secret, digest, subject identifier, or Durable Object id shape)`,
    ));
  }
  if (RAW_RESOURCE_ID_PREFIXES.some((prefix) => value.startsWith(prefix))) {
    violations.push(new EvidenceRedactionError('raw_resource_id', path, `raw resource-id-shaped value at ${path}`));
  }
  if (COOKIE_SHAPED_VALUE_PATTERN.test(value)) {
    violations.push(new EvidenceRedactionError('cookie', path, `cookie-shaped value at ${path}`));
  }
  if (D1_ERROR_PATTERN.test(value)) {
    violations.push(new EvidenceRedactionError('d1_error_body', path, `D1 error body at ${path}`));
  }
  if (SQL_KEYWORD_PATTERN.test(value)) {
    violations.push(new EvidenceRedactionError('sql', path, `SQL-shaped text at ${path}`));
  }
  return violations;
}

// Fail-fast: throws the first violation `stringViolations` finds, in the same
// order it always has. Used by `assertRedacted` (construction/parsing), where
// one bad value is already reason enough to reject the whole record — those
// callers have never needed, and do not now need, every violation in one
// value at once.
function checkStringValue(value, path) {
  const violations = stringViolations(value, path);
  if (violations.length > 0) {
    throw violations[0];
  }
}

// Handoff 065 §3.1: the exhaustive counterpart to `assertRedacted`'s
// tree-walk, collecting into `violations` instead of throwing on the first
// hit. Mirrors `assertRedacted`'s structure exactly (same HAR-shape check,
// same forbidden-key check, same recursion into arrays/objects) so the two
// can never see a different set of fields — only what happens once a
// violation is found (stop vs. collect-and-continue) differs.
function collectRedactionViolations(value, path, violations) {
  if (value === null || value === undefined) return;
  if (typeof value === 'string') {
    violations.push(...stringViolations(value, path));
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => collectRedactionViolations(item, `${path}[${index}]`, violations));
    return;
  }
  if (typeof value === 'object') {
    if (isHarShaped(value)) {
      violations.push(new EvidenceRedactionError('har_file', path, `HAR-shaped object at ${path}`));
      return;
    }
    for (const [key, nested] of Object.entries(value)) {
      if (FORBIDDEN_KEY_PATTERN.test(key) && valueCanCarryString(nested)) {
        violations.push(new EvidenceRedactionError('forbidden_key', `${path}.${key}`, `forbidden field name "${key}" at ${path}.${key}`));
      }
      // Recurse regardless of whether the key itself was just flagged — a
      // forbidden-keyed field's value can hold its own, distinct violation
      // (e.g. a raw resource id nested under a field also named badly), and
      // under-reporting one to avoid a second finding on the same branch is
      // exactly the failure mode this handoff exists to remove.
      collectRedactionViolations(nested, `${path}.${key}`, violations);
    }
    return;
  }
  // numbers and booleans carry no redaction risk.
}

// Handoff 067: `FORBIDDEN_KEY_PATTERN` matches a field *name* — but a name
// like `cookie` or `credential` is only a leak risk if the field's *value*
// could actually hold string content. A boolean or number can never carry a
// secret, no matter what the field is named (`sessionCookieIssued: true` is
// an assertion result, not a cookie). `null`/`undefined` fall out of the same
// principle for free. Arrays and plain objects are NOT exempted — a list
// literally named `cookies` could hold real ones, and the structural
// recursion that follows is a backstop, not a replacement for checking the
// name where it's still a meaningful signal.
function valueCanCarryString(value) {
  if (value === null || value === undefined) return false;
  const type = typeof value;
  return type === 'string' || type === 'object';
}

function isHarShaped(value) {
  return Boolean(
    value && typeof value === 'object' && !Array.isArray(value)
    && value.log && typeof value.log === 'object' && !Array.isArray(value.log)
    && Array.isArray(value.log.entries),
  );
}

// Mechanically rejects every forbidden content class from the RFC-050 local
// tooling handoff: credentials, cookies, form tokens, the pepper, raw or
// hashed secrets, subject identifiers, digests, Durable Object names/ids, raw
// resource ids, SQL, binds, D1 error bodies, and HAR files. Undeclared fields
// (a place business content could otherwise ride in on) are rejected by the
// closed-schema checks every builder/parser below runs before calling this.
// A *declared* free-text field's value (e.g. `observed`) is covered by the
// run-scoped registry above, not by pattern matching — see
// `registerRunSecrets`.
export function assertRedacted(value, path = '$') {
  if (value === null || value === undefined) return;
  if (typeof value === 'string') {
    checkStringValue(value, path);
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertRedacted(item, `${path}[${index}]`));
    return;
  }
  if (typeof value === 'object') {
    if (isHarShaped(value)) {
      throw new EvidenceRedactionError('har_file', path, `HAR-shaped object at ${path}`);
    }
    for (const [key, nested] of Object.entries(value)) {
      if (FORBIDDEN_KEY_PATTERN.test(key) && valueCanCarryString(nested)) {
        throw new EvidenceRedactionError('forbidden_key', `${path}.${key}`, `forbidden field name "${key}" at ${path}.${key}`);
      }
      assertRedacted(nested, `${path}.${key}`);
    }
    return;
  }
  // numbers and booleans carry no redaction risk.
}

const CANDIDATE_FIELDS = ['commit', 'label', 'workerVersionId', 'workerVersionTag', 'deployment'];

// Shared closed-schema check used by every builder and its parse-side
// counterpart (S-N3 from the Slices 1+2 review), so construction and parsing
// can never silently drift apart on which fields are allowed.
function assertClosedSchema(raw, requiredFields, kind) {
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
    throw new EvidenceSchemaError(`${kind} must be an object`);
  }
  // `raw[field] === undefined` (not `!(field in raw)`) so an object built by
  // destructuring an incomplete input — which keeps the key with an explicit
  // `undefined` value — is still caught as missing, not just true absence.
  const missing = requiredFields.filter((field) => raw[field] === undefined);
  if (missing.length > 0) {
    throw new EvidenceSchemaError(`${kind} missing required field(s): ${missing.join(', ')}`);
  }
  const extra = Object.keys(raw).filter((key) => !requiredFields.includes(key));
  if (extra.length > 0) {
    throw new EvidenceSchemaError(
      `${kind} has undeclared field(s): ${extra.join(', ')} `
      + '(closed schema — unlisted fields, including business content, are rejected)',
    );
  }
}

function assertCandidateShape(raw) {
  assertClosedSchema(raw, CANDIDATE_FIELDS, 'candidate tuple');
  for (const field of CANDIDATE_FIELDS) {
    if (typeof raw[field] !== 'string' || raw[field].length === 0) {
      throw new EvidenceSchemaError(`candidate tuple field "${field}" must be a non-empty string`);
    }
  }
  if (!COMMIT_PATTERN.test(raw.commit)) {
    throw new EvidenceSchemaError('candidate tuple "commit" must be a hex git commit sha (short or full)');
  }
}

// The exact-candidate identity every evidence record is scoped to (RFC-050's
// "pin evidence to an immutable Worker version" requirement).
export function buildCandidateTuple({ commit, label, workerVersionId, workerVersionTag, deployment }) {
  const tuple = { commit, label, workerVersionId, workerVersionTag, deployment };
  assertCandidateShape(tuple);
  const frozen = Object.freeze(tuple);
  assertRedacted(frozen, '$.candidate');
  return frozen;
}

// Shared local-only placeholder convention (S3-N3 from the Slices 3+4
// review), so every local-only collector (S4 onward) spells "this record
// has no real candidate identity" identically instead of each slice
// inventing its own literal. Pair with `localObserved` below on every
// `observed` string for the same reason.
export const LOCAL_CANDIDATE_PLACEHOLDER = 'local';
export const LOCAL_CANDIDATE_LABEL_SUFFIX = '-non-authoritative';
export const LOCAL_OBSERVED_PREFIX = '[local, non-authoritative] ';

export function buildLocalCandidateTuple({ commit, label }) {
  const normalizedLabel = label.endsWith(LOCAL_CANDIDATE_LABEL_SUFFIX)
    ? label
    : `${label}${LOCAL_CANDIDATE_LABEL_SUFFIX}`;
  return buildCandidateTuple({
    commit,
    label: normalizedLabel,
    workerVersionId: LOCAL_CANDIDATE_PLACEHOLDER,
    workerVersionTag: LOCAL_CANDIDATE_PLACEHOLDER,
    deployment: LOCAL_CANDIDATE_PLACEHOLDER,
  });
}

export function localObserved(text) {
  return `${LOCAL_OBSERVED_PREFIX}${text}`;
}

const ISO_UTC_TIMESTAMP_PATTERN = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/;
const TRIVIAL_OBSERVATION_PATTERN = /^(checked|ok|done|passed?|yes|no|n\/a)\.?$/i;

const RECORD_REQUIRED_FIELDS = [
  'schemaVersion', 'candidate', 'collectedAt', 'tool', 'toolVersion',
  'testId', 'observed', 'pass', 'artifactHash',
];

// One machine-readable evidence entry: schema version, candidate tuple,
// collection time, tool/version, test id, observed result, pass/fail, and
// artifact hash, per the handoff's "a record that merely says checked is a
// defect" requirement (enforced by `TRIVIAL_OBSERVATION_PATTERN`).
export function createEvidenceRecord({
  candidate, collectedAt, tool, toolVersion, testId, observed, pass, artifactHash,
}) {
  if (typeof pass !== 'boolean') {
    throw new EvidenceSchemaError('evidence record "pass" must be a boolean');
  }
  if (typeof observed !== 'string' || observed.trim().length === 0) {
    throw new EvidenceSchemaError('evidence record "observed" must be a non-empty description');
  }
  if (TRIVIAL_OBSERVATION_PATTERN.test(observed.trim())) {
    throw new EvidenceSchemaError(
      `evidence record "observed" must describe what was seen, not merely assert an outcome: "${observed}"`,
    );
  }
  if (typeof collectedAt !== 'string' || !ISO_UTC_TIMESTAMP_PATTERN.test(collectedAt)) {
    throw new EvidenceSchemaError('evidence record "collectedAt" must be an ISO-8601 UTC timestamp ending in "Z"');
  }
  if (typeof tool !== 'string' || tool.length === 0) {
    throw new EvidenceSchemaError('evidence record "tool" is required');
  }
  if (typeof toolVersion !== 'string' || toolVersion.length === 0) {
    throw new EvidenceSchemaError('evidence record "toolVersion" is required');
  }
  if (typeof testId !== 'string' || testId.length === 0) {
    throw new EvidenceSchemaError('evidence record "testId" is required');
  }
  if (typeof artifactHash !== 'string' || !ARTIFACT_HASH_PATTERN.test(artifactHash)) {
    throw new EvidenceSchemaError('evidence record "artifactHash" must match "sha256:<64 lowercase hex chars>"');
  }
  const record = Object.freeze({
    schemaVersion: MANIFEST_SCHEMA_VERSION,
    candidate,
    collectedAt,
    tool,
    toolVersion,
    testId,
    observed,
    pass,
    artifactHash,
  });
  assertRedacted(record, '$');
  return record;
}

// The top-level `00-manifest.json` content: candidate identity plus
// provenance for the whole local evidence package.
export function createManifest({ candidate, generatedAt, tool, toolVersion }) {
  if (typeof generatedAt !== 'string' || !ISO_UTC_TIMESTAMP_PATTERN.test(generatedAt)) {
    throw new EvidenceSchemaError('manifest "generatedAt" must be an ISO-8601 UTC timestamp ending in "Z"');
  }
  if (typeof tool !== 'string' || tool.length === 0) {
    throw new EvidenceSchemaError('manifest "tool" is required');
  }
  if (typeof toolVersion !== 'string' || toolVersion.length === 0) {
    throw new EvidenceSchemaError('manifest "toolVersion" is required');
  }
  const manifest = Object.freeze({
    schemaVersion: MANIFEST_SCHEMA_VERSION,
    candidate,
    generatedAt,
    tool,
    toolVersion,
  });
  assertRedacted(manifest, '$');
  return manifest;
}

// A before/after external-state snapshot (D1 table row counts and audit
// cardinality only — "never row dumps", per the handoff's S4/S5 instruction).
export function createExternalStateSnapshot({ candidate, collectedAt, label, tables }) {
  if (typeof label !== 'string' || label.length === 0) {
    throw new EvidenceSchemaError('external-state snapshot "label" is required (e.g. "before" or "after")');
  }
  if (typeof collectedAt !== 'string' || !ISO_UTC_TIMESTAMP_PATTERN.test(collectedAt)) {
    throw new EvidenceSchemaError('external-state snapshot "collectedAt" must be an ISO-8601 UTC timestamp ending in "Z"');
  }
  if (tables === null || typeof tables !== 'object' || Array.isArray(tables)) {
    throw new EvidenceSchemaError('external-state snapshot "tables" must be an object of table name to row count');
  }
  for (const [table, count] of Object.entries(tables)) {
    if (!Number.isInteger(count) || count < 0) {
      throw new EvidenceSchemaError(`external-state snapshot table "${table}" count must be a non-negative integer, not a row dump`);
    }
  }
  const snapshot = Object.freeze({
    schemaVersion: MANIFEST_SCHEMA_VERSION,
    candidate,
    collectedAt,
    label,
    tables: Object.freeze({ ...tables }),
  });
  assertRedacted(snapshot, '$');
  return snapshot;
}

function assertIsExternalStateSnapshot(snapshot, label) {
  if (
    snapshot === null || typeof snapshot !== 'object'
    || snapshot.tables === null || typeof snapshot.tables !== 'object' || Array.isArray(snapshot.tables)
  ) {
    throw new EvidenceSchemaError(
      `diffExternalStateSnapshots "${label}" must be an external-state snapshot object with a "tables" object`,
    );
  }
}

// Row-count-only diff between two snapshots, for later postcondition checks
// (RFC-050 Tooling Slice 5). Never touches raw rows.
export function diffExternalStateSnapshots(before, after) {
  assertIsExternalStateSnapshot(before, 'before');
  assertIsExternalStateSnapshot(after, 'after');
  const tables = new Set([...Object.keys(before.tables), ...Object.keys(after.tables)]);
  const diff = {};
  for (const table of tables) {
    const beforeCount = before.tables[table] ?? 0;
    const afterCount = after.tables[table] ?? 0;
    diff[table] = afterCount - beforeCount;
  }
  return Object.freeze(diff);
}

export function serializeManifestRecords(records) {
  return `${JSON.stringify(records, null, 2)}\n`;
}

// Parses a manifest/record-array JSON document, enforcing the closed schema
// (missing or undeclared fields are rejected) and re-running redaction on
// every parsed record, so a manifest can round-trip without silently
// widening what it is allowed to carry.
export function parseManifestRecords(text) {
  const parsed = JSON.parse(text);
  if (!Array.isArray(parsed)) {
    throw new EvidenceSchemaError('manifest must be a JSON array of evidence records');
  }
  return parsed.map((raw, index) => {
    assertClosedSchema(raw, RECORD_REQUIRED_FIELDS, `manifest record ${index}`);
    assertRedacted(raw, `$[${index}]`);
    return Object.freeze(raw);
  });
}

const MANIFEST_REQUIRED_FIELDS = ['schemaVersion', 'candidate', 'generatedAt', 'tool', 'toolVersion'];
const EXTERNAL_STATE_REQUIRED_FIELDS = ['schemaVersion', 'candidate', 'collectedAt', 'label', 'tables'];

// Symmetric parse-side counterpart to `createManifest` (S-N3 from the Slices
// 1+2 review): re-validates the closed schema, the candidate shape, and
// redaction on every read, so a persisted `00-manifest.json` cannot silently
// widen what it is allowed to carry between being written and being read.
export function parseManifest(text) {
  const parsed = JSON.parse(text);
  assertClosedSchema(parsed, MANIFEST_REQUIRED_FIELDS, 'manifest');
  assertCandidateShape(parsed.candidate);
  if (typeof parsed.generatedAt !== 'string' || !ISO_UTC_TIMESTAMP_PATTERN.test(parsed.generatedAt)) {
    throw new EvidenceSchemaError('manifest "generatedAt" must be an ISO-8601 UTC timestamp ending in "Z"');
  }
  if (typeof parsed.tool !== 'string' || parsed.tool.length === 0) {
    throw new EvidenceSchemaError('manifest "tool" is required');
  }
  if (typeof parsed.toolVersion !== 'string' || parsed.toolVersion.length === 0) {
    throw new EvidenceSchemaError('manifest "toolVersion" is required');
  }
  assertRedacted(parsed, '$');
  return Object.freeze({ ...parsed, candidate: Object.freeze({ ...parsed.candidate }) });
}

// Symmetric parse-side counterpart to `createExternalStateSnapshot` (S-N3).
export function parseExternalStateSnapshot(text) {
  const parsed = JSON.parse(text);
  assertClosedSchema(parsed, EXTERNAL_STATE_REQUIRED_FIELDS, 'external-state snapshot');
  assertCandidateShape(parsed.candidate);
  if (typeof parsed.collectedAt !== 'string' || !ISO_UTC_TIMESTAMP_PATTERN.test(parsed.collectedAt)) {
    throw new EvidenceSchemaError('external-state snapshot "collectedAt" must be an ISO-8601 UTC timestamp ending in "Z"');
  }
  if (typeof parsed.label !== 'string' || parsed.label.length === 0) {
    throw new EvidenceSchemaError('external-state snapshot "label" is required');
  }
  if (parsed.tables === null || typeof parsed.tables !== 'object' || Array.isArray(parsed.tables)) {
    throw new EvidenceSchemaError('external-state snapshot "tables" must be an object of table name to row count');
  }
  for (const [table, count] of Object.entries(parsed.tables)) {
    if (!Number.isInteger(count) || count < 0) {
      throw new EvidenceSchemaError(`external-state snapshot table "${table}" count must be a non-negative integer, not a row dump`);
    }
  }
  assertRedacted(parsed, '$');
  return Object.freeze({ ...parsed, tables: Object.freeze({ ...parsed.tables }) });
}

// == RFC-050 Tooling Slice 7: artifact hashing and evidence-tree leakage ====
// scanning. `assertRedacted` above is field-aware and built for one
// structured JSON value at a time (it can tell a `commit` field from an
// arbitrary string because it knows the field's path). A leakage sweep over
// the whole evidence tree also has to cover free-text files (manual
// evidence templates, notes) where there is no field path to reason about —
// `scanTextForLeakage` below is the bounded, deliberately narrower
// counterpart for that case.

export function hashToArtifactHash(bufferOrString) {
  return `sha256:${createHash('sha256').update(bufferOrString).digest('hex')}`;
}

export class EvidenceLeakageError extends Error {
  constructor(category, path, message) {
    super(message);
    this.name = 'EvidenceLeakageError';
    this.category = category;
    this.path = path;
  }
}

// Every one of these is already non-anchored (no `^`/`$`), so it works as a
// substring search over an arbitrarily large text blob without changes.
const TEXT_LEAKAGE_PATTERNS = [
  ['cookie', COOKIE_SHAPED_VALUE_PATTERN],
  ['d1_error_body', D1_ERROR_PATTERN],
  ['sql', SQL_KEYWORD_PATTERN],
];

const EMBEDDED_RESOURCE_ID_PATTERN = new RegExp(
  `\\b(?:${RAW_RESOURCE_ID_PREFIXES.map((prefix) => prefix.slice(0, -1)).join('|')})_[A-Za-z0-9]+`,
);

// S7-C1 from the Slice 7 review: a bare hex run of this project's actual
// secret/digest/token/Durable-Object-id length (56/64/96/128 — see
// `HEX_SECRET_LENGTHS`) is caught in free text too, unlike the field-aware
// `assertRedacted` above only applying to structured JSON. 32 and 40 are
// deliberately excluded here: a git commit sha is 7-40 hex characters, so a
// whole-document substring sweep for those two lengths would false-positive
// on a legitimate commit sha in prose constantly. The lengths do not
// otherwise overlap — `random_token()` (pepper, session secrets, join
// tickets), HMAC-SHA256 subject digests, and Durable Object ids are all
// exactly 64 hex characters, well outside the commit-sha range. `\b` on
// both ends of an exact-length pattern only matches a hex run of exactly
// that length, never a same-length substring of a longer run, since a
// contiguous hex string has no internal word boundary.
const EMBEDDED_SECRET_HEX_LENGTHS = [56, 64, 96, 128];
const EMBEDDED_HEX_PATTERNS = EMBEDDED_SECRET_HEX_LENGTHS.map(
  (length) => new RegExp(`\\b[0-9a-f]{${length}}\\b`, 'i'),
);

// The residual, deliberate gap versus the field-aware `assertRedacted`: a
// bare 32- or 40-hex-character value (md5/sha1-length, or this project's own
// commit-sha range) cannot be mechanically distinguished from a legitimate
// commit sha in free text, since there is no field path here to exempt one
// from the other. JSON evidence files remain fully covered for every length
// via `scanJsonValueForLeakage`'s path-scoped exemptions; this residual is
// free text only.
//
// Handoff 065 §3.1: returns every violation found, not just the first — a
// control whose purpose is to prove absence cannot stop at the first
// presence it finds. Returns an empty array when the text is clean; never
// throws for a leakage finding (an unrelated bug in this function itself
// would still throw normally).
export function scanTextForLeakage(text, path) {
  const violations = [];
  for (const secret of registeredRunSecrets) {
    if (text.toLowerCase().includes(secret.toLowerCase())) {
      violations.push(new EvidenceLeakageError(
        'registered_run_secret',
        path,
        `${path} contains a registered run-scoped value (never the value itself, only its presence)`,
      ));
    }
  }
  for (const pattern of EMBEDDED_HEX_PATTERNS) {
    if (pattern.test(text)) {
      violations.push(new EvidenceLeakageError(
        'raw_or_hashed_secret',
        path,
        `${path} contains a bare hex value at a secret/digest/token/Durable-Object-id length`,
      ));
    }
  }
  for (const [category, pattern] of TEXT_LEAKAGE_PATTERNS) {
    if (pattern.test(text)) {
      violations.push(new EvidenceLeakageError(category, path, `${path} contains ${category}-shaped content`));
    }
  }
  if (EMBEDDED_RESOURCE_ID_PATTERN.test(text)) {
    violations.push(new EvidenceLeakageError('raw_resource_id', path, `${path} contains a raw resource-id-shaped value`));
  }
  return violations;
}

// JSON evidence files get the full field-aware sweep. Handoff 065 §3.1: like
// `scanTextForLeakage`, returns every violation found (empty array if none)
// rather than throwing on the first — `assertRedacted` itself keeps its
// original throw-on-first contract for the construction/parsing callers
// below, which only ever need to know a record is invalid, not enumerate
// every way it is.
export function scanJsonValueForLeakage(value, path) {
  const violations = [];
  collectRedactionViolations(value, path, violations);
  return violations;
}
