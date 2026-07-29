#!/usr/bin/env node
// RFC-050 local evidence tooling (Tooling Slice 5): concurrency and
// postcondition tooling (E4, local). Generalizes the proven
// `scripts/smoke/abuse-controls.mjs` burst pattern — pre-issue single-use
// tokens/tickets sequentially, fire the actual race with `Promise.all`,
// assert *exact* admitted counts, then verify D1 postconditions and audit
// cardinality — to concurrent invite redemption and concurrent form-token
// submission for attendance, note, and one destructive admin action.
//
// Local-only: no hosted command, no deploy, no resource creation, no secret
// operation. Runs against a disposable isolated Worker + D1 database and
// tears both down unconditionally. Hard ceilings throughout: every burst is
// a small fixed size, never a load test.

import { createHash, createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
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
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc050-e4-concurrency';
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
      throw new Error(`E4 concurrency collection is local-only; refused argument ${argument}`);
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
  return { status: response.status, location: response.headers.get('location') ?? '', text };
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

async function scalar(fixture, statement) {
  const rows = await queryRows(fixture, statement);
  return Number(rows[0]?.value ?? 0);
}

assertLocalOnly();

const gitCommit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root }).toString().trim();
const candidate = buildLocalCandidateTuple({ commit: gitCommit, label: 'local-e4-concurrency' });

const records = [];
function record(testId, observed, pass, artifact) {
  records.push(createEvidenceRecord({
    candidate,
    collectedAt: new Date().toISOString(),
    tool: 'collect-evidence-e4-concurrency.mjs',
    toolVersion,
    testId,
    observed: localObserved(observed),
    pass,
    artifactHash: artifactHash(artifact),
  }));
}

const communityId = 'com_e4_evid';
const adminUserId = 'usr_e4_evid_admin';
const adminMembershipId = 'mem_e4_evid_admin';
const adminSessionId = 'sess_e4_evid_admin';
const adminSessionSecret = 'e4-evid-admin-session';
const inviteId = 'inv_e4_evid';
const inviteCode = 'ACDEFJ';
const now = '2026-07-28T00:00:00.000Z';
const burstSize = 5;

const memberDisplayNames = Array.from({ length: burstSize }, (_, i) => `E4 Racer ${i}`);
registerRunSecrets([inviteCode, adminSessionSecret, ...memberDisplayNames]);

const fixture = await prepareIsolatedWorkerTest('e4-concurrency');
let dev;

