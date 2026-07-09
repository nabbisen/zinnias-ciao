#!/usr/bin/env node
// Scenario smoke for RFC-066 event-copy workflows. Local wrangler dev only.

import { createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8797);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9249);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc066';
const reportName = process.env.REPORT_NAME ?? 'rfc066-event-copy-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-event-copy-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const pepper = 'dev-pepper-change-in-production';
const now = '2026-07-09T00:00:00.000Z';

const communityId = 'com_rfc066_primary';
const secondCommunityId = 'com_rfc066_second';
const adminUserId = 'usr_rfc066_admin';
const memberUserId = 'usr_rfc066_member';
const adminMembershipId = 'mem_rfc066_admin';
const adminSecondMembershipId = 'mem_rfc066_admin_second';
const memberMembershipId = 'mem_rfc066_member';
const adminSessionSecret = 'rfc066-smoke-admin-session';
const memberSessionSecret = 'rfc066-smoke-member-session';
const adminSessionHmac = createHmac('sha256', pepper).update(adminSessionSecret).digest('hex');
const memberSessionHmac = createHmac('sha256', pepper).update(memberSessionSecret).digest('hex');

const singleEventId = 'evt_rfc066_single';
const singleDayId = 'day_rfc066_single';
const multiEventId = 'evt_rfc066_multi';
const cancelledEventId = 'evt_rfc066_cancelled';
const recurringEventId = 'evt_rfc066_recurring_past';
const recurringSeriesId = 'ser_rfc066_recurring_past';
const singleTitle = 'RFC066 Single Source';
const multiTitle = 'RFC066 Multi Source';
const cancelledTitle = 'RFC066 Cancelled Source';
const recurringTitle = 'RFC066 Past Recurring Source';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`event-copy smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`event-copy smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) throw new Error('event-copy smoke refuses remote D1 operations');
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
  sql(`DELETE FROM event_series_exceptions WHERE community_id IN ('${communityId}','${secondCommunityId}') OR series_id='${recurringSeriesId}'`);
  sql(`DELETE FROM attendances WHERE event_day_id IN (SELECT id FROM event_days WHERE community_id IN ('${communityId}','${secondCommunityId}'))`);
  sql(`DELETE FROM event_notes WHERE event_id IN (SELECT id FROM events WHERE community_id IN ('${communityId}','${secondCommunityId}'))`);
  sql(`DELETE FROM event_days WHERE community_id IN ('${communityId}','${secondCommunityId}')`);
  sql(`DELETE FROM event_series WHERE community_id IN ('${communityId}','${secondCommunityId}')`);
  sql(`DELETE FROM events WHERE community_id IN ('${communityId}','${secondCommunityId}')`);
  sql(`DELETE FROM audit_log WHERE community_id IN ('${communityId}','${secondCommunityId}')`);
  sql(`DELETE FROM sessions WHERE id IN ('sess_rfc066_admin','sess_rfc066_member') OR session_hmac IN ('${adminSessionHmac}','${memberSessionHmac}')`);
  sql(`DELETE FROM form_tokens WHERE user_id IN ('${adminUserId}','${memberUserId}')`);
  sql(`DELETE FROM community_memberships WHERE community_id IN ('${communityId}','${secondCommunityId}') OR id IN ('${adminMembershipId}','${adminSecondMembershipId}','${memberMembershipId}')`);
  sql(`DELETE FROM users WHERE id IN ('${adminUserId}','${memberUserId}')`);
  sql(`DELETE FROM communities WHERE id IN ('${communityId}','${secondCommunityId}')`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'RFC066 Primary Community', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${secondCommunityId}', 'RFC066 Second Community', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${adminUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminMembershipId}', '${communityId}', '${adminUserId}', 'admin', 'RFC066 Admin', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminSecondMembershipId}', '${secondCommunityId}', '${adminUserId}', 'admin', 'RFC066 Admin Second', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${communityId}', '${memberUserId}', 'member', 'RFC066 Member', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc066_admin', '${adminUserId}', '${adminSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc066_member', '${memberUserId}', '${memberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    eventInsert(singleEventId, singleTitle, 'Main room', 'Single source description', 'scheduled'),
    dayInsert(singleDayId, singleEventId, 1, '2026-07-20', '2026-07-20T01:00:00.000Z', '2026-07-20T02:00:00.000Z'),
    eventInsert(multiEventId, multiTitle, 'Multi room', 'Multi source description', 'scheduled'),
    dayInsert('day_rfc066_multi_1', multiEventId, 1, '2026-07-21', '2026-07-21T01:00:00.000Z', '2026-07-21T02:00:00.000Z'),
    dayInsert('day_rfc066_multi_2', multiEventId, 2, '2026-07-22', '2026-07-22T01:00:00.000Z', '2026-07-22T02:00:00.000Z'),
    eventInsert(cancelledEventId, cancelledTitle, 'Cancelled room', 'Cancelled source description', 'cancelled'),
    dayInsert('day_rfc066_cancelled', cancelledEventId, 1, '2026-07-23', '2026-07-23T01:00:00.000Z', '2026-07-23T02:00:00.000Z'),
    eventInsert(recurringEventId, recurringTitle, 'Recurring room', 'Recurring source description', 'scheduled', 'weekly'),
    `INSERT INTO event_series (id, event_id, community_id, frequency, start_day_date, starts_at_local, ends_at_local, timezone, end_mode, occurrence_count, until_day_date, materialized_through_day_date, created_at, updated_at) VALUES ('${recurringSeriesId}', '${recurringEventId}', '${communityId}', 'weekly', '2026-06-01', '10:00', '11:00', 'Asia/Tokyo', 'after_count', 6, NULL, '2026-07-06', '${now}', '${now}')`,
    dayInsert('day_rfc066_recurring_1', recurringEventId, 1, '2026-06-01', '2026-06-01T01:00:00.000Z', '2026-06-01T02:00:00.000Z', recurringSeriesId, '2026-06-01'),
    `INSERT INTO attendances (id, event_day_id, membership_id, status, status_updated_at, updated_at) VALUES ('att_rfc066_single_member', '${singleDayId}', '${memberMembershipId}', 'going', '${now}', '${now}')`,
    `INSERT INTO event_notes (id, event_id, membership_id, note, note_updated_at) VALUES ('note_rfc066_single_member', '${singleEventId}', '${memberMembershipId}', 'Do not copy this note', '${now}')`,
  ];
  for (const statement of statements) sql(statement);
}

