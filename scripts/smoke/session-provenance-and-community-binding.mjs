#!/usr/bin/env node
// Scenario smoke for RFC-081 §2 / §2.1a (Handoff 048, external-identity
// Slice 1): a relink-derived session must be bound to the community that
// granted it, and refused everywhere else — indistinguishably from
// non-membership. Local wrangler dev only.
//
// Seeds the exact state RFC-081 §2.1a names as the live gap:
// community_create.rs lets a signed-in member gain a second membership
// under the same users.id. One test user with active memberships in TWO
// communities (A and B); an admin of B issues and a fresh context redeems
// a relink code; the resulting session must reach B, must not reach A
// (indistinguishably from a genuine non-member's denial), and a
// first-class (invite-redemption) session with the identical two
// memberships must reach both — proving the restriction is specific to
// relink sessions, not a general multi-community regression.
//
// Handoff 049 extends this to the two routes the review found the original
// package never covered (get_communities and get_switch, both previously
// gated by an unscoped list_communities_for_user call instead of
// authz::require_membership): the bound session must not reach A's
// calendar/matrix view, must not be able to pivot to A via /switch, and
// must not see A listed in its own community switcher — while the
// first-class comparison session must still reach both via the calendar
// route too.

import { prepareIsolatedWorkerTest } from "../lib/isolated-worker-test.mjs";
import { attachCspViolationCapture, readCspViolations } from "../lib/csp-violation-capture.mjs";

import { createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8801);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9253);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/handoff048';
const reportName = process.env.REPORT_NAME ?? 'handoff048-session-provenance-and-community-binding-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-handoff048-session-scope-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const isolated = await prepareIsolatedWorkerTest("handoff048-session-scope");
const pepper = isolated.pepper;
const now = '2026-08-10T00:00:00.000Z';

// Community A: the test user's original community, never named to the
// relink session under test. Community B: the community whose admin
// mediates the relink — the only one the resulting session may reach.
const communityAId = 'com_h048_a';
const communityBId = 'com_h048_b';
const testUserId = 'usr_h048_test';
const testMembershipAId = 'mem_h048_test_a';
const testMembershipBId = 'mem_h048_test_b';
const adminBUserId = 'usr_h048_admin_b';
const adminBMembershipId = 'mem_h048_admin_b';
const adminBSessionSecret = 'h048-smoke-admin-b-session';
const adminBSessionHmac = hmac(adminBSessionSecret);
// A genuine non-member of A, for the indistinguishability comparison —
// same shape of denial a real stranger gets, not a special case.
const outsiderUserId = 'usr_h048_outsider';
const outsiderSessionSecret = 'h048-smoke-outsider-session';
const outsiderSessionHmac = hmac(outsiderSessionSecret);
// Seeded directly as a first-class (invite-redemption) session for
// testUserId, to prove the restriction is relink-specific, not a general
// multi-community regression.
const firstClassSessionSecret = 'h048-smoke-first-class-session';
const firstClassSessionHmac = hmac(firstClassSessionSecret);

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function hmac(secret) {
  return createHmac('sha256', pepper).update(secret).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`session-provenance smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`session-provenance smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) {
    throw new Error('session-provenance smoke refuses remote D1 operations');
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

function query(statement) {
  const raw = runWrangler([
    'd1', 'execute', 'zinnias-ciao-dev', '--local', '--env', 'dev', '--json', '--command', statement,
  ]);
  const parsed = JSON.parse(raw);
  return parsed?.[0]?.results ?? parsed?.results ?? [];
}

function clean() {
  const communities = `'${communityAId}','${communityBId}'`;
  sql(`DELETE FROM membership_relink_codes WHERE community_id IN (${communities})`);
  sql(`DELETE FROM sessions WHERE session_hmac IN ('${adminBSessionHmac}', '${outsiderSessionHmac}', '${firstClassSessionHmac}') OR user_id IN ('${testUserId}', '${adminBUserId}', '${outsiderUserId}')`);
  sql(`DELETE FROM community_memberships WHERE community_id IN (${communities}) OR user_id IN ('${testUserId}', '${adminBUserId}', '${outsiderUserId}')`);
  sql(`DELETE FROM communities WHERE id IN (${communities})`);
  sql(`DELETE FROM users WHERE id IN ('${testUserId}', '${adminBUserId}', '${outsiderUserId}')`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityAId}', 'Handoff048 Community A', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityBId}', 'Handoff048 Community B', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${testUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${adminBUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${outsiderUserId}', '${now}')`,
    // The exact state RFC-081 §2.1a names: one users.id, active memberships
    // in two communities — the state community_create.rs produces for a
    // signed-in member who creates a second community.
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${testMembershipAId}', '${communityAId}', '${testUserId}', 'member', 'H048 Test Member', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${testMembershipBId}', '${communityBId}', '${testUserId}', 'member', 'H048 Test Member', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminBMembershipId}', '${communityBId}', '${adminBUserId}', 'admin', 'H048 Admin B', '${now}')`,
    // Outsider: active in B only, never in A — the genuine-non-member
    // baseline for the indistinguishability comparison against A.
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('mem_h048_outsider_b', '${communityBId}', '${outsiderUserId}', 'member', 'H048 Outsider', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance) VALUES ('sess_h048_admin_b', '${adminBUserId}', '${adminBSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}', 'invite_redemption')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance) VALUES ('sess_h048_outsider', '${outsiderUserId}', '${outsiderSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}', 'invite_redemption')`,
    // The comparison session: identical two-community memberships as the
    // relink-derived session under test, but first-class (unscoped) —
    // must reach both A and B, or the fix is over-broad rather than
    // relink-specific.
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance) VALUES ('sess_h048_first_class', '${testUserId}', '${firstClassSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}', 'invite_redemption')`,
  ];
  for (const statement of statements) sql(statement);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[session-provenance-smoke] ${message}`);
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

async function newPage(sessionSecret = null) {
  const target = await json(`http://127.0.0.1:${remotePort}/json/new`, { method: 'PUT' });
  const cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  await attachCspViolationCapture(cdp);
  if (sessionSecret) {
    await setSession(cdp, sessionSecret);
  } else {
    await cdp.send('Network.clearBrowserCookies');
    await cdp.send('Network.setExtraHTTPHeaders', { headers: {} });
  }
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
    `(() => {
      const links = [...document.querySelectorAll('a[href]')].map((a) => a.getAttribute('href'));
      return {
        path: location.pathname + location.search,
        status: undefined,
        text: document.body.innerText,
        hrefs: links,
      };
    })()`,
  );
}

