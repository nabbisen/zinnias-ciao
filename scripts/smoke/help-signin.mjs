#!/usr/bin/env node
// Scenario smoke for RFC-024 help-signin. Local wrangler dev only.

import { createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8795);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9247);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc024';
const reportName = process.env.REPORT_NAME ?? 'rfc024-help-signin-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-help-signin-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const pepper = 'dev-pepper-change-in-production';
const now = '2026-07-07T00:00:00.000Z';

const primaryCommunityId = 'com_rfc024_primary';
const otherCommunityId = 'com_rfc024_other';
const adminUserId = 'usr_rfc024_admin';
const memberUserId = 'usr_rfc024_member';
const removedUserId = 'usr_rfc024_removed';
const adminMembershipId = 'mem_rfc024_admin';
const memberMembershipId = 'mem_rfc024_member';
const removedMembershipId = 'mem_rfc024_removed';
const adminSessionSecret = 'rfc024-smoke-admin-session';
const oldMemberSessionSecret = 'rfc024-smoke-old-member-session';
const adminSessionHmac = hmac(adminSessionSecret);
const oldMemberSessionHmac = hmac(oldMemberSessionSecret);

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function hmac(secret) {
  return createHmac('sha256', pepper).update(secret).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`help-signin smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`help-signin smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) {
    throw new Error('help-signin smoke refuses remote D1 operations');
  }
  try {
    execFileSync('bunx', ['wrangler', ...args], {
      cwd: process.cwd(),
      stdio: ['ignore', 'pipe', 'pipe'],
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

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  const statements = [
    `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${primaryCommunityId}', 'RFC024 Primary Community', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${otherCommunityId}', 'RFC024 Other Community', 'Asia/Tokyo', 1, '${now}')`,
    `UPDATE communities SET name='RFC024 Primary Community', timezone='Asia/Tokyo', is_active=1 WHERE id='${primaryCommunityId}'`,
    `UPDATE communities SET name='RFC024 Other Community', timezone='Asia/Tokyo', is_active=1 WHERE id='${otherCommunityId}'`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${adminUserId}', '${now}')`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${removedUserId}', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminMembershipId}', '${primaryCommunityId}', '${adminUserId}', 'admin', 'RFC024 Admin', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${primaryCommunityId}', '${memberUserId}', 'member', 'RFC024 Active Member', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at, removed_at) VALUES ('${removedMembershipId}', '${primaryCommunityId}', '${removedUserId}', 'member', 'RFC024 Removed Member', '${now}', '${now}')`,
    `UPDATE community_memberships SET role='admin', display_name='RFC024 Admin', removed_at=NULL WHERE id='${adminMembershipId}'`,
    `UPDATE community_memberships SET role='member', display_name='RFC024 Active Member', removed_at=NULL WHERE id='${memberMembershipId}'`,
    `UPDATE community_memberships SET role='member', display_name='RFC024 Removed Member', removed_at='${now}' WHERE id='${removedMembershipId}'`,
    `DELETE FROM membership_relink_codes WHERE community_id IN ('${primaryCommunityId}', '${otherCommunityId}')`,
    `DELETE FROM sessions WHERE id IN ('sess_rfc024_admin', 'sess_rfc024_old_member') OR session_hmac IN ('${adminSessionHmac}', '${oldMemberSessionHmac}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc024_admin', '${adminUserId}', '${adminSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc024_old_member', '${memberUserId}', '${oldMemberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
  ];
  for (const statement of statements) sql(statement);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[help-signin-smoke] ${message}`);
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

function allChecksPass(checks) {
  return Object.values(checks).every(Boolean);
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

  const adminPage = await newPage(adminSessionSecret);

  logStep('checking member row exposes active help-signin and hides removed member');
  await navigate(adminPage, `/c/${primaryCommunityId}/admin/members`, { textScale: 2 });
  const membersPage = await collect(adminPage);
  results.push({
    name: 'members-page-shows-active-help-signin-only',
    screenshotPath: await screenshot(adminPage, 'members-page-shows-active-help-signin-only'),
    observed: membersPage,
    checks: {
      noHorizontalScroll: membersPage.noHorizontalScroll,
      activeHelpSigninLink: membersPage.hrefs.includes(
        `/c/${primaryCommunityId}/admin/members/${memberMembershipId}/help-signin`,
      ),
      removedMemberHidden: !membersPage.text.includes('RFC024 Removed Member'),
      noRemovedHelpSigninLink: !membersPage.hrefs.includes(
        `/c/${primaryCommunityId}/admin/members/${removedMembershipId}/help-signin`,
      ),
    },
  });

  logStep('checking help-signin confirmation copy at 200% text');
  await navigate(
    adminPage,
    `/c/${primaryCommunityId}/admin/members/${memberMembershipId}/help-signin`,
    { textScale: 2 },
  );
  const confirmPage = await collect(adminPage);
  results.push({
    name: 'help-signin-confirmation-copy-fits-at-200-percent',
    screenshotPath: await screenshot(adminPage, 'help-signin-confirmation-copy-fits-at-200-percent'),
    observed: confirmPage,
    checks: {
      noHorizontalScroll: confirmPage.noHorizontalScroll,
      titleVisible: confirmPage.text.includes('サインインし直すお手伝いをしますか'),
      warnsAccess: confirmPage.text.includes('このメンバーとしてサインインできます'),
      ttlVisible: confirmPage.text.includes('15分'),
      oneUseVisible: confirmPage.text.includes('1回だけ'),
    },
  });

  logStep('creating help-signin code');
  await submitFormByAction(
    adminPage,
    `/c/${primaryCommunityId}/admin/members/${memberMembershipId}/help-signin`,
  );
  const codePage = await collect(adminPage);
  const code = await codeFromPage(adminPage);
  results.push({
    name: 'help-signin-code-shown-once',
    screenshotPath: await screenshot(adminPage, 'help-signin-code-shown-once'),
    observed: { ...codePage, codeLength: code.length },
    checks: {
      noHorizontalScroll: codePage.noHorizontalScroll,
      codeLooksPresent: /^[A-F0-9]{16}$/.test(code),
      oneTimeHintVisible: codePage.text.includes('一度だけ表示されます'),
    },
  });

  logStep('redeeming code in a fresh browser context');
  const freshMemberPage = await newPage();
  await navigate(freshMemberPage, '/relink', { textScale: 2 });
  await fillAndSubmitRelink(freshMemberPage, code);
  const redeemedPage = await collect(freshMemberPage);
  results.push({
    name: 'fresh-context-redeems-code-and-lands-signed-in',
    screenshotPath: await screenshot(freshMemberPage, 'fresh-context-redeems-code-and-lands-signed-in'),
    observed: redeemedPage,
    checks: {
      noHorizontalScroll: redeemedPage.noHorizontalScroll,
      landedInPrimaryCommunity: redeemedPage.path === `/c/${primaryCommunityId}/home`,
      signedInAsMember: redeemedPage.text.includes('RFC024 Primary Community'),
      notOnRelinkPage: !redeemedPage.path.startsWith('/relink'),
    },
  });

  logStep('checking reused code gives generic error');
  const reusePage = await newPage();
  await navigate(reusePage, '/relink', { textScale: 2 });
  await fillAndSubmitRelink(reusePage, code);
  const reusedPage = await collect(reusePage);
  results.push({
    name: 'reused-code-shows-generic-error',
    screenshotPath: await screenshot(reusePage, 'reused-code-shows-generic-error'),
    observed: reusedPage,
    checks: {
      noHorizontalScroll: reusedPage.noHorizontalScroll,
      stillOnRelink: reusedPage.path === '/relink',
      genericError: reusedPage.text.includes('このコードは無効か、有効期限が切れています'),
      noSpecificReason: !/使用済み|期限切れ|別のコミュニティ|used|expired|community/i.test(
        reusedPage.text,
      ),
    },
  });

  logStep('checking relinked session does not grant another community');
  await navigate(freshMemberPage, `/c/${otherCommunityId}/home`, { textScale: 2 });
  const otherCommunityDenied = await collect(freshMemberPage);
  results.push({
    name: 'relinked-session-does-not-authorize-other-community',
    screenshotPath: await screenshot(
      freshMemberPage,
      'relinked-session-does-not-authorize-other-community',
    ),
    observed: otherCommunityDenied,
    checks: {
      noHorizontalScroll: otherCommunityDenied.noHorizontalScroll,
      notOtherHome: otherCommunityDenied.path !== `/c/${otherCommunityId}/home`
        || !otherCommunityDenied.text.includes('RFC024 Other Community'),
      genericDenial: !otherCommunityDenied.text.includes('RFC024 Other Community'),
    },
  });

  adminPage.close();
  freshMemberPage.close();
  reusePage.close();

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
    console.error('[help-signin-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[help-signin-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
}
