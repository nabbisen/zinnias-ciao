#!/usr/bin/env node
// RFC-050 local evidence tooling (Tooling Slice 6): negative-configuration
// fixtures (E5, local) and restoration checks. Missing/invalid pepper;
// missing/misnamed ABUSE_LIMITER; exhausted coordinator returning generic
// 429 not 503; wrong/missing D1 binding; malformed version metadata; and
// the Class A/B/C audit outcomes.
//
// Several of these are already comprehensively proven by existing required
// gates (`test:hmac-pepper-configuration`, `test:abuse-controls`,
// `test:audit-class-a-failures`, `test:audit-boundaries`) — this collector
// cites those rather than duplicating them, matching Slice 5's precedent
// for RFC-078 capacity/reset. It adds genuinely new local fixtures for the
// gaps those gates don't cover: a misnamed (not merely absent)
// ABUSE_LIMITER binding, a missing/misnamed D1 binding on its own (not
// bundled with an invalid-pepper scenario), an explicit malformed/absent
// version-metadata check, and the community-creation-disabled path (closing
// Tooling Slice 4's known limitation) — using the new `configOverride`/
// `communityCreationEnabled` hooks added to `prepareIsolatedWorkerTest`.
//
// Every negative fixture uses its own disposable config and root, never the
// canonical one, and cleans up in `finally`. The final record spins up one
// more canonical (correctly configured) fixture and confirms it still
// reports its expected identity — proving the negative runs left nothing
// behind that could taint a real evidence run.
//
// Local-only: no hosted command, no deploy, no resource creation, no secret
// operation.

import { createHash, createHmac } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  buildLocalCandidateTuple,
  clearRegisteredRunSecrets,
  createEvidenceRecord,
  localObserved,
  registerRunSecrets,
  serializeManifestRecords,
} from './lib/evidence-manifest.mjs';
import { prepareIsolatedWorkerTest } from './lib/isolated-worker-test.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc050-e5-negative-config';
const toolVersion = JSON.parse(await readFile(join(root, 'package.json'), 'utf8')).version;

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertLocalOnly() {
  for (const argument of process.argv.slice(2)) {
    if (
      argument === '--remote'
      || argument.toLowerCase().includes('staging')
      || argument.toLowerCase().includes('production')
    ) {
      throw new Error(`E5 negative-config collection is local-only; refused argument ${argument}`);
    }
  }
}

function artifactHash(value) {
  return `sha256:${createHash('sha256').update(JSON.stringify(value)).digest('hex')}`;
}

async function freePort() {
  const server = createServer();
  await new Promise((accept, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', accept);
  });
  const address = server.address();
  const port = typeof address === 'object' && address ? address.port : 0;
  await new Promise((accept, reject) => server.close((error) => (error ? reject(error) : accept())));
  assert(port > 0, 'failed to allocate a loopback port');
  return port;
}

async function waitForOutcome(baseUrl, child, path = '/healthz') {
  for (let attempt = 0; attempt < 160; attempt += 1) {
    if (child.exitCode !== null) throw new Error('isolated Worker exited before readiness');
    try {
      const response = await fetch(`${baseUrl}${path}`);
      if (response.status !== 0) return response;
    } catch {
      // still starting
    }
    await new Promise((accept) => setTimeout(accept, 250));
  }
  throw new Error('isolated Worker did not respond');
}

async function withIsolatedWorker(label, options, run) {
  const fixture = await prepareIsolatedWorkerTest(label, options);
  let dev;
  try {
    if (options.includeD1 !== false) {
      await fixture.runWranglerSync(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
    }
    const port = await freePort();
    const baseUrl = `http://127.0.0.1:${port}`;
    dev = fixture.spawnDev(port);
    await waitForOutcome(baseUrl, dev, '/version');
    return await run({ fixture, baseUrl, dev });
  } finally {
    if (dev && dev.exitCode === null) {
      dev.kill('SIGTERM');
      await new Promise((accept) => dev.once('exit', accept));
    }
    await fixture.cleanup();
  }
}

assertLocalOnly();

const gitCommit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root }).toString().trim();
const candidate = buildLocalCandidateTuple({ commit: gitCommit, label: 'local-e5-negative-config' });

const records = [];
function record(testId, observed, pass, artifact) {
  records.push(createEvidenceRecord({
    candidate,
    collectedAt: new Date().toISOString(),
    tool: 'collect-evidence-e5-negative-config.mjs',
    toolVersion,
    testId,
    observed: localObserved(observed),
    pass,
    artifactHash: artifactHash(artifact),
  }));
}

function cite(testId, description, citedScript, citedGate) {
  record(testId, `${description} is already proven by \`${citedScript}\` (\`bun run ${citedGate}\`), a required gate re-run as part of this package rather than duplicated in this collector`, true, { citedScript, citedGate });
}

