#!/usr/bin/env node
// End-to-end smoke for RFC-080 §5/§5.1/§5.2 (Handoff 054, Slice 4b): the
// nine-step authentication callback contract driven through real HTTP
// against a real (locally-run) worker, using the in-process fake OIDC
// issuer (`dev_fake_issuer` feature) as the provider. Local wrangler dev
// only.
//
// This is the first slice where an end-to-end proof is possible (4a built a
// verification path nothing could reach). Handoff 054 §6 names six required
// scenarios, all covered here:
//   1. successful sign-in issuing a session with provenance 'external_identity'
//   2. a replayed callback rejected
//   3. a tampered `state` rejected
//   4. a wrong `nonce` rejected
//   5. an out-of-allowlist return destination refused (seeded directly in
//      the transaction row — §5.2 never accepts this from a request
//      parameter, so there is no live route that lets a caller set it)
//   6. the whole flow with JavaScript disabled, in a real browser
//
// Handoff 054 §3's structural constraint means this smoke cannot reuse the
// shared `workers/ssr/build/` artifact the other smokes copy (that artifact
// is `bun run build`'s output — the `dev_fake_issuer` feature is off, so
// nothing here would be reachable). This script builds its own isolated
// artifact with the feature on, into a scratch directory, and overwrites
// only `prepareIsolatedWorkerTest`'s own already-isolated copy — never the
// shared `workers/ssr/build/` the other eleven smokes depend on.

import { prepareIsolatedWorkerTest } from '../lib/isolated-worker-test.mjs';
import { attachCspViolationCapture, readCspViolations } from '../lib/csp-violation-capture.mjs';
import { SMOKE_ACCEPT_LANGUAGE } from '../lib/smoke-locale.mjs';

import { execFileSync, spawn } from 'node:child_process';
import { createHash, createHmac, randomBytes } from 'node:crypto';
import { cp, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptsDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = resolve(scriptsDir, '..');

const port = Number(process.env.SMOKE_PORT ?? 8811);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9263);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/handoff054';
const reportName = process.env.REPORT_NAME ?? 'handoff054-external-identity-callback-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-handoff054-identity-callback-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`identity-callback smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`identity-callback smoke is local-only; refused argument ${arg}`);
    }
  }
}