function eventInsert(id, title, location, description, status, repeatRule = 'none') {
  return `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${id}', '${communityId}', '${adminMembershipId}', '${esc(title)}', '${esc(location)}', '${esc(description)}', '${status}', '${repeatRule}', NULL, '${now}', '${now}')`;
}

function dayInsert(id, eventId, seq, dayDate, startsAt, endsAt, seriesId = null, seriesDate = null) {
  const seriesColumns = seriesId
    ? `, occurrence_status, series_id, series_occurrence_date`
    : `, occurrence_status`;
  const seriesValues = seriesId
    ? `, 'scheduled', '${seriesId}', '${seriesDate}'`
    : `, 'scheduled'`;
  return `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at${seriesColumns}) VALUES ('${id}', '${eventId}', '${communityId}', ${seq}, '${dayDate}', '${startsAt}', '${endsAt}', '${now}'${seriesValues})`;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[event-copy-smoke] ${message}`);
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
      mobile: true,
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
      const fields = [...document.querySelectorAll('input[name], textarea[name], select[name]')];
      const links = [...document.querySelectorAll('a[href]')].map((a) => ({
        href: a.getAttribute('href'),
        text: a.innerText,
      }));
      const buttons = [...document.querySelectorAll('button')].map((b) => ({
        text: b.innerText,
        rect: b.getBoundingClientRect().toJSON(),
      }));
      return {
        path: location.pathname + location.search,
        text: document.body.innerText,
        hrefs: links.map((link) => link.href),
        links,
        buttons,
        values: Object.fromEntries(fields.map((el) => [el.getAttribute('name'), el.value])),
        noHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
      };
    })()`,
  );
}

