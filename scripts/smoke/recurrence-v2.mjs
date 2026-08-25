#!/usr/bin/env node
// Scenario smoke for recurrence v2 workflows. Local wrangler dev only.

import { prepareIsolatedWorkerTest } from "../lib/isolated-worker-test.mjs";
import { PIN_FIXTURE_UI_LANGUAGE_TO_JAPANESE_SQL } from "../lib/smoke-fixture-locale.mjs";
import { SMOKE_ACCEPT_LANGUAGE } from "../lib/smoke-locale.mjs";

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
const isolated = await prepareIsolatedWorkerTest("recurrence-v2");
const pepper = isolated.pepper;
const now = '2026-07-09T00:00:00.000Z';

// Handoff 063: the two Calendar-month checks below must be derived from the
// real wall clock, not a fixed literal — a hardcoded "far future" month
// eventually enters the rolling materialization horizon (it did, silently,
// once the real date passed 2026-08), and a hardcoded "near future" month
// eventually falls out of relevance the same way. `now` above is unrelated:
// it only timestamps fixture rows (created_at/joined_at), which the
// materialization window never reads.
//
// RECURRENCE_MATERIALIZATION_MONTHS_AHEAD must equal
// packages/domain/src/event_admin.rs's constant of the same name — pinned by
// rfc065_recurrence_smoke_pins_the_materialization_horizon_constant in
// release_gates.rs, which reads the live Rust value and fails if this
// literal drifts from it.
const RECURRENCE_MATERIALIZATION_MONTHS_AHEAD = 6;
const FAR_FUTURE_MARGIN_MONTHS = 2;

const realNow = new Date();
const pad2 = (n) => String(n).padStart(2, '0');
const fmtDate = (d) => `${d.getUTCFullYear()}-${pad2(d.getUTCMonth() + 1)}-${pad2(d.getUTCDate())}`;
const fmtMonth = (d) => `${d.getUTCFullYear()}-${pad2(d.getUTCMonth() + 1)}`;
const addDays = (d, days) => new Date(d.getTime() + days * 24 * 60 * 60 * 1000);
const addMonths = (d, months) => {
  const r = new Date(d.getTime());
  r.setUTCMonth(r.getUTCMonth() + months);
  return r;
};

// Series start: one day ahead of "today" (mirrors the original fixture's
// now/start relationship). Always well inside the horizon regardless of
// when this runs, since everything below is computed from this same
// `realNow`.
const materializeSeriesStartDate = addDays(realNow, 1);
const materializeSeriesStartDayDate = fmtDate(materializeSeriesStartDate);

// Near-future: the occurrence 11 weeks (77 days) after series start — the
// same distance the original fixed dates used (2026-07-10 -> 2026-09-25).
// 77 days is always comfortably inside a 6-month horizon, so this month
// (and the exact day within it) never goes stale.
const nearOccurrenceDate = addDays(materializeSeriesStartDate, 77);
const nearMaterializeMonth = fmtMonth(nearOccurrenceDate);
const nearOccurrenceDayDate = fmtDate(nearOccurrenceDate);

