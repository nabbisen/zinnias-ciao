#!/usr/bin/env node
// Scenario smoke for RFC-068 monthly attendance matrix CSV export.
// Local wrangler dev only; launches sandboxed incognito Chromium.

import { createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8799);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9251);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc068';
const reportName = process.env.REPORT_NAME ?? 'rfc068-calendar-matrix-csv-export-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-rfc068-matrix-csv-sandboxed-${Date.now()}`;
const downloadDir = `${outDir}/downloads-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const pepper = 'dev-pepper-change-in-production';
const now = '2026-07-10T00:00:00.000Z';

const communityId = 'com_rfc068_primary';
const adminUserId = 'usr_rfc068_admin';
const memberUserId = 'usr_rfc068_member';
const adminMembershipId = 'mem_rfc068_admin';
const memberMembershipId = 'mem_rfc068_member';
const adminSessionSecret = 'rfc068-smoke-admin-session';
const memberSessionSecret = 'rfc068-smoke-member-session';
const adminSessionHmac = hmac(adminSessionSecret);
const memberSessionHmac = hmac(memberSessionSecret);
const formulaMemberName = '  =RFC068 Formula, "Quoted"';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await mkdir(downloadDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function hmac(value) {
  return createHmac('sha256', pepper).update(value).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`RFC-068 smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`RFC-068 smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) throw new Error('RFC-068 smoke refuses remote D1 operations');
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

function sqlOutput(statement) {
  return runWrangler([
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
  sql(`DELETE FROM attendances WHERE event_day_id IN (SELECT id FROM event_days WHERE community_id = '${communityId}')`);
  sql(`DELETE FROM event_notes WHERE event_id IN (SELECT id FROM events WHERE community_id = '${communityId}')`);
  sql(`DELETE FROM event_days WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM event_series WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM events WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM audit_log WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM sessions WHERE session_hmac IN ('${adminSessionHmac}','${memberSessionHmac}')`);
  sql(`DELETE FROM form_tokens WHERE user_id IN ('${adminUserId}','${memberUserId}')`);
  sql(`DELETE FROM community_memberships WHERE community_id = '${communityId}' OR id IN ('${adminMembershipId}','${memberMembershipId}')`);
  sql(`DELETE FROM users WHERE id IN ('${adminUserId}','${memberUserId}')`);
  sql(`DELETE FROM communities WHERE id = '${communityId}'`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'RFC068 Primary', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${adminUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    membershipInsert(adminMembershipId, adminUserId, 'admin', 'RFC068 Admin'),
    membershipInsert(memberMembershipId, memberUserId, 'member', formulaMemberName),
    sessionInsert('sess_rfc068_admin', adminUserId, adminSessionHmac),
    sessionInsert('sess_rfc068_member', memberUserId, memberSessionHmac),
    eventInsert('evt_rfc068_single', adminMembershipId, 'RFC068 Single Event'),
    dayInsert('day_rfc068_single', 'evt_rfc068_single', 1, '2026-07-14', '2026-07-14T01:00:00.000Z', '2026-07-14T02:00:00.000Z'),
    eventInsert('evt_rfc068_cancelled', adminMembershipId, 'RFC068 Cancelled'),
    dayInsert('day_rfc068_cancelled', 'evt_rfc068_cancelled', 1, '2026-07-15', '2026-07-15T01:00:00.000Z', '2026-07-15T02:00:00.000Z', 'cancelled'),
    attendanceInsert('att_rfc068_admin', 'day_rfc068_single', adminMembershipId, 'going'),
  ];
  for (const statement of statements) sql(statement);
}

function membershipInsert(id, userId, role, displayName) {
  return `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${id}', '${communityId}', '${userId}', '${role}', '${esc(displayName)}', '${now}')`;
}

function sessionInsert(id, userId, sessionHmac) {
  return `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('${id}', '${userId}', '${sessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`;
}

function eventInsert(id, createdByMembershipId, title) {
  return `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${id}', '${communityId}', '${createdByMembershipId}', '${esc(title)}', 'RFC068 Room', NULL, 'scheduled', 'none', NULL, '${now}', '${now}')`;
}

function dayInsert(id, eventId, seq, dayDate, startsAt, endsAt, occurrenceStatus = 'scheduled') {
  return `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, occurrence_status) VALUES ('${id}', '${eventId}', '${communityId}', ${seq}, '${dayDate}', '${startsAt}', '${endsAt}', '${now}', '${occurrenceStatus}')`;
}

