#!/usr/bin/env node
// Scenario smoke for RFC-062 (v0.49.0). Local wrangler dev only.

import { createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8794);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9246);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc062';
const userDataDir = `.git-exclude/tmp/chrome-admin-role-transfer-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const pepper = 'dev-pepper-change-in-production';
const now = '2026-07-06T00:00:00.000Z';

const primaryCommunityId = 'com_rfc062_primary';
const nonAdminCommunityId = 'com_rfc062_member_only';
const adminUserId = 'usr_rfc062_admin';
const memberUserId = 'usr_rfc062_member';
const secondAdminUserId = 'usr_rfc062_second_admin';
const adminMembershipId = 'mem_rfc062_admin_primary';
const memberMembershipId = 'mem_rfc062_member_primary';
const secondAdminMembershipId = 'mem_rfc062_second_admin';
const adminMemberOnlyMembershipId = 'mem_rfc062_admin_member_only';
const adminSessionSecret = 'rfc062-smoke-admin-session';
const memberSessionSecret = 'rfc062-smoke-member-session';
const adminSessionHmac = createHmac('sha256', pepper).update(adminSessionSecret).digest('hex');
const memberSessionHmac = createHmac('sha256', pepper).update(memberSessionSecret).digest('hex');

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`admin-role-transfer smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`admin-role-transfer smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) {
    throw new Error('admin-role-transfer smoke refuses remote D1 operations');
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

function esc(value) {
  return String(value).replaceAll("'", "''");
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  const statements = [
    `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${primaryCommunityId}', 'RFC062 Primary Community', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${nonAdminCommunityId}', 'RFC062 Member Only Community', 'Asia/Tokyo', 1, '${now}')`,
    `UPDATE communities SET name='RFC062 Primary Community', timezone='Asia/Tokyo', is_active=1 WHERE id='${primaryCommunityId}'`,
    `UPDATE communities SET name='RFC062 Member Only Community', timezone='Asia/Tokyo', is_active=1 WHERE id='${nonAdminCommunityId}'`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${adminUserId}', '${now}')`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${secondAdminUserId}', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminMembershipId}', '${primaryCommunityId}', '${adminUserId}', 'admin', 'RFC062 Admin', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${primaryCommunityId}', '${memberUserId}', 'member', 'RFC062 Member', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${secondAdminMembershipId}', '${primaryCommunityId}', '${secondAdminUserId}', 'admin', 'RFC062 Second Admin', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminMemberOnlyMembershipId}', '${nonAdminCommunityId}', '${adminUserId}', 'member', 'RFC062 Admin As Member', '${now}')`,
    `UPDATE community_memberships SET role='admin', display_name='RFC062 Admin', removed_at=NULL WHERE id='${adminMembershipId}'`,
    `UPDATE community_memberships SET role='member', display_name='RFC062 Member', removed_at=NULL WHERE id='${memberMembershipId}'`,
    `UPDATE community_memberships SET role='admin', display_name='RFC062 Second Admin', removed_at=NULL WHERE id='${secondAdminMembershipId}'`,
    `UPDATE community_memberships SET role='member', display_name='RFC062 Admin As Member', removed_at=NULL WHERE id='${adminMemberOnlyMembershipId}'`,
    `DELETE FROM sessions WHERE id IN ('sess_rfc062_admin', 'sess_rfc062_member') OR session_hmac IN ('${adminSessionHmac}', '${memberSessionHmac}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc062_admin', '${adminUserId}', '${adminSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc062_member', '${memberUserId}', '${memberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
  ];
  for (const statement of statements) sql(statement);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[admin-role-transfer-smoke] ${message}`);
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

function allChecksPass(checks) {
  return Object.values(checks).every(Boolean);
}

