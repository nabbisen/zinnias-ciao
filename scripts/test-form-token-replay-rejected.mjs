#!/usr/bin/env node
// Regression test for the form-token replay-detection remediation
// (2026-07-28): proves a replayed single-use token is now correctly
// rejected for two non-idempotent destructive actions where the prior
// defect let a replay silently re-execute and report success again:
//   1. admin hide-note — hide-note's own SQL guard already prevented data
//      corruption, so this scenario proves the HTTP-observable behavior fix.
//   2. calendar-token regeneration (R-N1 from the remediation review) — this
//      action had NO independent fallback: a pre-fix replay minted a second
//      feed token and silently revoked the first, invalidating the URL the
//      admin had just been shown. This scenario proves the fix is
//      load-bearing on its own, not just a belt-and-suspenders HTTP check.
// See docs/src/tester/release-checklist.md's form-token gate and
// packages/contracts/tests/form_token_replay_detection.rs.
//
// Local-only: disposable isolated Worker + D1, torn down unconditionally.

import { createHmac } from 'node:crypto';
import { prepareIsolatedWorkerTest } from './lib/isolated-worker-test.mjs';

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function hiddenTokenForAction(html, actionSuffix) {
  const formIndex = html.indexOf(`action="${actionSuffix}`);
  assert(formIndex >= 0, `no form found with action starting "${actionSuffix}"`);
  const tail = html.slice(formIndex);
  const token = tail.match(/name="_token"\s+value="([^"]+)"/u)?.[1] ?? '';
  assert(/^[A-Za-z0-9_-]+$/u.test(token), 'response did not contain a bounded form token');
  return token;
}

async function request(baseUrl, path, { method = 'GET', form, cookie } = {}) {
  const headers = {};
  if (cookie) headers.Cookie = cookie;
  let body;
  if (form) {
    headers['Content-Type'] = 'application/x-www-form-urlencoded';
    body = new URLSearchParams(form);
  }
  const response = await fetch(`${baseUrl}${path}`, { method, headers, body, redirect: 'manual' });
  return { status: response.status, location: response.headers.get('location') ?? '' };
}

function sqlString(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}

function d1Args(fixture, extra) {
  return [
    'd1', ...extra, 'zinnias-ciao-dev', '--env', 'dev', '--local',
    '--persist-to', fixture.persistTo, '--config', fixture.configPath,
  ];
}

function queryRows(fixture, statement) {
  const result = fixture.runWranglerSync(
    d1Args(fixture, ['execute']).concat(['--yes', '--json', '--command', statement]),
    { encoding: 'utf8' },
  );
  const parsed = JSON.parse(result);
  return parsed?.[0]?.results ?? parsed?.results ?? [];
}

const fixture = await prepareIsolatedWorkerTest('form-token-replay-rejected');
let dev;

