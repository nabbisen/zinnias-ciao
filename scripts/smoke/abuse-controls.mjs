#!/usr/bin/env node
// Local compiled-Worker evidence for RFC-078's fail-closed abuse-control
// coordinator: missing-binding fail-closed behavior, the exact fixed-window
// boundary (10 allowed / 11th blocked for invite, 3 allowed / 4th blocked
// for community creation), and reset-after-valid-credential behavior.
//
// Raw HTTP against a local `wrangler dev` instance — no browser needed,
// since only status codes, headers, and D1 row state are asserted.

import assert from 'node:assert/strict';
import { createHmac } from 'node:crypto';
import { createServer } from 'node:net';
import { prepareIsolatedWorkerTest } from '../lib/isolated-worker-test.mjs';

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

async function request(baseUrl, path, { method = 'GET', cookie, form } = {}) {
  const headers = {};
  if (cookie) headers.Cookie = cookie;
  let body;
  if (form) {
    headers['Content-Type'] = 'application/x-www-form-urlencoded';
    body = new URLSearchParams(form);
  }
  const response = await fetch(`${baseUrl}${path}`, { method, headers, body, redirect: 'manual' });
  return { response, text: await response.text() };
}

function extractToken(html) {
  const match = /name="_token" value="([^"]+)"/u.exec(html);
  if (!match) throw new Error('no _token field found in rendered form');
  return match[1];
}

