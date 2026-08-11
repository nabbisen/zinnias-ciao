#!/usr/bin/env node
// End-to-end smoke for RFC-081 §4 / RFC-080 §6 (Handoff 056, external-
// identity Slice 5b): linking, collision rejection, and re-authentication
// rotation. Local wrangler dev only.
//
// Like the identity-callback smoke, this needs its own isolated,
// `dev_fake_issuer`-enabled artifact — never the shared `workers/ssr/build/`
// the other smokes depend on.
//
// The fake issuer authenticates exactly one fixed subject
// (`dev-fake-subject-1`), so this smoke's scenarios are ordered and
// composed deliberately:
//   1. Principal A links it — success, claiming the subject.
//   2. Principal B then attempts to link the same subject — collision,
//      since A already holds it.
//   3. Principal A's *own* newly-rotated session (from step 1) is staled
//      by directly updating its `authenticated_at` via SQL — the same
//      kind of fixture manipulation Handoff 054/055's smokes already use
//      — then A signs in again: this is the re-authentication case,
//      proven against the identity A itself just linked, not a separate
//      seeded identity (the fake issuer has only the one subject to give
//      out, so there is no other way to reach "already linked" here).
//
// Important plumbing note the earlier draft of this smoke got wrong: a
// real browser's top-level navigations carry cookies automatically across
// the whole redirect chain, but a hand-driven `fetch()` chain does not —
// each fetch here that needs the caller to still look authenticated
// (specifically, the re-authentication scenario's callback request) must
// forward the `Cookie` header explicitly, or `session::require_auth` at
// callback time sees no session and the flow silently falls back to an
// ordinary sign-in instead of a re-authentication.

import { prepareIsolatedWorkerTest } from '../lib/isolated-worker-test.mjs';
import { attachCspViolationCapture, readCspViolations } from '../lib/csp-violation-capture.mjs';

import { execFileSync, spawn } from 'node:child_process';
import { createHmac } from 'node:crypto';
import { cp, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptsDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = resolve(scriptsDir, '..');

const port = Number(process.env.SMOKE_PORT ?? 8831);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9283);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/handoff056';
const reportName = process.env.REPORT_NAME ?? 'handoff056-account-link-and-reauthentication-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-handoff056-link-reauth-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`link/reauth smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`link/reauth smoke is local-only; refused argument ${arg}`);
    }
  }
}