function logStep(message) {
  console.error(`[identity-callback-smoke] ${message}`);
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function hmac(pepper, value) {
  return createHmac('sha256', pepper).update(value).digest('hex');
}

function subjectLookup(pepper, namespaceId, subject) {
  // Matches `crypto::subject_lookup` exactly: an ASCII unit separator
  // (0x1F) sits between namespace and subject below, preventing
  // ("ns1","abc") and ("ns1a","bc") from colliding.
  return hmac(pepper, `${namespaceId}${subject}`);
}

function pkceChallenge(verifier) {
  return createHash('sha256').update(verifier).digest('base64url');
}

function randomToken() {
  return randomBytes(32).toString('hex');
}

// ── Build the isolated, feature-on artifact (never the shared build/) ────

async function buildFeatureOnArtifact() {
  const scratch = await mkdtemp(join(tmpdir(), 'handoff054-smoke-build-'));
  logStep('building an isolated dev_fake_issuer-enabled artifact (never touches workers/ssr/build/)');
  execFileSync(
    'worker-build',
    ['--release', '-d', scratch, 'workers/ssr', '--features', 'dev_fake_issuer'],
    { cwd: repositoryRoot, stdio: 'pipe' },
  );
  return scratch;
}

let dev;
let chrome;
let devStderr = '';
let chromeStderr = '';
const results = [];
let scratchBuildDir;
let isolated;

try {
  scratchBuildDir = await buildFeatureOnArtifact();

  isolated = await prepareIsolatedWorkerTest('handoff054-identity-callback');
  const pepper = isolated.pepper;

  // Overwrite only this isolation's own already-copied artifact with the
  // feature-on build — `prepareIsolatedWorkerTest` itself always copies the
  // shared `workers/ssr/build/`; this replaces that copy in place, inside
  // the isolated worker-root, never touching the shared directory itself.
  await cp(scratchBuildDir, isolated.workerArtifacts, { recursive: true });

  function runWrangler(args) {
    if (args.includes('--remote')) {
      throw new Error('identity-callback smoke refuses remote D1 operations');
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

  function query(statement) {
    const raw = runWrangler(['d1', 'execute', 'zinnias-ciao-dev', '--local', '--env', 'dev', '--json', '--command', statement]);
    const parsed = JSON.parse(raw);
    return parsed?.[0]?.results ?? parsed?.results ?? [];
  }

  const now = '2026-08-11T00:00:00.000Z';
  const farFutureExpiry = '2099-12-31T23:59:59.000Z';

  // The fake issuer authenticates exactly one fixed subject
  // (`dev-fake-subject-1`, `identity/dev_fake_issuer.rs`'s `FAKE_SUBJECT`) —
  // every successful round trip through it resolves to the SAME
  // subject_lookup digest, so exactly one seeded `user_identities` row
  // covers every "known identity" scenario below (sign-in success, replay,
  // tampered state, wrong nonce, the allowlist proof, and the no-JS pass
  // all authenticate as this same identity; minting a fresh session each
  // time is expected and not a conflict).
  const communityId = 'com_h054_identity';
  const userId = 'usr_h054_identity';
  const membershipId = 'mem_h054_identity';
  const identityId = 'idty_h054_identity';
  const fakeSubject = 'dev-fake-subject-1';
  const namespaceId = 'idns_local_fake';
  const seededSubjectLookup = subjectLookup(pepper, namespaceId, fakeSubject);

  function clean() {
    sql(`DELETE FROM auth_transactions WHERE callback_uri LIKE '${baseUrl}%'`);
    sql(`DELETE FROM sessions WHERE user_id = '${userId}'`);
    sql(`DELETE FROM user_identities WHERE id = '${identityId}'`);
    sql(`DELETE FROM community_memberships WHERE id = '${membershipId}'`);
    sql(`DELETE FROM communities WHERE id = '${communityId}'`);
    sql(`DELETE FROM users WHERE id = '${userId}'`);
  }

  function seed() {
    runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
    clean();
    const statements = [
      `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'Handoff054 Identity Community', 'Asia/Tokyo', 1, '${now}')`,
      `INSERT INTO users (id, created_at) VALUES ('${userId}', '${now}')`,
      `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${membershipId}', '${communityId}', '${userId}', 'member', 'H054 Identity Member', '${now}')`,
      `INSERT INTO user_identities (id, user_id, identity_namespace_id, subject_lookup, linked_at, status) VALUES ('${identityId}', '${userId}', '${namespaceId}', '${seededSubjectLookup}', '${now}', 'active')`,
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

  function sessionHmacFromSetCookie(setCookieHeader) {
    const match = /ciao_sid=([^;]+)/.exec(setCookieHeader ?? '');
    if (!match) return { secret: null, sessionHmac: null };
    const secret = match[1];
    return { secret, sessionHmac: hmac(pepper, secret) };
  }

  // ── Drives one full /identity/start → fake-issuer → /identity/callback
  // round trip via manual fetches (no cookie jar needed — the flow issues
  // its own session, it does not require one), returning every intermediate
  // response so a scenario can inspect or tamper with any hop.
  async function driveStart(action = 'sign_in') {
    const startRes = await fetch(`${baseUrl}/identity/start?action=${action}`, { redirect: 'manual' });
    const authorizeUrl = startRes.headers.get('location');
    return { startRes, authorizeUrl };
  }

  async function driveAuthorize(authorizeUrl) {
    const authorizeRes = await fetch(authorizeUrl, { redirect: 'manual' });
    const callbackUrl = authorizeRes.headers.get('location');
    return { authorizeRes, callbackUrl };
  }

  logStep('seeding local D1 fixtures (one active identity, one community membership)');
  seed();

  logStep(`starting local wrangler dev on ${baseUrl}`);
  dev = isolated.spawnDev(port);
  dev.stderr.on('data', (chunk) => {
    devStderr += chunk.toString();
  });
  await waitForServer(dev, () => devStderr);
  logStep('local wrangler dev (dev_fake_issuer build) is ready');

  // ── 1. Successful sign-in issuing a session with the new provenance ───

  logStep('scenario 1: successful sign-in issues a session with provenance external_identity');
  const s1Start = await driveStart('sign_in');
  const s1Authorize = await driveAuthorize(s1Start.authorizeUrl);
  const s1CallbackRes = await fetch(s1Authorize.callbackUrl, { redirect: 'manual' });
  const s1SetCookie = s1CallbackRes.headers.get('set-cookie');
  const { sessionHmac: s1SessionHmac } = sessionHmacFromSetCookie(s1SetCookie);
  const s1SessionRows = s1SessionHmac
    ? query(`SELECT provenance, user_id FROM sessions WHERE session_hmac = '${s1SessionHmac}'`)
    : [];
  results.push({
    name: 'successful-sign-in-issues-session-with-external-identity-provenance',
    observed: {
      startStatus: s1Start.startRes.status,
      authorizeStatus: s1Authorize.authorizeRes.status,
      callbackStatus: s1CallbackRes.status,
      callbackLocation: s1CallbackRes.headers.get('location'),
      hasSetCookie: Boolean(s1SetCookie),
      sessionRows: s1SessionRows,
    },
    checks: {
      startRedirectsToFakeIssuer: s1Start.startRes.status === 303
        && (s1Start.authorizeUrl ?? '').includes('/dev/identity/fake-issuer/authorize'),
      authorizeRedirectsToCallback: s1Authorize.authorizeRes.status === 303
        && (s1Authorize.callbackUrl ?? '').includes('/identity/callback'),
      callbackRedirectsToAllowedDestination: s1CallbackRes.status === 303
        && s1CallbackRes.headers.get('location') === '/',
      sessionCookieIssued: Boolean(s1SetCookie),
      exactlyOneSessionRow: s1SessionRows.length === 1,
      provenanceIsExternalIdentity: s1SessionRows[0]?.provenance === 'external_identity',
      sessionBelongsToSeededUser: s1SessionRows[0]?.user_id === userId,
    },
  });

  // ── 2. A replayed callback is rejected ─────────────────────────────────

  logStep('scenario 2: replaying the same callback URL is rejected');
  const s2ReplayRes = await fetch(s1Authorize.callbackUrl, { redirect: 'manual' });
  const s2Body = await s2ReplayRes.text();
  results.push({
    name: 'replayed-callback-rejected',
    observed: { status: s2ReplayRes.status, hasSetCookie: Boolean(s2ReplayRes.headers.get('set-cookie')) },
    checks: {
      genericFailureStatus: s2ReplayRes.status === 200,
      showsGenericFailureCopy: s2Body.includes('サインインを完了できませんでした'),
      noSessionCookieIssued: !s2ReplayRes.headers.get('set-cookie'),
      noRedirect: !s2ReplayRes.headers.get('location'),
    },
  });

  // ── 3. A tampered `state` is rejected ──────────────────────────────────

  logStep('scenario 3: a tampered state parameter is rejected');
  const s3Start = await driveStart('sign_in');
  const s3Authorize = await driveAuthorize(s3Start.authorizeUrl);
  const s3TamperedUrl = new URL(s3Authorize.callbackUrl);
  s3TamperedUrl.searchParams.set('state', `${s3TamperedUrl.searchParams.get('state')}-tampered`);
  const s3Res = await fetch(s3TamperedUrl.toString(), { redirect: 'manual' });
  const s3Body = await s3Res.text();
  results.push({
    name: 'tampered-state-rejected',
    observed: { status: s3Res.status },
    checks: {
      genericFailureStatus: s3Res.status === 200,
      showsGenericFailureCopy: s3Body.includes('サインインを完了できませんでした'),
      noSessionCookieIssued: !s3Res.headers.get('set-cookie'),
    },
  });

  // ── 4. A wrong `nonce` is rejected ─────────────────────────────────────

  logStep('scenario 4: a wrong nonce (tampered before reaching the provider) is rejected');
  const s4Start = await driveStart('sign_in');
  const s4TamperedAuthorizeUrl = new URL(s4Start.authorizeUrl);
  s4TamperedAuthorizeUrl.searchParams.set('nonce', randomToken());
  const s4Authorize = await driveAuthorize(s4TamperedAuthorizeUrl.toString());
  const s4Res = await fetch(s4Authorize.callbackUrl, { redirect: 'manual' });
  const s4Body = await s4Res.text();
  results.push({
    name: 'wrong-nonce-rejected',
    observed: { status: s4Res.status },
    checks: {
      genericFailureStatus: s4Res.status === 200,
      showsGenericFailureCopy: s4Body.includes('サインインを完了できませんでした'),
      noSessionCookieIssued: !s4Res.headers.get('set-cookie'),
    },
  });

  // ── 5. An out-of-allowlist return destination is refused ──────────────
  // §5.2: never accepted from a request parameter — there is no live route
  // through which a caller can set `return_to`, so this is seeded directly
  // in the transaction row (the only way it could ever arrive there),
  // proving the callback's own allowlist check fires regardless of what a
  // stored value claims.

  logStep('scenario 5: an out-of-allowlist return_to seeded directly in the transaction row is refused');
  const s5State = randomToken();
  const s5Nonce = randomToken();
  const s5Verifier = randomToken();
  const s5Challenge = pkceChallenge(s5Verifier);
  const s5TransactionId = randomToken();
  const s5CallbackUri = `${baseUrl}/identity/callback`;
  sql(
    `INSERT INTO auth_transactions ` +
      `(id, lookup_key_hmac, action, identity_namespace_id, nonce_hmac, pkce_verifier, ` +
      ` initiating_session_provenance, invite_reference, callback_uri, return_to, created_at, expires_at) ` +
      `VALUES ('${s5TransactionId}', '${hmac(pepper, s5State)}', 'sign_in', '${namespaceId}', ` +
      `'${hmac(pepper, s5Nonce)}', '${s5Verifier}', NULL, NULL, '${s5CallbackUri}', ` +
      `'//evil.example', '${now}', '${farFutureExpiry}')`,
  );
  const s5AuthorizeUrl =
    `${baseUrl}/dev/identity/fake-issuer/authorize?response_type=code` +
    `&client_id=zinnias-ciao-dev-fake-client&redirect_uri=${encodeURIComponent(s5CallbackUri)}` +
    `&state=${encodeURIComponent(s5State)}&nonce=${encodeURIComponent(s5Nonce)}` +
    `&code_challenge=${encodeURIComponent(s5Challenge)}&code_challenge_method=S256&scope=openid`;
  const s5Authorize = await driveAuthorize(s5AuthorizeUrl);
  const s5Res = await fetch(s5Authorize.callbackUrl, { redirect: 'manual' });
  results.push({
    name: 'out-of-allowlist-return-destination-refused',
    observed: {
      authorizeStatus: s5Authorize.authorizeRes.status,
      callbackStatus: s5Res.status,
      callbackLocation: s5Res.headers.get('location'),
      seededReturnTo: '//evil.example',
    },
    checks: {
      authorizeRedirectedToCallback: s5Authorize.authorizeRes.status === 303,
      callbackSucceeded: s5Res.status === 303,
      neverRedirectsToSeededDestination: s5Res.headers.get('location') !== '//evil.example',
      redirectsToTheSafeDefaultInstead: s5Res.headers.get('location') === '/',
      sessionCookieStillIssued: Boolean(s5Res.headers.get('set-cookie')),
    },
  });

  // ── 6. The whole flow with JavaScript disabled, in a real browser ─────

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
  // The proof: application JavaScript is fully disabled for the page before
  // the flow ever starts. `Runtime.evaluate` below still works — it runs
  // through the DevTools Protocol's own instrumentation, the same channel
  // that keeps a real DevTools console usable on a JS-disabled page — so it
  // remains a faithful, uninvolved observer rather than part of what is
  // disabled.
  await cdp.send('Emulation.setScriptExecutionDisabled', { value: true });
  await attachCspViolationCapture(cdp);

  logStep('scenario 6: driving /identity/start?action=sign_in via real browser navigation, JS disabled');
  const loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/identity/start?action=sign_in` });
  await withTimeout(loaded, 'no-JS identity sign-in navigation');
  await sleep(200);

  const finalState = await evalExpr(
    cdp,
    `(() => ({ path: location.pathname, text: document.body.innerText, cookiePresent: document.cookie.length > 0 }))()`,
  );
  const cspViolations = await readCspViolations(cdp);
  results.push({
    name: 'whole-flow-succeeds-with-javascript-disabled',
    observed: finalState,
    checks: {
      landedOnCommunityHome: finalState.path === `/c/${communityId}/home`,
      showsCommunityContent: finalState.text.includes('Handoff054 Identity Community'),
      doesNotShowFailureCopy: !finalState.text.includes('サインインを完了できませんでした'),
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
    note: 'RFC-080 §5/§5.1/§5.2 end-to-end proof (Handoff 054 §6): successful sign-in with external_identity provenance, replay/tampered-state/wrong-nonce rejections, a directly-seeded out-of-allowlist return_to refused server-side, and the whole flow succeeding with application JavaScript fully disabled.',
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
    console.error('[identity-callback-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[identity-callback-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  if (isolated) await isolated.cleanup();
  if (scratchBuildDir) await rm(scratchBuildDir, { recursive: true, force: true });
}