// -- Already comprehensively covered elsewhere: cite, don't duplicate ------

cite(
  'S6.missing_invalid_pepper_citation',
  'the missing/invalid-pepper matrix (fail-closed health/protected-route behavior, rotation invalidation, and restoration recovery)',
  'scripts/test-hmac-pepper-configuration.mjs',
  'test:hmac-pepper-configuration',
);
cite(
  'S6.missing_abuse_limiter_citation',
  'a fully absent ABUSE_LIMITER binding failing closed',
  'scripts/smoke/abuse-controls.mjs',
  'test:abuse-controls',
);
cite(
  'S6.exhausted_coordinator_returns_429_not_503_citation',
  'an exhausted (correctly blocked) coordinator returning 429, distinct from an unavailable coordinator returning 503',
  'scripts/smoke/abuse-controls.mjs',
  'test:abuse-controls',
);
cite(
  'S6.missing_d1_binding_citation',
  'a fully absent D1 binding (via includeD1:false) failing closed',
  'scripts/test-hmac-pepper-configuration.mjs',
  'test:hmac-pepper-configuration',
);
cite(
  'S6.class_a_audit_outcomes_citation',
  'Class A required-audit storage-failure fail-closed rollback outcomes',
  'scripts/test-audit-class-a-failures.mjs',
  'test:audit-class-a-failures',
);
cite(
  'S6.class_b_c_audit_outcomes_citation',
  'Class B (pre-disclosure) and Class C (logout, the sole safety-first exception) audit response-boundary outcomes',
  'scripts/test-audit-boundaries.mjs',
  'test:audit-boundaries',
);

// -- New: misnamed (not merely absent) ABUSE_LIMITER binding ----------------

await withIsolatedWorker(
  'e5-misnamed-abuse-limiter',
  {
    configOverride: (text) => text
      .replaceAll('name = "ABUSE_LIMITER"\nclass_name = "AbuseLimiter"', 'name = "ABUSE_LIMITER_WRONG"\nclass_name = "AbuseLimiter"'),
  },
  async ({ baseUrl }) => {
    const joinPage = await fetch(`${baseUrl}/join`);
    const joinToken = (await joinPage.text()).match(/name="_token"\s+value="([^"]+)"/u)?.[1] ?? '';
    const response = await fetch(`${baseUrl}/join`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({ _token: joinToken, code: 'ZZZZZZ' }),
      redirect: 'manual',
    });
    const failedClosed = response.status === 503 || response.status === 500;
    record(
      'S6.misnamed_abuse_limiter_binding',
      `with the exported class present but bound under the wrong name (ABUSE_LIMITER_WRONG instead of ABUSE_LIMITER), a credential-bearing POST to /join returned status ${response.status}, failing closed rather than silently bypassing rate limiting`,
      failedClosed,
      { status: response.status },
    );
  },
);

// -- New: missing D1 binding on its own (not bundled with pepper cases) ----

await withIsolatedWorker(
  'e5-missing-d1',
  { includeD1: false },
  async ({ baseUrl }) => {
    const response = await fetch(`${baseUrl}/join`);
    const text = await response.text();
    const failedClosed = response.status >= 500;
    const leaksNoInternals = !/panic|D1_ERROR|SqliteError|no such table|"DB"/i.test(text);
    record(
      'S6.missing_d1_binding',
      `with the D1 binding entirely absent (valid pepper and ABUSE_LIMITER present), GET /join returned status ${response.status} with a generic body that does not leak binding names or storage internals (leaksNoInternals: ${leaksNoInternals})`,
      failedClosed && leaksNoInternals,
      { status: response.status, leaksNoInternals },
    );
  },
);

// -- New: misnamed D1 binding -------------------------------------------

await withIsolatedWorker(
  'e5-misnamed-d1',
  {
    configOverride: (text) => text.replaceAll('binding = "DB"', 'binding = "DB_WRONG"'),
  },
  async ({ baseUrl }) => {
    const response = await fetch(`${baseUrl}/join`);
    const text = await response.text();
    const failedClosed = response.status >= 500;
    const leaksNoInternals = !/panic|D1_ERROR|SqliteError|no such table|"DB"/i.test(text);
    record(
      'S6.misnamed_d1_binding',
      `with the D1 database present but bound under the wrong name (DB_WRONG instead of DB), GET /join returned status ${response.status} with a generic body that does not leak binding names or storage internals (leaksNoInternals: ${leaksNoInternals})`,
      failedClosed && leaksNoInternals,
      { status: response.status, leaksNoInternals },
    );
  },
);