async function clickLinkByHref(cdp, href) {
  const loaded = cdp.once('Page.loadEventFired');
  const clicked = await evalExpr(
    cdp,
    `(() => {
      const link = [...document.querySelectorAll('a[href]')].find((a) => a.getAttribute('href') === ${JSON.stringify(href)});
      if (!link) return false;
      link.click();
      return true;
    })()`,
  );
  if (!clicked) throw new Error(`Link not found: ${href}`);
  await withTimeout(loaded, `click navigation to ${href}`);
}

async function clickSubmitButton(cdp, label) {
  const loaded = cdp.once('Page.loadEventFired');
  const clicked = await evalExpr(
    cdp,
    `(() => {
      const button = [...document.querySelectorAll('button[type="submit"]')].find((b) => b.innerText.includes(${JSON.stringify(label)}));
      if (!button) return false;
      button.click();
      return true;
    })()`,
  );
  if (!clicked) throw new Error(`Submit button not found: ${label}`);
  await withTimeout(loaded, `submit ${label}`);
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

  logStep('checking admin members role actions');
  await setSession(page, adminSessionSecret);
  await navigate(page, `/c/${primaryCommunityId}/admin/members`, { textScale: 2 });
  const membersStart = await collect(page);
  results.push({
    name: 'members-page-shows-promote-demote-and-hides-self-actions',
    screenshotPath: await screenshot(page, 'members-page-shows-promote-demote-and-hides-self-actions'),
    observed: membersStart,
    checks: {
      noHorizontalScroll: membersStart.noHorizontalScroll,
      showsPromoteAction: membersStart.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${memberMembershipId}/promote`),
      showsDemoteAction: membersStart.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${secondAdminMembershipId}/demote`),
      hidesSelfDemote: !membersStart.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${adminMembershipId}/demote`),
      hidesSelfRemove: !membersStart.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${adminMembershipId}/remove`),
      hasInviteChildLink: membersStart.hrefs.includes(`/c/${primaryCommunityId}/admin/invites`),
    },
  });

  logStep('checking non-admin cannot see role actions');
  await setSession(page, memberSessionSecret);
  await navigate(page, `/c/${primaryCommunityId}/admin/members`, { textScale: 2 });
  const nonAdminDenied = await collect(page);
  results.push({
    name: 'non-admin-direct-members-page-shows-no-role-actions',
    screenshotPath: await screenshot(page, 'non-admin-direct-members-page-shows-no-role-actions'),
    observed: nonAdminDenied,
    checks: {
      noHorizontalScroll: nonAdminDenied.noHorizontalScroll,
      deniedWithGenericCopy: nonAdminDenied.text.includes('問題が発生しました'),
      noPromoteLinks: !nonAdminDenied.hrefs.some((href) => href.includes('/promote')),
      noDemoteLinks: !nonAdminDenied.hrefs.some((href) => href.includes('/demote')),
    },
  });

  logStep('checking promote confirmation and submit');
  await setSession(page, adminSessionSecret);
  await navigate(page, `/c/${primaryCommunityId}/admin/members/${memberMembershipId}/promote`, {
    textScale: 2,
  });
  const promoteConfirm = await collect(page);
  results.push({
    name: 'promote-confirmation-copy-fits-at-200-percent',
    screenshotPath: await screenshot(page, 'promote-confirmation-copy-fits-at-200-percent'),
    observed: promoteConfirm,
    checks: {
      noHorizontalScroll: promoteConfirm.noHorizontalScroll,
      onPromotePage: promoteConfirm.path === `/c/${primaryCommunityId}/admin/members/${memberMembershipId}/promote`,
      titleVisible: promoteConfirm.text.includes('管理者にしますか'),
      consequenceVisible: promoteConfirm.text.includes('イベントの作成')
        && promoteConfirm.text.includes('招待コードの作成'),
    },
  });

  await clickSubmitButton(page, '管理者にする');
  const afterPromote = await collect(page);
  results.push({
    name: 'promote-submit-makes-member-admin',
    screenshotPath: await screenshot(page, 'promote-submit-makes-member-admin'),
    observed: afterPromote,
    checks: {
      noHorizontalScroll: afterPromote.noHorizontalScroll,
      returnedToMembers: afterPromote.path === `/c/${primaryCommunityId}/admin/members`,
      promotedMemberShowsDemote: afterPromote.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${memberMembershipId}/demote`),
      promotedMemberNoPromote: !afterPromote.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${memberMembershipId}/promote`),
    },
  });

  logStep('checking demote confirmation and submit');
  await navigate(page, `/c/${primaryCommunityId}/admin/members/${memberMembershipId}/demote`, {
    textScale: 2,
  });
  const demoteConfirm = await collect(page);
  results.push({
    name: 'demote-confirmation-copy-fits-at-200-percent',
    screenshotPath: await screenshot(page, 'demote-confirmation-copy-fits-at-200-percent'),
    observed: demoteConfirm,
    checks: {
      noHorizontalScroll: demoteConfirm.noHorizontalScroll,
      onDemotePage: demoteConfirm.path === `/c/${primaryCommunityId}/admin/members/${memberMembershipId}/demote`,
      titleVisible: demoteConfirm.text.includes('メンバーに戻しますか'),
      consequenceVisible: demoteConfirm.text.includes('できなくなります')
        && demoteConfirm.text.includes('過去の参加状況やメモは残ります'),
    },
  });

  await clickSubmitButton(page, 'メンバーに戻す');
  const afterDemote = await collect(page);
  results.push({
    name: 'demote-submit-makes-admin-member',
    screenshotPath: await screenshot(page, 'demote-submit-makes-admin-member'),
    observed: afterDemote,
    checks: {
      noHorizontalScroll: afterDemote.noHorizontalScroll,
      returnedToMembers: afterDemote.path === `/c/${primaryCommunityId}/admin/members`,
      demotedMemberShowsPromote: afterDemote.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${memberMembershipId}/promote`),
      demotedMemberNoDemote: !afterDemote.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${memberMembershipId}/demote`),
    },
  });

  logStep('checking self-demote and member-only admin route denial');
  await navigate(page, `/c/${primaryCommunityId}/admin/members/${adminMembershipId}/demote`, {
    textScale: 2,
  });
  const selfDemoteDenied = await collect(page);
  results.push({
    name: 'self-demote-direct-url-is-denied',
    screenshotPath: await screenshot(page, 'self-demote-direct-url-is-denied'),
    observed: selfDemoteDenied,
    checks: {
      noHorizontalScroll: selfDemoteDenied.noHorizontalScroll,
      deniedWithGenericCopy: selfDemoteDenied.text.includes('見つかりませんでした'),
      noLastAdminCopy: !selfDemoteDenied.text.includes('最後の管理者はメンバーに戻せません'),
    },
  });

  await navigate(page, `/c/${nonAdminCommunityId}/admin/members`, {
    textScale: 2,
  });
  const memberOnlyDenied = await collect(page);
  results.push({
    name: 'member-only-community-admin-route-is-denied',
    screenshotPath: await screenshot(page, 'member-only-community-admin-route-is-denied'),
    observed: memberOnlyDenied,
    checks: {
      noHorizontalScroll: memberOnlyDenied.noHorizontalScroll,
      deniedWithGenericCopy: memberOnlyDenied.text.includes('問題が発生しました'),
      noPromoteLinks: !memberOnlyDenied.hrefs.some((href) => href.includes('/promote')),
      noDemoteLinks: !memberOnlyDenied.hrefs.some((href) => href.includes('/demote')),
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
    results,
    passed: results.every((result) => result.passed),
  };

  await writeFile(
    `${outDir}/rfc062-admin-role-transfer-smoke-results.json`,
    JSON.stringify(report, null, 2),
  );
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
    console.error('[admin-role-transfer-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[admin-role-transfer-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
}