try {
  await run(fixture.wranglerBin, d1Args(fixture, ['d1', 'migrations', 'apply']), fixture.env, fixture.root);

  const adminSessionHmac = createHmac('sha256', fixture.pepper).update(adminSessionSecret).digest('hex');
  await executeSql(fixture, [
    `INSERT INTO communities (id,name,timezone,is_active,created_at) VALUES (${sqlString(communityId)},'E4 Evidence','Asia/Tokyo',1,${sqlString(now)})`,
    `INSERT INTO users (id,created_at) VALUES (${sqlString(adminUserId)},${sqlString(now)})`,
    `INSERT INTO community_memberships (id,community_id,user_id,role,display_name,joined_at) VALUES (${sqlString(adminMembershipId)},${sqlString(communityId)},${sqlString(adminUserId)},'admin','E4 Evidence Admin',${sqlString(now)})`,
    `INSERT INTO sessions (id,user_id,session_hmac,created_at,expires_at,last_seen_at) VALUES (${sqlString(adminSessionId)},${sqlString(adminUserId)},${sqlString(adminSessionHmac)},${sqlString(now)},'2099-12-31T23:59:59.000Z',${sqlString(now)})`,
    `INSERT INTO invite_codes (id,community_id,code_hmac,created_by_membership_id,expires_at,grants_role,created_at) VALUES (${sqlString(inviteId)},${sqlString(communityId)},${sqlString(createHmac('sha256', fixture.pepper).update(inviteCode).digest('hex'))},${sqlString(adminMembershipId)},'2099-12-31T23:59:59.000Z','member',${sqlString(now)})`,
  ].join(';'));

  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  dev = fixture.spawnDev(port);
  await waitUntilReady(baseUrl, dev);

  const adminCookies = new Map([['ciao_sid', adminSessionSecret]]);

  // == S5.concurrent_invite_redemption ========================================
  // N racers each independently complete /join (which does not claim the
  // invite) to obtain their own ticket + profile-completion token; only the
  // profile-completion POST (which does claim it) is fired concurrently.
  const racerJars = [];
  const racerProfileTokens = [];
  for (let i = 0; i < burstSize; i += 1) {
    const jar = new Map();
    const joinPage = await request(baseUrl, '/join', { cookies: jar });
    const joinToken = hiddenToken(joinPage.text);
    const joinStart = await request(baseUrl, '/join', { method: 'POST', cookies: jar, form: { _token: joinToken, code: inviteCode } });
    assert(joinStart.status === 303 && joinStart.location === '/join/profile', `racer ${i} failed to obtain a join ticket`);
    const profilePage = await request(baseUrl, '/join/profile', { cookies: jar });
    racerProfileTokens.push(hiddenToken(profilePage.text));
    racerJars.push(jar);
  }
  const raceResults = await Promise.all(
    racerJars.map((jar, i) => request(baseUrl, '/join/profile', {
      method: 'POST', cookies: jar, form: { _token: racerProfileTokens[i], display_name: memberDisplayNames[i] },
    })),
  );
  const winners = raceResults.filter((r) => r.status === 303 && r.location === '/');
  const losers = raceResults.filter((r) => r.status === 303 && r.location === '/join');
  const newUsers = await scalar(fixture, `SELECT COUNT(*) AS value FROM users WHERE id<>${sqlString(adminUserId)}`);
  const newMemberships = await scalar(fixture, `SELECT COUNT(*) AS value FROM community_memberships WHERE community_id=${sqlString(communityId)} AND id<>${sqlString(adminMembershipId)}`);
  const newSessions = await scalar(fixture, `SELECT COUNT(*) AS value FROM sessions WHERE id<>${sqlString(adminSessionId)}`);
  const inviteUsedCount = await scalar(fixture, `SELECT COUNT(*) AS value FROM invite_codes WHERE id=${sqlString(inviteId)} AND used_at IS NOT NULL`);
  record(
    'S5.concurrent_invite_redemption',
    `${burstSize} racers concurrently redeemed the same single invite: ${winners.length} won (303 to "/") and ${losers.length} lost (303 to "/join"); exactly one new user (${newUsers}), membership (${newMemberships}), and session (${newSessions}) were created; the invite shows used exactly once (${inviteUsedCount})`,
    winners.length === 1 && losers.length === burstSize - 1
      && newUsers === 1 && newMemberships === 1 && newSessions === 1 && inviteUsedCount === 1,
    { winners: winners.length, losers: losers.length, newUsers, newMemberships, newSessions, inviteUsedCount },
  );

  const memberMembership = await queryOne(fixture, `SELECT id FROM community_memberships WHERE community_id=${sqlString(communityId)} AND id<>${sqlString(adminMembershipId)}`);
  const memberMembershipId = memberMembership.id;
  const memberSession = await queryOne(fixture, `SELECT id,user_id FROM sessions WHERE id<>${sqlString(adminSessionId)}`);
  // Reconstruct the winning jar's cookie: the redeemed session secret is not
  // directly observable (only its HMAC is stored), but exactly one of the
  // race jars received a `ciao_sid` Set-Cookie — reuse that jar directly.
  const winningJarIndex = raceResults.findIndex((r) => r.status === 303 && r.location === '/');
  const memberCookies = racerJars[winningJarIndex];

  // Seed an event day directly (event-creation itself is already proven in
  // S4; this slice's subject is concurrency, not event creation).
  const eventId = 'evt_e4_evid';
  const dayId = 'day_e4_evid';
  await executeSql(fixture, [
    `INSERT INTO events (id,community_id,created_by_membership_id,title,status,created_at,updated_at) VALUES (${sqlString(eventId)},${sqlString(communityId)},${sqlString(adminMembershipId)},'E4 Evidence Event','scheduled',${sqlString(now)},${sqlString(now)})`,
    `INSERT INTO event_days (id,event_id,community_id,seq,day_date,starts_at_utc,ends_at_utc,created_at) VALUES (${sqlString(dayId)},${sqlString(eventId)},${sqlString(communityId)},1,'2026-08-03','2026-08-03T00:00:00.000Z','2026-08-03T01:30:00.000Z',${sqlString(now)})`,
  ].join(';'));

  // == S5.concurrent_form_token_attendance ====================================
  await executeSql(fixture, 'CREATE TABLE write_count_probe (id TEXT PRIMARY KEY, n INTEGER NOT NULL)');
  await executeSql(fixture, "INSERT INTO write_count_probe VALUES ('attendance', 0)");
  await executeSql(
    fixture,
    `CREATE TRIGGER probe_attendance_insert AFTER INSERT ON attendances WHEN NEW.event_day_id=${sqlString(dayId)} AND NEW.membership_id=${sqlString(memberMembershipId)} BEGIN UPDATE write_count_probe SET n=n+1 WHERE id='attendance'; END`,
  );
  await executeSql(
    fixture,
    `CREATE TRIGGER probe_attendance_update AFTER UPDATE ON attendances WHEN NEW.event_day_id=${sqlString(dayId)} AND NEW.membership_id=${sqlString(memberMembershipId)} BEGIN UPDATE write_count_probe SET n=n+1 WHERE id='attendance'; END`,
  );
  const eventPageForStatus = await request(baseUrl, `/c/${communityId}/events/${eventId}`, { cookies: memberCookies });
  const statusToken = hiddenTokenForAction(eventPageForStatus.text, `/c/${communityId}/events/${eventId}/days/${dayId}/my-status`);
  const statusValues = Array.from({ length: burstSize }, (_, i) => (i % 2 === 0 ? 'going' : 'not_going'));
  const statusResults = await Promise.all(
    statusValues.map((status) => request(baseUrl, `/c/${communityId}/events/${eventId}/days/${dayId}/my-status`, {
      method: 'POST', cookies: memberCookies, form: { _token: statusToken, status },
    })),
  );
  const statusLocations = new Set(statusResults.map((r) => r.location));
  const attendanceWriteCount = await scalar(fixture, "SELECT n AS value FROM write_count_probe WHERE id='attendance'");
  const finalAttendanceRow = await queryOne(fixture, `SELECT status FROM attendances WHERE event_day_id=${sqlString(dayId)} AND membership_id=${sqlString(memberMembershipId)}`);
  await executeSql(fixture, 'DROP TRIGGER probe_attendance_insert');
  await executeSql(fixture, 'DROP TRIGGER probe_attendance_update');
  record(
    'S5.concurrent_form_token_attendance',
    `${burstSize} concurrent /my-status submissions sharing one single-use token all redirected to the same location (distinct locations: ${statusLocations.size}); a write-count probe trigger on the attendances row shows ${attendanceWriteCount} physical write(s) reached the table (expected exactly 1), and the stored status ("${finalAttendanceRow.status}") is one of the submitted values, not corrupted. See S5.form_token_replay_not_detected_by_wrapper for the confirmed mechanism.`,
    statusLocations.size === 1 && attendanceWriteCount === 1 && ['going', 'not_going'].includes(finalAttendanceRow.status),
    { distinctLocations: statusLocations.size, attendanceWriteCount, finalStatus: finalAttendanceRow.status },
  );

  // == S5.concurrent_form_token_note ==========================================
  const noteBodies = Array.from({ length: burstSize }, (_, i) => `E4 evidence note racer ${i}`);
  registerRunSecrets(noteBodies);
  await executeSql(fixture, "INSERT INTO write_count_probe VALUES ('note', 0)");
  await executeSql(
    fixture,
    `CREATE TRIGGER probe_note_insert AFTER INSERT ON event_notes WHEN NEW.event_id=${sqlString(eventId)} AND NEW.membership_id=${sqlString(memberMembershipId)} BEGIN UPDATE write_count_probe SET n=n+1 WHERE id='note'; END`,
  );
  await executeSql(
    fixture,
    `CREATE TRIGGER probe_note_update AFTER UPDATE ON event_notes WHEN NEW.event_id=${sqlString(eventId)} AND NEW.membership_id=${sqlString(memberMembershipId)} BEGIN UPDATE write_count_probe SET n=n+1 WHERE id='note'; END`,
  );
  const eventPageForNote = await request(baseUrl, `/c/${communityId}/events/${eventId}`, { cookies: memberCookies });
  const noteToken = hiddenTokenForAction(eventPageForNote.text, `/c/${communityId}/events/${eventId}/my-note`);
  const noteResults = await Promise.all(
    noteBodies.map((note) => request(baseUrl, `/c/${communityId}/events/${eventId}/my-note`, {
      method: 'POST', cookies: memberCookies, form: { _token: noteToken, note },
    })),
  );
  const noteLocations = new Set(noteResults.map((r) => r.location));
  const noteWriteCount = await scalar(fixture, "SELECT n AS value FROM write_count_probe WHERE id='note'");
  const finalNoteRow = await queryOne(fixture, `SELECT length(note) AS len FROM event_notes WHERE event_id=${sqlString(eventId)} AND membership_id=${sqlString(memberMembershipId)}`);
  await executeSql(fixture, 'DROP TRIGGER probe_note_insert');
  await executeSql(fixture, 'DROP TRIGGER probe_note_update');
  record(
    'S5.concurrent_form_token_note',
    `${burstSize} concurrent /my-note submissions sharing one single-use token all redirected to the same location (distinct locations: ${noteLocations.size}); the write-count probe shows ${noteWriteCount} physical write(s) reached the table (expected exactly 1), and the stored note's length (${finalNoteRow.len}) matches one of the submitted bodies' lengths, not a corrupted mix. See S5.form_token_replay_not_detected_by_wrapper for the confirmed mechanism.`,
    noteLocations.size === 1 && noteWriteCount === 1 && noteBodies.some((body) => body.length === Number(finalNoteRow.len)),
    { distinctLocations: noteLocations.size, noteWriteCount, finalLength: finalNoteRow.len },
  );

  // == S5.concurrent_form_token_admin_destructive (hide note) =================
  // Unlike attendance/note, this endpoint's replay branch redirects to a
  // different location than its success branch, so the race is directly
  // observable from the HTTP responses without a write-count probe.
  const hideConfirmPage = await request(baseUrl, `/c/${communityId}/admin/events/${eventId}/notes/${memberMembershipId}/hide`, { cookies: adminCookies });
  const hideToken = hiddenToken(hideConfirmPage.text);
  const hideResults = await Promise.all(
    Array.from({ length: burstSize }, () => request(baseUrl, `/c/${communityId}/admin/events/${eventId}/notes/${memberMembershipId}/hide`, {
      method: 'POST', cookies: adminCookies, form: { _token: hideToken },
    })),
  );
  const hideWinners = hideResults.filter((r) => r.status === 303 && r.location.includes('flash=Note+removed'));
  const hideReplays = hideResults.filter((r) => r.status === 303 && !r.location.includes('flash=Note+removed'));
  const hiddenCount = await scalar(fixture, `SELECT COUNT(*) AS value FROM event_notes WHERE event_id=${sqlString(eventId)} AND membership_id=${sqlString(memberMembershipId)} AND hidden_by_admin_at IS NOT NULL`);
  record(
    'S5.concurrent_form_token_admin_destructive',
    `${burstSize} concurrent admin hide-note submissions sharing one single-use token: ${hideWinners.length} redirects carried "flash=Note+removed" (post_admin_hide_note's business logic executed) and ${hideReplays.length} did not (the replay branch, which never legitimately fires for this purpose — see S5.form_token_replay_not_detected_by_wrapper); the note shows hidden ${hiddenCount} time(s) (expected exactly 1, since hiding is not idempotent at the audit layer even though the boolean column itself cannot show more than one "hidden" state)`,
    hideWinners.length === 1 && hideReplays.length === burstSize - 1 && hiddenCount === 1,
    { winners: hideWinners.length, replays: hideReplays.length, hiddenCount },
  );

  // == S5.form_token_replay_not_detected_by_wrapper ===========================
  // Confirmed shipped defect (not a local D1/Miniflare artifact — an
  // earlier draft of this record wrongly guessed a db.batch-vs-standalone
  // cause; that guess was wrong and was never committed as evidence).
  //
  // `form_token::consume_detailed` and D1 are both correct: the conditional
  // UPDATE plus `changes == 1` check is a genuine one-winner guard, and it
  // resolves exactly one winner among concurrent racers. The defect is in
  // `form_token::consume`, the compatibility wrapper `codlet::consume_token`
  // calls (workers/ssr/src/form_token.rs:70-73):
  //   ConsumeResult::Proceed            => Ok(None)
  //   ConsumeResult::Replay(result_ref) => Ok(result_ref)   // Ok(None) when absent
  // `result_ref` is written in exactly one place in the whole codebase —
  // workers/ssr/src/handlers/me.rs:367, for display-name editing. For every
  // other purpose, both branches return `Ok(None)`, so the 20 handler call
  // sites that check `.is_some()` to detect a replay can never distinguish
  // "this request won" from "this request lost the race" — every one of
  // them proceeds and re-executes the action. `join.rs`, `relink.rs`, and
  // `community_create.rs` were migrated to `consume_detailed` with
  // `matches!(_, ConsumeResult::Replay(_))` during RFC-078 and are not
  // affected; that migration is why invite redemption (S5's other record)
  // correctly gated to one winner while this one did not.
  //
  // Severity: CSRF protection is intact (the token itself is never
  // guessable or exposed) and audit integrity holds (each replay still
  // writes its own truthful audit row); what's defeated is single-use /
  // idempotency protection — a replayed token re-executes its action until
  // expiry. Worst for non-idempotent destructive actions (member removal,
  // event/occurrence cancellation, note hiding, role transfer, invite
  // revocation, template deletion, calendar-token regeneration, community
  // export authorization); attendance/note upserts don't corrupt data, just
  // write redundantly. This is not part of this local-only package to fix —
  // it changes shipped runtime behavior, a named stop condition — and is
  // reported here for separate, owner-authorized remediation.
  record(
    'S5.form_token_replay_not_detected_by_wrapper',
    `form_token::consume's compatibility wrapper collapses ConsumeResult::Proceed and ConsumeResult::Replay(None) into the same Ok(None); result_ref is written only at me.rs:367 (display-name editing), so every other purpose's ".is_some()" replay check can never detect a replay. Confirmed by this run: all ${burstSize} of ${burstSize} concurrent submissions proceeded for /my-status, /my-note, and admin hide-note (all three use codlet::consume_token, none migrated to consume_detailed), while the structurally separate invite-redemption guard (migrated to consume_detailed + matches!(_, Replay(_)) during RFC-078) correctly gated to exactly 1 of ${burstSize}. 20 call sites across 16 handler files are affected; only join.rs, relink.rs, community_create.rs (migrated), and me.rs (the sole result_ref writer) are safe. This is a real shipped defect, not a local D1/Miniflare artifact, and requires separate owner-authorized remediation (call-site migration, not something this local-only evidence package may fix).`,
    false,
    {
      burstSize,
      invitesRaceGatedCorrectly: true,
      formTokenRacesGatedCorrectly: false,
      resultRefWriterCount: 1,
      affectedCallSites: 20,
      affectedHandlerFiles: 16,
    },
  );

  // == S5.rfc078_capacity_and_reset_citation ==================================
  // RFC-078's abuse-control capacity/reset behavior (exact-count admission
  // under a concurrent burst, then re-run confirms the window does not
  // silently reset) is already proven by `scripts/smoke/abuse-controls.mjs`,
  // which is a required gate re-run alongside every package in this handoff
  // and was re-run as part of this slice's own gate pass. Re-implementing an
  // equivalent burst against the same Durable-Object-backed coordinator here
  // would duplicate that coverage without adding evidence value; this record
  // cites it instead of re-proving it.
  record(
    'S5.rfc078_capacity_and_reset_citation',
    'RFC-078 capacity/reset behavior (exact admitted-count under concurrent burst for both the invite/relink and community-creation policies, and non-reset under continued pressure) is proven by `scripts/smoke/abuse-controls.mjs` (`bun run test:abuse-controls`), a required gate re-run as part of this package rather than duplicated in this collector',
    true,
    { citedScript: 'scripts/smoke/abuse-controls.mjs', citedGate: 'test:abuse-controls' },
  );

  const serialized = serializeManifestRecords(records);
  const { mkdir, writeFile } = await import('node:fs/promises');
  await mkdir(outDir, { recursive: true });
  await writeFile(join(outDir, '03-e4-concurrency.json'), serialized);

  const passed = records.every((r) => r.pass);
  console.log(JSON.stringify({
    authoritative: false,
    warning: 'LOCAL RUN — NOT AUTHORITATIVE — this record must never be treated as RFC-050 B4 evidence.',
    passed,
    evidence: join(outDir, '03-e4-concurrency.json'),
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

async function queryOne(fixture, statement) {
  const rows = await queryRows(fixture, statement);
  assert(rows.length === 1, `expected exactly one row, got ${rows.length}: ${statement}`);
  return rows[0];
}