try {
  fixture.runWranglerSync(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);

  const now = '2026-07-28T00:00:00.000Z';
  const communityId = 'com_replay_regress';
  const adminUserId = 'usr_replay_regress_admin';
  const adminMembershipId = 'mem_replay_regress_admin';
  const adminSessionId = 'sess_replay_regress_admin';
  const adminSessionSecret = 'replay-regress-admin-session';
  const memberUserId = 'usr_replay_regress_member';
  const memberMembershipId = 'mem_replay_regress_member';
  const eventId = 'evt_replay_regress';
  const dayId = 'day_replay_regress';
  const adminSessionHmac = createHmac('sha256', fixture.pepper).update(adminSessionSecret).digest('hex');

  const args = (extra) => [
    'd1', 'execute', 'zinnias-ciao-dev', '--env', 'dev', '--local',
    '--persist-to', fixture.persistTo, '--config', fixture.configPath, ...extra,
  ];
  fixture.runWranglerSync(args(['--yes', '--command', [
    `INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES (${sqlString(communityId)},'Replay Regression','Asia/Tokyo',1,${sqlString(now)})`,
    `INSERT INTO users (id,created_at) VALUES (${sqlString(adminUserId)},${sqlString(now)})`,
    `INSERT INTO users (id,created_at) VALUES (${sqlString(memberUserId)},${sqlString(now)})`,
    `INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES (${sqlString(adminMembershipId)},${sqlString(communityId)},${sqlString(adminUserId)},'admin','Replay Regress Admin',${sqlString(now)})`,
    `INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES (${sqlString(memberMembershipId)},${sqlString(communityId)},${sqlString(memberUserId)},'member','Replay Regress Member',${sqlString(now)})`,
    `INSERT INTO sessions (id,user_id,session_hmac,created_at,expires_at,last_seen_at) VALUES (${sqlString(adminSessionId)},${sqlString(adminUserId)},${sqlString(adminSessionHmac)},${sqlString(now)},'2099-12-31T23:59:59.000Z',${sqlString(now)})`,
    `INSERT INTO events (id,community_id,created_by_membership_id,title,status,created_at,updated_at) VALUES (${sqlString(eventId)},${sqlString(communityId)},${sqlString(adminMembershipId)},'Replay Regression Event','scheduled',${sqlString(now)},${sqlString(now)})`,
    `INSERT INTO event_days (id,event_id,community_id,seq,day_date,starts_at_utc,ends_at_utc,created_at) VALUES (${sqlString(dayId)},${sqlString(eventId)},${sqlString(communityId)},1,'2026-08-03','2026-08-03T00:00:00.000Z','2026-08-03T01:30:00.000Z',${sqlString(now)})`,
    `INSERT INTO event_notes (id,event_id,membership_id,note,note_updated_at) VALUES ('note_replay_regress',${sqlString(eventId)},${sqlString(memberMembershipId)},'note to hide','${now}')`,
  ].join(';')]));

  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  dev = fixture.spawnDev(port);
  await waitUntilReady(baseUrl, dev);

  const adminCookie = `ciao_sid=${adminSessionSecret}`;
  const hidePath = `/c/${communityId}/admin/events/${eventId}/notes/${memberMembershipId}/hide`;

  const confirmResponse = await fetch(`${baseUrl}${hidePath}`, { headers: { Cookie: adminCookie } });
  const confirmHtml = await confirmResponse.text();
  const token = hiddenTokenForAction(confirmHtml, hidePath);

  const first = await request(baseUrl, hidePath, { method: 'POST', cookie: adminCookie, form: { _token: token } });
  assert(first.status === 303, `first submission must succeed (303), got ${first.status}`);
  assert(
    first.location.includes('flash=Note+removed'),
    `first submission must carry the success flash, got location "${first.location}"`,
  );

  const replay = await request(baseUrl, hidePath, { method: 'POST', cookie: adminCookie, form: { _token: token } });
  assert(replay.status === 303, `replayed submission must still redirect (303), got ${replay.status}`);
  assert(
    !replay.location.includes('flash=Note+removed'),
    `replayed submission must NOT report success again — this is the defect: got location "${replay.location}"`,
  );

  // -- R-N1: calendar-token regeneration — no independent fallback guard ----
  // Unlike hide-note, `cal_db::rotate_required` revokes the current active
  // token and inserts a brand-new one on every call that reaches it; there
  // is no `WHERE ... IS NULL`-style self-guard against a second rotation.
  // A pre-fix replay would silently mint a second token and invalidate the
  // first one the admin had just been shown.
  const calendarPath = `/c/${communityId}/me/calendar`;
  const regeneratePath = `${calendarPath}/regenerate`;

  const calendarPage = await fetch(`${baseUrl}${calendarPath}`, { headers: { Cookie: adminCookie } });
  const calendarHtml = await calendarPage.text();
  const regenerateToken = hiddenTokenForAction(calendarHtml, regeneratePath);

  const regenerateFirst = await request(baseUrl, regeneratePath, {
    method: 'POST', cookie: adminCookie, form: { _token: regenerateToken },
  });
  assert(regenerateFirst.status === 303, `first regenerate must succeed (303), got ${regenerateFirst.status}`);
  assert(
    regenerateFirst.location.includes('flash=generated'),
    `first regenerate must carry the success flash, got location "${regenerateFirst.location}"`,
  );

  const activeAfterFirst = queryRows(
    fixture,
    `SELECT id FROM calendar_tokens WHERE membership_id=${sqlString(adminMembershipId)} AND revoked_at IS NULL`,
  );
  assert(activeAfterFirst.length === 1, `expected exactly one active calendar token after first regenerate, got ${activeAfterFirst.length}`);
  const tokenIdAfterFirst = activeAfterFirst[0].id;

  const regenerateReplay = await request(baseUrl, regeneratePath, {
    method: 'POST', cookie: adminCookie, form: { _token: regenerateToken },
  });
  assert(regenerateReplay.status === 303, `replayed regenerate must still redirect (303), got ${regenerateReplay.status}`);
  assert(
    !regenerateReplay.location.includes('flash=generated'),
    `replayed regenerate must NOT report success again — this is the defect: got location "${regenerateReplay.location}"`,
  );

  const activeAfterReplay = queryRows(
    fixture,
    `SELECT id FROM calendar_tokens WHERE membership_id=${sqlString(adminMembershipId)} AND revoked_at IS NULL`,
  );
  assert(activeAfterReplay.length === 1, `expected exactly one active calendar token after the replay, got ${activeAfterReplay.length}`);
  assert(
    activeAfterReplay[0].id === tokenIdAfterFirst,
    `replayed regenerate must not mint a second token — the admin's already-shown URL would otherwise be silently invalidated`,
  );

  console.log(JSON.stringify({
    ok: true,
    hideNote: {
      firstSubmission: { status: first.status, reportedSuccess: true },
      replayedSubmission: { status: replay.status, reportedSuccess: false },
    },
    calendarTokenRegeneration: {
      firstSubmission: { status: regenerateFirst.status, reportedSuccess: true },
      replayedSubmission: { status: regenerateReplay.status, reportedSuccess: false },
      activeTokenUnchangedAcrossReplay: true,
    },
    regression: 'a replayed single-use token no longer re-executes or re-reports success for two non-idempotent actions, one of which (calendar-token regeneration) had no independent SQL-level fallback',
  }));
} finally {
  if (dev && dev.exitCode === null) {
    dev.kill('SIGTERM');
    await new Promise((accept) => dev.once('exit', accept));
  }
  await fixture.cleanup();
}

async function freePort() {
  const { createServer } = await import('node:net');
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

async function waitUntilReady(baseUrl, child) {
  for (let attempt = 0; attempt < 160; attempt += 1) {
    if (child.exitCode !== null) throw new Error('isolated Worker exited before readiness');
    try {
      const response = await fetch(`${baseUrl}/healthz`);
      if (response.status === 200) return;
    } catch {
      // still starting
    }
    await new Promise((accept) => setTimeout(accept, 250));
  }
  throw new Error('isolated Worker did not become ready');
}