function attendanceInsert(id, eventDayId, membershipId, status) {
  return `INSERT INTO attendances (id, event_day_id, membership_id, status, status_updated_at, updated_at) VALUES ('${id}', '${eventDayId}', '${membershipId}', '${status}', '${now}', '${now}')`;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[rfc068-matrix-csv-smoke] ${message}`);
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

  on(method, cb) {
    this.events.set(method, [...(this.events.get(method) ?? []), cb]);
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

async function allowDownloads(cdp) {
  try {
    await cdp.send('Browser.setDownloadBehavior', {
      behavior: 'allow',
      downloadPath: downloadDir,
    });
  } catch (_) {
    await cdp.send('Page.setDownloadBehavior', {
      behavior: 'allow',
      downloadPath: downloadDir,
    });
  }
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

async function collect(cdp) {
  return await evalExpr(
    cdp,
    `(() => ({
      path: location.pathname + location.search,
      text: document.body.innerText,
      hasExportButton: Boolean(document.querySelector('[data-calendar-matrix-export-button]')),
      hasExportTable: Boolean(document.querySelector('table[data-calendar-matrix-export]')),
      exportValueCount: document.querySelectorAll('[data-export-value]').length,
      memberNameCount: document.querySelectorAll('[data-member-name]').length,
      dateHeaderCount: document.querySelectorAll('thead th[data-date]').length,
      pageDoesNotOverflow: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
    }))()`,
  );
}

async function waitForDownloadedCsv() {
  for (let i = 0; i < 80; i += 1) {
    const files = await readdir(downloadDir).catch(() => []);
    const csv = files.find((file) => file.endsWith('.csv'));
    if (csv) return `${downloadDir}/${csv}`;
    await sleep(125);
  }
  throw new Error('CSV download did not appear');
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
  const requestedUrls = [];
  adminPage.on('Network.requestWillBeSent', (params) => {
    if (params.request?.url) requestedUrls.push(params.request.url);
  });
  await allowDownloads(adminPage);

  logStep('checking member matrix has no export contract');
  await setSession(memberPage, memberSessionSecret);
  await navigate(memberPage, `/c/${communityId}/communities?month=2026-07&view=matrix`, {
    width: 390,
  });
  const memberMatrix = await collect(memberPage);
  results.push({
    name: 'member-matrix-no-export-contract',
    screenshotPath: await screenshot(memberPage, 'member-matrix-no-export-contract'),
    observed: {
      path: memberMatrix.path,
      exportValueCount: memberMatrix.exportValueCount,
    },
    checks: {
      routeIsMatrix: memberMatrix.path === `/c/${communityId}/communities?month=2026-07&view=matrix`,
      seesMatrix: memberMatrix.text.includes('月の回答表'),
      noExportButton: !memberMatrix.hasExportButton,
      noExportTable: !memberMatrix.hasExportTable,
      noExportValues: memberMatrix.exportValueCount === 0,
      noExportNames: memberMatrix.memberNameCount === 0,
      noDateHeaders: memberMatrix.dateHeaderCount === 0,
      pageDoesNotOverflow: memberMatrix.pageDoesNotOverflow,
    },
  });

  logStep('checking admin matrix export and downloaded CSV');
  await setSession(adminPage, adminSessionSecret);
  await navigate(adminPage, `/c/${communityId}/communities?month=2026-07&view=matrix`, {
    width: 1280,
    height: 900,
    mobile: false,
  });
  const adminMatrix = await collect(adminPage);
  await evalExpr(adminPage, `document.querySelector('[data-calendar-matrix-export-button]').click(); true`);
  const csvPath = await waitForDownloadedCsv();
  const csv = await readFile(csvPath, 'utf8');
  await sleep(250);
  const auditOutput = sqlOutput(
    `SELECT action, metadata_json FROM audit_log WHERE community_id = '${communityId}' AND action = 'calendar_matrix_csv.export_requested'`,
  );
  const auditRequests = requestedUrls.filter((url) => url.includes('/admin/calendar/matrix-export/audit'));
  const csvEndpointRequests = requestedUrls.filter((url) => (
    url.includes('/export/csv') ||
    url.includes('/matrix-export/csv') ||
    url.endsWith('.csv')
  ));
  results.push({
    name: 'admin-export-audited-browser-csv-download',
    screenshotPath: await screenshot(adminPage, 'admin-export-audited-browser-csv-download'),
    observed: {
      path: adminMatrix.path,
      csvPath,
      auditRequestCount: auditRequests.length,
      csvEndpointRequests,
    },
    checks: {
      adminHasExportButton: adminMatrix.hasExportButton,
      adminHasExportTable: adminMatrix.hasExportTable,
      hasAllDateHeaders: adminMatrix.dateHeaderCount === 31,
      hasExportValues: adminMatrix.exportValueCount === 62,
      csvHasBom: csv.charCodeAt(0) === 0xfeff,
      csvHasHeader: csv.includes('"member_name","2026-07-01"') && csv.includes('"2026-07-31"'),
      csvHasGoingAndCancelledCells: csv.includes('"RFC068 Admin"') && csv.includes('"○"') && csv.includes('"中"'),
      csvFormulaValueIsHardened: csv.includes('"\'  =RFC068 Formula, ""Quoted"""'),
      csvUsesCrLf: csv.includes('\r\n'),
      auditRequestHappened: auditRequests.length === 1,
      noCsvServerEndpointRequest: csvEndpointRequests.length === 0,
      auditRecorded: auditOutput.includes('calendar_matrix_csv.export_requested') &&
        auditOutput.includes('calendar_matrix_csv') &&
        auditOutput.includes('2026-07'),
      pageDoesNotOverflow: adminMatrix.pageDoesNotOverflow,
    },
  });

  memberPage.close();
  adminPage.close();

  for (const result of results) result.passed = allChecksPass(result.checks);

  const report = {
    generatedAt: new Date().toISOString(),
    chromium,
    baseUrl,
    userDataDir,
    downloadDir,
    flags,
    note: 'Chromium launched with --incognito and without --no-sandbox. Local wrangler dev only.',
    localOnlyGuard: true,
    coverage: [
      'member matrix omits export controls and export-only attributes',
      'admin matrix includes export controls and rendered export values',
      'browser-created CSV download',
      'formula injection hardening for leading-whitespace risky value',
      'metadata-only audit request before download',
      'no server CSV endpoint request',
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
