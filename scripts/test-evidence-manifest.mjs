#!/usr/bin/env node
// RFC-050 local evidence tooling (Tooling Slice 2): pins the candidate
// manifest schema and proves the redaction utility mechanically rejects
// every forbidden content class named in the local evidence tooling handoff.
// No hosted call, no Worker process, no D1 access — pure data validation.

import assert from 'node:assert/strict';
import {
  EvidenceRedactionError,
  MANIFEST_SCHEMA_VERSION,
  assertRedacted,
  buildCandidateTuple,
  createEvidenceRecord,
  createExternalStateSnapshot,
  createManifest,
  diffExternalStateSnapshots,
  parseManifestRecords,
  serializeManifestRecords,
} from './lib/evidence-manifest.mjs';

function assertRedactionCategory(value, category, description) {
  assert.throws(
    () => assertRedacted(value),
    (error) => {
      assert.ok(error instanceof EvidenceRedactionError, `${description}: expected EvidenceRedactionError, got ${error}`);
      assert.equal(error.category, category, `${description}: expected category "${category}", got "${error.category}"`);
      return true;
    },
    description,
  );
}

const candidate = buildCandidateTuple({
  commit: 'c991b820b31943219da207acc83f19182236f9bb',
  label: 'v0.60.0-rc1',
  workerVersionId: 'abcd1234-ef56-7890-abcd-1234ef567890',
  workerVersionTag: 'blue',
  deployment: 'staging',
});

// -- candidate tuple validation ---------------------------------------------

for (const missingField of ['commit', 'label', 'workerVersionId', 'workerVersionTag', 'deployment']) {
  const fields = {
    commit: 'c991b82',
    label: 'v0.60.0-rc1',
    workerVersionId: 'abcd1234-ef56-7890-abcd-1234ef567890',
    workerVersionTag: 'blue',
    deployment: 'staging',
  };
  delete fields[missingField];
  assert.throws(() => buildCandidateTuple(fields), /missing required field/u, `missing ${missingField}`);
}

assert.throws(
  () => buildCandidateTuple({
    commit: 'not-hex!!',
    label: 'v0.60.0-rc1',
    workerVersionId: 'abcd1234-ef56-7890-abcd-1234ef567890',
    workerVersionTag: 'blue',
    deployment: 'staging',
  }),
  /hex git commit sha/u,
  'non-hex commit is rejected',
);

// -- evidence record schema --------------------------------------------------

const collectedAt = '2026-07-28T00:00:00.000Z';

assert.throws(
  () => createEvidenceRecord({
    candidate, collectedAt, tool: 'test-evidence-manifest', toolVersion: '1', testId: 'S2.smoke', observed: 'ok', pass: true, artifactHash: `sha256:${'a'.repeat(64)}`,
  }),
  /must describe what was seen/u,
  '"a record that merely says checked is a defect" is enforced for "ok"',
);
assert.throws(
  () => createEvidenceRecord({
    candidate, collectedAt, tool: 'test-evidence-manifest', toolVersion: '1', testId: 'S2.smoke', observed: 'checked.', pass: true, artifactHash: `sha256:${'a'.repeat(64)}`,
  }),
  /must describe what was seen/u,
  '"checked." is rejected the same way',
);
assert.throws(
  () => createEvidenceRecord({
    candidate, collectedAt: '2026-07-28', tool: 't', toolVersion: '1', testId: 'S2.smoke', observed: 'admitted exactly 10 of 40 concurrent requests', pass: true, artifactHash: `sha256:${'a'.repeat(64)}`,
  }),
  /ISO-8601 UTC timestamp/u,
  'non-ISO timestamp is rejected',
);
assert.throws(
  () => createEvidenceRecord({
    candidate, collectedAt, tool: 't', toolVersion: '1', testId: 'S2.smoke', observed: 'admitted exactly 10 of 40 concurrent requests', pass: true, artifactHash: 'not-namespaced-hex',
  }),
  /artifactHash.*must match/u,
  'artifactHash without the sha256: namespace is rejected',
);
assert.throws(
  () => createEvidenceRecord({
    candidate, collectedAt, tool: 't', toolVersion: '1', testId: 'S2.smoke', observed: 'admitted exactly 10 of 40 concurrent requests', pass: 'yes', artifactHash: `sha256:${'a'.repeat(64)}`,
  }),
  /"pass" must be a boolean/u,
  'non-boolean pass is rejected',
);

