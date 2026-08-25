#!/usr/bin/env node
// End-to-end smoke for RFC-080 §6 / RFC-081 §6 (Handoff 055, external-
// identity Slice 5a): the read-only account surface. Local wrangler dev
// only. Covers the three scenarios named in Handoff 055 §6: a fresh
// session's full display (exercised with JavaScript disabled, in a real
// browser), a stale session's "sign in again" display, and the RFC-081 §6
// no-membership state — plus, as a fetch-based check, that a Relink-
// provenance session is refused the surface entirely (Handoff 055's own
// "Required tests" list).
//
// Reuses the shared `workers/ssr/build/` artifact via
// `prepareIsolatedWorkerTest` directly — unlike the identity-callback
// smoke, nothing here needs the `dev_fake_issuer` feature: every session
// is seeded straight into D1, no OIDC round trip involved.

import { prepareIsolatedWorkerTest } from '../lib/isolated-worker-test.mjs';
import { attachCspViolationCapture, readCspViolations } from '../lib/csp-violation-capture.mjs';
import { SMOKE_ACCEPT_LANGUAGE } from '../lib/smoke-locale.mjs';

import { createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8821);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9273);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/handoff055';
const reportName = process.env.REPORT_NAME ?? 'handoff055-account-surface-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-handoff055-account-surface-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`account-surface smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`account-surface smoke is local-only; refused argument ${arg}`);
    }
  }
}

