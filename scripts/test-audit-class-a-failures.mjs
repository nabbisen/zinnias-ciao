#!/usr/bin/env node
// Disposable local proof for compiled SSR Class A audit-failure telemetry.
// Never add remote targets, hosted configuration, credentials, or production data.

import { createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { prepareIsolatedWorkerTest } from './lib/isolated-worker-test.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const isolated = await prepareIsolatedWorkerTest('audit-class-a-failures');
const wranglerBin = join(root, 'node_modules/.bin/wrangler');
const wranglerPackage = join(root, 'node_modules/wrangler/package.json');
const database = 'zinnias-ciao-dev';
const config = isolated.configPath;
const pepper = isolated.pepper;
const now = '2026-07-17T00:00:00.000Z';
const communityId = 'com_class_a_proof';
const adminUserId = 'usr_class_a_proof';
const adminMembershipId = 'mem_class_a_proof';
const adminSessionId = 'sess_class_a_proof';
const adminSessionSecret = 'class-a-proof-session';
const joinInviteId = 'inv_class_a_join';
const joinCode = 'ACDEFG';

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
      throw new Error(`Class A proof is local-only; refused argument ${argument}`);
    }
  }
}

function hmac(value) {
  return createHmac('sha256', pepper).update(value).digest('hex');
}

function sqlString(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}

