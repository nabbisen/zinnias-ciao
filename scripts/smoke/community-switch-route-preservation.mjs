#!/usr/bin/env node
// Scenario smoke for RFC-074 community switch route preservation. Local wrangler dev only.

import { prepareIsolatedWorkerTest } from "../lib/isolated-worker-test.mjs";
import { SMOKE_ACCEPT_LANGUAGE } from "../lib/smoke-locale.mjs";
import { attachCspViolationCapture, readCspViolations } from "../lib/csp-violation-capture.mjs";

import { createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8800);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9252);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc074';
const reportName = process.env.REPORT_NAME ?? 'rfc074-community-switch-route-preservation-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-rfc074-switch-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const isolated = await prepareIsolatedWorkerTest("community-switch-route-preservation");
const pepper = isolated.pepper;
const now = '2026-07-10T00:00:00.000Z';

// A single test user, admin in two communities and a plain member in a
// third — the realistic "switch communities" shape this RFC is about, and
// enough to prove both the positive (target-admin) and negative
// (target-member-only, falls back to Home) cases for each admin-gated
// family without a separate outsider account.
const primaryCommunityId = 'com_rfc074_primary'; // admin
const adminSecondCommunityId = 'com_rfc074_admin2'; // admin
const memberOnlyCommunityId = 'com_rfc074_member'; // member only
const userId = 'usr_rfc074_user';
const primaryMembershipId = 'mem_rfc074_primary';
const adminSecondMembershipId = 'mem_rfc074_admin2';
const memberOnlyMembershipId = 'mem_rfc074_memberonly';
const sessionSecret = 'rfc074-smoke-session';
const sessionHmac = hmac(sessionSecret);
const eventId = 'evt_rfc074_detail';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function hmac(value) {
  return createHmac('sha256', pepper).update(value).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`RFC-074 smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`RFC-074 smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) throw new Error('RFC-074 smoke refuses remote D1 operations');
  try {
    return isolated.runWranglerSync(args, {
      cwd: process.cwd(),
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf8',
    });
  } catch (error) {
    throw new Error(`wrangler ${args.join(' ')} failed\n${error.stderr?.toString() ?? ''}`);
  }
}

function sql(statement) {
  runWrangler([
    'd1',
    'execute',
    'zinnias-ciao-dev',
    '--local',
    '--env',
    'dev',
    '--command',
    statement,
  ]);
}

function esc(value) {
  return String(value).replaceAll("'", "''");
}

function clean() {
  const communities = `'${primaryCommunityId}','${adminSecondCommunityId}','${memberOnlyCommunityId}'`;
  sql(`DELETE FROM attendances WHERE event_day_id IN (SELECT id FROM event_days WHERE community_id IN (${communities}))`);
  sql(`DELETE FROM event_notes WHERE event_id IN (SELECT id FROM events WHERE community_id IN (${communities}))`);
  sql(`DELETE FROM event_days WHERE community_id IN (${communities})`);
  sql(`DELETE FROM event_series WHERE community_id IN (${communities})`);
  sql(`DELETE FROM events WHERE community_id IN (${communities})`);
  sql(`DELETE FROM audit_log WHERE community_id IN (${communities})`);
  sql(`DELETE FROM sessions WHERE session_hmac = '${sessionHmac}'`);
  sql(`DELETE FROM form_tokens WHERE user_id = '${userId}'`);
  sql(`DELETE FROM community_memberships WHERE community_id IN (${communities}) OR id IN ('${primaryMembershipId}','${adminSecondMembershipId}','${memberOnlyMembershipId}')`);
  sql(`DELETE FROM users WHERE id = '${userId}'`);
  sql(`DELETE FROM communities WHERE id IN (${communities})`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${primaryCommunityId}', 'RFC074 Primary', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${adminSecondCommunityId}', 'RFC074 Admin Second', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${memberOnlyCommunityId}', 'RFC074 Member Only', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${userId}', '${now}')`,
    membershipInsert(primaryMembershipId, primaryCommunityId, 'admin', 'RFC074 User'),
    membershipInsert(adminSecondMembershipId, adminSecondCommunityId, 'admin', 'RFC074 User'),
    membershipInsert(memberOnlyMembershipId, memberOnlyCommunityId, 'member', 'RFC074 User'),
    sessionInsert('sess_rfc074_user', sessionHmac),
    eventInsert(eventId, primaryCommunityId, primaryMembershipId, 'RFC074 Detail Event', 'scheduled'),
    dayInsert('day_rfc074_detail', eventId, primaryCommunityId, 1, '2026-07-05', '2026-07-05T01:00:00.000Z', '2026-07-05T02:00:00.000Z'),
    eventInsert('evt_rfc074_list', primaryCommunityId, primaryMembershipId, 'RFC074 List Event', 'scheduled'),
    dayInsert('day_rfc074_list', 'evt_rfc074_list', primaryCommunityId, 1, '2026-07-06', '2026-07-06T01:00:00.000Z', '2026-07-06T02:00:00.000Z'),
  ];
  for (const statement of statements) sql(statement);
}

function membershipInsert(id, communityId, role, displayName) {
  return `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${id}', '${communityId}', '${userId}', '${role}', '${esc(displayName)}', '${now}')`;
}

