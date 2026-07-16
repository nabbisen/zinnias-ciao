// Disposable local-only required-audit transaction proof.
// Never add --remote, hosted configuration, credentials, or production data.

import { spawn } from 'node:child_process';
import { copyFile, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const fixtureDir = join(root, 'workers/ssr/tests/fixtures');
const workerFixture = join(fixtureDir, 'audit-atomicity-worker.mjs');
const schemaFixture = join(fixtureDir, 'audit_atomicity.sql');
const sourceMigrations = join(root, 'migrations');
const wranglerBin = join(root, 'node_modules/.bin/wrangler');
const wranglerPackage = join(root, 'node_modules/wrangler/package.json');
const database = 'zinnias-ciao-audit-atomicity-local';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function run(command, args, { cwd = root, env } = {}) {
  return new Promise((accept, reject) => {
    const child = spawn(command, args, {
      cwd,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.once('error', reject);
    child.once('exit', (code) => {
      if (code === 0) accept({ stdout, stderr });
      else reject(new Error(`${command} exited ${code}: ${`${stdout}\n${stderr}`.trim().slice(-2000)}`));
    });
  });
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

async function request(baseUrl, path) {
  const response = await fetch(`${baseUrl}${path}`, { method: path === '/health' ? 'GET' : 'POST' });
  const body = await response.json();
  assert(response.ok, `${path} returned HTTP ${response.status}`);
  return body;
}

async function waitUntilReady(baseUrl, child, diagnostics) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (child.exitCode !== null) {
      throw new Error(`wrangler dev exited ${child.exitCode} before readiness: ${diagnostics().slice(-2000)}`);
    }
    try {
      if ((await request(baseUrl, '/health')).ready === true) return;
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
const tempRoot = await mkdtemp(join(disposableRoot, 'audit-atomicity-'));
const migrationsDir = join(tempRoot, 'migrations');
const persistTo = join(tempRoot, 'state');
const xdgConfig = join(tempRoot, 'xdg');
const config = join(tempRoot, 'wrangler.toml');
const workerSource = join(tempRoot, 'audit-atomicity-worker.mjs');
const schema = join(tempRoot, 'audit_atomicity.sql');
const wranglerLog = join(tempRoot, 'wrangler.log');
await mkdir(migrationsDir, { recursive: true });
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

const sentinelEnvironment = { AUDIT_ATOMICITY_LOCAL_SENTINEL: 'retained' };
Object.defineProperty(sentinelEnvironment, 'CLOUDFLARE_API_TOKEN', {
  enumerable: true,
  get() {
    throw new Error('sentinel authority value was read');
  },
});
const sentinelResult = inheritNonAuthorityEnvironment(sentinelEnvironment);
assert(
  sentinelResult.AUDIT_ATOMICITY_LOCAL_SENTINEL === 'retained'
    && !Object.hasOwn(sentinelResult, 'CLOUDFLARE_API_TOKEN'),
  'authority-key filtering regression',
);

const env = {
  ...inheritNonAuthorityEnvironment(process.env),
  XDG_CONFIG_HOME: xdgConfig,
  WRANGLER_LOG_PATH: wranglerLog,
  WRANGLER_LOG: 'error',
  NO_COLOR: '1',
};

let dev;
try {
  const wranglerMetadata = JSON.parse(await readFile(wranglerPackage, 'utf8'));
  const wranglerVersion = String(wranglerMetadata.version ?? '');
  assert(/^4\./u.test(wranglerVersion), `Audit atomicity proof requires Wrangler 4.x, found ${wranglerVersion || 'unknown'}`);

  const fixtureSource = await readFile(workerFixture, 'utf8');
  assert(!fixtureSource.includes('console.'), 'proof Worker must not log audit or D1 details');
  assert(fixtureSource.includes('WHERE changes() = 1'), 'conditional audit shape changed');
  assert(fixtureSource.includes('db.batch(statements)'), 'conditional proof must use one D1 batch');

  for (const entry of await readdir(sourceMigrations)) {
    if (entry.endsWith('.sql')) {
      await copyFile(join(sourceMigrations, entry), join(migrationsDir, entry));
    }
  }
  await copyFile(workerFixture, workerSource);
  await copyFile(schemaFixture, schema);
  await writeFile(config, [
    'name = "zinnias-ciao-audit-atomicity-local"',
    'main = "audit-atomicity-worker.mjs"',
    'compatibility_date = "2026-06-08"',
    'workers_dev = false',
    '',
    '[[d1_databases]]',
    'binding = "PROOF_DB"',
    `database_name = "${database}"`,
    'database_id = "local"',
    'migrations_dir = "migrations"',
    '',
  ].join('\n'), 'utf8');

  await run(wranglerBin, [
    'd1', 'migrations', 'apply', database,
    '--local', '--persist-to', persistTo, '--config', config,
  ], { env });
  await run(wranglerBin, [
    'd1', 'execute', database,
    '--local', '--yes', '--persist-to', persistTo, '--config', config, '--file', schema,
  ], { env });

  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  dev = spawn(wranglerBin, [
    'dev', '--local', '--ip', '127.0.0.1', '--port', String(port),
    '--persist-to', persistTo, '--config', config, '--log-level', 'error',
    '--show-interactive-dev-session=false',
  ], { cwd: root, env, stdio: ['ignore', 'pipe', 'pipe'] });
  let devStdout = '';
  let devStderr = '';
  dev.stdout.on('data', (chunk) => { devStdout += chunk; });
  dev.stderr.on('data', (chunk) => { devStderr += chunk; });

  await waitUntilReady(baseUrl, dev, () => `${devStdout}\n${devStderr}`.trim());
  assert((await request(baseUrl, '/reset')).reset === true, 'fixture reset failed');

  const success = await request(baseUrl, '/conditional/success');
  assert(success.batchSucceeded === true, 'conditional success did not commit');
  assert(JSON.stringify(success.statementChanges) === '[1,1]', 'conditional success accounting changed');
  expectState(success.state, { businessState: 1, multiWrites: 0, audits: 1 }, 'conditional success');

  const replay = await request(baseUrl, '/conditional/replay');
  assert(replay.batchSucceeded === true, 'conditional replay should remain a benign no-op');
  assert(JSON.stringify(replay.statementChanges) === '[0,0]', 'conditional replay wrote state or audit');
  expectState(replay.state, { businessState: 1, multiWrites: 0, audits: 1 }, 'conditional replay');

  const authorization = await request(baseUrl, '/conditional/authorization');
  assert(authorization.batchSucceeded === true, 'authorization loss should remain a benign no-op');
  assert(JSON.stringify(authorization.statementChanges) === '[0,0]', 'authorization loss wrote state or audit');
  expectState(authorization.state, { businessState: 0, multiWrites: 0, audits: 0 }, 'authorization loss');

  const auditFailure = await request(baseUrl, '/conditional/audit-failure');
  assert(auditFailure.batchSucceeded === false, 'conditional audit rejection unexpectedly committed');
  expectState(auditFailure.state, { businessState: 0, multiWrites: 0, audits: 0 }, 'conditional audit failure');

  const multiFailure = await request(baseUrl, '/unconditional/audit-failure');
  assert(multiFailure.batchSucceeded === false, 'unconditional audit rejection unexpectedly committed');
  expectState(multiFailure.state, { businessState: 0, multiWrites: 0, audits: 0 }, 'unconditional audit failure');

  assert((await request(baseUrl, '/reset')).reset === true, 'Package 4 fixture reset failed');
  const eventCreate = await request(baseUrl, '/package4/event/success');
  assert(eventCreate.batchSucceeded === true, 'bounded event create did not commit');
  assert(JSON.stringify(eventCreate.statementChanges) === '[1,1,1,1]', 'event create statement accounting changed');
  assert(eventCreate.state.eventHeaders === 1 && eventCreate.state.eventParts === 2 && eventCreate.state.audits === 1,
    'event create did not commit all business rows with one audit');

  const eventFailure = await request(baseUrl, '/package4/event/audit-failure');
  assert(eventFailure.batchSucceeded === false, 'event create audit rejection unexpectedly committed');
  assert(eventFailure.state.eventHeaders === 1 && eventFailure.state.eventParts === 2 && eventFailure.state.audits === 0,
    'event create audit failure did not roll back its full multi-write batch');

  const editEligibilityLoss = await request(baseUrl, '/package4/edit/eligibility-loss');
  assert(JSON.stringify(editEligibilityLoss.statementChanges) === '[0,0,0]',
    'ineligible schedule edit reached its event tail or audit');
  assert(editEligibilityLoss.state.title === 'old title' && editEligibilityLoss.state.dayCount === 2
    && editEligibilityLoss.state.requestedDays === 0 && editEligibilityLoss.state.audits === 0,
  'ineligible schedule edit committed a partial transition');

  const occurrence = await request(baseUrl, '/package4/occurrence/success');
  assert(occurrence.batchSucceeded === true && JSON.stringify(occurrence.statementChanges) === '[1,1,1]',
    'occurrence cancellation did not commit its three-statement chain');
  assert(occurrence.state.status === 'cancelled' && occurrence.state.exceptions === 1 && occurrence.state.audits === 1,
    'occurrence cancellation success state is incomplete');
  const occurrenceReplay = await request(baseUrl, '/package4/occurrence/replay');
  assert(occurrenceReplay.batchSucceeded === true && JSON.stringify(occurrenceReplay.statementChanges) === '[0,0,0]',
    'occurrence cancellation replay wrote state or audit');
  const occurrenceFailure = await request(baseUrl, '/package4/occurrence/audit-failure');
  assert(occurrenceFailure.batchSucceeded === false && occurrenceFailure.state.status === 'scheduled'
    && occurrenceFailure.state.exceptions === 0 && occurrenceFailure.state.audits === 0,
  'occurrence audit failure did not roll back day and exception state');

  const attendance = await request(baseUrl, '/package4/attendance/success');
  assert(JSON.stringify(attendance.statementChanges) === '[2,1]' && attendance.changedCount === 2,
    'set-based attendance count was not database-derived');
  const attendanceReplay = await request(baseUrl, '/package4/attendance/replay');
  assert(JSON.stringify(attendanceReplay.statementChanges) === '[0,0]' && attendanceReplay.changedCount === 0,
    'set-based attendance replay wrote state or audit');

  const calendarFailure = await request(baseUrl, '/package4/calendar/audit-failure');
  assert(calendarFailure.batchSucceeded === false && calendarFailure.state.activeTokens === 1,
    'calendar rotation audit failure did not restore the predecessor token');

  process.stdout.write(`${JSON.stringify({
    wrangler: wranglerVersion,
    mode: 'local-only',
    primitive: 'required-audit batch',
    cases: {
      success: { committed: true, statementChanges: success.statementChanges, state: success.state },
      replay: { committed: true, statementChanges: replay.statementChanges, state: replay.state },
      authorizationLoss: { committed: true, statementChanges: authorization.statementChanges, state: authorization.state },
      conditionalAuditFailure: { committed: false, state: auditFailure.state },
      unconditionalAuditFailure: { committed: false, state: multiFailure.state },
      boundedEventCreate: { committed: true, statementChanges: eventCreate.statementChanges },
      boundedEventAuditFailure: { committed: false },
      editEligibilityLoss: { statementChanges: editEligibilityLoss.statementChanges, state: editEligibilityLoss.state },
      occurrenceCancellation: { committed: true, statementChanges: occurrence.statementChanges },
      occurrenceReplay: { statementChanges: occurrenceReplay.statementChanges },
      occurrenceAuditFailure: { committed: false, dayAndExceptionRolledBack: true },
      setBasedAttendance: { statementChanges: attendance.statementChanges, changedCount: attendance.changedCount },
      setBasedAttendanceReplay: { statementChanges: attendanceReplay.statementChanges, changedCount: 0 },
      calendarRotationAuditFailure: { committed: false, predecessorRestored: true },
    },
    privacy: { identifiersPrinted: false, metadataPrinted: false, workerConsoleCalls: 0 },
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