const record = createEvidenceRecord({
  candidate,
  collectedAt,
  tool: 'test-evidence-manifest',
  toolVersion: '1.0.0',
  testId: 'S2.smoke',
  observed: 'admitted exactly 10 of 40 concurrent requests, matching the invite-attempt policy limit',
  pass: true,
  artifactHash: `sha256:${'a'.repeat(64)}`,
});
assert.equal(record.schemaVersion, MANIFEST_SCHEMA_VERSION);
assert.deepEqual(Object.keys(record).sort(), [
  'artifactHash', 'candidate', 'collectedAt', 'observed', 'pass', 'schemaVersion', 'testId', 'tool', 'toolVersion',
].sort());

// -- redaction: one case per forbidden class ---------------------------------

assertRedactionCategory({ password: 'x' }, 'forbidden_key', 'credential-shaped key "password"');
assertRedactionCategory({ sessionCookie: 'x' }, 'forbidden_key', 'cookie-shaped key');
assertRedactionCategory({ formToken: 'x' }, 'forbidden_key', 'form-token-shaped key');
assertRedactionCategory({ hmacPepper: 'x' }, 'forbidden_key', 'pepper-shaped key');
assertRedactionCategory({ durableObjectId: 'x' }, 'forbidden_key', 'Durable Object id key');
assertRedactionCategory({ subjectDigest: 'x' }, 'forbidden_key', 'subject identifier key');
assertRedactionCategory({ dbBindings: 'x' }, 'forbidden_key', 'binding-shaped key');

assertRedactionCategory({ note: 'ciao_sid=abc123def456' }, 'cookie', 'cookie-shaped value');
assertRedactionCategory({ note: 'a'.repeat(64) }, 'raw_or_hashed_secret', 'bare 64-hex value (pepper/token/digest/DO-id shape)');
assertRedactionCategory({ note: 'b'.repeat(40) }, 'raw_or_hashed_secret', 'bare 40-hex value');
assertRedactionCategory({ note: 'com_abcdef0123456789' }, 'raw_resource_id', 'raw resource-id-shaped value');
assertRedactionCategory(
  { note: 'SELECT hmac_pepper FROM secrets WHERE id = 1' },
  'sql',
  'SQL-shaped text',
);
assertRedactionCategory({ note: 'D1_ERROR: no such table: members' }, 'd1_error_body', 'D1 error body');
assertRedactionCategory(
  { note: { log: { version: '1.2', creator: {}, entries: [] } } },
  'har_file',
  'HAR-shaped object',
);

// Ordinary prose that happens to contain SQL-ish English verbs must not
// false-positive.
assertRedacted({ observed: 'the admin can select an event and update its attendance note' });
// A campaign-local alias distinct from this project's real id prefixes must
// not false-positive as a raw resource id.
assertRedacted({ alias: 'evid_candidate_member_1' });
// The two hex-shaped fields with a legitimate reason to be hex are exempt.
assertRedacted({ commit: 'c991b82', artifactHash: `sha256:${'c'.repeat(64)}` });

// -- manifest / external-state builders --------------------------------------

const manifest = createManifest({
  candidate,
  generatedAt: collectedAt,
  tool: 'test-evidence-manifest',
  toolVersion: '1.0.0',
});
assert.equal(manifest.schemaVersion, MANIFEST_SCHEMA_VERSION);
assert.deepEqual(manifest.candidate, candidate);

const before = createExternalStateSnapshot({
  candidate, collectedAt, label: 'before', tables: { invites: 5, memberships: 10 },
});
const after = createExternalStateSnapshot({
  candidate, collectedAt, label: 'after', tables: { invites: 4, memberships: 11, sessions: 1 },
});
assert.deepEqual(diffExternalStateSnapshots(before, after), { invites: -1, memberships: 1, sessions: 1 });

assert.throws(
  () => createExternalStateSnapshot({ candidate, collectedAt, label: 'after', tables: { invites: 'five' } }),
  /must be a non-negative integer, not a row dump/u,
  'external-state snapshot rejects non-count table values',
);

// -- round trip ---------------------------------------------------------------

const serialized = serializeManifestRecords([record]);
const parsed = parseManifestRecords(serialized);
assert.deepEqual(parsed, [record]);

assert.throws(
  () => parseManifestRecords(JSON.stringify([{ ...record, businessNote: 'leaked content' }])),
  /undeclared field/u,
  'closed schema rejects an extra field on parse (the business-content leak path)',
);
assert.throws(
  () => parseManifestRecords(JSON.stringify([{ schemaVersion: 1 }])),
  /missing required field/u,
  'closed schema rejects a record missing required fields on parse',
);

console.log(JSON.stringify({
  ok: true,
  phases: {
    candidateTupleValidation: true,
    evidenceRecordSchema: true,
    redactionPerForbiddenClass: true,
    proseAndAliasFalsePositiveGuards: true,
    manifestAndExternalStateBuilders: true,
    roundTrip: true,
  },
}));