function logStep(message) {
  console.error(`[account-surface-smoke] ${message}`);
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

let dev;
let chrome;
let devStderr = '';
let chromeStderr = '';
const results = [];
let isolated;

try {
  isolated = await prepareIsolatedWorkerTest('handoff055-account-surface');
  const pepper = isolated.pepper;

  function hmac(value) {
    return createHmac('sha256', pepper).update(value).digest('hex');
  }

  function runWrangler(args) {
    if (args.includes('--remote')) {
      throw new Error('account-surface smoke refuses remote D1 operations');
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
    runWrangler(['d1', 'execute', 'zinnias-ciao-dev', '--local', '--env', 'dev', '--command', statement]);
  }

  // Real wall-clock time, not a fixed fictional date: `authz::is_fresh_for_account_operations`
  // compares `authenticated_at` against a threshold computed from the
  // worker's own `worker::Date::now()` at request time, so the fixtures
  // below must be anchored to the actual current time or "fresh" and
  // "stale" stop meaning anything relative to what the running server
  // actually checks against.
  const now = new Date().toISOString();
  const staleAuthenticatedAt = new Date(Date.now() - 30 * 60 * 1000).toISOString(); // 30 min ago — outside the 15-min window
  const farFutureExpiry = '2099-12-31T23:59:59.000Z';
  const namespaceId = 'idns_local_fake';

  const communityId = 'com_h055_account';

  const freshUserId = 'usr_h055_fresh';
  const freshMembershipId = 'mem_h055_fresh';
  const freshIdentityId = 'idty_h055_fresh';
  const freshSessionSecret = 'h055-smoke-fresh-session';
  const freshSessionHmac = hmac(freshSessionSecret);

  const staleUserId = 'usr_h055_stale';
  const staleMembershipId = 'mem_h055_stale';
  const staleSessionSecret = 'h055-smoke-stale-session';
  const staleSessionHmac = hmac(staleSessionSecret);

  const noMembershipUserId = 'usr_h055_nomembership';
  const noMembershipSessionSecret = 'h055-smoke-no-membership-session';
  const noMembershipSessionHmac = hmac(noMembershipSessionSecret);

  const relinkUserId = 'usr_h055_relink';
  const relinkMembershipId = 'mem_h055_relink';
  const relinkSessionSecret = 'h055-smoke-relink-session';
  const relinkSessionHmac = hmac(relinkSessionSecret);

  function clean() {
    sql(
      `DELETE FROM sessions WHERE session_hmac IN ` +
        `('${freshSessionHmac}','${staleSessionHmac}','${noMembershipSessionHmac}','${relinkSessionHmac}')`,
    );
    sql(`DELETE FROM user_identities WHERE id = '${freshIdentityId}'`);
    sql(
      `DELETE FROM community_memberships WHERE id IN ` +
        `('${freshMembershipId}','${staleMembershipId}','${relinkMembershipId}')`,
    );
    sql(`DELETE FROM communities WHERE id = '${communityId}'`);
    sql(
      `DELETE FROM users WHERE id IN ` +
        `('${freshUserId}','${staleUserId}','${noMembershipUserId}','${relinkUserId}')`,
    );
  }

  function seed() {
    runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
    clean();
    const statements = [
      `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'Handoff055 Account Community', 'Asia/Tokyo', 1, '${now}')`,

      // Fresh: one community, one linked identity, authenticated_at = now.
      `INSERT INTO users (id, created_at) VALUES ('${freshUserId}', '${now}')`,
      `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${freshMembershipId}', '${communityId}', '${freshUserId}', 'member', 'H055 Fresh Member', '${now}')`,
      `INSERT INTO user_identities (id, user_id, identity_namespace_id, subject_lookup, linked_at, status) VALUES ('${freshIdentityId}', '${freshUserId}', '${namespaceId}', '${hmac('fresh-subject-lookup')}', '${now}', 'active')`,
      `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) VALUES ('sess_h055_fresh', '${freshUserId}', '${freshSessionHmac}', '${now}', '${farFutureExpiry}', '${now}', 'invite_redemption', '${now}')`,

      // Stale: same shape, but authenticated_at is 30 minutes old.
      `INSERT INTO users (id, created_at) VALUES ('${staleUserId}', '${now}')`,
      `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${staleMembershipId}', '${communityId}', '${staleUserId}', 'member', 'H055 Stale Member', '${now}')`,
      `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) VALUES ('sess_h055_stale', '${staleUserId}', '${staleSessionHmac}', '${now}', '${farFutureExpiry}', '${now}', 'invite_redemption', '${staleAuthenticatedAt}')`,

      // No membership: RFC-081 §6 — reaches the account surface, nothing else.
      `INSERT INTO users (id, created_at) VALUES ('${noMembershipUserId}', '${now}')`,
      `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) VALUES ('sess_h055_nomembership', '${noMembershipUserId}', '${noMembershipSessionHmac}', '${now}', '${farFutureExpiry}', '${now}', 'invite_redemption', '${now}')`,

      // Relink: community-bound session — must be refused the account
      // surface entirely, regardless of freshness.
      `INSERT INTO users (id, created_at) VALUES ('${relinkUserId}', '${now}')`,
      `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${relinkMembershipId}', '${communityId}', '${relinkUserId}', 'member', 'H055 Relink Member', '${now}')`,
      `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, scope_community_id, authenticated_at) VALUES ('sess_h055_relink', '${relinkUserId}', '${relinkSessionHmac}', '${now}', '${farFutureExpiry}', '${now}', 'relink', '${communityId}', '${now}')`,
    ];
    for (const statement of statements) sql(statement);
  }

  async function waitForServer(proc, stderr) {
    for (let i = 0; i < 120; i += 1) {
      if (proc.exitCode !== null) break;
      try {
        const res = await fetch(`${baseUrl}/healthz`);
        if (res.ok) return;
      } catch {
        await sleep(250);
      }
    }
    throw new Error(`Wrangler dev server did not become ready\n${stderr()}`);
  }

  async function waitForDebugger(stderr) {
    for (let i = 0; i < 80; i += 1) {
      try {
        const res = await fetch(`http://127.0.0.1:${remotePort}/json/version`);
        if (res.ok) return await res.json();
      } catch {
        await sleep(125);
      }
    }
    throw new Error(`Chromium remote debugging port did not open. stderr=${stderr()}`);
  }

  function allChecksPass(checks) {
    return Object.values(checks).every(Boolean);
  }

  async function fetchAccount(sessionSecret) {
    return fetch(`${baseUrl}/account`, {
      redirect: 'manual',
      headers: { Cookie: `ciao_sid=${sessionSecret}` },
    });
  }

  logStep('seeding local D1 fixtures (fresh, stale, no-membership, and relink sessions)');
  seed();

  logStep(`starting local wrangler dev on ${baseUrl}`);
  dev = isolated.spawnDev(port);
  dev.stderr.on('data', (chunk) => {
    devStderr += chunk.toString();
  });
  await waitForServer(dev, () => devStderr);
  logStep('local wrangler dev is ready');

  // ── Fresh session: full display, via plain fetch ──────────────────────

  logStep('scenario: fresh session shows communities, linked identity, and "can manage"');
  const freshRes = await fetchAccount(freshSessionSecret);
  const freshBody = await freshRes.text();
  results.push({
    name: 'fresh-session-shows-full-account-display',
    observed: { status: freshRes.status },
    checks: {
      status200: freshRes.status === 200,
      showsCommunityName: freshBody.includes('Handoff055 Account Community'),
      showsLinkedNamespace: freshBody.includes(namespaceId),
      showsCanManage: freshBody.includes('これらの設定は今すぐ管理できます'),
      doesNotShowStaleMessage: !freshBody.includes('もう一度サインインしてください'),
      neverShowsSubjectOrDigest: !freshBody.toLowerCase().includes('subject')
        && !freshBody.includes(hmac('fresh-subject-lookup')),
    },
  });

  // ── Stale session: "sign in again" display ─────────────────────────────

  logStep('scenario: stale session (authenticated 30 minutes ago) shows "sign in again"');
  const staleRes = await fetchAccount(staleSessionSecret);
  const staleBody = await staleRes.text();
  results.push({
    name: 'stale-session-shows-sign-in-again',
    observed: { status: staleRes.status },
    checks: {
      status200: staleRes.status === 200,
      showsStaleMessage: staleBody.includes('もう一度サインインしてください'),
      doesNotShowCanManage: !staleBody.includes('これらの設定は今すぐ管理できます'),
      stillShowsOwnCommunity: staleBody.includes('Handoff055 Account Community'),
    },
  });

  // ── No-membership: RFC-081 §6 ──────────────────────────────────────────

  logStep('scenario: a principal with zero active memberships reaches the account surface');
  const noMembershipRes = await fetchAccount(noMembershipSessionSecret);
  const noMembershipBody = await noMembershipRes.text();
  results.push({
    name: 'no-membership-principal-reaches-account-surface-and-nothing-else',
    observed: { status: noMembershipRes.status },
    checks: {
      status200: noMembershipRes.status === 200,
      showsNoCommunitiesMessage: noMembershipBody.includes('参加しているコミュニティはありません'),
      neverShowsAnyCommunityName: !noMembershipBody.includes('Handoff055 Account Community'),
      showsNoLinkedIdentitiesMessage: noMembershipBody.includes('連携している外部アカウントはありません'),
    },
  });

  // ── Relink session: refused the account surface entirely ──────────────

  logStep('scenario: a Relink-provenance session is refused the account surface entirely');
  const relinkRes = await fetchAccount(relinkSessionSecret);
  const relinkBody = await relinkRes.text();
  results.push({
    name: 'relink_provenance_session_refused_entirely',
    observed: { status: relinkRes.status },
    checks: {
      statusIsNotFound: relinkRes.status === 404,
      showsGenericNotFound: relinkBody.includes('見つかりませんでした'),
      neverShowsCommunityName: !relinkBody.includes('Handoff055 Account Community'),
    },
  });

  // ── The whole flow with JavaScript disabled, in a real browser ────────

  logStep(`starting sandboxed incognito Chromium (JS disabled) on remote debugging port ${remotePort}`);
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

  async function evalExpr(cdp, expression) {
    const result = await cdp.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result?.value;
  }

  const target = await (await fetch(`http://127.0.0.1:${remotePort}/json/new`, { method: 'PUT' })).json();
  const cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  await cdp.send('Network.clearBrowserCookies');
  await cdp.send('Network.setExtraHTTPHeaders', {
    headers: { 'Accept-Language': SMOKE_ACCEPT_LANGUAGE },
  });
  await cdp.send('Network.setCookie', {
    name: 'ciao_sid',
    value: freshSessionSecret,
    domain: '127.0.0.1',
    path: '/',
    httpOnly: true,
    secure: false,
    sameSite: 'Strict',
  });
  // The proof: application JavaScript is fully disabled before navigation
  // — `Runtime.evaluate` below still works via the DevTools Protocol's own
  // channel, independent of what this disables (same reasoning as the
  // identity-callback smoke's no-JS scenario).
  await cdp.send('Emulation.setScriptExecutionDisabled', { value: true });
  await attachCspViolationCapture(cdp);

  logStep('driving /account via real browser navigation, JS disabled, fresh session');
  const loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/account` });
  await withTimeout(loaded, 'no-JS account navigation');
  await sleep(200);

  const finalState = await evalExpr(
    cdp,
    `(() => ({ path: location.pathname, text: document.body.innerText }))()`,
  );
  const cspViolations = await readCspViolations(cdp);
  results.push({
    name: 'account_surface_renders_with_javascript_disabled',
    observed: finalState,
    checks: {
      landedOnAccount: finalState.path === '/account',
      showsCommunityContent: finalState.text.includes('Handoff055 Account Community'),
      showsLinkedIdentity: finalState.text.includes(namespaceId),
      showsCanManage: finalState.text.includes('これらの設定は今すぐ管理できます'),
      zeroCspViolations: cspViolations.length === 0,
    },
  });

  cdp.close();

  for (const result of results) {
    result.passed = allChecksPass(result.checks);
  }

  const report = {
    generatedAt: new Date().toISOString(),
    chromium,
    baseUrl,
    userDataDir,
    localOnlyGuard: true,
    note: 'RFC-080 §6 / RFC-081 §6 account surface (Handoff 055 §6): fresh-session full display, stale-session "sign in again", the no-membership state reaching the surface and nothing else, a Relink-provenance session refused entirely, and the whole page rendering correctly with application JavaScript fully disabled.',
    results,
    passed: results.every((r) => r.passed),
  };

  await writeFile(`${outDir}/${reportName}`, JSON.stringify(report, null, 2));
  console.log(
    JSON.stringify(
      { passed: report.passed, results: results.map((r) => ({ name: r.name, passed: r.passed, checks: r.checks })) },
      null,
      2,
    ),
  );

  if (!report.passed) process.exitCode = 1;
} catch (error) {
  if (devStderr.trim()) {
    console.error('[account-surface-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[account-surface-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  if (isolated) await isolated.cleanup();
}