function sessionInsert(id, sessHmac) {
  return `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance) VALUES ('${id}', '${userId}', '${sessHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}', 'invite_redemption')`;
}

function eventInsert(id, communityId, createdByMembershipId, title, status) {
  return `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${id}', '${communityId}', '${createdByMembershipId}', '${esc(title)}', 'RFC074 Room', NULL, '${status}', 'none', NULL, '${now}', '${now}')`;
}

function dayInsert(id, eventId, communityId, seq, dayDate, startsAt, endsAt, occurrenceStatus = 'scheduled') {
  return `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, occurrence_status) VALUES ('${id}', '${eventId}', '${communityId}', ${seq}, '${dayDate}', '${startsAt}', '${endsAt}', '${now}', '${occurrenceStatus}')`;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[rfc074-switch-smoke] ${message}`);
}

async function withTimeout(promise, label, ms = 10000) {
  let timeout;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timeout = setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms);
      }),
    ]);
  } finally {
    clearTimeout(timeout);
  }
}

async function json(url, init) {
  const res = await fetch(url, init);
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}: ${url}`);
  return await res.json();
}

async function waitForServer(proc, stderr) {
  for (let i = 0; i < 120; i += 1) {
    if (proc.exitCode !== null) break;
    try {
      const res = await fetch(`${baseUrl}/healthz`);
      if (res.ok) return;
    } catch (_) {
      await sleep(250);
    }
  }
  throw new Error(`Wrangler dev server did not become ready\n${stderr()}`);
}

async function waitForDebugger(stderr) {
  for (let i = 0; i < 80; i += 1) {
    try {
      return await json(`http://127.0.0.1:${remotePort}/json/version`);
    } catch (_) {
      await sleep(125);
    }
  }
  throw new Error(`Chromium remote debugging port did not open. stderr=${stderr()}`);
}

class Cdp {
  constructor(wsUrl) {
    this.nextId = 1;
    this.pending = new Map();
    this.events = new Map();
    this.ws = new WebSocket(wsUrl);
    this.ws.addEventListener('message', (message) => {
      const data = JSON.parse(message.data);
      if (data.id && this.pending.has(data.id)) {
        const { resolve, reject } = this.pending.get(data.id);
        this.pending.delete(data.id);
        if (data.error) reject(new Error(JSON.stringify(data.error)));
        else resolve(data.result ?? {});
      } else if (data.method && this.events.has(data.method)) {
        for (const cb of this.events.get(data.method)) cb(data.params ?? {});
      }
    });
  }

  async open() {
    if (this.ws.readyState === WebSocket.OPEN) return;
    await new Promise((resolve, reject) => {
      this.ws.addEventListener('open', resolve, { once: true });
      this.ws.addEventListener('error', reject, { once: true });
    });
  }

  send(method, params = {}) {
    const id = this.nextId;
    this.nextId += 1;
    this.ws.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
  }

  once(method) {
    return new Promise((resolve) => {
      const cb = (params) => {
        const list = this.events.get(method) ?? [];
        this.events.set(method, list.filter((item) => item !== cb));
        resolve(params);
      };
      this.on(method, cb);
    });
  }