// Far-future: today + (horizon + margin) months — always outside the
// rolling window, by construction, no matter what today is.
const farFutureMonth = fmtMonth(
  addMonths(realNow, RECURRENCE_MATERIALIZATION_MONTHS_AHEAD + FAR_FUTURE_MARGIN_MONTHS),
);

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
const uiCreatedDayDate = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000)
  .toISOString()
  .slice(0, 10);
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
    return isolated.runWranglerSync(args, {
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
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance) VALUES ('sess_rfc065_admin', '${adminUserId}', '${adminSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}', 'invite_redemption')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance) VALUES ('sess_rfc065_member', '${memberUserId}', '${memberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}', 'invite_redemption')`,
    `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${materializeEventId}', '${communityId}', '${adminMembershipId}', '${materializeTitle}', 'Local room', '', 'scheduled', 'weekly', NULL, '${now}', '${now}')`,
    `INSERT INTO event_series (id, event_id, community_id, frequency, start_day_date, starts_at_local, ends_at_local, timezone, end_mode, occurrence_count, until_day_date, materialized_through_day_date, created_at, updated_at) VALUES ('${materializeSeriesId}', '${materializeEventId}', '${communityId}', 'weekly', '${materializeSeriesStartDayDate}', '09:00', '10:00', 'Asia/Tokyo', 'open_ended', NULL, NULL, '${materializeSeriesStartDayDate}', '${now}', '${now}')`,
    `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, occurrence_status, series_id, series_occurrence_date) VALUES ('${materializeDayId}', '${materializeEventId}', '${communityId}', 1, '${materializeSeriesStartDayDate}', '${materializeSeriesStartDayDate}T00:00:00.000Z', '${materializeSeriesStartDayDate}T01:00:00.000Z', '${now}', 'scheduled', '${materializeSeriesId}', '${materializeSeriesStartDayDate}')`,
  ];
  for (const statement of statements) sql(statement);
  sql(PIN_FIXTURE_UI_LANGUAGE_TO_JAPANESE_SQL);
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
    headers: { Cookie: `ciao_sid=${sessionSecret}`, "Accept-Language": SMOKE_ACCEPT_LANGUAGE },
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
      const links = [...document.querySelectorAll('a[href]')].map((a) => ({
        href: a.getAttribute('href'),
        text: a.innerText,
      }));
      return {
        path: location.pathname + location.search,
        text: document.body.innerText,
        hrefs: links.map((link) => link.href),
        links,
        noHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
      };
    })()`,
  );
}

// Handoff 066 §3.1: an explicit, named read of one field's value, used only
// where an assertion genuinely needs it — never folded into `collect()`,
// whose return value is what reaches `observed` and therefore evidence.
async function readFormValue(cdp, name) {
  return await evalExpr(
    cdp,
    `(() => {
      const el = document.querySelector('[name="' + ${JSON.stringify(name)} + '"]');
      return el ? el.value : null;
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
      set('day_date', ${JSON.stringify(uiCreatedDayDate)});
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
  const createFormRepeatCount = await readFormValue(page, 'repeat_count');
  await fillCreateRecurrenceForm(page);
  await submitFormByAction(page, `/c/${communityId}/admin/events`, 'submit create recurrence');
  const createdDetail = await collect(page);
  const createdEvent = queryUiCreatedEvent();
  const createdSummary = createdEvent ? queryEventDaySummary(createdEvent.id) : null;
  results.push({
    name: 'admin-ui-creates-open-ended-weekly-recurrence',
    screenshotPath: await screenshot(page, 'admin-ui-creates-open-ended-weekly-recurrence'),
    observed: {
      createFormPath: createForm.path,
      detailPath: createdDetail.path,
      createdEvent,
      createdSummary,
    },
    checks: {
      noHorizontalScroll: createdDetail.noHorizontalScroll,
      formHasNoDefaultEight: createFormRepeatCount === '' || createFormRepeatCount == null,
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
  await navigate(page, `/c/${communityId}/communities?month=${nearMaterializeMonth}`, {
    textScale: 2,
  });
  const nearMonthCalendar = await collect(page);
  const afterMaterialize = countRows(
    `SELECT COUNT(*) AS n FROM event_days WHERE event_id='${materializeEventId}'`,
  );
  const nearMonthRows = query(
    `SELECT day_date, seq FROM event_days WHERE event_id='${materializeEventId}' ORDER BY day_date ASC`,
  );
  results.push({
    name: 'calendar-materializes-rolling-open-ended-series',
    screenshotPath: await screenshot(page, 'calendar-materializes-rolling-open-ended-series'),
    observed: {
      beforeMaterialize,
      afterMaterialize,
      nearMaterializeMonth,
      nearOccurrenceDayDate,
      nearMonthRows,
      path: nearMonthCalendar.path,
    },
    checks: {
      noHorizontalScroll: nearMonthCalendar.noHorizontalScroll,
      rowCountIncreased: beforeMaterialize === 1 && afterMaterialize > beforeMaterialize,
      materializedThroughNearMonth: nearMonthRows.some(
        (row) => row.day_date === nearOccurrenceDayDate,
      ),
    },
  });

  // Handoff 070 Part A: RFC-073 (ed549be, 2026-07-29) moved event titles out
  // of the month grid into the day-detail panel and the list view — the
  // month grid's only `title` is the page heading. Do not repoint the month
  // navigation above to `&view=list`: that visit is what triggers
  // materialization (rowCountIncreased depends on it), and it is RFC-011
  // accessibility coverage for the hardest layout in the product to keep
  // scroll-free. This is a second, separate visit, list-view-only.
  logStep('checking Calendar list view shows the seeded title');
  await navigate(
    page,
    `/c/${communityId}/communities?month=${nearMaterializeMonth}&view=list`,
    { textScale: 2 },
  );
  const nearMonthList = await collect(page);
  results.push({
    name: 'calendar-list-view-shows-seeded-title',
    screenshotPath: await screenshot(page, 'calendar-list-view-shows-seeded-title'),
    observed: {
      path: nearMonthList.path,
    },
    checks: {
      listViewNoHorizontalScroll: nearMonthList.noHorizontalScroll,
      listViewShowsSeededTitle: nearMonthList.text.includes(materializeTitle),
    },
  });

  logStep('checking far-future Calendar month does not write');
  const beforeFarFuture = countRows(
    `SELECT COUNT(*) AS n FROM event_days WHERE event_id='${materializeEventId}'`,
  );
  await navigate(page, `/c/${communityId}/communities?month=${farFutureMonth}`, { textScale: 2 });
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
      farFutureMonth,
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
  await isolated.cleanup();
}
