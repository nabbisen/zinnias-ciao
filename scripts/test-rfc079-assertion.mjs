// RFC-079 Package 0A: local-only real-D1 assertion proof.
// Never add --remote, hosted configuration, credentials, or production data.

import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, readFile, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const fixtureDir = join(root, 'workers/ssr/tests/fixtures');
const fixture = join(fixtureDir, 'audit_change_assertion.sql');
const config = join(fixtureDir, 'wrangler.rfc079-assertion.toml');
const workerSource = join(fixtureDir, 'rfc079-assertion-worker.mjs');
const database = 'zinnias-ciao-rfc079-assertion-local';
const wranglerBin = join(root, 'node_modules/.bin/wrangler');
const wranglerPackage = join(root, 'node_modules/wrangler/package.json');

function assert(condition, message) {
  if (!condition) throw new Error(message);
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
  return port;
}

function run(command, args, options = {}) {
  return new Promise((accept, reject) => {
    const child = spawn(command, args, {
      cwd: root,
      env: options.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.once('error', reject);
    child.once('exit', (code) => {
      if (code === 0) accept({ stdout, stderr });
      else {
        const diagnostic = `${stdout}\n${stderr}`.trim().slice(-2000);
        reject(new Error(`${command} exited ${code}: ${diagnostic}`));
      }
    });
  });
}

async function request(baseUrl, path) {
  const response = await fetch(`${baseUrl}${path}`, { method: path === '/health' ? 'GET' : 'POST' });
  const body = await response.json();
  assert(response.ok, `${path} returned HTTP ${response.status}`);
  assert(!JSON.stringify(body).includes('ast_'), `${path} exposed an internal assertion identifier`);
  return body;
}

async function waitUntilReady(baseUrl, child, diagnostics) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (child.exitCode !== null) {
      throw new Error(`wrangler dev exited ${child.exitCode} before readiness: ${diagnostics().slice(-2000)}`);
    }
    try {
      const body = await request(baseUrl, '/health');
      if (body.ready === true) return;
    } catch {
      // Local server is still starting.
    }
    await new Promise((accept) => setTimeout(accept, 100));
  }
  throw new Error('local wrangler dev did not become ready');
}

function expectState(actual, expected, label) {
  assert(JSON.stringify(actual) === JSON.stringify(expected), `${label} state mismatch: ${JSON.stringify(actual)}`);
}

const disposableRoot = join(root, '.git-exclude/tmp');
await mkdir(disposableRoot, { recursive: true });
const tempRoot = await mkdtemp(join(disposableRoot, 'rfc079-package0a-'));
const persistTo = join(tempRoot, 'state');
const xdgConfig = join(tempRoot, 'xdg');
const wranglerLog = join(tempRoot, 'wrangler.log');
await mkdir(persistTo, { recursive: true });
await mkdir(xdgConfig, { recursive: true });

const cloudflareAuthorityKey = /^(?:CLOUDFLARE_|CF_(?:API_TOKEN|API_KEY|EMAIL|ACCOUNT_ID|ZONE_ID)$|WRANGLER_(?:API_TOKEN|OAUTH_TOKEN)$)/u;

function inheritNonAuthorityEnvironment(source) {
  const allowed = {};
  for (const key of Object.keys(source)) {
    if (cloudflareAuthorityKey.test(key)) continue;
    const value = source[key];
    if (value !== undefined) allowed[key] = value;
  }
  return allowed;
}

const sentinelEnvironment = { PACKAGE0A_LOCAL_SENTINEL: 'retained' };
Object.defineProperty(sentinelEnvironment, 'CLOUDFLARE_API_TOKEN', {
  enumerable: true,
  get() {
    throw new Error('sentinel authority value was read');
  },
});
const sentinelResult = inheritNonAuthorityEnvironment(sentinelEnvironment);
assert(
  sentinelResult.PACKAGE0A_LOCAL_SENTINEL === 'retained'
    && !Object.hasOwn(sentinelResult, 'CLOUDFLARE_API_TOKEN'),
  'authority-key filtering regression',
);

const inheritedLocalEnv = inheritNonAuthorityEnvironment(process.env);
const env = {
  ...inheritedLocalEnv,
  XDG_CONFIG_HOME: xdgConfig,
  WRANGLER_LOG_PATH: wranglerLog,
  WRANGLER_LOG: 'error',
  NO_COLOR: '1',
};