  on(method, cb) {
    this.events.set(method, [...(this.events.get(method) ?? []), cb]);
  }

  close() {
    this.ws.close();
  }
}

async function newPage() {
  const target = await json(`http://127.0.0.1:${remotePort}/json/new`, { method: 'PUT' });
  const cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  await attachCspViolationCapture(cdp);
  await setSession(cdp);
  return cdp;
}

async function setSession(cdp) {
  await cdp.send('Network.setCookie', {
    name: 'ciao_sid',
    value: sessionSecret,
    domain: '127.0.0.1',
    path: '/',
    httpOnly: true,
    secure: false,
    sameSite: 'Strict',
  });
  await cdp.send('Network.setExtraHTTPHeaders', {
    headers: { Cookie: `ciao_sid=${sessionSecret}`, "Accept-Language": SMOKE_ACCEPT_LANGUAGE },
  });
}

async function navigate(cdp, path, options = {}) {
  await withTimeout(
    cdp.send('Emulation.setDeviceMetricsOverride', {
      width: options.width ?? 390,
      height: options.height ?? 900,
      deviceScaleFactor: 1,
      mobile: options.mobile ?? true,
    }),
    `device metrics ${path}`,
  );
  const loaded = cdp.once('Page.loadEventFired');
  await withTimeout(cdp.send('Page.navigate', { url: `${baseUrl}${path}` }), `Page.navigate ${path}`);
  await withTimeout(loaded, `navigate ${path}`);
}

async function evalExpr(cdp, expression) {
  const result = await withTimeout(
    cdp.send('Runtime.evaluate', {
      expression,
      awaitPromise: true,
      returnByValue: true,
    }),
    'Runtime.evaluate',
  );
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result?.value;
}

async function screenshot(cdp, name) {
  const shot = await withTimeout(
    cdp.send('Page.captureScreenshot', {
      format: 'png',
      captureBeyondViewport: false,
    }),
    `screenshot ${name}`,
  );
  const path = `${outDir}/${name}.png`;
  await writeFile(path, Buffer.from(shot.data, 'base64'));
  return path;
}

// Reads the no-JS switcher form's hidden `next` value and its community
// options — proving the *source* page emits the token RFC-074's
// route-family matrix assigns it, without executing any app JavaScript.
async function readSwitcherNext(cdp) {
  return await evalExpr(
    cdp,
    `(() => {
      const form = document.querySelector('form[action="/switch"]');
      if (!form) return { hasForm: false, nextValue: null, options: [] };
      const input = form.querySelector('input[name="next"]');
      const select = form.querySelector('select[name="community"]');
      return {
        hasForm: true,
        nextValue: input ? input.value : null,
        options: select ? [...select.options].map((o) => o.value) : [],
      };
    })()`,
  );
}

// Drives the switcher exactly the way a browser would on a real click: set
// the target option, then call the form's own native GET submit — no app
// JavaScript is involved (there is none for this form), so this is the
// no-JS submit path itself, not a simulation of it.
async function submitSwitcher(cdp, targetCommunityId) {
  const loaded = cdp.once('Page.loadEventFired');
  await evalExpr(
    cdp,
    `(() => {
      const form = document.querySelector('form[action="/switch"]');
      const select = form.querySelector('select[name="community"]');
      select.value = ${JSON.stringify(targetCommunityId)};
      form.submit();
    })()`,
  );
  await withTimeout(loaded, 'switcher submit navigation');
}

async function collect(cdp) {
  return await evalExpr(
    cdp,
    `(() => ({
      path: location.pathname + location.search,
      hash: location.hash,
      text: document.body.innerText,
    }))()`,
  );
}

function allChecksPass(checks) {
  return Object.values(checks).every(Boolean);
}

let dev;
let chrome;
let devStderr = '';
let chromeStderr = '';
const results = [];

