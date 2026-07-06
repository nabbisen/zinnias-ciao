#!/usr/bin/env node
// Scenario smoke for RFC-061 (v0.48.0). Local wrangler dev only.

import { createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8794);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9246);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc061';
const userDataDir = `.git-exclude/tmp/chrome-member-management-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const pepper = 'dev-pepper-change-in-production';
const now = '2026-07-06T00:00:00.000Z';

const primaryCommunityId = 'com_rfc061_primary';
const secondAdminCommunityId = 'com_rfc061_admin_two';
const memberOnlyCommunityId = 'com_rfc061_member_only';
const adminUserId = 'usr_rfc061_admin';
const memberUserId = 'usr_rfc061_member';
const adminPrimaryMembershipId = 'mem_rfc061_admin_primary';
const adminSecondMembershipId = 'mem_rfc061_admin_second';
const adminMemberOnlyMembershipId = 'mem_rfc061_admin_member_only';
const memberMembershipId = 'mem_rfc061_member_primary';
const adminSessionSecret = 'rfc061-smoke-admin-session';
const memberSessionSecret = 'rfc061-smoke-member-session';
const adminSessionHmac = createHmac('sha256', pepper).update(adminSessionSecret).digest('hex');
const memberSessionHmac = createHmac('sha256', pepper).update(memberSessionSecret).digest('hex');

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`member-management smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`member-management smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) {
    throw new Error('member-management smoke refuses remote D1 operations');
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
    `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${primaryCommunityId}', 'RFC061 Primary Community', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${secondAdminCommunityId}', 'RFC061 Second Admin Community', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT OR IGNORE INTO communities (id, name, timezone, is_active, created_at) VALUES ('${memberOnlyCommunityId}', 'RFC061 Member Only Community', 'Asia/Tokyo', 1, '${now}')`,
    `UPDATE communities SET name='RFC061 Primary Community', timezone='Asia/Tokyo', is_active=1 WHERE id='${primaryCommunityId}'`,
    `UPDATE communities SET name='RFC061 Second Admin Community', timezone='Asia/Tokyo', is_active=1 WHERE id='${secondAdminCommunityId}'`,
    `UPDATE communities SET name='RFC061 Member Only Community', timezone='Asia/Tokyo', is_active=1 WHERE id='${memberOnlyCommunityId}'`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${adminUserId}', '${now}')`,
    `INSERT OR IGNORE INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminPrimaryMembershipId}', '${primaryCommunityId}', '${adminUserId}', 'admin', 'RFC061 Admin', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminSecondMembershipId}', '${secondAdminCommunityId}', '${adminUserId}', 'admin', 'RFC061 Admin Second', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${adminMemberOnlyMembershipId}', '${memberOnlyCommunityId}', '${adminUserId}', 'member', 'RFC061 Admin As Member', '${now}')`,
    `INSERT OR IGNORE INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${primaryCommunityId}', '${memberUserId}', 'member', 'RFC061 Member', '${now}')`,
    `UPDATE community_memberships SET role='admin', display_name='RFC061 Admin', removed_at=NULL WHERE id='${adminPrimaryMembershipId}'`,
    `UPDATE community_memberships SET role='admin', display_name='RFC061 Admin Second', removed_at=NULL WHERE id='${adminSecondMembershipId}'`,
    `UPDATE community_memberships SET role='member', display_name='RFC061 Admin As Member', removed_at=NULL WHERE id='${adminMemberOnlyMembershipId}'`,
    `UPDATE community_memberships SET role='member', display_name='RFC061 Member', removed_at=NULL WHERE id='${memberMembershipId}'`,
    `DELETE FROM sessions WHERE id IN ('sess_rfc061_admin', 'sess_rfc061_member') OR session_hmac IN ('${adminSessionHmac}', '${memberSessionHmac}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc061_admin', '${adminUserId}', '${adminSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc061_member', '${memberUserId}', '${memberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
  ];
  for (const statement of statements) sql(statement);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[member-management-smoke] ${message}`);
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

  logStep('checking admin home shortcut');
  await setSession(page, adminSessionSecret);
  await navigate(page, `/c/${primaryCommunityId}/home`, { textScale: 2 });
  const adminHome = await collect(page);
  results.push({
    name: 'admin-home-shows-member-management',
    screenshotPath: await screenshot(page, 'admin-home-shows-member-management'),
    observed: adminHome,
    checks: {
      noHorizontalScroll: adminHome.noHorizontalScroll,
      showsManageMembers: adminHome.text.includes('メンバーを管理'),
      linksToMembers: adminHome.hrefs.includes(`/c/${primaryCommunityId}/admin/members`),
    },
  });

  logStep('checking non-admin home hides shortcut');
  await setSession(page, memberSessionSecret);
  await navigate(page, `/c/${primaryCommunityId}/home`, { textScale: 2 });
  const memberHome = await collect(page);
  results.push({
    name: 'member-home-hides-admin-member-management',
    screenshotPath: await screenshot(page, 'member-home-hides-admin-member-management'),
    observed: memberHome,
    checks: {
      noHorizontalScroll: memberHome.noHorizontalScroll,
      hidesManageMembers: !memberHome.text.includes('メンバーを管理'),
      hidesMembersLink: !memberHome.hrefs.includes(`/c/${primaryCommunityId}/admin/members`),
    },
  });

  logStep('checking admin Me tools');
  await setSession(page, adminSessionSecret);
  await navigate(page, `/c/${primaryCommunityId}/me`, { textScale: 2 });
  const adminMe = await collect(page);
  results.push({
    name: 'admin-me-shows-admin-tools',
    screenshotPath: await screenshot(page, 'admin-me-shows-admin-tools'),
    observed: adminMe,
    checks: {
      noHorizontalScroll: adminMe.noHorizontalScroll,
      showsAdminSection: adminMe.text.includes('管理'),
      linksToMembers: adminMe.hrefs.includes(`/c/${primaryCommunityId}/admin/members`),
      linksToExport: adminMe.hrefs.includes(`/c/${primaryCommunityId}/admin/export`),
    },
  });

  logStep('checking non-admin Me hides tools');
  await setSession(page, memberSessionSecret);
  await navigate(page, `/c/${primaryCommunityId}/me`, { textScale: 2 });
  const memberMe = await collect(page);
  results.push({
    name: 'member-me-hides-admin-tools',
    screenshotPath: await screenshot(page, 'member-me-hides-admin-tools'),
    observed: memberMe,
    checks: {
      noHorizontalScroll: memberMe.noHorizontalScroll,
      hidesAdminMemberLink: !memberMe.hrefs.includes(`/c/${primaryCommunityId}/admin/members`),
      hidesExportLink: !memberMe.hrefs.includes(`/c/${primaryCommunityId}/admin/export`),
    },
  });

  logStep('checking members page semantics');
  await setSession(page, adminSessionSecret);
  await navigate(page, `/c/${primaryCommunityId}/admin/members`, { textScale: 2 });
  const membersPage = await collect(page);
  results.push({
    name: 'members-page-shows-role-labels-current-user-and-no-self-remove',
    screenshotPath: await screenshot(page, 'members-page-shows-role-labels-current-user-and-no-self-remove'),
    observed: membersPage,
    checks: {
      noHorizontalScroll: membersPage.noHorizontalScroll,
      showsAdminRole: membersPage.text.includes('管理者'),
      showsMemberRole: membersPage.text.includes('メンバー'),
      marksCurrentUser: membersPage.text.includes('あなた'),
      noSelfRemove: !membersPage.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${adminPrimaryMembershipId}/remove`),
      canRemoveOtherMember: membersPage.hrefs.includes(`/c/${primaryCommunityId}/admin/members/${memberMembershipId}/remove`),
      hasInviteChildLink: membersPage.hrefs.includes(`/c/${primaryCommunityId}/admin/invites`),
    },
  });

  logStep('checking invite child page and back navigation');
  await clickLinkByHref(page, `/c/${primaryCommunityId}/admin/invites`);
  const invitePage = await collect(page);
  results.push({
    name: 'members-page-opens-invite-child-page',
    screenshotPath: await screenshot(page, 'members-page-opens-invite-child-page'),
    observed: invitePage,
    checks: {
      noHorizontalScroll: invitePage.noHorizontalScroll,
      onInvitePage: invitePage.path === `/c/${primaryCommunityId}/admin/invites`,
      hasBackLink: invitePage.hrefs.includes(`/c/${primaryCommunityId}/admin/members`),
      backCopyVisible: invitePage.text.includes('メンバー管理へ戻る'),
    },
  });

  await clickLinkByHref(page, `/c/${primaryCommunityId}/admin/members`);
  const afterBack = await collect(page);
  results.push({
    name: 'invite-child-page-returns-to-members',
    observed: afterBack,
    checks: {
      returnedToMembers: afterBack.path === `/c/${primaryCommunityId}/admin/members`,
      membersTitleVisible: afterBack.text.includes('メンバー'),
    },
  });

  logStep('checking community switch preservation and fallback');
  await navigate(page, `/switch?community=${secondAdminCommunityId}&next=admin_members`, {
    textScale: 2,
  });
  const switchMembers = await collect(page);
  results.push({
    name: 'switcher-preserves-members-for-admin-owned-community',
    screenshotPath: await screenshot(page, 'switcher-preserves-members-for-admin-owned-community'),
    observed: switchMembers,
    checks: {
      noHorizontalScroll: switchMembers.noHorizontalScroll,
      landsOnSecondMembers: switchMembers.path === `/c/${secondAdminCommunityId}/admin/members`,
    },
  });

  await navigate(page, `/switch?community=${secondAdminCommunityId}&next=admin_invites`, {
    textScale: 2,
  });
  const switchInvites = await collect(page);
  results.push({
    name: 'switcher-preserves-invites-for-admin-owned-community',
    screenshotPath: await screenshot(page, 'switcher-preserves-invites-for-admin-owned-community'),
    observed: switchInvites,
    checks: {
      noHorizontalScroll: switchInvites.noHorizontalScroll,
      landsOnSecondInvites: switchInvites.path === `/c/${secondAdminCommunityId}/admin/invites`,
    },
  });

  await navigate(page, `/switch?community=${memberOnlyCommunityId}&next=admin_members`, {
    textScale: 2,
  });
  const switchFallback = await collect(page);
  results.push({
    name: 'switcher-falls-back-home-for-member-only-community',
    screenshotPath: await screenshot(page, 'switcher-falls-back-home-for-member-only-community'),
    observed: switchFallback,
    checks: {
      noHorizontalScroll: switchFallback.noHorizontalScroll,
      fallsBackToHome: switchFallback.path === `/c/${memberOnlyCommunityId}/home`,
      notAdminMembers: !switchFallback.path.includes('/admin/members'),
    },
  });

  logStep('checking removal confirmation at 200% text scale');
  await navigate(page, `/c/${primaryCommunityId}/admin/members/${memberMembershipId}/remove`, {
    textScale: 2,
  });
  const removeConfirm = await collect(page);
  results.push({
    name: 'remove-confirmation-copy-fits-at-200-percent',
    screenshotPath: await screenshot(page, 'remove-confirmation-copy-fits-at-200-percent'),
    observed: removeConfirm,
    checks: {
      noHorizontalScroll: removeConfirm.noHorizontalScroll,
      removeCopyVisible: removeConfirm.text.includes('メンバーから外しますか')
        && removeConfirm.text.includes('メンバーから外す'),
      recordsRemainCopyVisible: removeConfirm.text.includes('過去の参加状況やメモは残ります'),
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
    `${outDir}/rfc061-member-management-smoke-results.json`,
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
    console.error('[member-management-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[member-management-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
}