function logStep(message) {
  console.error(`[link-reauth-smoke] ${message}`);
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function buildFeatureOnArtifact() {
  const scratch = await mkdtemp(join(tmpdir(), 'handoff056-smoke-build-'));
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
  isolated = await prepareIsolatedWorkerTest('handoff056-link-reauth');
  const pepper = isolated.pepper;
  await cp(scratchBuildDir, isolated.workerArtifacts, { recursive: true });

  function hmac(value) {
    return createHmac('sha256', pepper).update(value).digest('hex');
  }

  function subjectLookup(namespaceId, subject) {
    return hmac(`${namespaceId}\u{1f}${subject}`);
  }

  function runWrangler(args) {
    if (args.includes('--remote')) {
      throw new Error('link/reauth smoke refuses remote D1 operations');
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

  const now = new Date().toISOString();
  const staleAuthenticatedAt = new Date(Date.now() - 30 * 60 * 1000).toISOString();
  const farFutureExpiry = '2099-12-31T23:59:59.000Z';
  const namespaceId = 'idns_local_fake';
  const fakeSubject = 'dev-fake-subject-1';
  const fakeSubjectLookup = subjectLookup(namespaceId, fakeSubject);

  // Principal A: link success (no pre-existing identity — must run first).
  const userAId = 'usr_h056_link';
  const sessionASecret = 'h056-smoke-link-session';
  const sessionAHmac = hmac(sessionASecret);

  // Principal B: collision (attempts to link the same fake subject after A already has it).
  const userBId = 'usr_h056_collision';
  const sessionBSecret = 'h056-smoke-collision-session';
  const sessionBHmac = hmac(sessionBSecret);

  // Re-authentication reuses principal A: after A's link scenario rotates
  // to a new session, that new session is staled directly via SQL (no
  // second identity link is possible — the fake issuer has only the one
  // fixed subject to give out).

  // Principal D: Relink-provenance, community-scoped — must be refused the link entry point entirely.
  const communityId = 'com_h056_link';
  const userDId = 'usr_h056_relink';
  const membershipDId = 'mem_h056_relink';
  const sessionDSecret = 'h056-smoke-relink-session';
  const sessionDHmac = hmac(sessionDSecret);

  // Principal E: any valid session, used only to prove action=join is unchanged.
  const userEId = 'usr_h056_join_unchanged';
  const sessionESecret = 'h056-smoke-join-unchanged-session';
  const sessionEHmac = hmac(sessionESecret);

  function clean() {
    sql(
      `DELETE FROM sessions WHERE session_hmac IN ` +
        `('${sessionAHmac}','${sessionBHmac}','${sessionDHmac}','${sessionEHmac}') ` +
        `OR user_id IN ('${userAId}','${userBId}','${userDId}','${userEId}')`,
    );
    sql(`DELETE FROM user_identities WHERE identity_namespace_id = '${namespaceId}' AND subject_lookup = '${fakeSubjectLookup}'`);
    sql(`DELETE FROM community_memberships WHERE id = '${membershipDId}'`);
    sql(`DELETE FROM communities WHERE id = '${communityId}'`);
    sql(`DELETE FROM users WHERE id IN ('${userAId}','${userBId}','${userDId}','${userEId}')`);
  }

  function seed() {
    runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
    clean();
    const statements = [
      `INSERT INTO users (id, created_at) VALUES ('${userAId}', '${now}')`,
      `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) VALUES ('sess_h056_a', '${userAId}', '${sessionAHmac}', '${now}', '${farFutureExpiry}', '${now}', 'invite_redemption', '${now}')`,

      `INSERT INTO users (id, created_at) VALUES ('${userBId}', '${now}')`,
      `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) VALUES ('sess_h056_b', '${userBId}', '${sessionBHmac}', '${now}', '${farFutureExpiry}', '${now}', 'invite_redemption', '${now}')`,

      `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'Handoff056 Link Community', 'Asia/Tokyo', 1, '${now}')`,
      `INSERT INTO users (id, created_at) VALUES ('${userDId}', '${now}')`,
      `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${membershipDId}', '${communityId}', '${userDId}', 'member', 'H056 Relink Member', '${now}')`,
      `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, scope_community_id, authenticated_at) VALUES ('sess_h056_d', '${userDId}', '${sessionDHmac}', '${now}', '${farFutureExpiry}', '${now}', 'relink', '${communityId}', '${now}')`,

      `INSERT INTO users (id, created_at) VALUES ('${userEId}', '${now}')`,
      `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) VALUES ('sess_h056_e', '${userEId}', '${sessionEHmac}', '${now}', '${farFutureExpiry}', '${now}', 'invite_redemption', '${now}')`,
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

  function cookieHeader(secret) {
    return { Cookie: `ciao_sid=${secret}` };
  }

  async function driveAuthorize(authorizeUrl) {
    const authorizeRes = await fetch(authorizeUrl, { redirect: 'manual' });
    const callbackUrl = authorizeRes.headers.get('location');
    return { authorizeRes, callbackUrl };
  }

  // Handoff 056 §10: evidence must never retain session identifiers. Row
  // `id`s are read for in-memory comparison (new vs. old, active-count)
  // but must never be written into a `results[].observed` field — this
  // strips them before a row set is recorded as evidence.
  function withoutSessionId(rows) {
    return rows.map(({ id: _id, ...rest }) => rest);
  }

  logStep('seeding local D1 fixtures');
  seed();

  logStep(`starting local wrangler dev on ${baseUrl}`);
  dev = isolated.spawnDev(port);
  dev.stderr.on('data', (chunk) => {
    devStderr += chunk.toString();
  });
  await waitForServer(dev, () => devStderr);
  logStep('local wrangler dev (dev_fake_issuer build) is ready');

  // ── Scenario: action=join unchanged for a signed-in session ───────────

  logStep('scenario: action=join is unchanged for an already-signed-in session');
  const joinRes = await fetch(`${baseUrl}/identity/start?action=join`, {
    redirect: 'manual',
    headers: cookieHeader(sessionESecret),
  });
  results.push({
    name: 'action_join_unchanged_for_signed_in_session',
    observed: { status: joinRes.status, location: joinRes.headers.get('location') },
    checks: {
      bouncesToHome: joinRes.status === 303 && joinRes.headers.get('location') === '/',
    },
  });

  // ── Scenario: link from a Relink session is refused entirely ──────────

  logStep('scenario: link entry point refuses a Relink-provenance session');
  const relinkLinkRes = await fetch(`${baseUrl}/account/link`, {
    redirect: 'manual',
    headers: cookieHeader(sessionDSecret),
  });
  results.push({
    name: 'link_refused_for_relink_session',
    observed: { status: relinkLinkRes.status },
    checks: { statusIsNotFound: relinkLinkRes.status === 404 },
  });

  // ── Scenario: link succeeds (principal A), rotates, audits ────────────

  logStep('scenario: principal A links the fake identity — success, rotation, audit');
  const getLinkA = await fetch(`${baseUrl}/account/link`, { headers: cookieHeader(sessionASecret) });
  const getLinkABody = await getLinkA.text();
  const tokenAMatch = /name="_token" value="([^"]+)"/.exec(getLinkABody);
  const tokenA = tokenAMatch?.[1];

  const postLinkA = await fetch(`${baseUrl}/account/link`, {
    method: 'POST',
    redirect: 'manual',
    headers: { ...cookieHeader(sessionASecret), 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({ _token: tokenA ?? '' }).toString(),
  });
  const linkAuthorizeUrl = postLinkA.headers.get('location');
  const linkAuthorize = await driveAuthorize(linkAuthorizeUrl ?? '');
  const linkCallbackRes = await fetch(linkAuthorize.callbackUrl, { redirect: 'manual' });
  // Handoff 057 §5.1: this is principal A's first-ever link, so the
  // callback now renders the account page directly (200, not a 303
  // redirect) with the newly-issued recovery credential revealed once —
  // never write the revealed code itself into evidence, only whether the
  // reveal markup is present.
  const linkCallbackBody = await linkCallbackRes.text();
  // The raw session secret from A's rotated cookie — reused below to drive
  // the re-authentication scenario against this same principal.
  const linkSessionSecret = /ciao_sid=([^;]+)/.exec(linkCallbackRes.headers.get('set-cookie') ?? '')?.[1];
  const linkSessionRows = linkSessionSecret
    ? query(`SELECT id, provenance, user_id FROM sessions WHERE session_hmac = '${hmac(linkSessionSecret)}'`)
    : [];
  const linkedIdentityRows = query(
    `SELECT user_id FROM user_identities WHERE identity_namespace_id='${namespaceId}' AND subject_lookup='${fakeSubjectLookup}'`,
  );
  const originalSessionARow = query(`SELECT revoked_at FROM sessions WHERE session_hmac = '${sessionAHmac}'`);
  const recoveryCredentialRows = query(
    `SELECT consumed_at, revoked_at FROM account_recovery_credentials WHERE user_id = '${userAId}'`,
  );
  results.push({
    name: 'link_succeeds_rotates_and_writes_identity',
    observed: {
      tokenFound: Boolean(tokenA),
      postLinkStatus: postLinkA.status,
      authorizeStatus: linkAuthorize.authorizeRes.status,
      callbackStatus: linkCallbackRes.status,
      callbackLocation: linkCallbackRes.headers.get('location'),
      linkSessionRows: withoutSessionId(linkSessionRows),
      linkedIdentityRows,
      originalSessionARow,
      recoveryCredentialRowCount: recoveryCredentialRows.length,
    },
    checks: {
      confirmationTokenPresent: Boolean(tokenA),
      postLinkRedirectsToProvider: postLinkA.status === 303 && (linkAuthorizeUrl ?? '').includes('/dev/identity/fake-issuer/authorize'),
      // Handoff 057 §5.1: the very first link reveals a just-issued
      // recovery credential directly (200), rather than redirecting —
      // there is nowhere else to carry the one-time plaintext code.
      callbackRevealsAccountPageWithNewCredential:
        linkCallbackRes.status === 200 && linkCallbackBody.includes('id="recovery-code-reveal"'),
      newSessionIssuedForUserA: linkSessionRows.length === 1 && linkSessionRows[0]?.user_id === userAId,
      newSessionProvenanceIsExternalIdentity: linkSessionRows[0]?.provenance === 'external_identity',
      identityLinkedToUserA: linkedIdentityRows.length === 1 && linkedIdentityRows[0]?.user_id === userAId,
      originalSessionRevokedByRotation: originalSessionARow[0]?.revoked_at != null,
      recoveryCredentialIssuedExactlyOnce: recoveryCredentialRows.length === 1,
    },
  });

  // ── Scenario: collision — principal B attempts the same fake identity ──

  logStep('scenario: principal B collides with the identity principal A already linked');
  const getLinkB = await fetch(`${baseUrl}/account/link`, { headers: cookieHeader(sessionBSecret) });
  const getLinkBBody = await getLinkB.text();
  const tokenB = /name="_token" value="([^"]+)"/.exec(getLinkBBody)?.[1];
  const postLinkB = await fetch(`${baseUrl}/account/link`, {
    method: 'POST',
    redirect: 'manual',
    headers: { ...cookieHeader(sessionBSecret), 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({ _token: tokenB ?? '' }).toString(),
  });
  const collisionAuthorizeUrl = postLinkB.headers.get('location');
  const collisionAuthorize = await driveAuthorize(collisionAuthorizeUrl ?? '');
  const collisionCallbackRes = await fetch(collisionAuthorize.callbackUrl, { redirect: 'manual' });
  const collisionCallbackBody = await collisionCallbackRes.text();
  const identityRowsAfterCollision = query(
    `SELECT user_id FROM user_identities WHERE identity_namespace_id='${namespaceId}' AND subject_lookup='${fakeSubjectLookup}'`,
  );
  results.push({
    name: 'collision_fails_closed_generically_no_row_written',
    observed: {
      callbackStatus: collisionCallbackRes.status,
      identityRowsAfterCollision,
    },
    checks: {
      genericFailureStatus: collisionCallbackRes.status === 200,
      showsGenericFailureCopy: collisionCallbackBody.includes('サインインを完了できませんでした'),
      noSessionCookieIssued: !collisionCallbackRes.headers.get('set-cookie'),
      stillExactlyOneIdentityRow: identityRowsAfterCollision.length === 1,
      identityStillBelongsToUserA: identityRowsAfterCollision[0]?.user_id === userAId,
    },
  });

  // ── Scenario: re-authentication (principal A's rotated session, staled) ──

  logStep("staling principal A's link-rotated session so it requires re-authentication");
  const linkSessionId = linkSessionRows[0]?.id;
  sql(`UPDATE sessions SET authenticated_at = '${staleAuthenticatedAt}' WHERE session_hmac = '${hmac(linkSessionSecret ?? '')}'`);

  logStep("scenario: principal A's stale valid session re-authenticates and rotates");
  const staleStartRes = await fetch(`${baseUrl}/identity/start?action=sign_in`, {
    redirect: 'manual',
    headers: cookieHeader(linkSessionSecret ?? ''),
  });
  const reauthAuthorizeUrl = staleStartRes.headers.get('location');
  const reauthAuthorize = await driveAuthorize(reauthAuthorizeUrl ?? '');
  // The callback request must carry the same session cookie as the
  // request that started this transaction — `sign_in_outcome` reads it via
  // `session::require_auth` on *this* request to decide re-authentication
  // vs. an ordinary sign-in. A hand-driven fetch chain doesn't forward
  // cookies across hops the way a real browser's navigation does, so it
  // must be forwarded explicitly here.
  const reauthCallbackRes = await fetch(reauthAuthorize.callbackUrl, {
    redirect: 'manual',
    headers: cookieHeader(linkSessionSecret ?? ''),
  });
  const reauthSessionSecret = /ciao_sid=([^;]+)/.exec(reauthCallbackRes.headers.get('set-cookie') ?? '')?.[1];
  const newSessionRowsForA = reauthSessionSecret
    ? query(`SELECT id, provenance, authenticated_at FROM sessions WHERE session_hmac = '${hmac(reauthSessionSecret)}'`)
    : [];
  const oldRotatedSessionRow = linkSessionSecret
    ? query(`SELECT id, revoked_at FROM sessions WHERE session_hmac = '${hmac(linkSessionSecret)}'`)
    : [];
  const activeSessionCountForA =
    query(`SELECT COUNT(*) AS active_count FROM sessions WHERE user_id='${userAId}' AND revoked_at IS NULL AND expires_at > '${now}'`)[0]
      ?.active_count ?? 0;
  results.push({
    name: 'stale_session_reauthenticates_with_new_session_id_and_old_revoked',
    observed: {
      startStatus: staleStartRes.status,
      startLocation: reauthAuthorizeUrl,
      authorizeStatus: reauthAuthorize.authorizeRes.status,
      callbackStatus: reauthCallbackRes.status,
      callbackLocation: reauthCallbackRes.headers.get('location'),
      newSessionRowsForA: withoutSessionId(newSessionRowsForA),
      oldRotatedSessionRowRevoked: oldRotatedSessionRow[0]?.revoked_at != null,
      activeSessionCountForA,
    },
    checks: {
      staleSessionNotBouncedToHome: staleStartRes.status === 303 && (reauthAuthorizeUrl ?? '').includes('/dev/identity/fake-issuer/authorize'),
      promptLoginSentOnReauth: (reauthAuthorizeUrl ?? '').includes('prompt=login'),
      callbackRedirectsToAccount: reauthCallbackRes.status === 303 && reauthCallbackRes.headers.get('location') === '/account',
      newSessionIssued: newSessionRowsForA.length === 1,
      newSessionHasDifferentIdThanOld: newSessionRowsForA[0]?.id !== linkSessionId,
      newSessionIsFresh: newSessionRowsForA[0]?.authenticated_at >= new Date(Date.now() - 60_000).toISOString(),
      oldSessionRevoked: oldRotatedSessionRow[0]?.revoked_at != null,
      exactlyOneActiveSessionRemains: activeSessionCountForA === 1,
    },
  });

  // ── The link flow with JavaScript disabled, in a real browser ─────────

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

  // Fresh principal for the browser-driven link pass. The fake issuer only
  // ever authenticates the one fixed subject, and principal A has already
  // claimed it above, so this attempt necessarily collides — which is
  // fine: the point of this pass is to prove the *whole* flow (the
  // confirmation form's plain HTML submit through to the generic failure
  // page) renders and navigates correctly with JavaScript disabled, not to
  // duplicate the already fetch-verified success case.
  const browserUserId = 'usr_h056_nojs_link_attempt';
  sql(`DELETE FROM sessions WHERE user_id = '${browserUserId}'`);
  sql(`DELETE FROM users WHERE id = '${browserUserId}'`);
  sql(`INSERT INTO users (id, created_at) VALUES ('${browserUserId}', '${now}')`);
  const browserSessionSecret = 'h056-smoke-nojs-link-session';
  const browserSessionHmac = hmac(browserSessionSecret);
  sql(
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) ` +
      `VALUES ('sess_h056_nojs', '${browserUserId}', '${browserSessionHmac}', '${now}', '${farFutureExpiry}', '${now}', 'invite_redemption', '${now}')`,
  );

  const target = await (await fetch(`http://127.0.0.1:${remotePort}/json/new`, { method: 'PUT' })).json();
  const cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  await cdp.send('Network.clearBrowserCookies');
  await cdp.send('Network.setCookie', {
    name: 'ciao_sid',
    value: browserSessionSecret,
    domain: '127.0.0.1',
    path: '/',
    httpOnly: true,
    secure: false,
    sameSite: 'Strict',
  });
  await cdp.send('Emulation.setScriptExecutionDisabled', { value: true });
  await attachCspViolationCapture(cdp);

  logStep('navigating to /account/link via real browser, JS disabled');
  const linkPageLoaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/account/link` });
  await withTimeout(linkPageLoaded, 'no-JS /account/link navigation');
  await sleep(150);

  logStep('submitting the confirmation form via real browser, JS disabled');
  const submitLoaded = cdp.once('Page.loadEventFired');
  const submitted = await evalExpr(
    cdp,
    `(() => {
      const form = document.querySelector('form[action="/account/link"]');
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  await withTimeout(submitLoaded, 'no-JS link confirmation submit');
  await sleep(150);

  const afterAuthorizeState = await evalExpr(cdp, `(() => ({ path: location.pathname, text: document.body.innerText }))()`);
  const cspViolations = await readCspViolations(cdp);
  results.push({
    name: 'link_flow_navigates_correctly_with_javascript_disabled',
    observed: { submitted, afterAuthorizeState },
    checks: {
      confirmationFormFoundAndSubmitted: submitted === true,
      // The fake issuer auto-approves with no interstitial (RFC-080 §9's
      // provider-page carve-out — not this application's own no-JS
      // surface), so a real, no-JS-driven navigation sails through the
      // whole redirect chain without needing a click or a script. It lands
      // on the *callback* URL rendering the generic collision-failure page
      // (the subject is already claimed by principal A), proving that
      // page also renders correctly with JavaScript disabled.
      landedOnCallbackWithGenericFailure:
        afterAuthorizeState.path === '/identity/callback' && afterAuthorizeState.text.includes('サインインを完了できませんでした'),
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
    note: 'RFC-081 §4 / RFC-080 §6 end-to-end proof (Handoff 056 §7): link success with rotation and audit, a Relink session refused the link entry point entirely, a collision failing closed with no row written, principal A\'s own newly-rotated session re-authenticating with a new session id and the old one revoked, action=join unchanged for a signed-in session, and the link confirmation flow (through to the generic collision-failure page) navigating correctly with application JavaScript fully disabled.',
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
    console.error('[link-reauth-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[link-reauth-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  if (isolated) await isolated.cleanup();
  if (scratchBuildDir) await rm(scratchBuildDir, { recursive: true, force: true });
}