function migrate(fixture) {
  fixture.runWranglerSync(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
}

function execute(fixture, statement, json = false) {
  const args = ['d1', 'execute', 'zinnias-ciao-dev', '--local', '--env', 'dev', '--command', statement];
  if (json) args.push('--json');
  return fixture.runWranglerSync(args, { encoding: 'utf8' });
}

function scalar(fixture, statement) {
  const parsed = JSON.parse(execute(fixture, statement, true));
  return Number((parsed?.[0]?.results ?? parsed?.results)?.[0]?.value ?? Number.NaN);
}

function normalizeCode(raw) {
  return raw.replace(/[\s-]/gu, '').toUpperCase();
}

function inviteHmac(pepper, rawCode) {
  return createHmac('sha256', pepper).update(normalizeCode(rawCode)).digest('hex');
}

// ── Phase 1: missing ABUSE_LIMITER binding fails closed ─────────────────────

async function missingBindingFailsClosed() {
  const fixture = await prepareIsolatedWorkerTest('abuse-missing-binding', {
    includeAbuseLimiter: false,
  });
  let running;
  try {
    migrate(fixture);
    running = await start(fixture);
    const before = scalar(fixture, 'SELECT COUNT(*) AS value FROM invite_codes');

    const joinPage = await request(running.baseUrl, '/join');
    const token = extractToken(joinPage.text);
    const result = await request(running.baseUrl, '/join', {
      method: 'POST',
      form: { code: 'ZZZZZZ', _token: token },
    });

    assert.equal(result.response.status, 503, 'missing binding must fail closed with 503');
    assert.equal(result.response.headers.get('retry-after'), null, '503 must not guess a retry interval');
    assert.equal(result.response.headers.get('cache-control'), 'no-store');
    assert.equal(
      scalar(fixture, 'SELECT COUNT(*) AS value FROM invite_codes'),
      before,
      'missing-binding rejection must perform no D1 mutation',
    );
    assert.equal(
      scalar(fixture, "SELECT COUNT(*) AS value FROM form_tokens WHERE consumed_at IS NOT NULL"),
      1,
      'ingress runs after form-token consumption, so exactly the submitted token is consumed even when the limiter is unavailable',
    );
    await fixture.assertChildEnvironmentAudit();
  } finally {
    await stop(running);
    await fixture.cleanup();
  }
}

// ── Phase 2: fixed invite window — 10 allowed, 11th blocked ────────────────

async function inviteWindowBoundary() {
  const fixture = await prepareIsolatedWorkerTest('abuse-invite-window');
  let running;
  try {
    migrate(fixture);
    running = await start(fixture);

    for (let attempt = 1; attempt <= 10; attempt += 1) {
      const page = await request(running.baseUrl, '/join');
      const token = extractToken(page.text);
      const result = await request(running.baseUrl, '/join', {
        method: 'POST',
        form: { code: 'ZZZZZZ', _token: token },
      });
      assert.equal(result.response.status, 200, `attempt ${attempt} must reach credential validation`);
      assert.equal(result.response.headers.get('retry-after'), null);
    }

    const page = await request(running.baseUrl, '/join');
    const token = extractToken(page.text);
    const blocked = await request(running.baseUrl, '/join', {
      method: 'POST',
      form: { code: 'ZZZZZZ', _token: token },
    });
    assert.equal(blocked.response.status, 429, '11th attempt must be blocked');
    const retryAfter = Number(blocked.response.headers.get('retry-after'));
    assert.ok(retryAfter >= 1 && retryAfter <= 300, `retry-after ${retryAfter} must be within 1..=300`);
    assert.equal(blocked.response.headers.get('cache-control'), 'no-store');

    // A fresh same-purpose token must still be rendered so a no-JS retry
    // remains possible.
    assert.ok(extractToken(blocked.text).length > 0);

    await fixture.assertChildEnvironmentAudit();
  } finally {
    await stop(running);
    await fixture.cleanup();
  }
}

// ── Phase 3: valid redemption resets the invite window ─────────────────────

async function validRedemptionResetsWindow() {
  const fixture = await prepareIsolatedWorkerTest('abuse-invite-reset');
  let running;
  try {
    migrate(fixture);
    const codeHmac = inviteHmac(fixture.pepper, 'ABCDEF');
    execute(
      fixture,
      [
        "INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES ('com_reset','Reset','Asia/Tokyo',1,'2026-07-28T00:00:00.000Z')",
        "INSERT INTO users (id,created_at) VALUES ('usr_reset_admin','2026-07-28T00:00:00.000Z')",
        "INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES ('mem_reset_admin','com_reset','usr_reset_admin','admin','Reset Admin','2026-07-28T00:00:00.000Z')",
        `INSERT INTO invite_codes (id,community_id,code_hmac,created_by_membership_id,expires_at,grants_role,created_at) VALUES ('inv_reset','com_reset','${codeHmac}','mem_reset_admin','2099-12-31T23:59:59Z','member','2026-07-28T00:00:00.000Z')`,
      ].join(';'),
    );
    running = await start(fixture);

    // 3 invalid attempts, then 1 valid redemption (still within the 10/300s
    // window — the 4th authenticated submission).
    for (let attempt = 1; attempt <= 3; attempt += 1) {
      const page = await request(running.baseUrl, '/join');
      const token = extractToken(page.text);
      const result = await request(running.baseUrl, '/join', {
        method: 'POST',
        form: { code: 'ZZZZZZ', _token: token },
      });
      assert.equal(result.response.status, 200);
    }
    const page = await request(running.baseUrl, '/join');
    const token = extractToken(page.text);
    const redeemed = await request(running.baseUrl, '/join', {
      method: 'POST',
      form: { code: 'ABCDEF', _token: token },
    });
    assert.equal(redeemed.response.status, 303, 'valid invite must redeem and redirect to the profile step');

    // If reset had not zeroed the window, only 6 more attempts would fit
    // before block (10 - 4 = 6); prove the window actually restarted by
    // running 10 more invalid attempts and observing the 11th (not the 7th)
    // as the first blocked one.
    for (let attempt = 1; attempt <= 10; attempt += 1) {
      const p = await request(running.baseUrl, '/join');
      const t = extractToken(p.text);
      const result = await request(running.baseUrl, '/join', {
        method: 'POST',
        form: { code: 'ZZZZZZ', _token: t },
      });
      assert.equal(result.response.status, 200, `post-reset attempt ${attempt} must reach credential validation`);
    }
    const p = await request(running.baseUrl, '/join');
    const t = extractToken(p.text);
    const blocked = await request(running.baseUrl, '/join', {
      method: 'POST',
      form: { code: 'ZZZZZZ', _token: t },
    });
    assert.equal(blocked.response.status, 429, 'the 11th post-reset attempt must be the first blocked one');

    await fixture.assertChildEnvironmentAudit();
  } finally {
    await stop(running);
    await fixture.cleanup();
  }
}

// ── Phase 4: community creation — 3 allowed, 4th blocked ───────────────────

async function communityCreationWindowBoundary() {
  const fixture = await prepareIsolatedWorkerTest('abuse-community-window');
  let running;
  try {
    migrate(fixture);
    const sessionSecret = 'synthetic-abuse-controls-session';
    const sessionHmac = createHmac('sha256', fixture.pepper).update(sessionSecret).digest('hex');
    execute(
      fixture,
      [
        "INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES ('com_seed','Seed','Asia/Tokyo',1,'2026-07-28T00:00:00.000Z')",
        "INSERT INTO users (id,created_at) VALUES ('usr_seed','2026-07-28T00:00:00.000Z')",
        "INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES ('mem_seed','com_seed','usr_seed','admin','Seed Admin','2026-07-28T00:00:00.000Z')",
        `INSERT INTO sessions (id,user_id,session_hmac,created_at,expires_at,last_seen_at) VALUES ('sess_seed','usr_seed','${sessionHmac}','2026-07-28T00:00:00.000Z','2099-12-31T23:59:59.000Z','2026-07-28T00:00:00.000Z')`,
      ].join(';'),
    );
    running = await start(fixture);
    const cookie = `ciao_sid=${sessionSecret}`;

    for (let attempt = 1; attempt <= 3; attempt += 1) {
      const page = await request(running.baseUrl, '/communities/new', { cookie });
      const token = extractToken(page.text);
      const result = await request(running.baseUrl, '/communities/new', {
        method: 'POST',
        cookie,
        form: {
          _token: token,
          community_name: `Community ${attempt}`,
          display_name: 'Seed Admin',
          timezone: 'Asia/Tokyo',
        },
      });
      assert.equal(result.response.status, 303, `attempt ${attempt} must create a community and redirect`);
    }

    const page = await request(running.baseUrl, '/communities/new', { cookie });
    const token = extractToken(page.text);
    const blocked = await request(running.baseUrl, '/communities/new', {
      method: 'POST',
      cookie,
      form: {
        _token: token,
        community_name: 'Community 4',
        display_name: 'Seed Admin',
        timezone: 'Asia/Tokyo',
      },
    });
    assert.equal(blocked.response.status, 429, '4th community creation must be blocked');
    assert.equal(
      scalar(fixture, 'SELECT COUNT(*) AS value FROM communities'),
      4, // com_seed + 3 created
      'the blocked 4th attempt must create no community',
    );

    await fixture.assertChildEnvironmentAudit();
  } finally {
    await stop(running);
    await fixture.cleanup();
  }
}

// ── Phase 5: concurrent bursts — no lost increments ─────────────────────────
//
// Phases 2-4 above prove the boundary with *sequential* requests, which
// cannot detect a lost-increment race (the original B3 defect: an unguarded
// read-then-write). This phase fires N requests at once via `Promise.all`
// and asserts the count of successes is *exact* — not "at least" or
// "at most" — since a race would let more than the policy limit through.
//
// Tokens are single-use and must be consumed before reservation, so they are
// pre-issued sequentially; only the submission itself is concurrent.

function countStatuses(results) {
  const counts = {};
  for (const { response } of results) {
    counts[response.status] = (counts[response.status] ?? 0) + 1;
  }
  return counts;
}

async function inviteConcurrencyBurst() {
  const fixture = await prepareIsolatedWorkerTest('abuse-invite-burst');
  let running;
  try {
    migrate(fixture);
    running = await start(fixture);

    const burstSize = 30;
    const tokens = [];
    for (let i = 0; i < burstSize; i += 1) {
      const page = await request(running.baseUrl, '/join');
      tokens.push(extractToken(page.text));
    }

    const results = await Promise.all(
      tokens.map((token) =>
        request(running.baseUrl, '/join', { method: 'POST', form: { code: 'ZZZZZZ', _token: token } }),
      ),
    );
    const counts = countStatuses(results);

    assert.equal(
      counts[200] ?? 0,
      10,
      `exactly 10 of ${burstSize} concurrent submissions must reach credential validation (no lost increments); got ${JSON.stringify(counts)}`,
    );
    assert.equal(counts[429] ?? 0, burstSize - 10, 'the remainder must be blocked');
    assert.equal(Object.keys(counts).length, 2, 'only 200 and 429 may appear');

    // Saturation check: the stored count is not directly observable (the
    // private protocol never returns it), so further sequential attempts
    // remaining blocked — never flipping back to 200 — is the externally
    // observable proof that the burst did not corrupt or reset the window.
    for (let i = 1; i <= 3; i += 1) {
      const page = await request(running.baseUrl, '/join');
      const token = extractToken(page.text);
      const result = await request(running.baseUrl, '/join', {
        method: 'POST',
        form: { code: 'ZZZZZZ', _token: token },
      });
      assert.equal(result.response.status, 429, `post-burst attempt ${i} must remain blocked`);
    }

    await fixture.assertChildEnvironmentAudit();
  } finally {
    await stop(running);
    await fixture.cleanup();
  }
}

async function communityCreationConcurrencyBurst() {
  const fixture = await prepareIsolatedWorkerTest('abuse-community-burst');
  let running;
  try {
    migrate(fixture);
    const sessionSecret = 'synthetic-abuse-burst-session';
    const sessionHmac = createHmac('sha256', fixture.pepper).update(sessionSecret).digest('hex');
    execute(
      fixture,
      [
        "INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES ('com_burst_seed','Seed','Asia/Tokyo',1,'2026-07-28T00:00:00.000Z')",
        "INSERT INTO users (id,created_at) VALUES ('usr_burst','2026-07-28T00:00:00.000Z')",
        "INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES ('mem_burst','com_burst_seed','usr_burst','admin','Burst Admin','2026-07-28T00:00:00.000Z')",
        `INSERT INTO sessions (id,user_id,session_hmac,created_at,expires_at,last_seen_at) VALUES ('sess_burst','usr_burst','${sessionHmac}','2026-07-28T00:00:00.000Z','2099-12-31T23:59:59.000Z','2026-07-28T00:00:00.000Z')`,
      ].join(';'),
    );
    running = await start(fixture);
    const cookie = `ciao_sid=${sessionSecret}`;

    const burstSize = 10;
    const tokens = [];
    for (let i = 0; i < burstSize; i += 1) {
      const page = await request(running.baseUrl, '/communities/new', { cookie });
      tokens.push(extractToken(page.text));
    }

    const results = await Promise.all(
      tokens.map((token, i) =>
        request(running.baseUrl, '/communities/new', {
          method: 'POST',
          cookie,
          form: {
            _token: token,
            community_name: `Burst ${i}`,
            display_name: 'Burst Admin',
            timezone: 'Asia/Tokyo',
          },
        }),
      ),
    );
    const counts = countStatuses(results);

    assert.equal(
      counts[303] ?? 0,
      3,
      `exactly 3 of ${burstSize} concurrent community-creation submissions must succeed (no lost increments); got ${JSON.stringify(counts)}`,
    );
    assert.equal(counts[429] ?? 0, burstSize - 3, 'the remainder must be blocked');
    assert.equal(Object.keys(counts).length, 2, 'only 303 and 429 may appear');
    assert.equal(
      scalar(fixture, 'SELECT COUNT(*) AS value FROM communities'),
      4, // com_burst_seed + exactly 3 created
      'the burst must create exactly 3 communities, never more',
    );

    await fixture.assertChildEnvironmentAudit();
  } finally {
    await stop(running);
    await fixture.cleanup();
  }
}

await missingBindingFailsClosed();
await inviteWindowBoundary();
await validRedemptionResetsWindow();
await communityCreationWindowBoundary();
await inviteConcurrencyBurst();
await communityCreationConcurrencyBurst();

console.log(
  JSON.stringify({
    ok: true,
    phases: {
      missingBindingFailsClosed: true,
      inviteWindowBoundary: true,
      validRedemptionResetsWindow: true,
      communityCreationWindowBoundary: true,
      inviteConcurrencyBurst: true,
      communityCreationConcurrencyBurst: true,
    },
  }),
);