async function collectSwitcherOptions(cdp) {
  // Handoff 049 §5: the community switcher is a <select name="community">
  // with one <option value="community_id"> per entry in the scope-filtered
  // list_communities_for_user result — reading option values, not hrefs,
  // is what actually exercises that filter.
  return await evalExpr(
    cdp,
    `(() => {
      const select = document.querySelector('select[name="community"]');
      if (!select) return [];
      return [...select.querySelectorAll('option')].map((o) => o.value);
    })()`,
  );
}

async function statusOf(cdp, path) {
  // Independent of `collect()`'s DOM read: the actual HTTP status code for
  // the navigation, read from the Network domain response. Filtered to the
  // Document-typed response for this exact URL — `once('Network.responseReceived')`
  // would resolve on whichever response (including a subresource) arrives
  // first, which is not necessarily the main navigation.
  const targetUrl = `${baseUrl}${path}`;
  let resolveStatus;
  const statusPromise = new Promise((resolve) => {
    resolveStatus = resolve;
  });
  const handler = (params) => {
    if (params.type === 'Document' && params.response?.url === targetUrl) {
      resolveStatus(params.response.status);
    }
  };
  cdp.on('Network.responseReceived', handler);
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: 390, height: 900, deviceScaleFactor: 1, mobile: true,
  });
  const loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: targetUrl });
  const status = await withTimeout(statusPromise, `network response for ${path}`);
  await withTimeout(loaded, `navigate ${path}`);
  const remaining = (cdp.events.get('Network.responseReceived') ?? []).filter(
    (item) => item !== handler,
  );
  cdp.events.set('Network.responseReceived', remaining);
  return status;
}

