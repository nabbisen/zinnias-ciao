// Disposable local-only Class B/Class C response-boundary proof.
// Never add --remote, hosted configuration, credentials, or production data.

import { spawn } from 'node:child_process';
import { copyFile, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const fixtures = join(root, 'workers/ssr/tests/fixtures');
const wranglerBin = join(root, 'node_modules/.bin/wrangler');
const wranglerPackage = join(root, 'node_modules/wrangler/package.json');
const database = 'zinnias-audit-boundaries-local';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function run(command, args, { cwd = root, env } = {}) {
  return new Promise((accept, reject) => {
    const child = spawn(command, args, { cwd, env, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.once('error', reject);
    child.once('exit', async (code) => {
      if (code === 0) accept({ stdout, stderr });
      else {
        let log = '';
        try {
          log = env?.WRANGLER_LOG_PATH ? await readFile(env.WRANGLER_LOG_PATH, 'utf8') : '';
        } catch {
          // The diagnostic log may not have been created.
        }
        reject(new Error(`${command} ${args.join(' ')} exited ${code}: ${`${stdout}\n${stderr}\n${log}`.trim().slice(-4000)}`));
      }
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

async function request(baseUrl, path, method = 'POST') {
  const response = await fetch(`${baseUrl}${path}`, { method, redirect: 'manual' });
  const text = await response.text();
  let body = null;
  if (text) body = JSON.parse(text);
  return { response, body, text };
}

async function waitUntilReady(baseUrl, child, diagnostics) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (child.exitCode !== null) {
      throw new Error(`wrangler dev exited ${child.exitCode} before readiness: ${diagnostics().slice(-2000)}`);
    }
    try {
      const result = await request(baseUrl, '/health', 'GET');
      if (result.response.ok && result.body?.ready === true) return;
    } catch {
      // Local server is still starting.
    }
    await new Promise((accept) => setTimeout(accept, 100));
  }
  throw new Error('local wrangler dev did not become ready');
}

const authorityKey = /^(?:CLOUDFLARE_|CF_(?:API_TOKEN|API_KEY|EMAIL|ACCOUNT_ID|ZONE_ID)$|WRANGLER_(?:API_TOKEN|OAUTH_TOKEN)$)/u;
function inheritNonAuthorityEnvironment(source) {
  const allowed = {};
  for (const key of Object.keys(source)) {
    if (authorityKey.test(key)) continue;
    const value = source[key];
    if (value !== undefined) allowed[key] = value;
  }
  return allowed;
}

const disposableRoot = join(root, '.git-exclude/tmp');
await mkdir(disposableRoot, { recursive: true });
const tempRoot = await mkdtemp(join(disposableRoot, 'audit-boundaries-'));
const migrationsDir = join(tempRoot, 'migrations');
const persistTo = join(tempRoot, 'state');
const xdgConfig = join(tempRoot, 'xdg');
const config = join(tempRoot, 'wrangler.toml');
const workerSource = join(tempRoot, 'audit-boundaries-worker.mjs');
const schema = join(tempRoot, 'audit_response_boundaries.sql');
const wranglerLog = join(tempRoot, 'wrangler.log');
await mkdir(persistTo, { recursive: true });
await mkdir(xdgConfig, { recursive: true });
await mkdir(migrationsDir, { recursive: true });

const env = {
  ...inheritNonAuthorityEnvironment(process.env),
  XDG_CONFIG_HOME: xdgConfig,
  WRANGLER_LOG_PATH: wranglerLog,
  WRANGLER_LOG: 'error',
  NO_COLOR: '1',
};

let dev;
try {
  const wrangler = JSON.parse(await readFile(wranglerPackage, 'utf8'));
  assert(/^4\./u.test(String(wrangler.version ?? '')), 'audit-boundary proof requires Wrangler 4.x');
  const fixtureSource = await readFile(join(fixtures, 'audit-boundaries-worker.mjs'), 'utf8');
  assert(!fixtureSource.includes('console.'), 'proof Worker must not log protected data');
  assert(
    fixtureSource.indexOf("'community.export_authorized'")
      < fixtureSource.indexOf('SELECT protected_value'),
    'community proof must audit before reading the protected payload',
  );
  assert(
    fixtureSource.indexOf("UPDATE proof_boundary_sessions SET revoked=1")
      < fixtureSource.indexOf("'session.logout'"),
    'logout proof must revoke before attempting its audit',
  );
  await copyFile(join(fixtures, 'audit-boundaries-worker.mjs'), workerSource);
  await copyFile(join(fixtures, 'audit_response_boundaries.sql'), schema);
  for (const entry of await readdir(join(root, 'migrations'))) {
    if (entry.endsWith('.sql')) {
      await copyFile(join(root, 'migrations', entry), join(migrationsDir, entry));
    }
  }
  await writeFile(config, [
    'name = "zinnias-ciao-audit-boundaries-local"',
    'main = "audit-boundaries-worker.mjs"',
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
    'd1', 'migrations', 'apply', database, '--local', '--persist-to', persistTo,
    '--config', config,
  ], { env });
  await run(wranglerBin, [
    'd1', 'execute', database, '--local', '--yes', '--persist-to', persistTo,
    '--config', config, '--file', schema,
  ], { env });

  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  dev = spawn(wranglerBin, [
    'dev', '--local', '--ip', '127.0.0.1', '--port', String(port),
    '--persist-to', persistTo, '--config', config, '--log-level', 'error',
    '--show-interactive-dev-session=false',
  ], { cwd: root, env, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = '';
  let stderr = '';
  dev.stdout.on('data', (chunk) => { stdout += chunk; });
  dev.stderr.on('data', (chunk) => { stderr += chunk; });
  await waitUntilReady(baseUrl, dev, () => `${stdout}\n${stderr}`);

  await request(baseUrl, '/reset');
  const communityFailure = await request(baseUrl, '/class-b/community/audit-failure');
  assert(communityFailure.response.status === 503, 'community audit failure must return 503');
  assert(!communityFailure.text.includes('protected-community-json') && !Object.hasOwn(communityFailure.body, 'payload'), 'community audit failure disclosed protected payload');
  let state = (await request(baseUrl, '/state', 'GET')).body;
  assert(state.communityAudits === 0, 'failed community authorization must not leave audit evidence');
  const communitySuccess = await request(baseUrl, '/class-b/community/success');
  assert(communitySuccess.response.ok && communitySuccess.body?.payload === 'protected-community-json', 'community success must disclose only after audit');

  await request(baseUrl, '/reset');
  const matrixFailure = await request(baseUrl, '/class-b/matrix/audit-failure');
  assert(matrixFailure.response.status === 503 && matrixFailure.body?.ok === false, 'matrix audit failure must return non-acknowledging 503');
  state = (await request(baseUrl, '/state', 'GET')).body;
  assert(state.matrixAudits === 0, 'failed matrix acknowledgement must not leave audit evidence');
  const matrixSuccess = await request(baseUrl, '/class-b/matrix/success');
  assert(matrixSuccess.response.ok && matrixSuccess.body?.ok === true, 'matrix success must acknowledge after audit');

  await request(baseUrl, '/reset');
  const logoutFailure = await request(baseUrl, '/class-c/logout/audit-failure');
  assert(logoutFailure.response.status === 303, 'logout audit failure must still complete logout');
  assert(logoutFailure.response.headers.get('set-cookie')?.includes('Max-Age=0'), 'logout audit failure must clear the cookie');
  state = (await request(baseUrl, '/state', 'GET')).body;
  assert(state.revoked === 1 && state.logoutAudits === 0, 'logout audit failure must preserve revocation without false audit evidence');

  await request(baseUrl, '/reset');
  const logoutSuccess = await request(baseUrl, '/class-c/logout/success');
  assert(logoutSuccess.response.status === 303 && logoutSuccess.response.headers.get('set-cookie')?.includes('Max-Age=0'), 'successful logout must revoke and clear cookie');
  state = (await request(baseUrl, '/state', 'GET')).body;
  assert(state.revoked === 1 && state.logoutAudits === 1, 'successful logout must leave one revocation and one audit');

  process.stdout.write(`${JSON.stringify({
    communityFailure: { status: communityFailure.response.status, disclosed: false },
    matrixFailure: { status: matrixFailure.response.status, acknowledged: false },
    logoutAuditFailure: { status: logoutFailure.response.status, revoked: 1, cookieCleared: true, audits: 0 },
    logoutSuccess: { status: logoutSuccess.response.status, revoked: 1, cookieCleared: true, audits: 1 },
  })}\n`);
} finally {
  if (dev && dev.exitCode === null) {
    dev.kill('SIGTERM');
    await new Promise((accept) => dev.once('exit', accept));
  }
  await rm(tempRoot, { recursive: true, force: true });
}
