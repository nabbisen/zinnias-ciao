#!/usr/bin/env node
// Scenario smoke for RFC-067 monthly attendance matrix. Local wrangler dev only.

import { createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8798);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9250);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc067';
const reportName = process.env.REPORT_NAME ?? 'rfc067-monthly-attendance-matrix-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-rfc067-matrix-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const pepper = 'dev-pepper-change-in-production';
const now = '2026-07-10T00:00:00.000Z';

const primaryCommunityId = 'com_rfc067_primary';
const secondCommunityId = 'com_rfc067_second';
const adminUserId = 'usr_rfc067_admin';
const memberUserId = 'usr_rfc067_member';
const outsiderUserId = 'usr_rfc067_outsider';
const adminMembershipId = 'mem_rfc067_admin';
const memberMembershipId = 'mem_rfc067_member';
const memberSecondMembershipId = 'mem_rfc067_member_second';
const outsiderMembershipId = 'mem_rfc067_outsider';
const adminSessionSecret = 'rfc067-smoke-admin-session';
const memberSessionSecret = 'rfc067-smoke-member-session';
const outsiderSessionSecret = 'rfc067-smoke-outsider-session';
const adminSessionHmac = hmac(adminSessionSecret);
const memberSessionHmac = hmac(memberSessionSecret);
const outsiderSessionHmac = hmac(outsiderSessionSecret);

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function hmac(value) {
  return createHmac('sha256', pepper).update(value).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`RFC-067 smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`RFC-067 smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) throw new Error('RFC-067 smoke refuses remote D1 operations');
  try {
    return execFileSync('bunx', ['wrangler', ...args], {
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
  const communities = `'${primaryCommunityId}','${secondCommunityId}'`;
  sql(`DELETE FROM attendances WHERE event_day_id IN (SELECT id FROM event_days WHERE community_id IN (${communities}))`);
  sql(`DELETE FROM event_notes WHERE event_id IN (SELECT id FROM events WHERE community_id IN (${communities}))`);
  sql(`DELETE FROM event_days WHERE community_id IN (${communities})`);
  sql(`DELETE FROM event_series WHERE community_id IN (${communities})`);
  sql(`DELETE FROM events WHERE community_id IN (${communities})`);
  sql(`DELETE FROM audit_log WHERE community_id IN (${communities})`);
  sql(`DELETE FROM sessions WHERE session_hmac IN ('${adminSessionHmac}','${memberSessionHmac}','${outsiderSessionHmac}')`);
  sql(`DELETE FROM form_tokens WHERE user_id IN ('${adminUserId}','${memberUserId}','${outsiderUserId}')`);
  sql(`DELETE FROM community_memberships WHERE community_id IN (${communities}) OR id IN ('${adminMembershipId}','${memberMembershipId}','${memberSecondMembershipId}','${outsiderMembershipId}')`);
  sql(`DELETE FROM users WHERE id IN ('${adminUserId}','${memberUserId}','${outsiderUserId}')`);
  sql(`DELETE FROM communities WHERE id IN (${communities})`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${primaryCommunityId}', 'RFC067 Primary', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${secondCommunityId}', 'RFC067 Second', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${adminUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${outsiderUserId}', '${now}')`,
    membershipInsert(adminMembershipId, primaryCommunityId, adminUserId, 'admin', 'RFC067 Admin'),
    membershipInsert(memberMembershipId, primaryCommunityId, memberUserId, 'member', 'RFC067 Member'),
    membershipInsert(memberSecondMembershipId, secondCommunityId, memberUserId, 'member', 'RFC067 Member Second'),
    membershipInsert(outsiderMembershipId, secondCommunityId, outsiderUserId, 'member', 'RFC067 Outsider'),
    sessionInsert('sess_rfc067_admin', adminUserId, adminSessionHmac),
    sessionInsert('sess_rfc067_member', memberUserId, memberSessionHmac),
    sessionInsert('sess_rfc067_outsider', outsiderUserId, outsiderSessionHmac),
    eventInsert('evt_rfc067_single', primaryCommunityId, adminMembershipId, 'RFC067 Single Event', 'scheduled'),
    dayInsert('day_rfc067_single', 'evt_rfc067_single', primaryCommunityId, 1, '2026-07-14', '2026-07-14T01:00:00.000Z', '2026-07-14T02:00:00.000Z'),
    eventInsert('evt_rfc067_multi_a', primaryCommunityId, adminMembershipId, 'RFC067 Multi A', 'scheduled'),
    dayInsert('day_rfc067_multi_a', 'evt_rfc067_multi_a', primaryCommunityId, 1, '2026-07-15', '2026-07-15T01:00:00.000Z', '2026-07-15T02:00:00.000Z'),
    eventInsert('evt_rfc067_multi_b', primaryCommunityId, adminMembershipId, 'RFC067 Multi B', 'scheduled'),
    dayInsert('day_rfc067_multi_b', 'evt_rfc067_multi_b', primaryCommunityId, 1, '2026-07-15', '2026-07-15T03:00:00.000Z', '2026-07-15T04:00:00.000Z'),
    eventInsert('evt_rfc067_cancelled', primaryCommunityId, adminMembershipId, 'RFC067 Cancelled', 'scheduled'),
    dayInsert('day_rfc067_cancelled', 'evt_rfc067_cancelled', primaryCommunityId, 1, '2026-07-16', '2026-07-16T01:00:00.000Z', '2026-07-16T02:00:00.000Z', 'cancelled'),
    eventInsert('evt_rfc067_second', secondCommunityId, memberSecondMembershipId, 'RFC067 Second Event', 'scheduled'),
    dayInsert('day_rfc067_second', 'evt_rfc067_second', secondCommunityId, 1, '2026-07-20', '2026-07-20T01:00:00.000Z', '2026-07-20T02:00:00.000Z'),
    attendanceInsert('att_rfc067_single_admin', 'day_rfc067_single', adminMembershipId, 'going'),
    attendanceInsert('att_rfc067_single_member', 'day_rfc067_single', memberMembershipId, 'going'),
    attendanceInsert('att_rfc067_multi_admin', 'day_rfc067_multi_a', adminMembershipId, 'not_going'),
    attendanceInsert('att_rfc067_multi_member', 'day_rfc067_multi_a', memberMembershipId, 'going'),
  ];
  for (const statement of statements) sql(statement);
}

function membershipInsert(id, communityId, userId, role, displayName) {
  return `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${id}', '${communityId}', '${userId}', '${role}', '${esc(displayName)}', '${now}')`;
}

function sessionInsert(id, userId, sessionHmac) {
  return `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('${id}', '${userId}', '${sessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`;
}

function eventInsert(id, communityId, createdByMembershipId, title, status) {
  return `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${id}', '${communityId}', '${createdByMembershipId}', '${esc(title)}', 'RFC067 Room', NULL, '${status}', 'none', NULL, '${now}', '${now}')`;
}

function dayInsert(id, eventId, communityId, seq, dayDate, startsAt, endsAt, occurrenceStatus = 'scheduled') {
  return `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, occurrence_status) VALUES ('${id}', '${eventId}', '${communityId}', ${seq}, '${dayDate}', '${startsAt}', '${endsAt}', '${now}', '${occurrenceStatus}')`;
}

function attendanceInsert(id, eventDayId, membershipId, status) {
  return `INSERT INTO attendances (id, event_day_id, membership_id, status, status_updated_at, updated_at) VALUES ('${id}', '${eventDayId}', '${membershipId}', '${status}', '${now}', '${now}')`;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[rfc067-matrix-smoke] ${message}`);
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
      this.events.set(method, [...(this.events.get(method) ?? []), cb]);
    });
  }

  close() {
    this.ws.close();
  }
}

async function newPage(sessionSecret) {
  const target = await json(`http://127.0.0.1:${remotePort}/json/new`, { method: 'PUT' });
  const cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  await setSession(cdp, sessionSecret);
  return cdp;
}

async function setSession(cdp, sessionSecret) {
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
    headers: { Cookie: `ciao_sid=${sessionSecret}` },
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
  if (options.textScale === 2) {
    await evalExpr(cdp, `document.documentElement.style.fontSize = '200%'`);
    await sleep(150);
  }
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

async function collect(cdp) {
  return await evalExpr(
    cdp,
    `(() => {
      const scroller = document.querySelector('[data-rfc067-matrix-scroller]') ||
        [...document.querySelectorAll('div')].find((el) => el.querySelector('table'));
      const links = [...document.querySelectorAll('a[href]')].map((a) => ({
        href: a.getAttribute('href'),
        text: a.innerText,
      }));
      return {
        path: location.pathname + location.search,
        text: document.body.innerText,
        hrefs: links.map((link) => link.href),
        links,
        labels: [...document.querySelectorAll('[aria-label]')].map((el) => el.getAttribute('aria-label')),
        noPageHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
        matrixScrollerHasHorizontalScroll: scroller ? scroller.scrollWidth > scroller.clientWidth : false,
      };
    })()`,
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
  dev = spawn('bun', ['run', 'dev', '--', '--port', String(port)], {
    cwd: process.cwd(),
    stdio: ['ignore', 'ignore', 'pipe'],
  });
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

  const memberPage = await newPage(memberSessionSecret);
  const adminPage = await newPage(adminSessionSecret);
  const outsiderPage = await newPage(outsiderSessionSecret);

  logStep('checking member-visible matrix at mobile 200% text');
  await setSession(memberPage, memberSessionSecret);
  await navigate(memberPage, `/c/${primaryCommunityId}/communities?month=2026-07&day=2026-07-15&view=matrix`, {
    width: 390,
    textScale: 2,
  });
  const memberMatrix = await collect(memberPage);
  results.push({
    name: 'member-matrix-mobile-200-percent',
    screenshotPath: await screenshot(memberPage, 'member-matrix-mobile-200-percent'),
    observed: { path: memberMatrix.path },
    checks: {
      routeIsMatrix: memberMatrix.path === `/c/${primaryCommunityId}/communities?month=2026-07&day=2026-07-15&view=matrix`,
      showsMatrixCopy: memberMatrix.text.includes('回答表') && memberMatrix.text.includes('月の回答表'),
      showsMemberRows: memberMatrix.text.includes('RFC067 Admin') && memberMatrix.text.includes('RFC067 Member'),
      showsSingleAndMultiCells: memberMatrix.text.includes('○') && memberMatrix.text.includes('1/2'),
      showsCancelledCell: memberMatrix.text.includes('中'),
      detailLinksToEvents: memberMatrix.hrefs.includes(`/c/${primaryCommunityId}/events/evt_rfc067_multi_a`),
      accessibleBreakdown: memberMatrix.labels.some((label) => label.includes('予定2件') && label.includes('未回答1件')),
      pageDoesNotOverflow: memberMatrix.noPageHorizontalScroll,
      matrixScrollerScrolls: memberMatrix.matrixScrollerHasHorizontalScroll,
    },
  });

  logStep('checking admin-visible matrix at desktop width');
  await setSession(adminPage, adminSessionSecret);
  await navigate(adminPage, `/c/${primaryCommunityId}/communities?month=2026-07&view=matrix`, {
    width: 1280,
    height: 900,
    mobile: false,
  });
  const adminMatrix = await collect(adminPage);
  results.push({
    name: 'admin-matrix-desktop',
    screenshotPath: await screenshot(adminPage, 'admin-matrix-desktop'),
    observed: { path: adminMatrix.path },
    checks: {
      adminCanOpenMatrix: adminMatrix.path === `/c/${primaryCommunityId}/communities?month=2026-07&view=matrix`,
      includesModeSwitcher: adminMatrix.hrefs.includes(`/c/${primaryCommunityId}/communities?month=2026-07`),
      noCsvOrExport: !adminMatrix.text.toLowerCase().includes('csv') && !adminMatrix.text.includes('エクスポート'),
      pageDoesNotOverflow: adminMatrix.noPageHorizontalScroll,
    },
  });

  logStep('checking non-member direct URL denial');
  await setSession(outsiderPage, outsiderSessionSecret);
  await navigate(outsiderPage, `/c/${primaryCommunityId}/communities?month=2026-07&view=matrix`, {
    width: 390,
    textScale: 2,
  });
  const outsiderMatrix = await collect(outsiderPage);
  results.push({
    name: 'non-member-direct-matrix-url-denied',
    screenshotPath: await screenshot(outsiderPage, 'non-member-direct-matrix-url-denied'),
    observed: { path: outsiderMatrix.path, text: outsiderMatrix.text },
    checks: {
      doesNotShowMatrix: !outsiderMatrix.text.includes('月の回答表'),
      doesNotShowMembers: !outsiderMatrix.text.includes('RFC067 Admin') && !outsiderMatrix.text.includes('RFC067 Member'),
      showsGenericNotFound: outsiderMatrix.text.includes('見つかりませんでした'),
      pageDoesNotOverflow: outsiderMatrix.noPageHorizontalScroll,
    },
  });

  logStep('checking community switcher matrix state preservation');
  await setSession(memberPage, memberSessionSecret);
  await navigate(memberPage, `/switch?community=${secondCommunityId}&next=communities:2026-07:matrix`, {
    width: 390,
    textScale: 2,
  });
  const switchedMatrix = await collect(memberPage);
  results.push({
    name: 'switcher-preserves-matrix-for-active-target-community',
    screenshotPath: await screenshot(memberPage, 'switcher-preserves-matrix'),
    observed: { path: switchedMatrix.path },
    checks: {
      landsOnSecondMatrix: switchedMatrix.path === `/c/${secondCommunityId}/communities?month=2026-07&view=matrix`,
      showsSecondCommunityEvent: switchedMatrix.text.includes('RFC067 Second Event'),
      keepsMemberScopedRows: switchedMatrix.text.includes('RFC067 Member Second'),
      pageDoesNotOverflow: switchedMatrix.noPageHorizontalScroll,
    },
  });

  memberPage.close();
  adminPage.close();
  outsiderPage.close();

  for (const result of results) result.passed = allChecksPass(result.checks);

  const report = {
    generatedAt: new Date().toISOString(),
    chromium,
    baseUrl,
    userDataDir,
    flags,
    note: 'Chromium launched with --incognito and without --no-sandbox. Local wrangler dev only.',
    localOnlyGuard: true,
    coverage: [
      'member matrix access',
      'admin matrix access',
      'non-member direct URL denial',
      'community switcher matrix state preservation',
      'multi-event answered/total cells',
      'cancelled occurrence marker',
      'mobile 390px viewport with 200% text scaling',
      'matrix-only horizontal scrolling',
      'CSV/export absence from rendered matrix',
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
}