function run(command, args, env) {
  return new Promise((accept, reject) => {
    const child = spawn(command, args, {
      cwd: isolated.root,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    child.once('error', reject);
    child.once('exit', (code) => {
      if (code === 0) {
        accept({ stdout, stderr });
      } else {
        reject(new Error(`${command} exited ${code}; local command failed`));
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
  await new Promise((accept, reject) =>
    server.close((error) => (error ? reject(error) : accept())),
  );
  assert(port > 0, 'failed to allocate a loopback port');
  return port;
}

function d1Args(persistTo, extra) {
  return [
    'd1',
    ...extra,
    database,
    '--env',
    'dev',
    '--local',
    '--persist-to',
    persistTo,
    '--config',
    config,
  ];
}

async function executeSql(statement, persistTo, env) {
  assert(!statement.includes('PRAGMA database_list'), 'proof must not inspect external D1 paths');
  await run(
    wranglerBin,
    d1Args(persistTo, ['execute']).concat(['--yes', '--command', statement]),
    env,
  );
}

async function query(statement, persistTo, env) {
  const result = await run(
    wranglerBin,
    d1Args(persistTo, ['execute']).concat(['--yes', '--json', '--command', statement]),
    env,
  );
  if (!result.stdout.trim()) {
    throw new Error(
      `local D1 query returned no JSON: ${JSON.stringify({
        stdoutBytes: result.stdout.length,
        stderr: result.stderr.trim().slice(-1000),
      })}`,
    );
  }
  const parsed = JSON.parse(result.stdout);
  return parsed?.[0]?.results ?? parsed?.results ?? [];
}

async function scalar(statement, persistTo, env) {
  const rows = await query(statement, persistTo, env);
  return Number(rows[0]?.value ?? 0);
}

function hiddenToken(html) {
  const token = html.match(/name="_token"\s+value="([^"]+)"/u)?.[1] ?? '';
  assert(/^[A-Za-z0-9_-]+$/u.test(token), 'response did not contain a bounded form token');
  return token;
}

function absorbCookies(headers, jar) {
  const values =
    typeof headers.getSetCookie === 'function'
      ? headers.getSetCookie()
      : (headers.get('set-cookie') ?? '').split(/,(?=\s*[^;,=\s]+=)/u);
  for (const value of values) {
    const pair = value.split(';', 1)[0];
    const separator = pair.indexOf('=');
    if (separator > 0) jar.set(pair.slice(0, separator).trim(), pair.slice(separator + 1));
  }
}

function cookieHeader(jar) {
  return [...jar].map(([name, value]) => `${name}=${value}`).join('; ');
}

async function request(baseUrl, path, { method = 'GET', form, cookies } = {}) {
  const headers = {};
  if (cookies?.size) headers.Cookie = cookieHeader(cookies);
  let body;
  if (form) {
    headers['Content-Type'] = 'application/x-www-form-urlencoded';
    body = new URLSearchParams(form);
  }
  const response = await fetch(`${baseUrl}${path}`, {
    method,
    headers,
    body,
    redirect: 'manual',
  });
  const text = await response.text();
  if (cookies) absorbCookies(response.headers, cookies);
  return {
    status: response.status,
    location: response.headers.get('location') ?? '',
    requestId: response.headers.get('x-request-id') ?? '',
    text,
  };
}

async function waitUntilReady(baseUrl, child) {
  for (let attempt = 0; attempt < 160; attempt += 1) {
    if (child.exitCode !== null) throw new Error('compiled SSR Worker exited before readiness');
    try {
      const response = await request(baseUrl, '/healthz');
      if (response.status === 200) return;
    } catch {
      // The local compiled Worker is still starting.
    }
    await new Promise((accept) => setTimeout(accept, 250));
  }
  throw new Error('compiled SSR Worker did not become ready');
}

function failureEvents(stderr, offset) {
  return stderr
    .slice(offset)
    .split(/\r?\n/u)
    .map((line) => {
      const start = line.indexOf('event=audit.required_batch_failed');
      return start >= 0
        ? line.slice(start).replaceAll(/\u001b\[[0-9;]*m/gu, '').trim()
        : '';
    })
    .filter(Boolean);
}

async function waitForOneEvent(stderr, offset) {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const events = failureEvents(stderr(), offset);
    if (events.length > 0) return events;
    await new Promise((accept) => setTimeout(accept, 50));
  }
  return failureEvents(stderr(), offset);
}

function assertExactEvent(events, requestId, action) {
  assert(/^[A-Za-z0-9_-]{1,96}$/u.test(requestId), 'response request ID is not bounded');
  const expected =
    `event=audit.required_batch_failed request_id=${requestId} action=${action} ` +
    'failure_category=storage route_class=class_a';
  assert(events.length === 1, `expected one Class A event, observed ${events.length}`);
  assert(
    events[0] === expected,
    `Class A event differed from its bounded contract: ${JSON.stringify({ expected, actual: events[0] })}`,
  );
  for (const forbidden of [
    'community_id',
    'actor_membership_id',
    'target_id',
    'metadata',
    'operation_id',
    'ast_',
    'INSERT',
    'SELECT',
    'bind',
    'error',
    'invite_hmac',
    adminSessionSecret,
    joinCode,
  ]) {
    assert(!events[0].includes(forbidden), `Class A event retained forbidden field ${forbidden}`);
  }
  return expected;
}

assertLocalOnly();
const persistTo = isolated.persistTo;
assert(resolve(persistTo).startsWith(resolve(isolated.root)), 'disposable D1 path escaped its root');

const env = isolated.env;

let dev;
let stderr = '';
const triggerNames = [
  'proof_fail_invite_audit',
  'proof_fail_community_membership_audit',
  'proof_fail_join_audit',
];
const summary = {};

try {
  const wrangler = JSON.parse(await readFile(wranglerPackage, 'utf8'));
  assert(/^4\./u.test(String(wrangler.version ?? '')), 'Class A proof requires Wrangler 4.x');

  await run(
    wranglerBin,
    d1Args(persistTo, ['migrations', 'apply']),
    env,
  );
  await executeSql(
    [
      `INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES (${sqlString(communityId)},'Class A Proof','Asia/Tokyo',1,${sqlString(now)})`,
      `INSERT INTO users (id,created_at) VALUES (${sqlString(adminUserId)},${sqlString(now)})`,
      `INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES (${sqlString(adminMembershipId)},${sqlString(communityId)},${sqlString(adminUserId)},'admin','Proof Admin',${sqlString(now)})`,
      `INSERT INTO sessions (id,user_id,session_hmac,created_at,expires_at,last_seen_at) VALUES (${sqlString(adminSessionId)},${sqlString(adminUserId)},${sqlString(hmac(adminSessionSecret))},${sqlString(now)},'2099-12-31T23:59:59.000Z',${sqlString(now)})`,
      `INSERT INTO invite_codes (id,community_id,code_hmac,created_by_membership_id,expires_at,grants_role,created_at) VALUES (${sqlString(joinInviteId)},${sqlString(communityId)},${sqlString(hmac(joinCode))},${sqlString(adminMembershipId)},'2099-12-31T23:59:59.000Z','member',${sqlString(now)})`,
    ].join(';'),
    persistTo,
    env,
  );

  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  dev = isolated.spawnDev(port);
  dev.stderr.on('data', (chunk) => {
    stderr += chunk.toString();
  });
  await waitUntilReady(baseUrl, dev);

  const adminCookies = new Map([['ciao_sid', adminSessionSecret]]);

  const invitePage = await request(baseUrl, `/c/${communityId}/admin/invites`, {
    cookies: adminCookies,
  });
  assert(invitePage.status === 200, 'invite setup page failed');
  const inviteToken = hiddenToken(invitePage.text);
  await executeSql(
    "CREATE TRIGGER proof_fail_invite_audit BEFORE INSERT ON audit_log WHEN NEW.action='invite_code.generated' BEGIN SELECT RAISE(ABORT,'proof failure'); END",
    persistTo,
    env,
  );
  const inviteOffset = stderr.length;
  const inviteFailure = await request(baseUrl, `/c/${communityId}/admin/invites`, {
    method: 'POST',
    cookies: adminCookies,
    form: { _token: inviteToken },
  });
  const inviteEvents = await waitForOneEvent(() => stderr, inviteOffset);
  summary.executeRequired = {
    event: assertExactEvent(inviteEvents, inviteFailure.requestId, 'invite_code.generated'),
    businessRows: await scalar(
      `SELECT COUNT(*) AS value FROM invite_codes WHERE community_id=${sqlString(communityId)} AND id<>${sqlString(joinInviteId)}`,
      persistTo,
      env,
    ),
    auditRows: await scalar(
      "SELECT COUNT(*) AS value FROM audit_log WHERE action='invite_code.generated'",
      persistTo,
      env,
    ),
  };
  assert(
    summary.executeRequired.businessRows === 0 && summary.executeRequired.auditRows === 0,
    'execute_required storage failure did not roll back business and audit rows',
  );
  await executeSql('DROP TRIGGER proof_fail_invite_audit', persistTo, env);

  const communityPage = await request(baseUrl, '/communities/new', { cookies: adminCookies });
  assert(communityPage.status === 200, 'community setup page failed');
  const communityToken = hiddenToken(communityPage.text);
  const baselineCommunities = await scalar(
    'SELECT COUNT(*) AS value FROM communities',
    persistTo,
    env,
  );
  const baselineMemberships = await scalar(
    'SELECT COUNT(*) AS value FROM community_memberships',
    persistTo,
    env,
  );
  await executeSql(
    "CREATE TRIGGER proof_fail_community_membership_audit BEFORE INSERT ON audit_log WHEN NEW.action='membership.created_first_admin' BEGIN SELECT RAISE(ABORT,'proof failure'); END",
    persistTo,
    env,
  );
  const communityOffset = stderr.length;
  const communityFailure = await request(baseUrl, '/communities/new', {
    method: 'POST',
    cookies: adminCookies,
    form: {
      _token: communityToken,
      community_name: 'Proof New Community',
      display_name: 'Proof Admin',
      timezone: 'Asia/Tokyo',
    },
  });
  const communityEvents = await waitForOneEvent(() => stderr, communityOffset);
  summary.executeRequiredBatch = {
    event: assertExactEvent(communityEvents, communityFailure.requestId, 'community.created'),
    communityDelta:
      (await scalar('SELECT COUNT(*) AS value FROM communities', persistTo, env)) -
      baselineCommunities,
    membershipDelta:
      (await scalar('SELECT COUNT(*) AS value FROM community_memberships', persistTo, env)) -
      baselineMemberships,
    auditRows: await scalar(
      "SELECT COUNT(*) AS value FROM audit_log WHERE action IN ('community.created','membership.created_first_admin')",
      persistTo,
      env,
    ),
  };
  assert(
    summary.executeRequiredBatch.communityDelta === 0 &&
      summary.executeRequiredBatch.membershipDelta === 0 &&
      summary.executeRequiredBatch.auditRows === 0,
    'execute_required_batch storage failure did not roll back the entire operation',
  );
  assert(
    !communityEvents.some((event) => event.includes('membership.created_first_admin')),
    'multi-audit failure emitted an additional-action incident',
  );
  await executeSql('DROP TRIGGER proof_fail_community_membership_audit', persistTo, env);

  const joinCookies = new Map();
  const joinPage = await request(baseUrl, '/join', { cookies: joinCookies });
  const joinToken = hiddenToken(joinPage.text);
  const joinStart = await request(baseUrl, '/join', {
    method: 'POST',
    cookies: joinCookies,
    form: { _token: joinToken, code: joinCode },
  });
  assert(joinStart.status === 303 && joinStart.location === '/join/profile', 'join setup failed');
  const profilePage = await request(baseUrl, '/join/profile', { cookies: joinCookies });
  const profileToken = hiddenToken(profilePage.text);
  await executeSql(
    "CREATE TRIGGER proof_fail_join_audit BEFORE INSERT ON audit_log WHEN NEW.action='invite_code.redeemed' BEGIN SELECT RAISE(ABORT,'proof failure'); END",
    persistTo,
    env,
  );
  const joinOffset = stderr.length;
  const joinFailure = await request(baseUrl, '/join/profile', {
    method: 'POST',
    cookies: joinCookies,
    form: { _token: profileToken, display_name: 'Proof Join Member' },
  });
  const joinEvents = await waitForOneEvent(() => stderr, joinOffset);
  summary.executeAssertedRequired = {
    event: assertExactEvent(joinEvents, joinFailure.requestId, 'invite_code.redeemed'),
    inviteUsed: await scalar(
      `SELECT COUNT(*) AS value FROM invite_codes WHERE id=${sqlString(joinInviteId)} AND used_at IS NOT NULL`,
      persistTo,
      env,
    ),
    users: await scalar(
      "SELECT COUNT(*) AS value FROM users WHERE id<>'usr_class_a_proof'",
      persistTo,
      env,
    ),
    memberships: await scalar(
      `SELECT COUNT(*) AS value FROM community_memberships WHERE community_id=${sqlString(communityId)} AND id<>${sqlString(adminMembershipId)}`,
      persistTo,
      env,
    ),
    sessions: await scalar(
      `SELECT COUNT(*) AS value FROM sessions WHERE id<>${sqlString(adminSessionId)}`,
      persistTo,
      env,
    ),
    audits: await scalar(
      "SELECT COUNT(*) AS value FROM audit_log WHERE action='invite_code.redeemed'",
      persistTo,
      env,
    ),
    guards: await scalar('SELECT COUNT(*) AS value FROM audit_change_assertions', persistTo, env),
  };
  assert(
    summary.executeAssertedRequired.inviteUsed === 0 &&
      summary.executeAssertedRequired.users === 0 &&
      summary.executeAssertedRequired.memberships === 0 &&
      summary.executeAssertedRequired.sessions === 0 &&
      summary.executeAssertedRequired.audits === 0 &&
      summary.executeAssertedRequired.guards === 0,
    'execute_asserted_required storage failure did not roll back all asserted state',
  );
  await executeSql('DROP TRIGGER proof_fail_join_audit', persistTo, env);

  const successPage = await request(baseUrl, `/c/${communityId}/admin/invites`, {
    cookies: adminCookies,
  });
  const successOffset = stderr.length;
  const success = await request(baseUrl, `/c/${communityId}/admin/invites`, {
    method: 'POST',
    cookies: adminCookies,
    form: { _token: hiddenToken(successPage.text) },
  });
  await new Promise((accept) => setTimeout(accept, 100));
  assert(
    success.status === 200,
    `post-trigger RFC-076 reveal success control failed with status ${success.status}; worker stderr tail: ${stderr.slice(-1200)}`,
  );
  assert(failureEvents(stderr, successOffset).length === 0, 'success control emitted a failure');
  summary.successControl = {
    failureEvents: 0,
    businessRows: await scalar(
      `SELECT COUNT(*) AS value FROM invite_codes WHERE community_id=${sqlString(communityId)} AND id<>${sqlString(joinInviteId)}`,
      persistTo,
      env,
    ),
    auditRows: await scalar(
      "SELECT COUNT(*) AS value FROM audit_log WHERE action='invite_code.generated'",
      persistTo,
      env,
    ),
  };
  assert(
    summary.successControl.businessRows === 1 && summary.successControl.auditRows === 1,
    'success control did not persist one business row and one audit row',
  );

  process.stdout.write(`${JSON.stringify({ localOnly: true, disposableD1: true, ...summary })}\n`);
} finally {
  for (const trigger of triggerNames) {
    try {
      await executeSql(`DROP TRIGGER IF EXISTS ${trigger}`, persistTo, env);
    } catch {
      // The disposable database may not have reached migration/setup.
    }
  }
  if (dev && dev.exitCode === null) {
    dev.kill('SIGTERM');
    await new Promise((accept) => dev.once('exit', accept));
  }
  stderr = '';
  await isolated.cleanup();
}
