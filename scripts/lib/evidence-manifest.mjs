// RFC-050 local evidence tooling (Tooling Slice 2): candidate manifest, schema
// validation, and redaction utilities shared by every local evidence
// collector. Nothing in this module performs a hosted call, deploy, or
// resource mutation — it is pure data validation and serialization.
//
// A record produced here is local evidence only. It becomes RFC-050 B4
// evidence solely when the same tooling runs against a frozen hosted
// candidate (RFC-050 Tooling Slice 3 onward).

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

class EvidenceSchemaError extends Error {}

// Fields whose value is required to be hex-shaped for a legitimate reason
// (a git commit sha, a namespaced artifact hash). Every other bare hex string
// of a length matching a common secret/digest/token/Durable-Object-id length
// is rejected wherever it appears, since this project's own tokens, session
// secrets, subject digests, and Durable Object ids are all bare lowercase hex.
const ARTIFACT_HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;
const COMMIT_PATTERN = /^[0-9a-f]{7,40}$/i;
const HEX_FIELD_EXEMPTIONS = new Map([
  ['artifactHash', ARTIFACT_HASH_PATTERN],
  ['commit', COMMIT_PATTERN],
]);

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

function fieldNameFromPath(path) {
  const match = /\.?([^.[\]]+)$/.exec(path);
  return match ? match[1] : path;
}

function checkStringValue(value, path) {
  const field = fieldNameFromPath(path);
  const exemption = HEX_FIELD_EXEMPTIONS.get(field);
  if (exemption) {
    if (!exemption.test(value)) {
      throw new EvidenceRedactionError(
        'malformed_exempt_field',
        path,
        `"${field}" at ${path} does not match its required shape`,
      );
    }
  } else if (HEX_ONLY_PATTERN.test(value) && HEX_SECRET_LENGTHS.has(value.length)) {
    throw new EvidenceRedactionError(
      'raw_or_hashed_secret',
      path,
      `bare ${value.length}-hex-char value at ${path} (secret, digest, subject identifier, or Durable Object id shape)`,
    );
  }
  if (RAW_RESOURCE_ID_PREFIXES.some((prefix) => value.startsWith(prefix))) {
    throw new EvidenceRedactionError('raw_resource_id', path, `raw resource-id-shaped value at ${path}`);
  }
  if (COOKIE_SHAPED_VALUE_PATTERN.test(value)) {
    throw new EvidenceRedactionError('cookie', path, `cookie-shaped value at ${path}`);
  }
  if (D1_ERROR_PATTERN.test(value)) {
    throw new EvidenceRedactionError('d1_error_body', path, `D1 error body at ${path}`);
  }
  if (SQL_KEYWORD_PATTERN.test(value)) {
    throw new EvidenceRedactionError('sql', path, `SQL-shaped text at ${path}`);
  }
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
// resource ids, SQL, binds, D1 error bodies, and HAR files. "Business
// content" is rejected structurally: callers only reach this function through
// `createEvidenceRecord`/`parseManifestRecords`, whose closed schema has no
// field business content could ride in on.
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
      if (FORBIDDEN_KEY_PATTERN.test(key)) {
        throw new EvidenceRedactionError('forbidden_key', `${path}.${key}`, `forbidden field name "${key}" at ${path}.${key}`);
      }
      assertRedacted(nested, `${path}.${key}`);
    }
    return;
  }
  // numbers and booleans carry no redaction risk.
}

const CANDIDATE_FIELDS = ['commit', 'label', 'workerVersionId', 'workerVersionTag', 'deployment'];

// The exact-candidate identity every evidence record is scoped to (RFC-050's
// "pin evidence to an immutable Worker version" requirement).
export function buildCandidateTuple({ commit, label, workerVersionId, workerVersionTag, deployment }) {
  const values = { commit, label, workerVersionId, workerVersionTag, deployment };
  const missing = CANDIDATE_FIELDS.filter((field) => typeof values[field] !== 'string' || values[field].length === 0);
  if (missing.length > 0) {
    throw new EvidenceSchemaError(`candidate tuple missing required field(s): ${missing.join(', ')}`);
  }
  if (!COMMIT_PATTERN.test(commit)) {
    throw new EvidenceSchemaError('candidate tuple "commit" must be a hex git commit sha (short or full)');
  }
  const tuple = Object.freeze({ commit, label, workerVersionId, workerVersionTag, deployment });
  assertRedacted(tuple, '$.candidate');
  return tuple;
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

// Row-count-only diff between two snapshots, for later postcondition checks
// (RFC-050 Tooling Slice 5). Never touches raw rows.
export function diffExternalStateSnapshots(before, after) {
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
    if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
      throw new EvidenceSchemaError(`manifest record ${index} must be an object`);
    }
    const missing = RECORD_REQUIRED_FIELDS.filter((field) => !(field in raw));
    if (missing.length > 0) {
      throw new EvidenceSchemaError(`manifest record ${index} missing required field(s): ${missing.join(', ')}`);
    }
    const extra = Object.keys(raw).filter((key) => !RECORD_REQUIRED_FIELDS.includes(key));
    if (extra.length > 0) {
      throw new EvidenceSchemaError(
        `manifest record ${index} has undeclared field(s): ${extra.join(', ')} `
        + '(closed schema — unlisted fields, including business content, are rejected)',
      );
    }
    assertRedacted(raw, `$[${index}]`);
    return Object.freeze(raw);
  });
}