try {
  logStep('seeding local D1 fixtures');
  seed();

  logStep(`starting local wrangler dev on ${baseUrl}`);
  dev = isolated.spawnDev(port);
  dev.stderr.on('data', (chunk) => {
    devStderr += chunk.toString();
  });
  await waitForServer(dev, () => devStderr);
  logStep('local wrangler dev is ready');

  const flags = [
    '--headless=new',
    '--incognito',
    '--disable-gpu',
    '--disable-dev-shm-usage',
    '--disable-breakpad',
    '--disable-crash-reporter',
    '--disable-crashpad',
    `--remote-debugging-port=${remotePort}`,
    `--user-data-dir=${userDataDir}`,
  ];
  chrome = spawn(chromium, flags, { stdio: ['ignore', 'ignore', 'pipe'] });
  chrome.stderr.on('data', (chunk) => {
    chromeStderr += chunk.toString();
  });
  await waitForDebugger(() => chromeStderr);
  logStep('sandboxed incognito Chromium is ready');

  const page = await newPage();

  // -- 1. Calendar: month/day/view=list preserved to a member-only target --
  logStep('checking Calendar switch preserves month/day/view=list');
  await navigate(page, `/c/${primaryCommunityId}/communities?month=2026-07&day=2026-07-05&view=list`, { width: 390 });
  const calendarNext = await readSwitcherNext(page);
  await submitSwitcher(page, memberOnlyCommunityId);
  const calendarResult = await collect(page);
  results.push({
    name: 'calendar-switch-preserves-month-day-list',
    screenshotPath: await screenshot(page, 'calendar-switch-preserves-month-day-list'),
    observed: { emittedNext: calendarNext.nextValue, path: calendarResult.path, hash: calendarResult.hash },
    checks: {
      sourcePageEmittedListToken: calendarNext.nextValue === 'communities:2026-07:2026-07-05:list',
      landsOnTargetCalendarWithState:
        calendarResult.path === `/c/${memberOnlyCommunityId}/communities?month=2026-07&day=2026-07-05&view=list`,
      emitsNoFragment: calendarResult.hash === '',
    },
  });

  // -- 2. My Page: "me" preserved to a member-only target ------------------
  logStep('checking My Page switch preserves "me"');
  await navigate(page, `/c/${primaryCommunityId}/me`, { width: 390 });
  const meNext = await readSwitcherNext(page);
  await submitSwitcher(page, memberOnlyCommunityId);
  const meResult = await collect(page);
  results.push({
    name: 'my-page-switch-preserves-me',
    screenshotPath: await screenshot(page, 'my-page-switch-preserves-me'),
    observed: { emittedNext: meNext.nextValue, path: meResult.path, hash: meResult.hash },
    checks: {
      sourcePageEmittedMeToken: meNext.nextValue === 'me',
      landsOnTargetMyPage: meResult.path === `/c/${memberOnlyCommunityId}/me`,
      emitsNoFragment: meResult.hash === '',
    },
  });

  // -- 3. Member management: admin target preserved, member-only target falls back to Home --
  logStep('checking member management switch: admin target preserved, member-only target falls back');
  await navigate(page, `/c/${primaryCommunityId}/admin/members`, { width: 390 });
  const membersNext = await readSwitcherNext(page);
  await submitSwitcher(page, adminSecondCommunityId);
  const membersAdminResult = await collect(page);
  results.push({
    name: 'member-management-switch-to-admin-target-preserved',
    screenshotPath: await screenshot(page, 'member-management-switch-to-admin-target'),
    observed: { emittedNext: membersNext.nextValue, path: membersAdminResult.path },
    checks: {
      sourcePageEmittedAdminMembersToken: membersNext.nextValue === 'admin_members',
      landsOnTargetMemberManagement: membersAdminResult.path === `/c/${adminSecondCommunityId}/admin/members`,
    },
  });

  await navigate(page, `/c/${primaryCommunityId}/admin/members`, { width: 390 });
  await submitSwitcher(page, memberOnlyCommunityId);
  const membersFallbackResult = await collect(page);
  results.push({
    name: 'member-management-switch-to-member-only-target-falls-back-home',
    screenshotPath: await screenshot(page, 'member-management-switch-falls-back-home'),
    observed: { path: membersFallbackResult.path, hash: membersFallbackResult.hash },
    checks: {
      fallsBackToTargetHome: membersFallbackResult.path === `/c/${memberOnlyCommunityId}/home`,
      emitsNoFragment: membersFallbackResult.hash === '',
    },
  });

  // -- 4. Create Event: admin target preserved, member-only target falls back to Home --
  logStep('checking Create Event switch: admin target preserved, member-only target falls back');
  await navigate(page, `/c/${primaryCommunityId}/admin/events/new`, { width: 390 });
  const createEventNext = await readSwitcherNext(page);
  await submitSwitcher(page, adminSecondCommunityId);
  const createEventAdminResult = await collect(page);
  results.push({
    name: 'create-event-switch-to-admin-target-preserved',
    screenshotPath: await screenshot(page, 'create-event-switch-to-admin-target'),
    observed: { emittedNext: createEventNext.nextValue, path: createEventAdminResult.path },
    checks: {
      sourcePageEmittedAdminEventsNewToken: createEventNext.nextValue === 'admin_events_new',
      landsOnTargetCreateEvent: createEventAdminResult.path === `/c/${adminSecondCommunityId}/admin/events/new`,
    },
  });

  await navigate(page, `/c/${primaryCommunityId}/admin/events/new`, { width: 390 });
  await submitSwitcher(page, memberOnlyCommunityId);
  const createEventFallbackResult = await collect(page);
  results.push({
    name: 'create-event-switch-to-member-only-target-falls-back-home',
    screenshotPath: await screenshot(page, 'create-event-switch-falls-back-home'),
    observed: { path: createEventFallbackResult.path },
    checks: {
      fallsBackToTargetHome: createEventFallbackResult.path === `/c/${memberOnlyCommunityId}/home`,
    },
  });

  // -- 5. Event Detail: always falls back to Home, even for an admin target --
  logStep('checking Event Detail switch always falls back to Home, no event id preserved');
  await navigate(page, `/c/${primaryCommunityId}/events/${eventId}`, { width: 390 });
  const eventDetailNext = await readSwitcherNext(page);
  await submitSwitcher(page, adminSecondCommunityId);
  const eventDetailResult = await collect(page);
  results.push({
    name: 'event-detail-switch-falls-back-to-home',
    screenshotPath: await screenshot(page, 'event-detail-switch-falls-back-home'),
    observed: { emittedNext: eventDetailNext.nextValue, path: eventDetailResult.path, hash: eventDetailResult.hash },
    checks: {
      sourcePageEmittedHomeToken: eventDetailNext.nextValue === 'home',
      landsOnTargetHomeNotEventDetail: eventDetailResult.path === `/c/${adminSecondCommunityId}/home`,
      noEventIdPreserved: !eventDetailResult.path.includes(eventId),
      emitsNoFragment: eventDetailResult.hash === '',
    },
  });

  const cspViolations = await readCspViolations(page);
  results.push({
    name: 'no-csp-violations',
    observed: { cspViolations },
    checks: { zeroCspViolations: cspViolations.length === 0 },
  });

  page.close();

  for (const result of results) result.passed = allChecksPass(result.checks);

  const report = {
    generatedAt: new Date().toISOString(),
    chromium,
    baseUrl,
    userDataDir,
    flags,
    note: 'Chromium launched with --incognito and without --no-sandbox. Local wrangler dev only. The switcher form is submitted via its own native GET action — no app JavaScript is involved for this form, so this exercises the no-JS submit path directly.',
    localOnlyGuard: true,
    coverage: [
      'Calendar switch preserves month/day/view=list to a member-only target',
      'My Page switch preserves "me" to a member-only target',
      'Member management switch preserves admin_members to an admin target, falls back to Home for a member-only target',
      'Create Event switch preserves admin_events_new to an admin target, falls back to Home for a member-only target',
      'Event Detail switch always falls back to Home, even for an admin target, with no event id preserved',
      'No switch emits a fragment',
      'no-JS switcher submit path (native form GET submit, no app JavaScript)',
    ],
    results,
    passed: results.every((result) => result.passed),
  };

  await writeFile(`${outDir}/${reportName}`, JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ passed: report.passed, report: `${outDir}/${reportName}`, results }, null, 2));
  if (!report.passed) process.exitCode = 1;
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  if (chrome) chrome.kill();
  if (dev) dev.kill();
  await rm(userDataDir, { recursive: true, force: true }).catch(() => {});
  await isolated.cleanup();
}