let dev;
try {
  const wranglerMetadata = JSON.parse(await readFile(wranglerPackage, 'utf8'));
  const wranglerVersion = String(wranglerMetadata.version ?? '');
  assert(/^4\./u.test(wranglerVersion), `Package 0A requires Wrangler 4.x, found ${wranglerVersion || 'unknown'}`);

  const source = await readFile(workerSource, 'utf8');
  const schema = await readFile(fixture, 'utf8');
  assert(!source.includes('console.'), 'proof Worker must not log assertion identifiers or D1 details');
  const auditTable = schema.split('CREATE TABLE proof_audits')[1] ?? '';
  assert(!auditTable.includes('operation_id'), 'proof audit table must not accept assertion operation IDs');

  await run(wranglerBin, [
    'd1', 'execute', database,
    '--local',
    '--yes',
    '--persist-to', persistTo,
    '--config', config,
    '--file', fixture,
  ], { env });

  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  dev = spawn(wranglerBin, [
    'dev',
    '--local',
    '--ip', '127.0.0.1',
    '--port', String(port),
    '--persist-to', persistTo,
    '--config', config,
    '--log-level', 'error',
    '--show-interactive-dev-session=false',
  ], { cwd: root, env, stdio: ['ignore', 'pipe', 'pipe'] });
  let devStdout = '';
  let devStderr = '';
  dev.stdout.on('data', (chunk) => { devStdout += chunk; });
  dev.stderr.on('data', (chunk) => { devStderr += chunk; });

  await waitUntilReady(baseUrl, dev, () => `${devStdout}\n${devStderr}`.trim());
  assert((await request(baseUrl, '/reset')).reset === true, 'fixture reset failed');

  const idProperties = await request(baseUrl, '/id-properties');
  assert(idProperties.generated === 128, 'operation ID sample count changed');
  assert(idProperties.unique === true && idProperties.valid === true, 'operation IDs were not fresh and bounded');

  const zero = await request(baseUrl, '/case/zero');
  assert(zero.batchSucceeded === false, 'zero-row claim unexpectedly committed');
  assert(zero.statementCount === 5, 'zero-row proof statement budget changed');
  expectState(zero.state, { winners: 0, dependents: 0, audits: 0, guards: 0 }, 'zero-row');

  const one = await request(baseUrl, '/case/one');
  assert(one.batchSucceeded === true, 'one-row claim did not commit');
  assert(one.statementCount === 5, 'one-row proof statement budget changed');
  assert(JSON.stringify(one.statementChanges) === '[1,1,1,1,1]', `one-row write accounting changed: ${JSON.stringify(one.statementChanges)}`);
  expectState(one.state, { winners: 1, dependents: 1, audits: 1, guards: 0 }, 'one-row');

  const multi = await request(baseUrl, '/case/multi');
  assert(multi.batchSucceeded === false, 'multi-row claim unexpectedly committed');
  assert(multi.statementCount === 5, 'multi-row proof statement budget changed');
  expectState(multi.state, { winners: 0, dependents: 0, audits: 0, guards: 0 }, 'multi-row');

  const auditFailure = await request(baseUrl, '/case/audit-fail');
  assert(auditFailure.batchSucceeded === false, 'later audit failure unexpectedly committed');
  assert(auditFailure.statementCount === 5, 'audit-failure proof statement budget changed');
  expectState(auditFailure.state, { winners: 0, dependents: 0, audits: 0, guards: 0 }, 'audit-failure');

  const contenders = await Promise.all([
    request(baseUrl, '/case/concurrent'),
    request(baseUrl, '/case/concurrent'),
  ]);
  assert(contenders.filter((result) => result.batchSucceeded).length === 1, 'concurrent proof did not produce exactly one batch winner');
  assert(contenders.every((result) => result.statementCount === 5), 'concurrent statement budget changed');
  const concurrent = await request(baseUrl, '/summary/concurrent');
  expectState(concurrent, { winners: 1, dependents: 1, audits: 1, guards: 0 }, 'concurrent');

  process.stdout.write(`${JSON.stringify({
    wrangler: wranglerVersion,
    mode: 'local-only',
    operationIds: idProperties,
    cases: {
      zero: { committed: false, statements: zero.statementCount, state: zero.state },
      one: { committed: true, statements: one.statementCount, statementChanges: one.statementChanges, state: one.state },
      multiple: { committed: false, statements: multi.statementCount, state: multi.state },
      laterAuditFailure: { committed: false, statements: auditFailure.statementCount, state: auditFailure.state },
      concurrent: { winners: 1, statementsPerAttempt: contenders.map((result) => result.statementCount), state: concurrent },
    },
    privacy: { internalIdsInResponses: false, workerConsoleCalls: 0, assertionIdAuditColumns: 0 },
  }, null, 2)}\n`);
} finally {
  if (dev && dev.exitCode === null) {
    dev.kill('SIGTERM');
    await new Promise((accept) => {
      const timeout = setTimeout(() => {
        if (dev.exitCode === null) dev.kill('SIGKILL');
        accept();
      }, 3000);
      dev.once('exit', () => {
        clearTimeout(timeout);
        accept();
      });
    });
  }
  await rm(tempRoot, { recursive: true, force: true });
}
