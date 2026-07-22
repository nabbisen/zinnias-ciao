#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHmac } from 'node:crypto';
import { writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { prepareIsolatedWorkerTest } from './lib/isolated-worker-test.mjs';

const unavailable = 'ただいまサービスを利用できません。しばらくしてから、もう一度お試しください。';
const unavailableBody = `<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="theme-color" content="#007AFF">
<title>${unavailable} — ciao.zinnias</title>
<link rel="manifest" href="/manifest.webmanifest">
<link rel="stylesheet" href="/static/app.css">
</head>
<body>
<main style="padding:2rem;font-family:system-ui,sans-serif;max-width:480px;margin:auto"><p>${unavailable}</p></main>
<script src="/static/app.js?v=0.59.0-rfc056-rfc065-rfc066-rfc067-rfc068-rfc064-rfc069" defer></script>
</body>
</html>`;

async function freePort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  const port = typeof address === 'object' && address ? address.port : 0;
  await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  assert.ok(port > 0);
  return port;
}

async function start(fixture) {
  const port = await freePort();
  const child = fixture.spawnDev(port);
  let stderr = '';
  child.stderr.on('data', (chunk) => {
    stderr += chunk.toString();
  });
  const baseUrl = `http://127.0.0.1:${port}`;
  for (let attempt = 0; attempt < 160; attempt += 1) {
    if (child.exitCode !== null) throw new Error(`Wrangler exited before startup: ${stderr.slice(-1200)}`);
    try {
      const response = await fetch(`${baseUrl}/version`);
      if (response.status === 200) return { baseUrl, child, stderr: () => stderr };
    } catch {
      // Local Worker is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  child.kill('SIGTERM');
  throw new Error(`Wrangler did not start: ${stderr.slice(-1200)}`);
}

async function stop(running) {
  if (!running || running.child.exitCode !== null) return;
  running.child.kill('SIGTERM');
  await Promise.race([
    new Promise((resolve) => running.child.once('exit', resolve)),
    new Promise((resolve) => setTimeout(resolve, 2000)),
  ]);
}

async function request(baseUrl, path, { method = 'GET', cookie, form, json, bearer } = {}) {
  const headers = {};
  if (cookie) headers.Cookie = cookie;
  if (bearer) headers.Authorization = `Bearer ${bearer}`;
  let body;
  if (form) {
    headers['Content-Type'] = 'application/x-www-form-urlencoded';
    body = new URLSearchParams(form);
  }
  if (json) {
    headers['Content-Type'] = 'application/json';
    body = JSON.stringify(json);
  }
  const response = await fetch(`${baseUrl}${path}`, { method, headers, body, redirect: 'manual' });
  return { response, text: await response.text() };
}

function assertUnavailable(result) {
  assert.equal(result.response.status, 503);
  assert.equal(result.text, unavailableBody);
  assert.equal(result.response.headers.get('cache-control'), 'no-store');
  assert.equal(result.response.headers.get('location'), null);
  assert.equal(result.response.headers.get('set-cookie'), null);
  assert.match(result.response.headers.get('x-request-id') ?? '', /^[a-f0-9]{16}$/u);
  assert.ok((result.response.headers.get('content-security-policy') ?? '').includes("default-src 'self'"));
  for (const forbidden of [
    'HMAC_PEPPER',
    'missing',
    'empty',
    'surrounding_whitespace',
    'legacy_sentinel',
    'too_short',
    'too_long',
    'dev-pepper',
  ]) {
    assert.ok(!result.text.includes(forbidden), `unavailable body exposed ${forbidden}`);
  }
}

function migrate(fixture) {
  fixture.runWranglerSync([
    'd1',
    'migrations',
    'apply',
    'zinnias-ciao-dev',
    '--local',
    '--env',
    'dev',
  ]);
}

function execute(fixture, statement, json = false) {
  const args = [
    'd1',
    'execute',
    'zinnias-ciao-dev',
    '--local',
    '--env',
    'dev',
    '--command',
    statement,
  ];
  if (json) args.push('--json');
  return fixture.runWranglerSync(args, { encoding: 'utf8' });
}

function scalar(fixture, statement) {
  const parsed = JSON.parse(execute(fixture, statement, true));
  return Number((parsed?.[0]?.results ?? parsed?.results)?.[0]?.value ?? Number.NaN);
}

async function invalidPhase(label, options, fullMatrix = false) {
  const fixture = await prepareIsolatedWorkerTest(`pepper-${label}`, {
    includeD1: false,
    includeKv: false,
    ...options,
  });
  let running;
  try {
    running = await start(fixture);
    const health = await request(running.baseUrl, '/healthz');
    assert.equal(health.response.status, 503);
    assert.deepEqual(JSON.parse(health.text), {
      ok: false,
      ready: false,
      service: 'ciao.zinnias',
    });
    assert.equal(health.text.includes('HMAC_PEPPER'), false);

    assertUnavailable(await request(running.baseUrl, '/join'));
    if (fullMatrix) {
      assertUnavailable(
        await request(running.baseUrl, '/c/com_missing/home', {
          cookie: 'ciao_sid=synthetic-session',
        }),
      );
      assertUnavailable(await request(running.baseUrl, '/join', { method: 'POST', form: {} }));
      assertUnavailable(await request(running.baseUrl, '/unknown'));
      assertUnavailable(await request(running.baseUrl, '/version', { method: 'POST' }));
      for (const path of [
        '/manifest.webmanifest',
        '/sw.js',
        '/static/app.css',
        '/static/app.js',
        '/offline',
        '/version',
      ]) {
        assert.equal((await request(running.baseUrl, path)).response.status, 200, path);
      }
    }
    await fixture.assertChildEnvironmentAudit();
    assert.ok(fixture.workerArtifacts.startsWith(`${fixture.root}/`));
    assert.ok(!fixture.canaryPath.startsWith(`${fixture.root}/`));
  } finally {
    await stop(running);
    await fixture.cleanup();
  }
}

await invalidPhase('missing', { includeSecretFile: false }, true);
await invalidPhase('empty', { secretContents: 'HMAC_PEPPER=\n' });
await invalidPhase('whitespace', { secretContents: 'HMAC_PEPPER="                                "\n' });
await invalidPhase('surrounding', { secretContents: `HMAC_PEPPER=" ${'a'.repeat(32)}"\n` });
await invalidPhase('short', { pepper: 'a'.repeat(31) });
await invalidPhase('sentinel', { pepper: 'dev-pepper-change-in-production' });
await invalidPhase('long', { pepper: 'a'.repeat(4097) });

const nonMutation = await prepareIsolatedWorkerTest('pepper-non-mutation', {
  pepper: 'dev-pepper',
});
let nonMutationServer;
try {
  migrate(nonMutation);
  execute(
    nonMutation,
    "INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES ('com_guard','Guard','Asia/Tokyo',1,'2026-07-21T00:00:00.000Z')",
  );
  nonMutation.runWranglerSync([
    'kv',
    'key',
    'put',
    'guard-key',
    'guard-value',
    '--binding',
    'RATE_LIMIT',
    '--local',
    '--env',
    'dev',
  ]);
  const d1Before = scalar(nonMutation, 'SELECT COUNT(*) AS value FROM communities');
  const kvBefore = nonMutation.runWranglerSync(
    ['kv', 'key', 'get', 'guard-key', '--binding', 'RATE_LIMIT', '--local', '--env', 'dev', '--text'],
    { encoding: 'utf8' },
  );
  nonMutationServer = await start(nonMutation);
  assertUnavailable(await request(nonMutationServer.baseUrl, '/join'));
  assertUnavailable(await request(nonMutationServer.baseUrl, '/join', { method: 'POST', form: {} }));
  await stop(nonMutationServer);
  nonMutationServer = undefined;
  assert.equal(scalar(nonMutation, 'SELECT COUNT(*) AS value FROM communities'), d1Before);
  assert.equal(
    nonMutation.runWranglerSync(
      ['kv', 'key', 'get', 'guard-key', '--binding', 'RATE_LIMIT', '--local', '--env', 'dev', '--text'],
      { encoding: 'utf8' },
    ),
    kvBefore,
  );
} finally {
  await stop(nonMutationServer);
  await nonMutation.cleanup();
}

const restore = await prepareIsolatedWorkerTest('pepper-restore');
let restoreServer;
try {
  migrate(restore);
  const sessionSecret = 'synthetic-restore-session';
  const sessionHmac = createHmac('sha256', restore.pepper).update(sessionSecret).digest('hex');
  execute(
    restore,
    [
      "INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES ('com_restore','Restore','Asia/Tokyo',1,'2026-07-21T00:00:00.000Z')",
      "INSERT INTO users (id,created_at) VALUES ('usr_restore','2026-07-21T00:00:00.000Z')",
      "INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES ('mem_restore','com_restore','usr_restore','member','Restore User','2026-07-21T00:00:00.000Z')",
      `INSERT INTO sessions (id,user_id,session_hmac,created_at,expires_at,last_seen_at) VALUES ('sess_restore','usr_restore','${sessionHmac}','2026-07-21T00:00:00.000Z','2099-12-31T23:59:59.000Z','2026-07-21T00:00:00.000Z')`,
    ].join(';'),
  );
  restoreServer = await start(restore);
  const ready = await request(restoreServer.baseUrl, '/healthz');
  assert.equal(ready.response.status, 200);
  assert.deepEqual(JSON.parse(ready.text), {
    ok: true,
    ready: true,
    service: 'ciao.zinnias',
  });
  assert.equal(
    (await request(restoreServer.baseUrl, '/c/com_restore/home', { cookie: `ciao_sid=${sessionSecret}` }))
      .response.status,
    200,
  );
  await stop(restoreServer);
  restoreServer = undefined;

  await writeFile(restore.secretPath, `HMAC_PEPPER=${'b'.repeat(64)}\n`, { mode: 0o600 });
  restoreServer = await start(restore);
  assert.equal(
    (await request(restoreServer.baseUrl, '/c/com_restore/home', { cookie: `ciao_sid=${sessionSecret}` }))
      .response.status,
    401,
  );
  await stop(restoreServer);
  restoreServer = undefined;

  await writeFile(restore.secretPath, `HMAC_PEPPER=${restore.pepper}\n`, { mode: 0o600 });
  restoreServer = await start(restore);
  assert.equal(
    (await request(restoreServer.baseUrl, '/c/com_restore/home', { cookie: `ciao_sid=${sessionSecret}` }))
      .response.status,
    200,
  );
  await restore.assertChildEnvironmentAudit();
} finally {
  await stop(restoreServer);
  await restore.cleanup();
}

const recoveryToken = 'synthetic-optional-recovery-token';
const recovery = await prepareIsolatedWorkerTest('pepper-optional-recovery', {
  recoveryEnabled: true,
  recoveryToken,
  requiredSecrets: ['HMAC_PEPPER', 'COMMUNITY_RECOVERY_TOKEN'],
});
let recoveryServer;
try {
  migrate(recovery);
  execute(
    recovery,
    [
      "INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES ('com_recovery','Recovery','Asia/Tokyo',1,'2026-07-22T00:00:00.000Z')",
      "INSERT INTO users (id,created_at) VALUES ('usr_recovery','2026-07-22T00:00:00.000Z')",
      "INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES ('mem_recovery','com_recovery','usr_recovery','admin','Recovery Admin','2026-07-22T00:00:00.000Z')",
    ].join(';'),
  );
  recoveryServer = await start(recovery);
  const result = await request(recoveryServer.baseUrl, '/operator/recovery/community-access', {
    method: 'POST',
    bearer: recoveryToken,
    json: {
      community_id: 'com_recovery',
      admin_membership_id: 'mem_recovery',
      operator_label: 'rfc077-local-proof',
    },
  });
  assert.equal(result.response.status, 200);
  const parsed = JSON.parse(result.text);
  assert.equal(parsed.ok, true);
  assert.equal(parsed.community_id, 'com_recovery');
  assert.equal(parsed.admin_membership_id, 'mem_recovery');
  assert.equal(scalar(recovery, 'SELECT COUNT(*) AS value FROM membership_relink_codes'), 1);
  await recovery.assertChildEnvironmentAudit();
} finally {
  await stop(recoveryServer);
  await recovery.cleanup();
}

console.log(
  JSON.stringify({
    ok: true,
    phases: {
      invalidMatrix: true,
      exactStaticAllowlist: true,
      omittedBindings: true,
      nonMutation: true,
      validReadiness: true,
      rotationInvalidates: true,
      restorationRecovers: true,
      optionalRecoverySecret: true,
      copiedArtifactsAndCanary: true,
      childEnvironmentAudit: true,
      isolatedCleanup: true,
    },
  }),
);
