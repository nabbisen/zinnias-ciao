#!/usr/bin/env node
// RFC-050 local evidence tooling (Tooling Slice 2): pins the candidate
// manifest schema and proves the redaction utility mechanically rejects
// every forbidden content class named in the local evidence tooling handoff.
// No hosted call, no Worker process, no D1 access — pure data validation.

import assert from 'node:assert/strict';
import {
  EvidenceLeakageError,
  EvidenceRedactionError,
  LOCAL_CANDIDATE_PLACEHOLDER,
  LOCAL_OBSERVED_PREFIX,
  MANIFEST_SCHEMA_VERSION,
  assertRedacted,
  buildCandidateTuple,
  buildLocalCandidateTuple,
  clearRegisteredRunSecrets,
  createEvidenceRecord,
  createExternalStateSnapshot,
  createManifest,
  diffExternalStateSnapshots,
  hashToArtifactHash,
  localObserved,
  parseExternalStateSnapshot,
  parseManifest,
  parseManifestRecords,
  registerRunSecrets,
  scanJsonValueForLeakage,
  scanTextForLeakage,
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

// -- local candidate placeholder convention (S3-N3) --------------------------

const localCandidate = buildLocalCandidateTuple({ commit: 'c991b82', label: 'local-s5-collection' });
assert.equal(localCandidate.workerVersionId, LOCAL_CANDIDATE_PLACEHOLDER);
assert.equal(localCandidate.workerVersionTag, LOCAL_CANDIDATE_PLACEHOLDER);
assert.equal(localCandidate.deployment, LOCAL_CANDIDATE_PLACEHOLDER);
assert.equal(localCandidate.label, 'local-s5-collection-non-authoritative');
const alreadySuffixed = buildLocalCandidateTuple({ commit: 'c991b82', label: 'local-s5-collection-non-authoritative' });
assert.equal(alreadySuffixed.label, 'local-s5-collection-non-authoritative', 'the suffix is not doubled');
assert.equal(localObserved('admitted 3 requests'), `${LOCAL_OBSERVED_PREFIX}admitted 3 requests`);

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
// The two hex-shaped fields with a legitimate reason to be hex are exempt
// only at their real schema position (S-N2: path-scoped, not name-scoped).
assertRedacted({ candidate: { commit: 'c991b820b31943219da207acc83f19182236f9bb' } });
assertRedacted({ artifactHash: `sha256:${'c'.repeat(64)}` });
assertRedactionCategory(
  { somethingElse: { commit: 'a'.repeat(40) } },
  'raw_or_hashed_secret',
  'a field merely named "commit" outside candidate.commit does not inherit the exemption (S-N2)',
);

// -- run-scoped value registry (S-N1) ----------------------------------------

clearRegisteredRunSecrets();
try {
  registerRunSecrets(['ACDEFG', 'Yamada Taro', 'see you at the park']);
  assertRedactionCategory(
    { observed: 'redeemed invite code ACDEFG successfully' },
    'registered_run_secret',
    'a registered invite code inside free-text observed is rejected',
  );
  assertRedactionCategory(
    { observed: 'member display name was Yamada Taro' },
    'registered_run_secret',
    'a registered display name inside free-text observed is rejected',
  );
  assertRedactionCategory(
    { observed: 'note content: see you at the park' },
    'registered_run_secret',
    'a registered note body inside free-text observed is rejected',
  );
  assertRedactionCategory(
    { observed: 'redeemed invite code acdefg successfully' },
    'registered_run_secret',
    'a lowercase-case variant of a registered invite code is rejected (S3-N1: matching is case-insensitive)',
  );
  assertRedacted({ observed: 'admitted exactly 10 of 40 concurrent requests' });
  assert.throws(
    () => registerRunSecrets(['']),
    /non-empty strings/u,
    'registerRunSecrets rejects an empty-string value (would match everything)',
  );
  assert.throws(
    () => registerRunSecrets(['ok']),
    /shorter than 4 characters/u,
    'registerRunSecrets rejects a value below the minimum length (S3-N2: would over-reject unrelated prose)',
  );
} finally {
  clearRegisteredRunSecrets();
}

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

assert.throws(
  () => diffExternalStateSnapshots({ tables: { invites: 1 } }, { notTables: {} }),
  /must be an external-state snapshot object/u,
  'diffExternalStateSnapshots rejects a malformed "after" input (S-N4)',
);
assert.throws(
  () => diffExternalStateSnapshots(null, after),
  /must be an external-state snapshot object/u,
  'diffExternalStateSnapshots rejects a null "before" input (S-N4)',
);

// -- manifest / external-state round trip (S-N3) -----------------------------

const parsedManifest = parseManifest(JSON.stringify(manifest));
assert.deepEqual(parsedManifest, manifest);
assert.throws(
  () => parseManifest(JSON.stringify({ ...manifest, businessNote: 'leaked content' })),
  /undeclared field/u,
  'parseManifest rejects an extra field (S-N3 closed-schema symmetry)',
);
assert.throws(
  () => parseManifest(JSON.stringify({ schemaVersion: 1 })),
  /missing required field/u,
  'parseManifest rejects a document missing required fields',
);

const parsedBefore = parseExternalStateSnapshot(JSON.stringify(before));
assert.deepEqual(parsedBefore, before);
assert.throws(
  () => parseExternalStateSnapshot(JSON.stringify({ ...before, businessNote: 'leaked content' })),
  /undeclared field/u,
  'parseExternalStateSnapshot rejects an extra field (S-N3 closed-schema symmetry)',
);
assert.throws(
  () => parseExternalStateSnapshot(JSON.stringify({ ...before, tables: { invites: 'five' } })),
  /must be a non-negative integer, not a row dump/u,
  'parseExternalStateSnapshot rejects a row dump smuggled in on parse',
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

// -- artifact hashing and leakage scanning (Tooling Slice 7) -----------------

assert.equal(hashToArtifactHash('hello'), 'sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824');
assert.match(hashToArtifactHash(Buffer.from('hello')), /^sha256:[0-9a-f]{64}$/u, 'accepts a Buffer, not just a string');

function assertTextLeakageCategory(text, category, description) {
  assert.throws(
    () => scanTextForLeakage(text, 'evidence-templates/test.md'),
    (error) => {
      assert.ok(error instanceof EvidenceLeakageError, `${description}: expected EvidenceLeakageError, got ${error}`);
      assert.equal(error.category, category, `${description}: expected category "${category}", got "${error.category}"`);
      return true;
    },
    description,
  );
}

assertTextLeakageCategory('the cookie was ciao_sid=abc123def456', 'cookie', 'cookie-shaped text in a free-text document');
assertTextLeakageCategory('response body: D1_ERROR: no such table: members', 'd1_error_body', 'D1 error body in a free-text document');
assertTextLeakageCategory('ran SELECT hmac_pepper FROM secrets WHERE id = 1', 'sql', 'SQL-shaped text in a free-text document');
assertTextLeakageCategory('observed community id com_abcdef0123456789 in the response', 'raw_resource_id', 'raw resource-id-shaped text in a free-text document');

// S7-C1 from the Slice 7 review: this project's actual secret/digest/token/
// Durable-Object-id length (56/64/96/128 hex chars) IS caught in free text,
// pinning all three probes the review demonstrated against the shipped
// module.
assertTextLeakageCategory(`pepper pasted by accident: ${'a'.repeat(64)}`, 'raw_or_hashed_secret', '64-hex pepper pasted in a template (S7-C1)');
assertTextLeakageCategory(`session token observed: ${'b'.repeat(64)}`, 'raw_or_hashed_secret', '64-hex session token in prose (S7-C1)');
// A bare hex value at this length is flagged regardless of a "sha256:"
// label — the length is what matters, and the templates instead direct
// testers to reference the HASHES.sha256 manifest path rather than paste a
// hash value into prose (see docs/src/tester/evidence-templates/index.md).
assertTextLeakageCategory(`artifact hash: sha256:${'c'.repeat(64)}`, 'raw_or_hashed_secret', 'a bare 64-hex run is caught even directly after a "sha256:" label');

// The residual, deliberate gap: 32/40-hex values remain unscannable in free
// text (no field path to exempt a legitimate commit sha with) — this test
// pins that the residual is intentional and narrow, not the whole class.
scanTextForLeakage(
  `candidate commit: ${'c991b820b31943219da207acc83f19182236f9bb'}`,
  'evidence-templates/test.md',
);
scanTextForLeakage('the admin can select an event and update its attendance note', 'evidence-templates/test.md');

try {
  registerRunSecrets(['ACDEFG']);
  assert.throws(
    () => scanTextForLeakage('redeemed invite code ACDEFG successfully', 'evidence-templates/test.md'),
    (error) => error instanceof EvidenceLeakageError && error.category === 'registered_run_secret',
    'a registered run secret is caught in free-text scanning too',
  );
} finally {
  clearRegisteredRunSecrets();
}

// JSON evidence files still get the full field-aware sweep (commit/artifactHash
// exemptions included), via the same `assertRedacted` used elsewhere.
scanJsonValueForLeakage(record, '$');
assert.throws(
  () => scanJsonValueForLeakage({ pepper: 'x' }, '$'),
  (error) => error instanceof EvidenceRedactionError && error.category === 'forbidden_key',
  'scanJsonValueForLeakage rejects a forbidden key the same way assertRedacted does',
);

console.log(JSON.stringify({
  ok: true,
  phases: {
    candidateTupleValidation: true,
    evidenceRecordSchema: true,
    redactionPerForbiddenClass: true,
    proseAndAliasFalsePositiveGuards: true,
    manifestAndExternalStateBuilders: true,
    hexExemptionIsPathScoped: true,
    runScopedValueRegistry: true,
    diffExternalStateSnapshotsInputGuard: true,
    manifestAndExternalStateParseSymmetry: true,
    localCandidatePlaceholderConvention: true,
    registerRunSecretsMinimumLengthGuard: true,
    roundTrip: true,
    artifactHashing: true,
    textLeakageScanning: true,
    jsonLeakageScanning: true,
  },
}));
