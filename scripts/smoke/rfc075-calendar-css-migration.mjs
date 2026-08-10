#!/usr/bin/env node
// Screenshot evidence for RFC-075 Slice 1 (Calendar CSS migration): every
// migrated surface at mobile width and 200% text, in both Japanese and
// English. Local wrangler dev only. Functional behavior is already covered
// by smoke:calendar-views, smoke:matrix, smoke:matrix-csv,
// smoke:community-switch, and smoke:language — this script exists only to
// capture the required visual evidence for the CSS extraction itself.

import { prepareIsolatedWorkerTest } from "../lib/isolated-worker-test.mjs";

import { createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8799);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9251);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc075';
const reportName = process.env.REPORT_NAME ?? 'rfc075-calendar-css-migration-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-rfc075-calendar-css-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const isolated = await prepareIsolatedWorkerTest("rfc075-calendar-css-migration");
const pepper = isolated.pepper;
const now = '2026-07-30T00:00:00.000Z';

const communityId = 'com_rfc075_primary';
const memberUserId = 'usr_rfc075_member';
const memberMembershipId = 'mem_rfc075_member';
const eventId = 'evt_rfc075_visible';
const eventDayId = 'day_rfc075_visible';
const memberSessionSecret = 'rfc075-smoke-member-session';
const memberSessionHmac = hmac(memberSessionSecret);

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function hmac(secret) {
  return createHmac('sha256', pepper).update(secret).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`rfc075 evidence smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`rfc075 evidence smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) {
    throw new Error('rfc075 evidence smoke refuses remote D1 operations');
  }
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
    'd1', 'execute', 'zinnias-ciao-dev', '--local', '--env', 'dev', '--command', statement,
  ]);
}

function esc(value) {
  return String(value).replaceAll("'", "''");
}

function clean() {
  sql(`DELETE FROM attendances WHERE event_day_id = '${eventDayId}'`);
  sql(`DELETE FROM event_days WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM events WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM audit_log WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM form_tokens WHERE user_id = '${memberUserId}'`);
  sql(`DELETE FROM sessions WHERE session_hmac = '${memberSessionHmac}' OR user_id = '${memberUserId}'`);
  sql(`DELETE FROM community_memberships WHERE id = '${memberMembershipId}' OR community_id = '${communityId}'`);
  sql(`DELETE FROM users WHERE id = '${memberUserId}'`);
  sql(`DELETE FROM communities WHERE id = '${communityId}'`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'RFC075 Primary', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${communityId}', '${memberUserId}', 'admin', '${esc('RFC075 Admin')}', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance) VALUES ('sess_rfc075_member', '${memberUserId}', '${memberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}', 'invite_redemption')`,
    `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${eventId}', '${communityId}', '${memberMembershipId}', 'RFC075 Visible Event', 'RFC075 Room', '', 'scheduled', 'none', NULL, '${now}', '${now}')`,
    // 2026-08-03T00:00:00Z is 09:00 JST on Monday, 3 Aug 2026 — same fixture
    // date used by the RFC-072 native tests and smoke:language.
    `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, occurrence_status) VALUES ('${eventDayId}', '${eventId}', '${communityId}', 1, '2026-08-03', '2026-08-03T00:00:00.000Z', '2026-08-03T01:00:00.000Z', '${now}', 'scheduled')`,
  ];
  for (const statement of statements) sql(statement);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[rfc075-evidence-smoke] ${message}`);
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
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: options.width ?? 390,
    height: options.height ?? 900,
    deviceScaleFactor: 1,
    mobile: options.mobile ?? true,
  });
  const loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}${path}` });
  await withTimeout(loaded, `navigate ${path}`);
  if (options.textScale === 2) {
    await evalExpr(cdp, `(() => { document.documentElement.style.fontSize = '200%'; })()`);
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
  const shot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
  const path = `${outDir}/${name}.png`;
  await writeFile(path, Buffer.from(shot.data, 'base64'));
  return path;
}

async function collect(cdp) {
  return await evalExpr(
    cdp,
    `(() => ({
      htmlLang: document.documentElement.getAttribute('lang'),
      noHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
    }))()`,
  );
}

async function setLanguage(cdp, locale) {
  await navigate(cdp, `/c/${communityId}/me/language`);
  await evalExpr(
    cdp,
    `(() => {
      const radio = document.querySelector(${JSON.stringify(`input[name="ui_language"][value="${locale}"]`)});
      radio.checked = true;
      radio.dispatchEvent(new Event('change', { bubbles: true }));
      const form = [...document.querySelectorAll('form[action]')].find(
        (item) => item.getAttribute('action') === ${JSON.stringify(`/c/${communityId}/me/language`)},
      );
      form.requestSubmit();
    })()`,
  );
  await sleep(150);
}

function allChecksPass(checks) {
  return Object.values(checks).every(Boolean);
}

let dev;
let chrome;
let devStderr = '';
let chromeStderr = '';
const results = [];

const surfaces = [
  { name: 'calendar-month', path: `/c/${communityId}/communities?month=2026-08` },
  { name: 'calendar-list', path: `/c/${communityId}/communities?month=2026-08&view=list` },
  { name: 'calendar-matrix', path: `/c/${communityId}/communities?month=2026-08&view=matrix` },
];

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

  const page = await newPage(memberSessionSecret);

  for (const locale of ['ja', 'en']) {
    logStep(`setting language to ${locale}`);
    await setLanguage(page, locale);

    for (const surface of surfaces) {
      logStep(`capturing ${surface.name} (${locale}, mobile, 200% text)`);
      await navigate(page, surface.path, { textScale: 2 });
      const observed = await collect(page);
      const screenshotPath = await screenshot(page, `${surface.name}-${locale}-mobile-200-percent`);
      results.push({
        name: `${surface.name}-${locale}`,
        screenshotPath,
        observed,
        checks: {
          htmlLangMatches: observed.htmlLang === locale,
          noHorizontalScrollAt200Percent: observed.noHorizontalScroll,
        },
      });
    }
  }

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
    note: 'Evidence capture only (screenshots + no-horizontal-scroll-at-200% check). Functional behavior is covered by smoke:calendar-views, smoke:matrix, smoke:matrix-csv, smoke:community-switch, and smoke:language, all re-run unmodified for this slice.',
    localOnlyGuard: true,
    results,
    passed: results.every((result) => result.passed),
  };

  await writeFile(`${outDir}/${reportName}`, JSON.stringify(report, null, 2));
  console.log(
    JSON.stringify(
      {
        passed: report.passed,
        results: results.map((result) => ({ name: result.name, passed: result.passed, checks: result.checks })),
      },
      null,
      2,
    ),
  );

  if (!report.passed) process.exitCode = 1;
} catch (error) {
  if (devStderr.trim()) {
    console.error('[rfc075-evidence-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[rfc075-evidence-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  await isolated.cleanup();
}
