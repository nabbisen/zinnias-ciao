#!/usr/bin/env node
// RFC-050 local evidence tooling (Tooling Slice 4): bounded local collection
// over synthetic fixtures for the E3 authenticated/browser flows — join,
// logout, relink/help-signin, attendance, note, community switch, admin
// flows, authorization-denied checks, the Asia/Tokyo 09:00-10:30 round-trip
// and its 13:00 edit, ICS DTSTART/DTEND, and community-creation flag
// behavior.
//
// Local-only: no hosted command, no deploy, no resource creation, no secret
// operation. Runs against a disposable isolated Worker + D1 database and
// tears both down unconditionally. Records outcome counts and campaign-local
// aliases only — never row dumps. Every record is explicitly self-labeled
// non-authoritative (S3's convention): this is local evidence, not RFC-050
// B4 evidence, until the same collection runs against a frozen hosted
// candidate.

import { execFileSync, spawn } from 'node:child_process';
import { createHash, createHmac } from 'node:crypto';
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
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc050-e3-flows';
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
      throw new Error(`E3 flow collection is local-only; refused argument ${argument}`);
    }
  }
}

function sqlString(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}

function artifactHash(value) {
  return `sha256:${createHash('sha256').update(JSON.stringify(value)).digest('hex')}`;
}

