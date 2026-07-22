#!/usr/bin/env node
// Scenario smoke for admin-generated invite redemption. Local wrangler dev only.

import { prepareIsolatedWorkerTest } from "../lib/isolated-worker-test.mjs";

import { createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8799);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9251);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/invite-redemption';
const reportName = process.env.REPORT_NAME ?? 'invite-redemption-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-invite-redemption-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const isolated = await prepareIsolatedWorkerTest("invite-redemption");
const pepper = isolated.pepper;
const now = '2026-07-12T00:00:00.000Z';

const communityId = 'com_smoke_invite_redemption';
const adminUserId = 'usr_smoke_invite_admin';
const adminMembershipId = 'mem_smoke_invite_admin';
const adminSessionSecret = 'smoke-invite-redemption-admin-session';
const adminSessionHmac = hmac(adminSessionSecret);
const runSuffix = Date.now().toString().slice(-6);
const newMemberDisplayName = `Invite Smoke ${runSuffix}`;

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });
await rm(
  `${outDir}/admin-generated-invite-is-stored-and-shown-once.png`,
  { force: true },
);

function hmac(secret) {
  return createHmac('sha256', pepper).update(secret).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`invite-redemption smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`invite-redemption smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) {
    throw new Error('invite-redemption smoke refuses remote D1 operations');
  }
  try {
    return isolated.runWranglerSync(args, {
      cwd: process.cwd(),
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf8',
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

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  const statements = [
    'DROP TRIGGER IF EXISTS proof_fail_invite_generation_audit',
    `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'Invite Redemption Smoke Community', 'Asia/Tokyo', 1, '${now}')`,
    `UPDATE communities SET name='Invite Redemption Smoke Community', timezone='Asia/Tokyo', is_active=1 WHERE id='${communityId}'`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${adminUserId}', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminMembershipId}', '${communityId}', '${adminUserId}', 'admin', 'Invite Smoke Admin', '${now}')`,
    `UPDATE community_memberships SET role='admin', display_name='Invite Smoke Admin', removed_at=NULL WHERE id='${adminMembershipId}'`,
    `DELETE FROM sessions WHERE id = 'sess_smoke_invite_admin' OR session_hmac = '${adminSessionHmac}'`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_smoke_invite_admin', '${adminUserId}', '${adminSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    `DELETE FROM invite_codes WHERE community_id = '${communityId}'`,
    `DELETE FROM form_tokens WHERE user_id = '' OR user_id = '${adminUserId}'`,
  ];
  for (const statement of statements) sql(statement);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[invite-redemption-smoke] ${message}`);
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

  onceMatching(method, predicate) {
    return new Promise((resolve) => {
      const cb = (params) => {
        if (!predicate(params)) return;
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

async function newPage(sessionSecret = null) {
  const target = await json(`http://127.0.0.1:${remotePort}/json/new`, {
    method: 'PUT',
  });
  const cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
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
        hasFormToken: Boolean(document.querySelector('input[name="_token"][value]')),
        noHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
      };
    })()`,
  );
}

function allChecksPass(checks) {
  return Object.values(checks).every(Boolean);
}

async function submitFormByAction(cdp, action) {
  const loaded = cdp.once('Page.loadEventFired');
  const responseReceived = cdp.onceMatching(
    'Network.responseReceived',
    ({ response, type }) => type === 'Document' && response?.url === `${baseUrl}${action}`,
  );
  const redirectRequest = cdp.onceMatching(
    'Network.requestWillBeSent',
    ({ request, redirectResponse, type }) =>
      type === 'Document'
      && request?.url === `${baseUrl}${action}`
      && redirectResponse?.url === `${baseUrl}${action}`,
  );
  const networkOutcome = Promise.race([
    responseReceived.then(({ response }) => response),
    redirectRequest.then(({ redirectResponse }) => redirectResponse),
  ]);
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
  const response = await withTimeout(networkOutcome, `response from ${action}`);
  await sleep(150);
  return {
    status: response.status,
    url: response.url,
    headers: Object.fromEntries(
      Object.entries(response.headers ?? {}).map(([key, value]) => [
        key.toLowerCase(),
        String(value),
      ]),
    ),
  };
}

async function fillAndSubmitJoin(cdp, code) {
  const loaded = cdp.once('Page.loadEventFired');
  const submitted = await evalExpr(
    cdp,
    `(() => {
      const input = document.querySelector('input[name="code"]');
      const form = document.querySelector('form[action="/join"]');
      if (!input || !form) return false;
      input.value = ${JSON.stringify(code)};
      form.requestSubmit();
      return true;
    })()`,
  );
  if (!submitted) throw new Error('Join form not found');
  await withTimeout(loaded, 'submit join form');
  await sleep(250);
}

async function fillAndSubmitProfile(cdp, displayName) {
  const loaded = cdp.once('Page.loadEventFired');
  const submitted = await evalExpr(
    cdp,
    `(() => {
      const input = document.querySelector('input[name="display_name"]');
      const form = document.querySelector('form[action="/join/profile"]');
      if (!input || !form) return false;
      input.value = ${JSON.stringify(displayName)};
      form.requestSubmit();
      return true;
    })()`,
  );
  if (!submitted) throw new Error('Join profile form not found');
  await withTimeout(loaded, 'submit join profile form');
  await sleep(250);
}

async function formToken(cdp) {
  return await evalExpr(
    cdp,
    `(() => document.querySelector('input[name="_token"]')?.value || '')()`,
  );
}

async function inviteReveal(cdp) {
  return await evalExpr(
    cdp,
    `(() => {
      const panel = document.querySelector('#invite-code-reveal');
      if (!panel) return { present: false, code: '', codeOccurrences: 0, warning: false };
      const code = panel.querySelector('div[aria-label]')?.innerText.trim() || '';
      return {
        present: true,
        code,
        codeOccurrences: code ? document.documentElement.innerHTML.split(code).length - 1 : 0,
        warning: panel.innerText.includes('二度と表示されません'),
      };
    })()`,
  );
}

function countRows(statement) {
  return Number(query(statement)[0]?.count ?? 0);
}

function inviteCounts() {
  return {
    invites: countRows(
      `SELECT COUNT(*) AS count FROM invite_codes WHERE community_id='${communityId}'`,
    ),
    generatedAudits: countRows(
      `SELECT COUNT(*) AS count FROM audit_log WHERE community_id='${communityId}' AND action='invite_code.generated'`,
    ),
  };
}

function failureEvents(stderr, offset) {
  return stderr
    .slice(offset)
    .split(/\r?\n/u)
    .map((line) => {
      const start = line.indexOf('event=audit.required_batch_failed');
      return start >= 0
        ? line.slice(start).replaceAll(/\u001b\[[0-9;]*m/gu, '').trim()
        : '';
    })
    .filter(Boolean);
}

let dev;
let chrome;
let devStderr = '';
let chromeStderr = '';
let failureTriggerInstalled = false;
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

  const adminPage = await newPage(adminSessionSecret);

  logStep('checking legacy query containment before authentication');
  const legacyPage = await newPage();
  const legacyPath = `/c/${communityId}/admin/invites?code=synthetic-marker`;
  const cleanInviteUrl = `${baseUrl}/c/${communityId}/admin/invites`;
  const cleanRedirectRequestPromise = legacyPage.onceMatching(
    'Network.requestWillBeSent',
    ({ request, redirectResponse, type }) =>
      type === 'Document'
      && request?.url === cleanInviteUrl
      && redirectResponse?.url === `${baseUrl}${legacyPath}`,
  );
  await navigate(legacyPage, legacyPath);
  const cleanRedirectRequest = await withTimeout(
    cleanRedirectRequestPromise,
    'legacy-query canonical redirect request',
  );
  const legacyResponse = cleanRedirectRequest.redirectResponse;
  const legacyFinal = await collect(legacyPage);
  const legacyHeaders = Object.fromEntries(
    Object.entries(legacyResponse.headers ?? {}).map(([key, value]) => [
      key.toLowerCase(),
      String(value),
    ]),
  );
  results.push({
    name: 'legacy-code-query-is-contained-before-authentication',
    checks: {
      redirectStatus: legacyResponse.status === 303,
      canonicalLocation:
        legacyHeaders.location === `/c/${communityId}/admin/invites`,
      noReferrerPolicy: legacyHeaders['referrer-policy'] === 'no-referrer',
      redirectRequestHasNoReferrer:
        !Object.keys(cleanRedirectRequest.request.headers ?? {}).some(
          (key) => key.toLowerCase() === 'referer',
        ),
      finalPathClean: legacyFinal.path === `/c/${communityId}/admin/invites`,
      authenticationReachedOnlyAfterRedirect:
        legacyFinal.text.includes('もう一度入る必要があります'),
      markerNotRendered: !legacyFinal.text.includes('synthetic-marker'),
    },
  });

  logStep('checking invite page before generation');
  await navigate(adminPage, `/c/${communityId}/admin/invites`, { textScale: 2 });
  const inviteStart = await collect(adminPage);
  const originalGenerateToken = await formToken(adminPage);
  results.push({
    name: 'admin-can-open-invite-page',
    screenshotPath: await screenshot(adminPage, 'admin-can-open-invite-page'),
    checks: {
      noHorizontalScroll: inviteStart.noHorizontalScroll,
      onInvitePage: inviteStart.path === `/c/${communityId}/admin/invites`,
      hasGenerateForm: inviteStart.hasFormToken && originalGenerateToken.length > 0,
      generateCopyVisible: inviteStart.text.includes('コードを生成'),
    },
  });

  logStep('generating invite code');
  const beforeGeneration = inviteCounts();
  const generationResponse = await submitFormByAction(
    adminPage,
    `/c/${communityId}/admin/invites`,
  );
  const inviteGenerated = await collect(adminPage);
  const reveal = await inviteReveal(adminPage);
  const inviteCode = reveal.code;
  const generatedCounts = inviteCounts();
  results.push({
    name: 'admin-generated-invite-is-stored-and-shown-once',
    checks: {
      noHorizontalScroll: inviteGenerated.noHorizontalScroll,
      directStatus: generationResponse.status === 200,
      canonicalUrl: generationResponse.url === cleanInviteUrl,
      noLocation: !Object.hasOwn(generationResponse.headers, 'location'),
      noStorePrivate:
        generationResponse.headers['cache-control'] === 'no-store, private',
      noReferrer:
        generationResponse.headers['referrer-policy'] === 'no-referrer',
      htmlContentType:
        /^text\/html;\s*charset=utf-8$/iu.test(
          generationResponse.headers['content-type'] ?? '',
        ),
      codeFreeBrowserUrl:
        inviteGenerated.path === `/c/${communityId}/admin/invites`,
      codeLooksPresent: /^[A-Z2-9]{6}$/.test(inviteCode),
      codeAppearsOnce: reveal.codeOccurrences === 1,
      warningVisible: reveal.warning,
      storedOneInvite:
        generatedCounts.invites === beforeGeneration.invites + 1,
      storedOneAudit:
        generatedCounts.generatedAudits ===
        beforeGeneration.generatedAudits + 1,
      storedAsMemberInvite:
        countRows(
          `SELECT COUNT(*) AS count FROM invite_codes WHERE community_id='${communityId}' AND grants_role='member' AND used_at IS NULL AND revoked_at IS NULL`,
        ) === 1,
    },
  });

  logStep('checking replay redirects cleanly without regeneration or redisplay');
  const tokenReplaced = await evalExpr(
    adminPage,
    `(() => {
      const token = document.querySelector('input[name="_token"]');
      if (!token) return false;
      token.value = ${JSON.stringify(originalGenerateToken)};
      return true;
    })()`,
  );
  const beforeReplay = inviteCounts();
  const replayResponse = await submitFormByAction(
    adminPage,
    `/c/${communityId}/admin/invites`,
  );
  const afterReplay = inviteCounts();
  const replayPage = await collect(adminPage);
  const replayReveal = await inviteReveal(adminPage);
  results.push({
    name: 'consumed-generation-token-replay-is-clean-and-non-mutating',
    checks: {
      originalTokenRestored: tokenReplaced === true,
      redirectStatus: replayResponse.status === 303,
      canonicalLocation:
        replayResponse.headers.location === `/c/${communityId}/admin/invites`,
      finalPathClean: replayPage.path === `/c/${communityId}/admin/invites`,
      revealAbsent: replayReveal.present === false,
      inviteCountUnchanged: afterReplay.invites === beforeReplay.invites,
      auditCountUnchanged:
        afterReplay.generatedAudits === beforeReplay.generatedAudits,
    },
  });

  logStep('forcing required audit failure at the real generation boundary');
  sql(
    "CREATE TRIGGER proof_fail_invite_generation_audit BEFORE INSERT ON audit_log WHEN NEW.action='invite_code.generated' BEGIN SELECT RAISE(ABORT,'proof failure'); END",
  );
  failureTriggerInstalled = true;
  const beforeFailure = inviteCounts();
  const failureOffset = devStderr.length;
  const failureResponse = await submitFormByAction(
    adminPage,
    `/c/${communityId}/admin/invites`,
  );
  await sleep(150);
  const afterFailure = inviteCounts();
  const failurePage = await collect(adminPage);
  const failureReveal = await inviteReveal(adminPage);
  const boundedFailureEvents = failureEvents(devStderr, failureOffset);
  const failureRequestId = failureResponse.headers['x-request-id'] ?? '';
  const expectedFailureEvent =
    `event=audit.required_batch_failed request_id=${failureRequestId} ` +
    'action=invite_code.generated failure_category=storage route_class=class_a';
  results.push({
    name: 'required-audit-failure-rolls-back-and-discloses-nothing',
    checks: {
      serviceUnavailable: failureResponse.status === 503,
      noLocation: !Object.hasOwn(failureResponse.headers, 'location'),
      normalCacheControl: failureResponse.headers['cache-control'] === 'no-store',
      normalReferrerPolicy:
        failureResponse.headers['referrer-policy'] === 'same-origin',
      boundedRequestId: /^[A-Za-z0-9_-]{1,96}$/u.test(failureRequestId),
      exactlyOneCentralEvent:
        boundedFailureEvents.length === 1 &&
        boundedFailureEvents[0] === expectedFailureEvent,
      inviteRolledBack: afterFailure.invites === beforeFailure.invites,
      auditRolledBack:
        afterFailure.generatedAudits === beforeFailure.generatedAudits,
      revealAbsent:
        failureReveal.present === false &&
        !failurePage.text.includes('二度と表示されません'),
    },
  });
  sql('DROP TRIGGER proof_fail_invite_generation_audit');
  failureTriggerInstalled = false;

  logStep('checking normal generation after trigger cleanup');
  await navigate(adminPage, `/c/${communityId}/admin/invites`);
  const beforeCleanupSuccess = inviteCounts();
  const cleanupSuccessResponse = await submitFormByAction(
    adminPage,
    `/c/${communityId}/admin/invites`,
  );
  const cleanupReveal = await inviteReveal(adminPage);
  const afterCleanupSuccess = inviteCounts();
  results.push({
    name: 'generation-recovers-after-local-trigger-cleanup',
    checks: {
      directStatus: cleanupSuccessResponse.status === 200,
      strictCache:
        cleanupSuccessResponse.headers['cache-control'] === 'no-store, private',
      strictReferrer:
        cleanupSuccessResponse.headers['referrer-policy'] === 'no-referrer',
      revealPresentOnce:
        /^[A-Z2-9]{6}$/u.test(cleanupReveal.code) &&
        cleanupReveal.codeOccurrences === 1,
      oneInviteAdded:
        afterCleanupSuccess.invites === beforeCleanupSuccess.invites + 1,
      oneAuditAdded:
        afterCleanupSuccess.generatedAudits ===
        beforeCleanupSuccess.generatedAudits + 1,
    },
  });

  logStep('redeeming invite in a fresh browser context');
  const freshPage = await newPage();
  await navigate(freshPage, `/c/${communityId}/home`, { textScale: 2 });
  const sessionExpired = await collect(freshPage);
  results.push({
    name: 'session-expired-page-links-sign-in-again-flow',
    screenshotPath: await screenshot(freshPage, 'session-expired-page-links-sign-in-again-flow'),
    checks: {
      noHorizontalScroll: sessionExpired.noHorizontalScroll,
      sessionExpiredCopyVisible: sessionExpired.text.includes('もう一度入る必要があります'),
      relinkLinkVisible: sessionExpired.hrefs.includes('/relink'),
      joinLinkVisible: sessionExpired.hrefs.includes('/join'),
    },
  });
  await navigate(freshPage, '/join', { textScale: 2 });
  const joinStart = await collect(freshPage);
  results.push({
    name: 'join-page-links-sign-in-again-flow',
    screenshotPath: await screenshot(freshPage, 'join-page-links-sign-in-again-flow'),
    checks: {
      noHorizontalScroll: joinStart.noHorizontalScroll,
      onJoinPage: joinStart.path === '/join',
      relinkHintVisible: joinStart.text.includes('サインインし直すためのコード'),
      relinkLinkVisible: joinStart.hrefs.includes('/relink'),
    },
  });
  await fillAndSubmitJoin(freshPage, inviteCode);
  const profilePage = await collect(freshPage);
  results.push({
    name: 'fresh-context-invite-opens-profile-step',
    screenshotPath: await screenshot(freshPage, 'fresh-context-invite-opens-profile-step'),
    checks: {
      noHorizontalScroll: profilePage.noHorizontalScroll,
      onProfileStep: profilePage.path === '/join/profile',
      hasProfileToken: profilePage.hasFormToken,
      noInviteError: !profilePage.text.includes('招待コードはコミュニティの管理者にお問い合わせください'),
    },
  });

  logStep('completing join profile');
  await fillAndSubmitProfile(freshPage, newMemberDisplayName);
  const joinedHome = await collect(freshPage);
  const redeemedInvite = query(
    `SELECT used_at FROM invite_codes WHERE code_hmac='${hmac(inviteCode)}' LIMIT 1`,
  );
  const joinedMembershipRows = query(
    `SELECT id, community_id, role, display_name, removed_at FROM community_memberships WHERE community_id='${communityId}' AND display_name='${esc(newMemberDisplayName)}'`,
  );
  results.push({
    name: 'fresh-context-completes-profile-and-lands-signed-in',
    screenshotPath: await screenshot(freshPage, 'fresh-context-completes-profile-and-lands-signed-in'),
    checks: {
      noHorizontalScroll: joinedHome.noHorizontalScroll,
      landedInCommunity: joinedHome.path === `/c/${communityId}/home`,
      communityVisible: joinedHome.text.includes('Invite Redemption Smoke Community'),
      inviteMarkedUsed: Boolean(redeemedInvite[0]?.used_at),
      membershipCreated: joinedMembershipRows.length === 1,
      membershipRoleMember: joinedMembershipRows[0]?.role === 'member',
      membershipActive: joinedMembershipRows[0]?.removed_at == null,
    },
  });

  logStep('checking reused invite code fails generically');
  const reusePage = await newPage();
  await navigate(reusePage, '/join', { textScale: 2 });
  await fillAndSubmitJoin(reusePage, inviteCode);
  const reusedJoin = await collect(reusePage);
  results.push({
    name: 'reused-invite-shows-generic-join-error',
    screenshotPath: await screenshot(reusePage, 'reused-invite-shows-generic-join-error'),
    checks: {
      noHorizontalScroll: reusedJoin.noHorizontalScroll,
      stillOnJoin: reusedJoin.path === '/join',
      genericError: reusedJoin.text.includes('招待コードはコミュニティの管理者にお問い合わせください'),
      notProfileStep: reusedJoin.path !== '/join/profile',
    },
  });

  adminPage.close();
  legacyPage.close();
  freshPage.close();
  reusePage.close();

  for (const result of results) {
    result.passed = allChecksPass(result.checks);
  }

  const report = {
    note: 'Chromium launched with --incognito and without --no-sandbox. Local wrangler dev only. Plain invite code is not stored in the report.',
    localOnlyGuard: true,
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
  throw new Error(
    `invite-redemption smoke failed; bounded diagnostics=${JSON.stringify({
      message: error instanceof Error ? error.message : String(error),
      workerStderrBytes: devStderr.length,
      chromiumStderrBytes: chromeStderr.length,
    })}`,
  );
} finally {
  if (failureTriggerInstalled) {
    try {
      sql('DROP TRIGGER IF EXISTS proof_fail_invite_generation_audit');
    } catch {
      // The local database may already be unavailable during teardown.
    }
  }
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  await isolated.cleanup();
}