async function submitFormByAction(cdp, action) {
  const loaded = cdp.once('Page.loadEventFired');
  const submitted = await evalExpr(
    cdp,
    `(() => {
      const form = [...document.querySelectorAll('form[action]')].find((item) => item.getAttribute('action') === ${JSON.stringify(action)});
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  if (!submitted) throw new Error(`Form not found: ${action}`);
  await withTimeout(loaded, `submit form to ${action}`);
  await sleep(150);
}

async function fillAndSubmitRelink(cdp, code) {
  const loaded = cdp.once('Page.loadEventFired');
  const submitted = await evalExpr(
    cdp,
    `(() => {
      const input = document.querySelector('input[name="code"]');
      const form = document.querySelector('form[action="/relink"]');
      if (!input || !form) return false;
      input.value = ${JSON.stringify(code)};
      form.requestSubmit();
      return true;
    })()`,
  );
  if (!submitted) throw new Error('Relink form not found');
  await withTimeout(loaded, 'submit relink form');
  await sleep(250);
}

async function codeFromPage(cdp) {
  return await evalExpr(
    cdp,
    `(() => {
      const node = document.querySelector('[aria-label="コード"]');
      return node ? node.innerText.trim() : '';
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

  // ── The assertion that keeps the fix honest: first-class reaches both.
  // Checked FIRST, before any relink activity: redeeming a relink code
  // revokes every other session for the same user_id (RFC-024's own
  // `revoke_others_for_user`, unrelated to this package and correct
  // behaviour) — so this session would no longer exist to check afterward.

  logStep('confirming a first-class session with the same two memberships reaches both');
  const firstClassPage = await newPage(firstClassSessionSecret);
  await navigate(firstClassPage, `/c/${communityAId}/home`);
  const firstClassA = await collect(firstClassPage);
  await navigate(firstClassPage, `/c/${communityBId}/home`);
  const firstClassB = await collect(firstClassPage);
  results.push({
    name: 'first-class-session-with-same-memberships-reaches-both-communities',
    screenshotPath: await screenshot(firstClassPage, 'first-class-session-reaches-both'),
    observed: { firstClassA, firstClassB },
    checks: {
      reachesA: firstClassA.text.includes('Handoff048 Community A'),
      reachesB: firstClassB.text.includes('Handoff048 Community B'),
      neitherDenied: !firstClassA.text.includes('見つかりませんでした')
        && !firstClassB.text.includes('見つかりませんでした'),
    },
  });

  // Handoff 049 §5: the calendar/matrix route (get_communities) and the
  // switch route (get_switch) were the two uncovered gates the review
  // found — both must remain unrestricted for a first-class session, or
  // the fix would have to have narrowed access generally, not just for
  // relink-derived sessions.
  logStep('confirming the first-class session also reaches both via /communities');
  await navigate(firstClassPage, `/c/${communityAId}/communities`);
  const firstClassCalendarA = await collect(firstClassPage);
  await navigate(firstClassPage, `/c/${communityBId}/communities`);
  const firstClassCalendarB = await collect(firstClassPage);
  results.push({
    name: 'first-class-session-reaches-both-communities-via-calendar-route',
    observed: { firstClassCalendarA, firstClassCalendarB },
    checks: {
      reachesA: !firstClassCalendarA.text.includes('見つかりませんでした'),
      reachesB: !firstClassCalendarB.text.includes('見つかりませんでした'),
    },
  });
  firstClassPage.close();

  // ── Issue and redeem a relink code targeting the test user's B membership ──

  const adminBPage = await newPage(adminBSessionSecret);

  logStep('admin of B issues a relink code for the test user\'s B membership');
  await navigate(adminBPage, `/c/${communityBId}/admin/members/${testMembershipBId}/help-signin`);
  await submitFormByAction(adminBPage, `/c/${communityBId}/admin/members/${testMembershipBId}/help-signin`);
  const code = await codeFromPage(adminBPage);
  results.push({
    name: 'admin-b-issues-relink-code',
    observed: { codeLength: code.length },
    checks: {
      codeLooksPresent: /^[A-F0-9]{16}$/.test(code),
    },
  });

  logStep('redeeming the code in a fresh browser context');
  const relinkPage = await newPage();
  await navigate(relinkPage, '/relink');
  await fillAndSubmitRelink(relinkPage, code);
  const redeemedPage = await collect(relinkPage);
  results.push({
    name: 'relink-redemption-lands-in-granting-community-b',
    screenshotPath: await screenshot(relinkPage, 'relink-redemption-lands-in-b'),
    observed: redeemedPage,
    checks: {
      landedInB: redeemedPage.path === `/c/${communityBId}/home`,
      showsCommunityB: redeemedPage.text.includes('Handoff048 Community B'),
    },
  });

  // ── The core assertions: B reachable, A refused, indistinguishably ──

  logStep('confirming the relink-derived session can reach community B');
  const statusB = await statusOf(relinkPage, `/c/${communityBId}/home`);
  const reachesB = await collect(relinkPage);
  results.push({
    name: 'relink-session-reaches-its-granting-community',
    screenshotPath: await screenshot(relinkPage, 'relink-session-reaches-b'),
    observed: { ...reachesB, statusB },
    checks: {
      statusOk: statusB === 200,
      showsCommunityB: reachesB.text.includes('Handoff048 Community B'),
      notFoundCopyAbsent: !reachesB.text.includes('見つかりませんでした'),
    },
  });

  logStep('confirming the relink-derived session cannot reach community A');
  const statusDeniedA = await statusOf(relinkPage, `/c/${communityAId}/home`);
  const deniedA = await collect(relinkPage);
  results.push({
    name: 'relink-session-cannot-reach-a-different-community',
    screenshotPath: await screenshot(relinkPage, 'relink-session-denied-a'),
    observed: { ...deniedA, statusDeniedA },
    checks: {
      statusIsNotFound: statusDeniedA === 404,
      showsGenericNotFound: deniedA.text.includes('見つかりませんでした'),
      neverShowsCommunityAName: !deniedA.text.includes('Handoff048 Community A'),
      neverShowsCommunityBName: !deniedA.text.includes('Handoff048 Community B'),
    },
  });

  // ── Handoff 049 §5: the two routes the review found uncovered ──

  logStep('confirming the relink-derived session cannot reach community A via /communities');
  const statusDeniedACalendar = await statusOf(relinkPage, `/c/${communityAId}/communities`);
  const deniedACalendar = await collect(relinkPage);
  results.push({
    name: 'relink-session-cannot-reach-a-different-community-via-calendar-route',
    screenshotPath: await screenshot(relinkPage, 'relink-session-denied-a-calendar'),
    observed: { ...deniedACalendar, statusDeniedACalendar },
    checks: {
      statusIsNotFound: statusDeniedACalendar === 404,
      showsGenericNotFound: deniedACalendar.text.includes('見つかりませんでした'),
      neverShowsCommunityAName: !deniedACalendar.text.includes('Handoff048 Community A'),
    },
  });

  logStep('confirming the relink-derived session\'s switcher does not list community A');
  await navigate(relinkPage, `/c/${communityBId}/communities`);
  const switcherOptions = await collectSwitcherOptions(relinkPage);
  results.push({
    name: 'relink-session-switcher-does-not-list-out-of-scope-community',
    observed: { switcherOptions },
    checks: {
      listsB: switcherOptions.includes(communityBId),
      omitsA: !switcherOptions.includes(communityAId),
    },
  });

  logStep('confirming /switch?community=A cannot pivot the relink session out of scope');
  await navigate(relinkPage, `/switch?community=${communityAId}`);
  const afterSwitchAttempt = await collect(relinkPage);
  results.push({
    name: 'relink-session-cannot-pivot-out-of-scope-via-switch',
    screenshotPath: await screenshot(relinkPage, 'relink-session-switch-pivot-refused'),
    observed: afterSwitchAttempt,
    checks: {
      landsInGrantingCommunity: afterSwitchAttempt.path === `/c/${communityBId}/home`,
      neverShowsCommunityAName: !afterSwitchAttempt.text.includes('Handoff048 Community A'),
    },
  });

  logStep('confirming the denial is indistinguishable from a genuine non-member\'s');
  const outsiderPage = await newPage(outsiderSessionSecret);
  const statusOutsider = await statusOf(outsiderPage, `/c/${communityAId}/home`);
  const outsiderDenied = await collect(outsiderPage);
  results.push({
    name: 'denial-is-indistinguishable-from-genuine-non-membership',
    observed: {
      relinkSessionText: deniedA.text,
      outsiderText: outsiderDenied.text,
      statusDeniedA,
      statusOutsider,
    },
    checks: {
      identicalBodyText: deniedA.text === outsiderDenied.text,
      identicalStatus: statusDeniedA === statusOutsider,
      identicalHrefSet: JSON.stringify(deniedA.hrefs.sort()) === JSON.stringify(outsiderDenied.hrefs.sort()),
    },
  });

  const cspViolations = await readCspViolations(relinkPage);
  results.push({
    name: 'no-csp-violations',
    observed: { cspViolations },
    checks: { zeroCspViolations: cspViolations.length === 0 },
  });

  adminBPage.close();
  relinkPage.close();
  outsiderPage.close();

  for (const result of results) {
    result.passed = allChecksPass(result.checks);
  }

  const report = {
    generatedAt: new Date().toISOString(),
    chromium,
    baseUrl,
    userDataDir,
    flags,
    note: 'Chromium launched with --incognito and without --no-sandbox. Local wrangler dev only. Proves RFC-081 §2.1a: a relink-derived session is bound to its granting community, refused elsewhere indistinguishably from non-membership, while a first-class session with the identical memberships is unrestricted.',
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
    console.error('[session-provenance-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[session-provenance-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  await isolated.cleanup();
}