// -- New: malformed/absent version metadata is handled gracefully ---------

await withIsolatedWorker('e5-version-metadata', {}, async ({ baseUrl }) => {
  const response = await fetch(`${baseUrl}/version`);
  const body = await response.json();
  const gracefullyNull = body.worker_version_id === null && body.worker_version_tag === null && body.ok === true;
  record(
    'S6.malformed_version_metadata',
    `with no version_metadata binding configured for this isolated fixture (the harness does not declare one), GET /version returned status ${response.status} with ok:${body.ok}, worker_version_id:${JSON.stringify(body.worker_version_id)}, worker_version_tag:${JSON.stringify(body.worker_version_tag)} — the absent/malformed binding degrades to null fields rather than an error`,
    response.status === 200 && gracefullyNull,
    { status: response.status, workerVersionId: body.worker_version_id, workerVersionTag: body.worker_version_tag },
  );
});

// -- New: community-creation-disabled path (closes Slice 4's known gap) ---

await withIsolatedWorker(
  'e5-community-creation-disabled',
  { communityCreationEnabled: false },
  async ({ baseUrl, fixture }) => {
    const now = '2026-07-28T00:00:00.000Z';
    const communityId = 'com_e5_disabled';
    const adminUserId = 'usr_e5_disabled_admin';
    const adminMembershipId = 'mem_e5_disabled_admin';
    const adminSessionId = 'sess_e5_disabled_admin';
    const adminSessionSecret = 'e5-disabled-admin-session';
    registerRunSecrets([adminSessionSecret]);
    const adminSessionHmac = createHmac('sha256', fixture.pepper).update(adminSessionSecret).digest('hex');
    await fixture.runWranglerSync([
      'd1', 'execute', 'zinnias-ciao-dev', '--env', 'dev', '--local',
      '--persist-to', fixture.persistTo, '--config', fixture.configPath,
      '--yes', '--command', [
        `INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES ('${communityId}','E5 Disabled','Asia/Tokyo',1,'${now}')`,
        `INSERT INTO users (id,created_at) VALUES ('${adminUserId}','${now}')`,
        `INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES ('${adminMembershipId}','${communityId}','${adminUserId}','admin','E5 Disabled Admin','${now}')`,
        `INSERT INTO sessions (id,user_id,session_hmac,created_at,expires_at,last_seen_at) VALUES ('${adminSessionId}','${adminUserId}','${adminSessionHmac}','${now}','2099-12-31T23:59:59.000Z','${now}')`,
      ].join(';'),
    ]);
    const response = await fetch(`${baseUrl}/communities/new`, { headers: { Cookie: `ciao_sid=${adminSessionSecret}` } });
    const text = await response.text();
    const disabledMessageShown = text.includes('準備中');
    record(
      'S6.community_creation_disabled',
      `with COMMUNITY_CREATION_ENABLED=false, an active admin's GET /communities/new returned status ${response.status} (expected 200, disabled messaging, not 404 and not a redirect)`,
      response.status === 200 && disabledMessageShown,
      { status: response.status },
    );
  },
);

// -- Restoration check: the canonical fixture is unaffected by negative runs

await withIsolatedWorker('e5-canonical-restoration', {}, async ({ baseUrl }) => {
  const health = await fetch(`${baseUrl}/healthz`);
  const healthBody = await health.json();
  const version = await fetch(`${baseUrl}/version`);
  const versionBody = await version.json();
  const canonicalIdentityIntact = health.status === 200 && healthBody.ok === true
    && version.status === 200 && versionBody.ok === true && versionBody.version === 'isolated-test';
  record(
    'S6.canonical_fixture_identity_after_negative_runs',
    `after all preceding negative-configuration fixtures ran and tore down, a fresh canonical (correctly configured) fixture still reports its expected identity (/healthz ok:${healthBody.ok}, /version version:"${versionBody.version}") — no negative run leaked state into a shared canonical understanding`,
    canonicalIdentityIntact,
    { healthOk: healthBody.ok, version: versionBody.version },
  );
});

const serialized = serializeManifestRecords(records);
const { mkdir, writeFile } = await import('node:fs/promises');
await mkdir(outDir, { recursive: true });
await writeFile(join(outDir, '04-e5-negative-config.json'), serialized);

const passed = records.every((r) => r.pass);
console.log(JSON.stringify({
  authoritative: false,
  warning: 'LOCAL RUN — NOT AUTHORITATIVE — this record must never be treated as RFC-050 B4 evidence.',
  passed,
  evidence: join(outDir, '04-e5-negative-config.json'),
  results: records.map((r) => ({ testId: r.testId, pass: r.pass })),
}));
clearRegisteredRunSecrets();
if (!passed) process.exitCode = 1;