function run(command, args, env, cwd) {
  return new Promise((accept, reject) => {
    const child = spawn(command, args, { cwd, env, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.once('error', reject);
    child.once('exit', (code) => {
      if (code === 0) accept({ stdout, stderr });
      else reject(new Error(`${command} exited ${code}; local command failed: ${stderr.slice(-800)}`));
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

function hiddenToken(html) {
  const token = html.match(/name="_token"\s+value="([^"]+)"/u)?.[1] ?? '';
  assert(/^[A-Za-z0-9_-]+$/u.test(token), 'response did not contain a bounded form token');
  return token;
}

// Some pages (event detail: status + note forms; calendar: revoke + regenerate
// forms) render more than one `_token` field. `hiddenToken` grabs whichever
// comes first in the HTML, which is only correct for the first form on the
// page. This targets the specific `<form action="...actionSuffix">` block.
function hiddenTokenForAction(html, actionSuffix) {
  const formIndex = html.indexOf(`action="${actionSuffix}`);
  assert(formIndex >= 0, `no form found with action starting "${actionSuffix}"`);
  return hiddenToken(html.slice(formIndex));
}

function absorbCookies(headers, jar) {
  const values = typeof headers.getSetCookie === 'function'
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
  const response = await fetch(`${baseUrl}${path}`, { method, headers, body, redirect: 'manual' });
  const text = await response.text();
  if (cookies) absorbCookies(response.headers, cookies);
  return {
    status: response.status,
    location: response.headers.get('location') ?? '',
    contentType: response.headers.get('content-type') ?? '',
    text,
  };
}

function d1Args(fixture, extra) {
  return [...extra, 'zinnias-ciao-dev', '--env', 'dev', '--local', '--persist-to', fixture.persistTo, '--config', fixture.configPath];
}

async function executeSql(fixture, statement) {
  await run(fixture.wranglerBin, d1Args(fixture, ['d1', 'execute']).concat(['--yes', '--command', statement]), fixture.env, fixture.root);
}

async function queryRows(fixture, statement) {
  const result = await run(fixture.wranglerBin, d1Args(fixture, ['d1', 'execute']).concat(['--yes', '--json', '--command', statement]), fixture.env, fixture.root);
  const parsed = JSON.parse(result.stdout);
  return parsed?.[0]?.results ?? parsed?.results ?? [];
}

async function queryOne(fixture, statement) {
  const rows = await queryRows(fixture, statement);
  assert(rows.length === 1, `expected exactly one row, got ${rows.length}: ${statement}`);
  return rows[0];
}

assertLocalOnly();

const gitCommit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root }).toString().trim();
const candidate = buildLocalCandidateTuple({ commit: gitCommit, label: 'local-e3-collection' });

const records = [];
function record(testId, observed, pass, artifact) {
  records.push(createEvidenceRecord({
    candidate,
    collectedAt: new Date().toISOString(),
    tool: 'collect-evidence-e3-flows.mjs',
    toolVersion,
    testId,
    observed: localObserved(observed),
    pass,
    artifactHash: artifactHash(artifact),
  }));
}

const communityId = 'com_e3_evid';
const adminUserId = 'usr_e3_evid_admin';
const adminMembershipId = 'mem_e3_evid_admin';
const adminSessionId = 'sess_e3_evid_admin';
const adminSessionSecret = 'e3-evid-admin-session';
const inviteId = 'inv_e3_evid';
const inviteCode = 'ACDEFH';
const memberDisplayName = 'E3 Evidence Member';
const eventTitle = 'E3 Evidence Gathering';
const eventLocation = 'E3 Evidence Hall';
const eventDescription = 'Synthetic event for RFC-050 local E3 evidence collection.';
const noteBody = 'E3 evidence note body.';
const newCommunityName = 'E3 Evidence Second Community';
const newCommunityAdminDisplayName = 'E3 Evidence Second Admin';
const now = '2026-07-28T00:00:00.000Z';

registerRunSecrets([
  inviteCode, memberDisplayName, eventTitle, eventLocation, eventDescription,
  noteBody, newCommunityName, newCommunityAdminDisplayName, adminSessionSecret,
]);

const fixture = await prepareIsolatedWorkerTest('e3-flows');
let dev;

try {
  await run(fixture.wranglerBin, d1Args(fixture, ['d1', 'migrations', 'apply']), fixture.env, fixture.root);

  const adminSessionHmac = createHmac('sha256', fixture.pepper).update(adminSessionSecret).digest('hex');
  await executeSql(fixture, [
    `INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES (${sqlString(communityId)},'E3 Evidence','Asia/Tokyo',1,${sqlString(now)})`,
    `INSERT INTO users (id,created_at) VALUES (${sqlString(adminUserId)},${sqlString(now)})`,
    `INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES (${sqlString(adminMembershipId)},${sqlString(communityId)},${sqlString(adminUserId)},'admin','E3 Evidence Admin',${sqlString(now)})`,
    `INSERT INTO sessions (id,user_id,session_hmac,created_at,expires_at,last_seen_at) VALUES (${sqlString(adminSessionId)},${sqlString(adminUserId)},${sqlString(adminSessionHmac)},${sqlString(now)},'2099-12-31T23:59:59.000Z',${sqlString(now)})`,
    `INSERT INTO invite_codes (id,community_id,code_hmac,created_by_membership_id,expires_at,grants_role,created_at) VALUES (${sqlString(inviteId)},${sqlString(communityId)},${sqlString(createHmac('sha256', fixture.pepper).update(inviteCode).digest('hex'))},${sqlString(adminMembershipId)},'2099-12-31T23:59:59.000Z','member',${sqlString(now)})`,
  ].join(';'));

  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  dev = fixture.spawnDev(port);
  await waitUntilReady(baseUrl, dev);

  const adminCookies = new Map([['ciao_sid', adminSessionSecret]]);

  // -- S4.join --------------------------------------------------------------
  const memberCookies = new Map();
  const joinPage = await request(baseUrl, '/join', { cookies: memberCookies });
  const joinToken = hiddenToken(joinPage.text);
  const joinStart = await request(baseUrl, '/join', {
    method: 'POST', cookies: memberCookies, form: { _token: joinToken, code: inviteCode },
  });
  const joinProfilePage = await request(baseUrl, '/join/profile', { cookies: memberCookies });
  const profileToken = hiddenToken(joinProfilePage.text);
  const profileStart = await request(baseUrl, '/join/profile', {
    method: 'POST', cookies: memberCookies, form: { _token: profileToken, display_name: memberDisplayName },
  });
  const joinPassed = joinStart.status === 303 && joinStart.location === '/join/profile'
    && profileStart.status === 303 && profileStart.location === '/' && memberCookies.has('ciao_sid');
  record(
    'S4.join',
    `invite redemption reached /join/profile (status ${joinStart.status}) and profile completion issued a session (status ${profileStart.status}, session cookie present: ${memberCookies.has('ciao_sid')})`,
    joinPassed,
    { joinStatus: joinStart.status, joinLocation: joinStart.location, profileStatus: profileStart.status, profileLocation: profileStart.location },
  );

  const memberMembership = await queryOne(fixture, `SELECT id FROM community_memberships WHERE community_id=${sqlString(communityId)} AND id<>${sqlString(adminMembershipId)}`);
  const memberMembershipId = memberMembership.id;

  // -- S4.event_create_and_edit_asia_tokyo -----------------------------------
  const newEventPage = await request(baseUrl, `/c/${communityId}/admin/events/new`, { cookies: adminCookies });
  const newEventToken = hiddenToken(newEventPage.text);
  const createResult = await request(baseUrl, `/c/${communityId}/admin/events`, {
    method: 'POST',
    cookies: adminCookies,
    form: {
      _token: newEventToken, title: eventTitle, location: eventLocation, description: eventDescription,
      day_date: '2026-08-03', starts_at: '09:00', ends_at: '10:30',
    },
  });
  const createdEventId = createResult.location.match(/\/events\/([^/?]+)/u)?.[1] ?? '';
  const createdDay = createdEventId
    ? await queryOne(fixture, `SELECT id,starts_at_utc,ends_at_utc FROM event_days WHERE event_id=${sqlString(createdEventId)}`)
    : { starts_at_utc: '', ends_at_utc: '' };
  const createUtcCorrect = createdDay.starts_at_utc === '2026-08-03T00:00:00.000Z' && createdDay.ends_at_utc === '2026-08-03T01:30:00.000Z';

  const editPage = await request(baseUrl, `/c/${communityId}/admin/events/${createdEventId}/edit`, { cookies: adminCookies });
  const editToken = hiddenToken(editPage.text);
  const editResult = await request(baseUrl, `/c/${communityId}/admin/events/${createdEventId}/edit`, {
    method: 'POST',
    cookies: adminCookies,
    form: {
      _token: editToken, title: eventTitle, location: eventLocation, description: eventDescription,
      day_date: '2026-08-03', starts_at: '13:00', ends_at: '14:30',
    },
  });
  const editedDay = await queryOne(fixture, `SELECT starts_at_utc,ends_at_utc FROM event_days WHERE id=${sqlString(createdDay.id)}`);
  const editUtcCorrect = editedDay.starts_at_utc === '2026-08-03T04:00:00.000Z' && editedDay.ends_at_utc === '2026-08-03T05:30:00.000Z';
  record(
    'S4.event_create_and_edit_asia_tokyo',
    `created a 09:00-10:30 Asia/Tokyo event day (converted UTC correct: ${createUtcCorrect}) and edited it to 13:00-14:30 (converted UTC correct: ${editUtcCorrect}), both via 303 redirects (create ${createResult.status}, edit ${editResult.status})`,
    createResult.status === 303 && editResult.status === 303 && createUtcCorrect && editUtcCorrect,
    {
      createStatus: createResult.status, editStatus: editResult.status,
      createdStartsAtUtc: createdDay.starts_at_utc, createdEndsAtUtc: createdDay.ends_at_utc,
      editedStartsAtUtc: editedDay.starts_at_utc, editedEndsAtUtc: editedDay.ends_at_utc,
    },
  );

  // -- S4.attendance (member self-status, then admin override) --------------
  const eventPageForStatus = await request(baseUrl, `/c/${communityId}/events/${createdEventId}`, { cookies: memberCookies });
  const statusToken = hiddenTokenForAction(eventPageForStatus.text, `/c/${communityId}/events/${createdEventId}/days/${createdDay.id}/my-status`);
  const myStatusResult = await request(baseUrl, `/c/${communityId}/events/${createdEventId}/days/${createdDay.id}/my-status`, {
    method: 'POST', cookies: memberCookies, form: { _token: statusToken, status: 'going' },
  });
  const selfStatusRow = await queryOne(fixture, `SELECT status FROM attendances WHERE event_day_id=${sqlString(createdDay.id)} AND membership_id=${sqlString(memberMembershipId)}`);

  const attendanceMatrixPage = await request(baseUrl, `/c/${communityId}/admin/events/${createdEventId}/attendance`, { cookies: adminCookies });
  const attendanceToken = hiddenToken(attendanceMatrixPage.text);
  const attendanceField = `att_${createdDay.id}_${memberMembershipId}`;
  const adminOverrideResult = await request(baseUrl, `/c/${communityId}/admin/events/${createdEventId}/attendance`, {
    method: 'POST', cookies: adminCookies, form: { _token: attendanceToken, [attendanceField]: 'attended' },
  });
  const overriddenStatusRow = await queryOne(fixture, `SELECT status FROM attendances WHERE event_day_id=${sqlString(createdDay.id)} AND membership_id=${sqlString(memberMembershipId)}`);
  record(
    'S4.attendance',
    `member self-status set "going" (status ${myStatusResult.status}, stored "${selfStatusRow.status}") and admin override matrix set "attended" (status ${adminOverrideResult.status}, stored "${overriddenStatusRow.status}")`,
    myStatusResult.status === 303 && selfStatusRow.status === 'going'
      && adminOverrideResult.status === 303 && overriddenStatusRow.status === 'attended',
    { myStatusStatus: myStatusResult.status, storedAfterSelf: selfStatusRow.status, overrideStatus: adminOverrideResult.status, storedAfterOverride: overriddenStatusRow.status },
  );

  // -- S4.note ----------------------------------------------------------------
  const eventPageForNote = await request(baseUrl, `/c/${communityId}/events/${createdEventId}`, { cookies: memberCookies });
  const noteToken = hiddenTokenForAction(eventPageForNote.text, `/c/${communityId}/events/${createdEventId}/my-note`);
  const noteResult = await request(baseUrl, `/c/${communityId}/events/${createdEventId}/my-note`, {
    method: 'POST', cookies: memberCookies, form: { _token: noteToken, note: noteBody },
  });
  const noteRow = await queryOne(fixture, `SELECT length(note) AS len FROM event_notes WHERE event_id=${sqlString(createdEventId)} AND membership_id=${sqlString(memberMembershipId)}`);
  record(
    'S4.note',
    `note creation redirected with status ${noteResult.status} and stored a note of the expected length (${noteRow.len} characters)`,
    noteResult.status === 303 && Number(noteRow.len) === noteBody.length,
    { noteStatus: noteResult.status, storedLength: noteRow.len, expectedLength: noteBody.length },
  );

  // -- S4.ics_feed_dtstart_dtend ----------------------------------------------
  const calendarPage = await request(baseUrl, `/c/${communityId}/me/calendar`, { cookies: memberCookies });
  const regenerateToken = hiddenTokenForAction(calendarPage.text, `/c/${communityId}/me/calendar/regenerate`);
  const regenerateResult = await request(baseUrl, `/c/${communityId}/me/calendar/regenerate`, {
    method: 'POST', cookies: memberCookies, form: { _token: regenerateToken },
  });
  const calendarPageAfter = await request(baseUrl, `/c/${communityId}/me/calendar`, { cookies: memberCookies });
  const feedPath = calendarPageAfter.text.match(new RegExp(`(/c/${communityId}/cal/[A-Za-z0-9]+)`, 'u'))?.[1] ?? '';
  const icsResult = feedPath ? await request(baseUrl, feedPath) : { status: 0, contentType: '', text: '' };
  const dtstart = icsResult.text.match(/DTSTART:(\d{8}T\d{6}Z)/u)?.[1] ?? '';
  const dtend = icsResult.text.match(/DTEND:(\d{8}T\d{6}Z)/u)?.[1] ?? '';
  const expectedDtstart = editedDay.starts_at_utc.replace(/[-:]/gu, '').replace('.000Z', 'Z');
  const expectedDtend = editedDay.ends_at_utc.replace(/[-:]/gu, '').replace('.000Z', 'Z');
  record(
    'S4.ics_feed_dtstart_dtend',
    `regenerated the calendar feed (status ${regenerateResult.status}), fetched it unauthenticated (status ${icsResult.status}, content-type "${icsResult.contentType}"), and found DTSTART/DTEND matching the edited event's stored UTC times`,
    regenerateResult.status === 303 && icsResult.status === 200
      && icsResult.contentType.includes('text/calendar')
      && dtstart === expectedDtstart && dtend === expectedDtend,
    { regenerateStatus: regenerateResult.status, icsStatus: icsResult.status, contentType: icsResult.contentType, dtstart, dtend, expectedDtstart, expectedDtend },
  );

  // -- S4.community_creation_flag_enabled + S4.community_switch --------------
  const newCommunityPage = await request(baseUrl, '/communities/new', { cookies: adminCookies });
  const newCommunityToken = hiddenToken(newCommunityPage.text);
  const newCommunityResult = await request(baseUrl, '/communities/new', {
    method: 'POST',
    cookies: adminCookies,
    form: { _token: newCommunityToken, community_name: newCommunityName, display_name: newCommunityAdminDisplayName, timezone: 'Asia/Tokyo' },
  });
  const newCommunityId = newCommunityResult.location.match(/\/c\/([^/]+)\/home/u)?.[1] ?? '';
  record(
    'S4.community_creation_flag_enabled',
    `with COMMUNITY_CREATION_ENABLED=true (the isolated harness's default), the create page returned 200 and submission redirected to the new community's home (status ${newCommunityResult.status}, new community created: ${Boolean(newCommunityId)}). The disabled-flag path is not covered by this harness — see known limitations.`,
    newCommunityPage.status === 200 && newCommunityResult.status === 303 && Boolean(newCommunityId),
    { createPageStatus: newCommunityPage.status, createResultStatus: newCommunityResult.status, newCommunityCreated: Boolean(newCommunityId) },
  );

  const switchToAdminMembers = await request(baseUrl, `/switch?community=${communityId}&next=admin_members`, { cookies: adminCookies });
  const switchToNewHome = await request(baseUrl, `/switch?community=${newCommunityId}`, { cookies: adminCookies });
  record(
    'S4.community_switch',
    `switching with next=admin_members redirected toward the admin members route (status ${switchToAdminMembers.status}, location "${switchToAdminMembers.location}") and switching to the second community with no next redirected to its home (status ${switchToNewHome.status}, location "${switchToNewHome.location}")`,
    switchToAdminMembers.status === 303 && switchToAdminMembers.location.includes('admin/members')
      && switchToNewHome.status === 303 && switchToNewHome.location === `/c/${newCommunityId}/home`,
    { switchToAdminMembersStatus: switchToAdminMembers.status, switchToAdminMembersLocation: switchToAdminMembers.location, switchToNewHomeStatus: switchToNewHome.status, switchToNewHomeLocation: switchToNewHome.location },
  );

  // -- S4.authorization_denied_checks -----------------------------------------
  // Runs while `memberCookies` is still the member's own valid session —
  // both the help-signin/relink redemption below and logout revoke it.
  const memberOnAdminRoute = await request(baseUrl, `/c/${communityId}/admin/members`, { cookies: memberCookies });
  const loggedOutOnProtectedRoute = await request(baseUrl, `/c/${communityId}/me`, { cookies: new Map() });
  record(
    'S4.authorization_denied_checks',
    `a non-admin member requesting an admin-only route received status ${memberOnAdminRoute.status} (expected 404, indistinguishable from not-found) and a logged-out request to a protected route received status ${loggedOutOnProtectedRoute.status} (expected 401)`,
    memberOnAdminRoute.status === 404 && loggedOutOnProtectedRoute.status === 401,
    { memberOnAdminRouteStatus: memberOnAdminRoute.status, loggedOutOnProtectedRouteStatus: loggedOutOnProtectedRoute.status },
  );

  // -- S4.logout ----------------------------------------------------------------
  const mePage = await request(baseUrl, `/c/${communityId}/me`, { cookies: memberCookies });
  const logoutToken = hiddenTokenForAction(mePage.text, '/logout');
  const logoutResult = await request(baseUrl, '/logout', { method: 'POST', cookies: memberCookies, form: { _token: logoutToken } });
  const meAfterLogout = await request(baseUrl, `/c/${communityId}/me`, { cookies: memberCookies });
  record(
    'S4.logout',
    `logout redirected to /join (status ${logoutResult.status}, location "${logoutResult.location}") and the same cookie jar was rejected on a subsequent protected request (status ${meAfterLogout.status})`,
    logoutResult.status === 303 && logoutResult.location === '/join' && meAfterLogout.status === 401,
    { logoutStatus: logoutResult.status, logoutLocation: logoutResult.location, meAfterLogoutStatus: meAfterLogout.status },
  );

  // -- S4.help_signin_relink ---------------------------------------------------
  // Runs after logout: this is the realistic RFC-024 scenario (member has no
  // valid session; an admin mints a one-time relink code) and it exercises
  // the DB fact (`redeem_required`'s `revoke_others`) that redemption revokes
  // every other active session for the same underlying user — which would
  // otherwise silently invalidate `memberCookies` out from under the checks
  // above if this ran first.
  const helpSigninPage = await request(baseUrl, `/c/${communityId}/admin/members/${memberMembershipId}/help-signin`, { cookies: adminCookies });
  const helpSigninToken = hiddenToken(helpSigninPage.text);
  const helpSigninResult = await request(baseUrl, `/c/${communityId}/admin/members/${memberMembershipId}/help-signin`, {
    method: 'POST', cookies: adminCookies, form: { _token: helpSigninToken },
  });
  const relinkCode = helpSigninResult.text.match(/data-copy-code-value="true">([^<]+)</u)?.[1] ?? '';
  registerRunSecrets([relinkCode].filter(Boolean));
  const relinkCookies = new Map();
  const relinkPage = await request(baseUrl, '/relink', { cookies: relinkCookies });
  const relinkToken = hiddenToken(relinkPage.text);
  const relinkResult = await request(baseUrl, '/relink', {
    method: 'POST', cookies: relinkCookies, form: { _token: relinkToken, code: relinkCode },
  });
  record(
    'S4.help_signin_relink',
    `admin-generated help-signin code (status ${helpSigninResult.status}, code produced: ${Boolean(relinkCode)}) was redeemed via /relink (status ${relinkResult.status}, session cookie present: ${relinkCookies.has('ciao_sid')})`,
    helpSigninResult.status === 200 && Boolean(relinkCode) && relinkResult.status === 303 && relinkResult.location === '/' && relinkCookies.has('ciao_sid'),
    { helpSigninStatus: helpSigninResult.status, codeProduced: Boolean(relinkCode), relinkStatus: relinkResult.status, relinkLocation: relinkResult.location },
  );

  const serialized = serializeManifestRecords(records);
  const { mkdir, writeFile } = await import('node:fs/promises');
  await mkdir(outDir, { recursive: true });
  await writeFile(join(outDir, '02-e3-flows.json'), serialized);

  const passed = records.every((r) => r.pass);
  console.log(JSON.stringify({
    authoritative: false,
    warning: 'LOCAL RUN — NOT AUTHORITATIVE — this record must never be treated as RFC-050 B4 evidence.',
    passed,
    evidence: join(outDir, '02-e3-flows.json'),
    results: records.map((r) => ({ testId: r.testId, pass: r.pass })),
  }));
  if (!passed) process.exitCode = 1;
} finally {
  if (dev && dev.exitCode === null) {
    dev.kill('SIGTERM');
    await new Promise((accept) => dev.once('exit', accept));
  }
  clearRegisteredRunSecrets();
  await fixture.cleanup();
}
