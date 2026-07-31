#!/usr/bin/env node
// Screenshot evidence for RFC-075 Slice 5 (admin member management CSS
// migration): the member list (the §7.3 table-overflow risk — measured
// numerically at both 100% and 200%), the invites page, the promote
// confirmation, the remove confirmation, and both help-signin renders
// (confirm + the synthetic relink-code reveal). Japanese only, by
// documented decision (RFC-072 Slice D) — no English rendering exists for
// any of these pages. Functional behavior is already covered by
// smoke:calendar-views, smoke:matrix, smoke:matrix-csv,
// smoke:community-switch, smoke:language, and smoke:admin-event-forms, all
// re-run unmodified for this slice — this script exists only to capture
// the required visual evidence and numeric 200% margins.

import { prepareIsolatedWorkerTest } from "../lib/isolated-worker-test.mjs";

import { createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8799);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9251);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc075-slice5';
const reportName = process.env.REPORT_NAME ?? 'rfc075-slice5-admin-member-management-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-rfc075-slice5-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const isolated = await prepareIsolatedWorkerTest("rfc075-slice5-admin-member-management");
const pepper = isolated.pepper;
const now = '2026-07-31T00:00:00.000Z';

const communityId = 'com_rfc075s5_primary';
const adminUserId = 'usr_rfc075s5_admin';
const adminMembershipId = 'mem_rfc075s5_admin';
const secondAdminMembershipId = 'mem_rfc075s5_admin2';
const secondAdminUserId = 'usr_rfc075s5_admin2';
const memberUserId = 'usr_rfc075s5_member';
const memberMembershipId = 'mem_rfc075s5_member';
const adminSessionSecret = 'rfc075s5-smoke-admin-session';
const adminSessionHmac = hmac(adminSessionSecret);
// Deliberately long, to stress the member row's overflow-wrap:anywhere at
// 200% text — this is the §7.3 risk (a table/list row cannot wrap the way
// a flex button row can).
const longMemberDisplayName = 'RFC075 Slice 5 Member With An Unusually Long Display Name';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function hmac(secret) {
  return createHmac('sha256', pepper).update(secret).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`rfc075-slice5 evidence smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`rfc075-slice5 evidence smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) {
    throw new Error('rfc075-slice5 evidence smoke refuses remote D1 operations');
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
  sql(`DELETE FROM community_memberships WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM membership_relink_codes WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM audit_log WHERE community_id = '${communityId}'`);
  sql(`DELETE FROM form_tokens WHERE user_id IN ('${adminUserId}','${secondAdminUserId}','${memberUserId}')`);
  sql(`DELETE FROM sessions WHERE session_hmac = '${adminSessionHmac}' OR user_id IN ('${adminUserId}','${secondAdminUserId}','${memberUserId}')`);
  sql(`DELETE FROM users WHERE id IN ('${adminUserId}','${secondAdminUserId}','${memberUserId}')`);
  sql(`DELETE FROM communities WHERE id = '${communityId}'`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'RFC075 Slice 5 Primary', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${adminUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${secondAdminUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminMembershipId}', '${communityId}', '${adminUserId}', 'admin', '${esc('RFC075 Slice 5 Admin')}', '${now}')`,
    // A second admin so the demote path is reachable without hitting the
    // "last admin" block — not screenshotted, but seeded for completeness.
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${secondAdminMembershipId}', '${communityId}', '${secondAdminUserId}', 'admin', '${esc('RFC075 Slice 5 Second Admin')}', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${communityId}', '${memberUserId}', 'member', '${esc(longMemberDisplayName)}', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc075s5_admin', '${adminUserId}', '${adminSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
  ];
  for (const statement of statements) sql(statement);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[rfc075-slice5-evidence-smoke] ${message}`);
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

// Handoff 032 §8/§7.3: report the 200% margin numerically, per Handoff
// 028's precedent, plus the member-row-specific measurement §7.3 requires.
async function collect(cdp, denseRowSelector) {
  return await evalExpr(
    cdp,
    `(() => ({
      path: location.pathname,
      htmlLang: document.documentElement.getAttribute('lang'),
      noHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
      pageOverflowPx: Math.max(
        0,
        document.documentElement.scrollWidth - document.documentElement.clientWidth,
      ),
      viewportWidth: document.documentElement.clientWidth,
      denseRowMeasurement: (() => {
        const row = document.querySelector(${JSON.stringify(denseRowSelector)});
        if (!row) return null;
        const r = row.getBoundingClientRect();
        return {
          viewportWidth: document.documentElement.clientWidth,
          rowRight: Math.round(r.right),
          marginPx: Math.round(document.documentElement.clientWidth - r.right),
        };
      })(),
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

const surfaces = [
  {
    name: 'member-list',
    path: `/c/${communityId}/admin/members`,
    denseRowSelector: '.cz-admin-member-row',
  },
  {
    name: 'invites-page',
    path: `/c/${communityId}/admin/invites`,
    denseRowSelector: null,
  },
  {
    name: 'promote-confirmation',
    path: `/c/${communityId}/admin/members/${memberMembershipId}/promote`,
    denseRowSelector: '.cz-admin-role-actions',
  },
  {
    name: 'remove-confirmation',
    path: `/c/${communityId}/admin/members/${memberMembershipId}/remove`,
    denseRowSelector: '.cz-admin-role-actions',
  },
  {
    name: 'help-signin-confirmation',
    path: `/c/${communityId}/admin/members/${memberMembershipId}/help-signin`,
    denseRowSelector: '.cz-admin-role-actions',
  },
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

  const page = await newPage(adminSessionSecret);

  // §7.3: the member list, measured at BOTH scales, not just 200% — the
  // handoff's own precedent (cancel.rs in Slice 4) measured before/after.
  logStep('capturing member-list at 100% text (baseline)');
  await navigate(page, `/c/${communityId}/admin/members`);
  const memberListNormal = await collect(page, '.cz-admin-member-row');
  const memberListNormalShot = await screenshot(page, 'member-list-japanese-100-percent');
  results.push({
    name: 'member-list-100-percent',
    screenshotPath: memberListNormalShot,
    observed: memberListNormal,
    checks: {
      htmlLangJa: memberListNormal.htmlLang === 'ja',
      noHorizontalScroll: memberListNormal.noHorizontalScroll,
      denseRowFound: memberListNormal.denseRowMeasurement !== null,
    },
  });

  for (const surface of surfaces) {
    logStep(`capturing ${surface.name} (Japanese, mobile, 200% text)`);
    await navigate(page, surface.path, { textScale: 2 });
    const observed = await collect(page, surface.denseRowSelector);
    const screenshotPath = await screenshot(page, `${surface.name}-japanese-200-percent`);
    results.push({
      name: surface.name,
      screenshotPath,
      observed,
      checks: {
        htmlLangJa: observed.htmlLang === 'ja',
        noHorizontalScrollAt200Percent: observed.noHorizontalScroll,
        denseRowFound: surface.denseRowSelector === null || observed.denseRowMeasurement !== null,
      },
    });
  }

  // help_signin.rs's second render: submit the confirm form (the real
  // no-JS POST path) and capture the synthetic relink-code reveal.
  logStep('submitting help-signin confirm to capture the code-reveal page');
  const submitted = await evalExpr(
    page,
    `(() => {
      const form = document.querySelector('form[action*="/help-signin"]');
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  if (submitted) {
    const loaded = page.once('Page.loadEventFired');
    await withTimeout(loaded, 'help-signin POST submit');
    await evalExpr(page, `(() => { document.documentElement.style.fontSize = '200%'; })()`);
    await sleep(150);
    const revealObserved = await collect(page, '.cz-admin-reveal-box');
    const revealShot = await screenshot(page, 'help-signin-code-reveal-japanese-200-percent');
    results.push({
      name: 'help-signin-code-reveal',
      screenshotPath: revealShot,
      observed: revealObserved,
      checks: {
        htmlLangJa: revealObserved.htmlLang === 'ja',
        noHorizontalScrollAt200Percent: revealObserved.noHorizontalScroll,
        denseRowFound: revealObserved.denseRowMeasurement !== null,
      },
    });
  } else {
    results.push({
      name: 'help-signin-code-reveal',
      observed: { submitted: false },
      checks: { formFound: false },
    });
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
    note: 'Evidence capture only (screenshots + numeric 200% margins, plus the §7.3 member-row measurement at both scales). Functional behavior is covered by smoke:calendar-views, smoke:matrix, smoke:matrix-csv, smoke:community-switch, smoke:language, and smoke:admin-event-forms, all re-run unmodified for this slice. Japanese only — these are RFC-072 Slice D admin surfaces with no English rendering. The relink code captured on help-signin-code-reveal is synthetic test data generated by this run, never a real one.',
    localOnlyGuard: true,
    results,
    passed: results.every((result) => result.passed),
  };

  await writeFile(`${outDir}/${reportName}`, JSON.stringify(report, null, 2));
  console.log(
    JSON.stringify(
      {
        passed: report.passed,
        results: results.map((result) => ({ name: result.name, passed: result.passed, checks: result.checks, observed: result.observed })),
      },
      null,
      2,
    ),
  );

  if (!report.passed) process.exitCode = 1;
} catch (error) {
  if (devStderr.trim()) {
    console.error('[rfc075-slice5-evidence-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[rfc075-slice5-evidence-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  await isolated.cleanup();
}