async function submitFirstForm(cdp, label) {
  const loaded = cdp.once('Page.loadEventFired');
  const submitted = await evalExpr(
    cdp,
    `(() => {
      const form = document.querySelector('form');
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  if (!submitted) throw new Error(`No form found for ${label}`);
  await withTimeout(loaded, label);
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

function countRows(statement) {
  const rows = query(statement);
  return Number(rows[0]?.n ?? rows[0]?.N ?? 0);
}

function copiedSingleEvent() {
  return query(
    `SELECT id FROM events WHERE community_id='${communityId}' AND title='${esc(singleTitle)}' AND id!='${singleEventId}' ORDER BY created_at DESC LIMIT 1`,
  )[0] ?? null;
}

function eventDayIds(eventId) {
  return query(`SELECT id FROM event_days WHERE event_id='${esc(eventId)}' ORDER BY seq ASC`).map(
    (row) => row.id,
  );
}

function latestCopyAudit(eventId) {
  return query(
    `SELECT metadata_json FROM audit_log WHERE target_kind='event' AND target_id='${esc(eventId)}' AND action='created' ORDER BY created_at DESC LIMIT 1`,
  )[0]?.metadata_json ?? '';
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

  const adminPage = await newPage(adminSessionSecret);
  const memberPage = await newPage(memberSessionSecret);

  logStep('checking admin copy action across source states');
  await setSession(adminPage, adminSessionSecret);
  await navigate(adminPage, `/c/${communityId}/events/${singleEventId}`, { textScale: 2 });
  const adminSingleDetail = await collect(adminPage);
  await setSession(adminPage, adminSessionSecret);
  await navigate(adminPage, `/c/${communityId}/events/${recurringEventId}`, { textScale: 2 });
  const adminRecurringDetail = await collect(adminPage);
  await setSession(adminPage, adminSessionSecret);
  await navigate(adminPage, `/c/${communityId}/events/${cancelledEventId}`, { textScale: 2 });
  const adminCancelledDetail = await collect(adminPage);
  results.push({
    name: 'admin-sees-copy-action-for-scheduled-past-recurring-and-cancelled',
    screenshotPath: await screenshot(adminPage, 'admin-sees-copy-action-for-cancelled'),
    observed: {
      scheduledPath: adminSingleDetail.path,
      recurringPath: adminRecurringDetail.path,
      cancelledPath: adminCancelledDetail.path,
    },
    checks: {
      scheduledHasCopy: adminSingleDetail.hrefs.includes(`/c/${communityId}/admin/events/${singleEventId}/copy`),
      pastRecurringHasCopy: adminRecurringDetail.hrefs.includes(`/c/${communityId}/admin/events/${recurringEventId}/copy`),
      cancelledHasCopy: adminCancelledDetail.hrefs.includes(`/c/${communityId}/admin/events/${cancelledEventId}/copy`),
      cancelledKeepsRecreate: adminCancelledDetail.hrefs.includes(`/c/${communityId}/admin/events/${cancelledEventId}/recreate`),
      noHorizontalScroll: adminCancelledDetail.noHorizontalScroll,
    },
  });

  logStep('checking member cannot see or access copy');
  await setSession(memberPage, memberSessionSecret);
  await navigate(memberPage, `/c/${communityId}/events/${singleEventId}`, { textScale: 2 });
  const memberDetail = await collect(memberPage);
  await setSession(memberPage, memberSessionSecret);
  await navigate(memberPage, `/c/${communityId}/admin/events/${singleEventId}/copy`, {
    textScale: 2,
  });
  const memberDirectCopy = await collect(memberPage);
  results.push({
    name: 'non-admin-copy-action-and-direct-url-are-denied',
    screenshotPath: await screenshot(memberPage, 'non-admin-copy-direct-url-denied'),
    observed: {
      detailPath: memberDetail.path,
      directPath: memberDirectCopy.path,
      directText: memberDirectCopy.text,
    },
    checks: {
      detailHidesCopy: !memberDetail.hrefs.includes(`/c/${communityId}/admin/events/${singleEventId}/copy`),
      directDoesNotShowCopyForm: !memberDirectCopy.values.copy_source_event_id && !memberDirectCopy.text.includes(singleTitle),
      noHorizontalScroll: memberDirectCopy.noHorizontalScroll,
    },
  });

  logStep('checking single-day copy prefill and successful create');
  await setSession(adminPage, adminSessionSecret);
  await navigate(adminPage, `/c/${communityId}/admin/events/${singleEventId}/copy`, {
    textScale: 2,
  });
  const singleCopyForm = await collect(adminPage);
  await submitFormByAction(adminPage, `/c/${communityId}/admin/events`, 'submit single copy');
  const afterSingleCopy = await collect(adminPage);
  const copied = copiedSingleEvent();
  const copiedDayIds = copied ? eventDayIds(copied.id) : [];
  const copiedAttendanceCount = copied
    ? countRows(
        `SELECT COUNT(*) AS n FROM attendances WHERE event_day_id IN (SELECT id FROM event_days WHERE event_id='${esc(copied.id)}')`,
      )
    : -1;
  const copiedNoteCount = copied
    ? countRows(`SELECT COUNT(*) AS n FROM event_notes WHERE event_id='${esc(copied.id)}'`)
    : -1;
  const copiedExceptionCount = copied
    ? countRows(
        `SELECT COUNT(*) AS n FROM event_series_exceptions WHERE event_day_id IN (SELECT id FROM event_days WHERE event_id='${esc(copied.id)}')`,
      )
    : -1;
  const auditMetadata = copied ? latestCopyAudit(copied.id) : '';
  results.push({
    name: 'single-day-copy-prefills-and-creates-fresh-event-without-member-state',
    screenshotPath: await screenshot(adminPage, 'single-day-copy-created-event'),
    observed: {
      formPath: singleCopyForm.path,
      values: singleCopyForm.values,
      afterPath: afterSingleCopy.path,
      copied,
      copiedDayIds,
      copiedAttendanceCount,
      copiedNoteCount,
      copiedExceptionCount,
      auditMetadata,
    },
    checks: {
      formHasCopyMode: singleCopyForm.values.copy_mode === 'event_copy',
      prefilledDetails:
        singleCopyForm.values.title === singleTitle &&
        singleCopyForm.values.location === 'Main room' &&
        singleCopyForm.values.description === 'Single source description',
      prefilledSchedule:
        singleCopyForm.values.day_date === '2026-07-20' &&
        singleCopyForm.values.starts_at === '10:00' &&
        singleCopyForm.values.ends_at === '11:00',
      redirectedToCopiedEvent: Boolean(copied?.id) && afterSingleCopy.path === `/c/${communityId}/events/${copied.id}`,
      newDayIds: copiedDayIds.length === 1 && copiedDayIds[0] !== singleDayId,
      noAttendanceCopied: copiedAttendanceCount === 0,
      noNotesCopied: copiedNoteCount === 0,
      noExceptionsCopied: copiedExceptionCount === 0,
      auditMetadataMinimal:
        auditMetadata.includes('"copy_source_event_id":"evt_rfc066_single"') &&
        auditMetadata.includes('"copy_mode":"event_copy"') &&
        !auditMetadata.includes('description') &&
        !auditMetadata.includes('Do not copy this note'),
      noHorizontalScroll: afterSingleCopy.noHorizontalScroll,
    },
  });

  logStep('checking multi-day and past-recurring copy forms');
  await setSession(adminPage, adminSessionSecret);
  logStep('checking multi-day copy form');
  await navigate(adminPage, `/c/${communityId}/admin/events/${multiEventId}/copy`, {
    textScale: 2,
  });
  const multiCopyForm = await collect(adminPage);
  await setSession(adminPage, adminSessionSecret);
  logStep('checking past-recurring copy form');
  await navigate(adminPage, `/c/${communityId}/admin/events/${recurringEventId}/copy`, {
    textScale: 2,
  });
  const recurringCopyForm = await collect(adminPage);
  results.push({
    name: 'multi-day-and-past-recurring-copy-prefill-rules',
    screenshotPath: await screenshot(adminPage, 'past-recurring-copy-form'),
    observed: {
      multi: multiCopyForm.values,
      multiText: multiCopyForm.text,
      recurring: recurringCopyForm.values,
      recurringText: recurringCopyForm.text,
    },
    checks: {
      multiCopiesDetails:
        multiCopyForm.values.title === multiTitle &&
        multiCopyForm.values.location === 'Multi room' &&
        multiCopyForm.values.description === 'Multi source description',
      multiScheduleBlank:
        multiCopyForm.values.day_date === '' &&
        multiCopyForm.values.starts_at === '' &&
        multiCopyForm.values.ends_at === '',
      multiShowsHelper: multiCopyForm.text.includes('複数日の予定です'),
      recurringCopiesTemplate:
        recurringCopyForm.values.title === recurringTitle &&
        recurringCopyForm.values.repeat_rule === 'weekly' &&
        recurringCopyForm.values.starts_at === '10:00' &&
        recurringCopyForm.values.ends_at === '11:00',
      recurringResetsInvalidSchedule:
        recurringCopyForm.values.day_date === '' &&
        recurringCopyForm.values.repeat_end_mode === 'open_ended' &&
        recurringCopyForm.values.repeat_count === '' &&
        recurringCopyForm.values.repeat_until === '',
      recurringShowsPastHelper: recurringCopyForm.text.includes('繰り返しの開始日が過去'),
      noHorizontalScroll: multiCopyForm.noHorizontalScroll && recurringCopyForm.noHorizontalScroll,
    },
  });

  logStep('checking community switcher drops copy state');
  await setSession(adminPage, adminSessionSecret);
  await navigate(adminPage, `/switch?community=${secondCommunityId}&next=admin_events_new`, {
    textScale: 2,
  });
  const switchedCreate = await collect(adminPage);
  results.push({
    name: 'community-switcher-drops-copy-state',
    screenshotPath: await screenshot(adminPage, 'switcher-drops-copy-state'),
    observed: {
      path: switchedCreate.path,
      values: switchedCreate.values,
    },
    checks: {
      landsOnNormalCreate: switchedCreate.path === `/c/${secondCommunityId}/admin/events/new`,
      noCopySource: !switchedCreate.values.copy_source_event_id && !switchedCreate.values.copy_mode,
      noHorizontalScroll: switchedCreate.noHorizontalScroll,
    },
  });

  adminPage.close();
  memberPage.close();

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
      'admin copy action on scheduled, past recurring, and cancelled sources',
      'non-admin absence and direct URL denial',
      'single-day prefill and successful copied create',
      'no copied attendance, notes, exceptions, or source day IDs',
      'minimal event-copy audit metadata',
      'multi-day details-only schedule reset',
      'past recurring normalization',
      'community switcher drops source-copy state',
      'mobile 390px viewport with 200% text scaling',
    ],
    results,
    passed: results.every((result) => result.passed),
  };

  await writeFile(`${outDir}/${reportName}`, JSON.stringify(report, null, 2));
  console.log(
    JSON.stringify(
      {
        passed: report.passed,
        report: `${outDir}/${reportName}`,
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
    console.error('[event-copy-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[event-copy-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  console.error(error);
  process.exitCode = 1;
} finally {
  if (chrome && chrome.exitCode === null) chrome.kill('SIGTERM');
  if (dev && dev.exitCode === null) dev.kill('SIGTERM');
  await sleep(500);
  await rm(userDataDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
}
