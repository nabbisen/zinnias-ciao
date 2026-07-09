#!/usr/bin/env node
// Scenario smoke for recurrence v2 workflows. Local wrangler dev only.

import { createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8796);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9248);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc065';
const reportName = process.env.REPORT_NAME ?? 'rfc065-recurrence-v2-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-recurrence-v2-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const pepper = 'dev-pepper-change-in-production';
const now = '2026-07-09T00:00:00.000Z';

const communityId = 'com_rfc065_primary';
const adminUserId = 'usr_rfc065_admin';
const memberUserId = 'usr_rfc065_member';
const adminMembershipId = 'mem_rfc065_admin';
const memberMembershipId = 'mem_rfc065_member';
const adminSessionSecret = 'rfc065-smoke-admin-session';
const memberSessionSecret = 'rfc065-smoke-member-session';
const adminSessionHmac = createHmac('sha256', pepper).update(adminSessionSecret).digest('hex');
const memberSessionHmac = createHmac('sha256', pepper).update(memberSessionSecret).digest('hex');

const materializeEventId = 'evt_rfc065_materialize';
const materializeDayId = 'day_rfc065_materialize_base';
const materializeSeriesId = 'ser_rfc065_materialize';
const uiCreatedTitle = 'RFC065 UI Open Recurrence';
const materializeTitle = 'RFC065 Rolling Materialization';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`recurrence-v2 smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`recurrence-v2 smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args, options = {}) {
  if (args.includes('--remote')) {
    throw new Error('recurrence-v2 smoke refuses remote D1 operations');
  }
  try {
    return execFileSync('bunx', ['wrangler', ...args], {
      cwd: process.cwd(),
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf8',
      ...options,
    });
  } catch (error) {
    throw new Error(
      `wrangler ${args.join(' ')} failed\n${error.stderr?.toString() ?? ''}`,
    );
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

function query(statement) {
  const raw = runWrangler([
    'd1',
    'execute',
    'zinnias-ciao-dev',
    '--local',
    '--env',
    'dev',
    '--json',
    '--command',
    statement,
  ]);
  const parsed = JSON.parse(raw);
  return parsed?.[0]?.results ?? parsed?.results ?? [];
}

function esc(value) {
  return String(value).replaceAll("'", "''");
}

function clean() {
  const titleFilter = `title IN ('${esc(uiCreatedTitle)}','${esc(materializeTitle)}')`;
  sql(`DELETE FROM event_series_exceptions WHERE community_id='${communityId}' OR series_id='${materializeSeriesId}'`);
  sql(`DELETE FROM attendances WHERE event_day_id IN (SELECT id FROM event_days WHERE community_id='${communityId}' OR event_id='${materializeEventId}')`);
  sql(`DELETE FROM event_days WHERE community_id='${communityId}' OR event_id IN (SELECT id FROM events WHERE ${titleFilter}) OR event_id='${materializeEventId}'`);
  sql(`DELETE FROM event_series WHERE community_id='${communityId}' OR event_id IN (SELECT id FROM events WHERE ${titleFilter}) OR id='${materializeSeriesId}'`);
  sql(`DELETE FROM event_notes WHERE event_id IN (SELECT id FROM events WHERE community_id='${communityId}' OR ${titleFilter})`);
  sql(`DELETE FROM events WHERE community_id='${communityId}' OR ${titleFilter} OR id='${materializeEventId}'`);
  sql(`DELETE FROM sessions WHERE id IN ('sess_rfc065_admin','sess_rfc065_member') OR session_hmac IN ('${adminSessionHmac}','${memberSessionHmac}')`);
  sql(`DELETE FROM form_tokens WHERE user_id IN ('${adminUserId}','${memberUserId}')`);
  sql(`DELETE FROM community_memberships WHERE id IN ('${adminMembershipId}','${memberMembershipId}') OR community_id='${communityId}'`);
  sql(`DELETE FROM users WHERE id IN ('${adminUserId}','${memberUserId}')`);
  sql(`DELETE FROM communities WHERE id='${communityId}'`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'RFC065 Primary Community', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${adminUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminMembershipId}', '${communityId}', '${adminUserId}', 'admin', 'RFC065 Admin', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${communityId}', '${memberUserId}', 'member', 'RFC065 Member', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc065_admin', '${adminUserId}', '${adminSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc065_member', '${memberUserId}', '${memberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${materializeEventId}', '${communityId}', '${adminMembershipId}', '${materializeTitle}', 'Local room', '', 'scheduled', 'weekly', NULL, '${now}', '${now}')`,
    `INSERT INTO event_series (id, event_id, community_id, frequency, start_day_date, starts_at_local, ends_at_local, timezone, end_mode, occurrence_count, until_day_date, materialized_through_day_date, created_at, updated_at) VALUES ('${materializeSeriesId}', '${materializeEventId}', '${communityId}', 'weekly', '2026-07-10', '09:00', '10:00', 'Asia/Tokyo', 'open_ended', NULL, NULL, '2026-07-10', '${now}', '${now}')`,
    `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, occurrence_status, series_id, series_occurrence_date) VALUES ('${materializeDayId}', '${materializeEventId}', '${communityId}', 1, '2026-07-10', '2026-07-10T00:00:00.000Z', '2026-07-10T01:00:00.000Z', '${now}', 'scheduled', '${materializeSeriesId}', '2026-07-10')`,
  ];
  for (const statement of statements) sql(statement);
}

function countRows(statement) {
  const rows = query(statement);
  return Number(rows[0]?.n ?? rows[0]?.N ?? 0);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[recurrence-v2-smoke] ${message}`);
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
        this.events.set(
          method,
          list.filter((item) => item !== cb),
        );
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
  const target = await json(`http://127.0.0.1:${remotePort}/json/new`, {
    method: 'PUT',
  });
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
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: options.width ?? 390,
    height: options.height ?? 900,
    deviceScaleFactor: 1,
    mobile: true,
  });
  const loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}${path}` });
  await withTimeout(loaded, `navigate ${path}`);
  if (options.textScale === 2) {
    await evalExpr(
      cdp,
      `(() => {
        document.documentElement.style.fontSize = '200%';
      })()`,
    );
    await sleep(150);
  }
}

async function evalExpr(cdp, expression) {
  const result = await cdp.send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result?.value;
}

async function screenshot(cdp, name) {
  const shot = await cdp.send('Page.captureScreenshot', {
    format: 'png',
    captureBeyondViewport: true,
  });
  const path = `${outDir}/${name}.png`;
  await writeFile(path, Buffer.from(shot.data, 'base64'));
  return path;
}

async function collect(cdp) {
  return await evalExpr(
    cdp,
    `(() => {
      const fields = [...document.querySelectorAll('input[name], textarea[name], select[name]')];
      const links = [...document.querySelectorAll('a[href]')].map((a) => ({
        href: a.getAttribute('href'),
        text: a.innerText,
      }));
      return {
        path: location.pathname + location.search,
        text: document.body.innerText,
        hrefs: links.map((link) => link.href),
        links,
        values: Object.fromEntries(fields.map((el) => [el.getAttribute('name'), el.value])),
        noHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
      };
    })()`,
  );
}

async function fillCreateRecurrenceForm(cdp) {
  await evalExpr(
    cdp,
    `(() => {
      const set = (name, value) => {
        const el = document.querySelector('[name="' + name + '"]');
        if (!el) throw new Error('missing field ' + name);
        el.value = value;
        el.dispatchEvent(new Event('input', { bubbles: true }));
        el.dispatchEvent(new Event('change', { bubbles: true }));
      };
      set('title', ${JSON.stringify(uiCreatedTitle)});
      set('day_date', '2026-07-16');
      set('starts_at', '09:00');
      set('ends_at', '10:00');
      set('location', 'Smoke room');
      set('repeat_rule', 'weekly');
      set('repeat_end_mode', 'open_ended');
      set('repeat_count', '');
      set('repeat_until', '');
    })()`,
  );
}

async function submitFormByAction(cdp, action, label) {
  const loaded = cdp.once('Page.loadEventFired');
  const submitted = await evalExpr(
    cdp,
    `(() => {
      const form = [...document.querySelectorAll('form[action]')]
        .find((item) => item.getAttribute('action') === ${JSON.stringify(action)});
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  if (!submitted) throw new Error(`No form found for ${label}: ${action}`);
  await withTimeout(loaded, label);
}

async function clickFirstOccurrenceCancel(cdp) {
  const loaded = cdp.once('Page.loadEventFired');
  const href = await evalExpr(
    cdp,
    `(() => {
      const link = [...document.querySelectorAll('a[href]')]
        .find((a) => a.getAttribute('href').includes('/days/') && a.getAttribute('href').endsWith('/cancel'));
      if (!link) return null;
      const href = link.getAttribute('href');
      link.click();
      return href;
    })()`,
  );
  if (!href) throw new Error('No occurrence cancel link found');
  await withTimeout(loaded, `click occurrence cancel ${href}`);
  return href;
}

function allChecksPass(checks) {
  return Object.values(checks).every(Boolean);
}

function queryUiCreatedEvent() {
  const events = query(
    `SELECT id, repeat_rule, repeat_count FROM events WHERE community_id='${communityId}' AND title='${esc(uiCreatedTitle)}' ORDER BY created_at DESC LIMIT 1`,
  );
  return events[0] ?? null;
}

function queryEventDaySummary(eventId) {
  const rows = query(
    `SELECT COUNT(*) AS n, SUM(CASE WHEN occurrence_status='cancelled' THEN 1 ELSE 0 END) AS cancelled FROM event_days WHERE event_id='${esc(eventId)}'`,
  );
  return {
    count: Number(rows[0]?.n ?? 0),
    cancelled: Number(rows[0]?.cancelled ?? 0),
  };
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
  chrome = spawn(chromium, flags, {
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  chrome.stderr.on('data', (chunk) => {
    chromeStderr += chunk.toString();
  });
  await waitForDebugger(() => chromeStderr);
  logStep('sandboxed incognito Chromium is ready');

  const page = await newPage(adminSessionSecret);

  logStep('creating open-ended weekly recurrence through admin UI');
  await navigate(page, `/c/${communityId}/admin/events/new`, { textScale: 2 });
  const createForm = await collect(page);
  await fillCreateRecurrenceForm(page);
  const filledCreateForm = await collect(page);
  await submitFormByAction(page, `/c/${communityId}/admin/events`, 'submit create recurrence');
  const createdDetail = await collect(page);
  const createdEvent = queryUiCreatedEvent();
  const createdSummary = createdEvent ? queryEventDaySummary(createdEvent.id) : null;
  results.push({
    name: 'admin-ui-creates-open-ended-weekly-recurrence',
    screenshotPath: await screenshot(page, 'admin-ui-creates-open-ended-weekly-recurrence'),
    observed: {
      createFormPath: createForm.path,
      filledCreateFormValues: filledCreateForm.values,
      detailPath: createdDetail.path,
      createdEvent,
      createdSummary,
    },
    checks: {
      noHorizontalScroll: createdDetail.noHorizontalScroll,
      formHasNoDefaultEight:
        createForm.values.repeat_count === '' || createForm.values.repeat_count === undefined,
      redirectedToEventDetail:
        Boolean(createdEvent?.id) && createdDetail.path === `/c/${communityId}/events/${createdEvent.id}`,
      storedOpenEndedSummary:
        createdEvent?.repeat_rule === 'weekly' && (createdEvent.repeat_count ?? null) === null,
      materializedInitialWindow: (createdSummary?.count ?? 0) > 8,
      showsOccurrenceCancelAction: createdDetail.text.includes('この日だけ中止する'),
    },
  });

  logStep('materializing seeded rolling series through Calendar month');
  const beforeMaterialize = countRows(
    `SELECT COUNT(*) AS n FROM event_days WHERE event_id='${materializeEventId}'`,
  );
  await navigate(page, `/c/${communityId}/communities?month=2026-09`, { textScale: 2 });
  const septemberCalendar = await collect(page);
  const afterMaterialize = countRows(
    `SELECT COUNT(*) AS n FROM event_days WHERE event_id='${materializeEventId}'`,
  );
  const septemberRows = query(
    `SELECT day_date, seq FROM event_days WHERE event_id='${materializeEventId}' ORDER BY day_date ASC`,
  );
  results.push({
    name: 'calendar-materializes-rolling-open-ended-series',
    screenshotPath: await screenshot(page, 'calendar-materializes-rolling-open-ended-series'),
    observed: {
      beforeMaterialize,
      afterMaterialize,
      septemberRows,
      path: septemberCalendar.path,
    },
    checks: {
      noHorizontalScroll: septemberCalendar.noHorizontalScroll,
      rowCountIncreased: beforeMaterialize === 1 && afterMaterialize > beforeMaterialize,
      materializedThroughSeptember: septemberRows.some((row) => row.day_date === '2026-09-25'),
      calendarShowsSeededTitle: septemberCalendar.text.includes(materializeTitle),
    },
  });

  logStep('checking far-future Calendar month does not write');
  const beforeFarFuture = countRows(
    `SELECT COUNT(*) AS n FROM event_days WHERE event_id='${materializeEventId}'`,
  );
  await navigate(page, `/c/${communityId}/communities?month=2027-02`, { textScale: 2 });
  const farFutureCalendar = await collect(page);
  const afterFarFuture = countRows(
    `SELECT COUNT(*) AS n FROM event_days WHERE event_id='${materializeEventId}'`,
  );
  results.push({
    name: 'far-future-calendar-month-does-not-materialize',
    screenshotPath: await screenshot(page, 'far-future-calendar-month-does-not-materialize'),
    observed: {
      beforeFarFuture,
      afterFarFuture,
      path: farFutureCalendar.path,
      text: farFutureCalendar.text,
    },
    checks: {
      noHorizontalScroll: farFutureCalendar.noHorizontalScroll,
      rowCountUnchanged: beforeFarFuture === afterFarFuture,
      showsOutOfRangeNotice:
        farFutureCalendar.text.includes('繰り返し予定は、近い月から順に表示できるように準備します'),
    },
  });

  logStep('cancelling one materialized occurrence through admin UI');
  if (!createdEvent?.id) throw new Error('Cannot cancel occurrence without created event id');
  await navigate(page, `/c/${communityId}/events/${createdEvent.id}`, { textScale: 2 });
  const cancelHref = await clickFirstOccurrenceCancel(page);
  const cancelConfirm = await collect(page);
  await submitFormByAction(page, cancelHref, 'submit occurrence cancel');
  const afterCancel = await collect(page);
  const afterCancelSummary = queryEventDaySummary(createdEvent.id);
  const exceptions = query(
    `SELECT action, event_day_id, exception_day_date FROM event_series_exceptions WHERE community_id='${communityId}' AND action='cancel'`,
  );
  results.push({
    name: 'admin-cancels-one-recurring-occurrence',
    screenshotPath: await screenshot(page, 'admin-cancels-one-recurring-occurrence'),
    observed: {
      cancelHref,
      confirmPath: cancelConfirm.path,
      afterPath: afterCancel.path,
      afterCancelSummary,
      exceptions,
    },
    checks: {
      noHorizontalScroll: afterCancel.noHorizontalScroll,
      confirmationShown: cancelConfirm.text.includes('この日だけ中止'),
      returnedToEventDetail: afterCancel.path === `/c/${communityId}/events/${createdEvent.id}`,
      cancelledOneOccurrence: afterCancelSummary.cancelled === 1,
      exceptionRecorded: exceptions.length === 1 && exceptions[0].action === 'cancel',
      detailShowsCancelledBadge: afterCancel.text.includes('この日は中止です'),
    },
  });

  page.close();

  for (const result of results) {
    result.passed = allChecksPass(result.checks);
  }

  const report = {
    generatedAt: new Date().toISOString(),
    chromium,
    baseUrl,
    userDataDir,
    flags,
    note: 'Chromium launched with --incognito and without --no-sandbox. Local wrangler dev only.',
    localOnlyGuard: true,
    coverage: [
      'admin recurrence creation through browser form',
      'repeat count default is blank, not 8',
      'Calendar-triggered rolling materialization',
      'far-future Calendar no-write behavior',
      'occurrence-only cancellation through browser form',
    ],
    results,
    passed: results.every((result) => result.passed),
  };

  await writeFile(`${outDir}/${reportName}`, JSON.stringify(report, null, 2));
  console.log(
    JSON.stringify(
      {
        passed: report.passed,
        results: results.map((result) => ({
          name: result.name,
          passed: result.passed,
          checks: result.checks,
        })),
      },
      null,
      2,
    ),
  );

  if (!report.passed) process.exitCode = 1;
} catch (error) {
  if (devStderr.trim()) {
    console.error('[recurrence-v2-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[recurrence-v2-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (dev && dev.exitCode === null) dev.kill('SIGTERM');
  if (chrome && chrome.exitCode === null) chrome.kill('SIGTERM');
  await rm(userDataDir, { recursive: true, force: true }).catch(() => {});
}
