#!/usr/bin/env node
// End-to-end smoke for RFC-081 §3 (Handoff 057, external-identity Slice
// 5c): the recovery credential and unlink. Local wrangler dev only.
//
// Reuses the shared `workers/ssr/build/` artifact via
// `prepareIsolatedWorkerTest` directly — like `smoke:account-surface`,
// nothing here needs the `dev_fake_issuer` feature: every identity and
// credential is seeded straight into D1, no OIDC round trip involved
// (the fake-issuer-dependent link/re-authentication flows are already
// covered by `smoke:account-link-reauth`).
//
// Scenario coverage (Handoff 057 §7's required-test list):
//   - Consumption: valid (account-tier, fresh session minted), consumed,
//     revoked, expired, unknown — all four failure causes generic and
//     identical to the caller.
//   - Unlink: a second identity → succeeds; only a recovery credential →
//     succeeds; neither → refused, row untouched.
//   - The concurrency case, driven against real D1: two unlinks racing
//     on a two-identity account leave at least one active.
//   - Unlink refused from a Relink-provenance (community-scoped) session
//     and from a stale session (redirected to re-authenticate, not a
//     dead-end).
//   - The no-JS walkthrough Handoff 057 §7 names explicitly: refused (no
//     fallback yet) → generate → succeeds (the generated credential is
//     now the fallback) → consume (the revealed code, anonymously) — in
//     that order, which is logically coherent (generating *before*
//     attempting the now-succeeding unlink) even though it reads
//     differently from the handoff's own prose ordering; recorded in the
//     review request as a deliberate reading, not an oversight.
//
// Handoff 057 §10 / a lesson from Handoff 056's own review: the revealed
// plaintext code must never reach evidence. Every check below reads only
// whether reveal *markup* is present, or extracts the code into an
// in-memory variable consumed immediately by the next fetch — never
// written into a `results[].observed` field.

import { prepareIsolatedWorkerTest } from '../lib/isolated-worker-test.mjs';
import { attachCspViolationCapture, readCspViolations } from '../lib/csp-violation-capture.mjs';
import { SMOKE_ACCEPT_LANGUAGE } from '../lib/smoke-locale.mjs';

import { createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8841);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9293);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/handoff057';
const reportName = process.env.REPORT_NAME ?? 'handoff057-account-recovery-and-unlink-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-handoff057-recovery-unlink-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`recovery/unlink smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`recovery/unlink smoke is local-only; refused argument ${arg}`);
    }
  }
}

function logStep(message) {
  console.error(`[recovery-unlink-smoke] ${message}`);
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
  isolated = await prepareIsolatedWorkerTest('handoff057-recovery-unlink');
  const pepper = isolated.pepper;

  function hmac(value) {
    return createHmac('sha256', pepper).update(value).digest('hex');
  }

  // Mirrors `crypto::normalize_invite_code` exactly (strip whitespace and
  // hyphens, uppercase) — the server normalizes any submitted code before
  // hashing it, so a seeded `code_hmac` must be computed from the
  // *normalized* form or `find_valid_by_hmac` will never match it.
  function normalizeCode(raw) {
    return raw.replace(/[\s-]/g, '').toUpperCase();
  }

  function codeHmac(rawCode) {
    return hmac(normalizeCode(rawCode));
  }

  function runWrangler(args) {
    if (args.includes('--remote')) {
      throw new Error('recovery/unlink smoke refuses remote D1 operations');
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

  // Real wall-clock time — `authz::is_fresh_for_account_operations` and
  // the credential's own unexpired check both compare against the
  // worker's actual `worker::Date::now()` at request time.
  const now = new Date().toISOString();
  const pastExpiry = new Date(Date.now() - 60_000).toISOString();
  const staleAuthenticatedAt = new Date(Date.now() - 30 * 60 * 1000).toISOString();
  const farFutureExpiry = '2099-12-31T23:59:59.000Z';
  const namespaceId = 'idns_local_fake';

  function cookieHeader(secret) {
    return { Cookie: `ciao_sid=${secret}` };
  }

  function withoutSessionId(rows) {
    return rows.map(({ id: _id, ...rest }) => rest);
  }

  // ── Fixture identities ───────────────────────────────────────────────
  //
  // Every identity/credential row below is seeded directly via SQL, never
  // through the fake issuer — this smoke tests the DB/handler layer of
  // recovery and unlink, not the OIDC linking flow itself (already
  // covered by `smoke:account-link-reauth`).

  const noFallbackUserId = 'usr_h057_no_fallback';
  const noFallbackIdentityId = 'idty_h057_no_fallback';
  const noFallbackSessionSecret = 'h057-smoke-no-fallback-session';
  const noFallbackSessionHmac = hmac(noFallbackSessionSecret);

  const twoIdUserId = 'usr_h057_two_identities';
  const twoIdIdentityAId = 'idty_h057_two_a';
  const twoIdIdentityBId = 'idty_h057_two_b';
  const twoIdSessionSecret = 'h057-smoke-two-identities-session';
  const twoIdSessionHmac = hmac(twoIdSessionSecret);

  const credFallbackUserId = 'usr_h057_cred_fallback';
  const credFallbackIdentityId = 'idty_h057_cred_fallback';
  const credFallbackCredentialId = 'rec_h057_cred_fallback';
  const credFallbackSessionSecret = 'h057-smoke-cred-fallback-session';
  const credFallbackSessionHmac = hmac(credFallbackSessionSecret);

  const raceUserId = 'usr_h057_race';
  const raceIdentityAId = 'idty_h057_race_a';
  const raceIdentityBId = 'idty_h057_race_b';
  const raceSessionSecret = 'h057-smoke-race-session';
  const raceSessionHmac = hmac(raceSessionSecret);

  const relinkUserId = 'usr_h057_relink';
  const relinkIdentityId = 'idty_h057_relink';
  const communityId = 'com_h057_recovery';
  const relinkMembershipId = 'mem_h057_relink';
  const relinkSessionSecret = 'h057-smoke-relink-session';
  const relinkSessionHmac = hmac(relinkSessionSecret);

  const staleUserId = 'usr_h057_stale';
  const staleIdentityId = 'idty_h057_stale';
  const staleSessionSecret = 'h057-smoke-stale-session';
  const staleSessionHmac = hmac(staleSessionSecret);

  // Consumption scenarios: one principal per failure cause, one shared
  // "valid" credential belongs to its own principal.
  const consumeValidUserId = 'usr_h057_consume_valid';
  const consumeValidCredentialId = 'rec_h057_consume_valid';
  const consumeValidCode = 'CONSUME-VALID-CODE'; // never bound as a real cookie/secret; just the plaintext this smoke hashes itself.

  const consumeConsumedUserId = 'usr_h057_consume_consumed';
  const consumeConsumedCredentialId = 'rec_h057_consume_consumed';

  const consumeRevokedUserId = 'usr_h057_consume_revoked';
  const consumeRevokedCredentialId = 'rec_h057_consume_revoked';

  const consumeExpiredUserId = 'usr_h057_consume_expired';
  const consumeExpiredCredentialId = 'rec_h057_consume_expired';

  const noJsUserId = 'usr_h057_nojs';
  const noJsIdentityId = 'idty_h057_nojs';
  const noJsSessionSecret = 'h057-smoke-nojs-session';
  const noJsSessionHmac = hmac(noJsSessionSecret);

  const allUserIds = [
    noFallbackUserId,
    twoIdUserId,
    credFallbackUserId,
    raceUserId,
    relinkUserId,
    staleUserId,
    consumeValidUserId,
    consumeConsumedUserId,
    consumeRevokedUserId,
    consumeExpiredUserId,
    noJsUserId,
  ];
  const allSessionHmacs = [
    noFallbackSessionHmac,
    twoIdSessionHmac,
    credFallbackSessionHmac,
    raceSessionHmac,
    relinkSessionHmac,
    staleSessionHmac,
    noJsSessionHmac,
  ];

  function clean() {
    sql(
      `DELETE FROM sessions WHERE session_hmac IN ('${allSessionHmacs.join("','")}') ` +
        `OR user_id IN ('${allUserIds.join("','")}')`,
    );
    sql(`DELETE FROM account_recovery_credentials WHERE user_id IN ('${allUserIds.join("','")}')`);
    sql(`DELETE FROM user_identities WHERE user_id IN ('${allUserIds.join("','")}')`);
    sql(`DELETE FROM community_memberships WHERE id = '${relinkMembershipId}'`);
    sql(`DELETE FROM communities WHERE id = '${communityId}'`);
    sql(`DELETE FROM users WHERE id IN ('${allUserIds.join("','")}')`);
  }

  function seedIdentity(id, userId, subjectSuffix) {
    return `INSERT INTO user_identities (id, user_id, identity_namespace_id, subject_lookup, linked_at, status) VALUES ('${id}', '${userId}', '${namespaceId}', '${hmac(`h057-${subjectSuffix}`)}', '${now}', 'active')`;
  }

  function seedSession(id, userId, sessionHmac, provenance, scopeCommunityId, authenticatedAt) {
    if (scopeCommunityId) {
      return `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, scope_community_id, authenticated_at) VALUES ('${id}', '${userId}', '${sessionHmac}', '${now}', '${farFutureExpiry}', '${now}', '${provenance}', '${scopeCommunityId}', '${authenticatedAt}')`;
    }
    return `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) VALUES ('${id}', '${userId}', '${sessionHmac}', '${now}', '${farFutureExpiry}', '${now}', '${provenance}', '${authenticatedAt}')`;
  }

  function seed() {
    runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
    clean();
    const statements = [
      `INSERT INTO users (id, created_at) VALUES ('${noFallbackUserId}', '${now}')`,
      seedIdentity(noFallbackIdentityId, noFallbackUserId, 'no-fallback'),
      seedSession('sess_h057_no_fallback', noFallbackUserId, noFallbackSessionHmac, 'external_identity', null, now),

      `INSERT INTO users (id, created_at) VALUES ('${twoIdUserId}', '${now}')`,
      seedIdentity(twoIdIdentityAId, twoIdUserId, 'two-a'),
      seedIdentity(twoIdIdentityBId, twoIdUserId, 'two-b'),
      seedSession('sess_h057_two', twoIdUserId, twoIdSessionHmac, 'external_identity', null, now),

      `INSERT INTO users (id, created_at) VALUES ('${credFallbackUserId}', '${now}')`,
      seedIdentity(credFallbackIdentityId, credFallbackUserId, 'cred-fallback'),
      `INSERT INTO account_recovery_credentials (id, user_id, code_hmac, created_at, expires_at) VALUES ('${credFallbackCredentialId}', '${credFallbackUserId}', '${hmac('h057-cred-fallback-code')}', '${now}', NULL)`,
      seedSession('sess_h057_cred_fallback', credFallbackUserId, credFallbackSessionHmac, 'external_identity', null, now),

      `INSERT INTO users (id, created_at) VALUES ('${raceUserId}', '${now}')`,
      seedIdentity(raceIdentityAId, raceUserId, 'race-a'),
      seedIdentity(raceIdentityBId, raceUserId, 'race-b'),
      seedSession('sess_h057_race', raceUserId, raceSessionHmac, 'external_identity', null, now),

      `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'Handoff057 Recovery Community', 'Asia/Tokyo', 1, '${now}')`,
      `INSERT INTO users (id, created_at) VALUES ('${relinkUserId}', '${now}')`,
      seedIdentity(relinkIdentityId, relinkUserId, 'relink'),
      `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${relinkMembershipId}', '${communityId}', '${relinkUserId}', 'member', 'H057 Relink Member', '${now}')`,
      seedSession('sess_h057_relink', relinkUserId, relinkSessionHmac, 'relink', communityId, now),

      `INSERT INTO users (id, created_at) VALUES ('${staleUserId}', '${now}')`,
      seedIdentity(staleIdentityId, staleUserId, 'stale'),
      seedSession('sess_h057_stale', staleUserId, staleSessionHmac, 'external_identity', null, staleAuthenticatedAt),

      `INSERT INTO users (id, created_at) VALUES ('${consumeValidUserId}', '${now}')`,
      `INSERT INTO account_recovery_credentials (id, user_id, code_hmac, created_at, expires_at) VALUES ('${consumeValidCredentialId}', '${consumeValidUserId}', '${codeHmac(consumeValidCode)}', '${now}', NULL)`,

      `INSERT INTO users (id, created_at) VALUES ('${consumeConsumedUserId}', '${now}')`,
      `INSERT INTO account_recovery_credentials (id, user_id, code_hmac, created_at, expires_at, consumed_at) VALUES ('${consumeConsumedCredentialId}', '${consumeConsumedUserId}', '${codeHmac('h057-consumed-code')}', '${now}', NULL, '${now}')`,

      `INSERT INTO users (id, created_at) VALUES ('${consumeRevokedUserId}', '${now}')`,
      `INSERT INTO account_recovery_credentials (id, user_id, code_hmac, created_at, expires_at, revoked_at) VALUES ('${consumeRevokedCredentialId}', '${consumeRevokedUserId}', '${codeHmac('h057-revoked-code')}', '${now}', NULL, '${now}')`,

      `INSERT INTO users (id, created_at) VALUES ('${consumeExpiredUserId}', '${now}')`,
      // The application itself never sets expires_at (migration 0017's
      // comment explains why) — this row proves the schema's defensive
      // expiry check still works correctly on the rare path that does
      // set one.
      `INSERT INTO account_recovery_credentials (id, user_id, code_hmac, created_at, expires_at) VALUES ('${consumeExpiredCredentialId}', '${consumeExpiredUserId}', '${codeHmac('h057-expired-code')}', '${now}', '${pastExpiry}')`,

      `INSERT INTO users (id, created_at) VALUES ('${noJsUserId}', '${now}')`,
      seedIdentity(noJsIdentityId, noJsUserId, 'nojs'),
      seedSession('sess_h057_nojs', noJsUserId, noJsSessionHmac, 'external_identity', null, now),
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

  function extractToken(body) {
    return /name="_token" value="([^"]+)"/.exec(body)?.[1];
  }

  async function getUnlinkToken(sessionSecret, identityId) {
    const res = await fetch(`${baseUrl}/account/unlink/${identityId}`, { headers: cookieHeader(sessionSecret) });
    const body = await res.text();
    return { status: res.status, token: extractToken(body), body };
  }

  async function postUnlink(sessionSecret, identityId, token) {
    return fetch(`${baseUrl}/account/unlink/${identityId}`, {
      method: 'POST',
      redirect: 'manual',
      headers: { ...cookieHeader(sessionSecret), 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({ _token: token ?? '' }).toString(),
    });
  }

  async function getRegenerateToken(sessionSecret) {
    const res = await fetch(`${baseUrl}/account`, { headers: cookieHeader(sessionSecret) });
    const body = await res.text();
    return extractToken(body);
  }

  async function postRegenerate(sessionSecret, token) {
    return fetch(`${baseUrl}/account/recovery/regenerate`, {
      method: 'POST',
      redirect: 'manual',
      headers: { ...cookieHeader(sessionSecret), 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({ _token: token ?? '' }).toString(),
    });
  }

  function extractRevealedCode(body) {
    const match = /cz-account-recovery-code-display[^>]*>([^<]+)</.exec(body);
    return match?.[1];
  }

  // RFC-078 / Handoff 057 §5.2: `Scope::Recovery`'s abuse-limiter budget
  // (5 requests / 5 minutes) is keyed on the caller's canonical client
  // network — every request this smoke makes shares the same local
  // 127.0.0.1 address unless told otherwise, so without a distinct
  // synthetic `CF-Connecting-IP` per scenario, this smoke's *own*
  // fetch-based consumption scenarios exhaust the budget before the
  // no-JS scenario's own legitimate attempt runs, producing a false
  // failure that looks like a product bug but is a shared-fixture
  // collision (confirmed by reproducing it in isolation before writing
  // this fix). TEST-NET-3 (203.0.113.0/24, RFC 5737), never a real
  // address.
  function syntheticClientIp(index) {
    return `203.0.113.${20 + index}`;
  }

  async function getRecoveryToken(clientIp) {
    const res = await fetch(`${baseUrl}/recovery`, { headers: { 'CF-Connecting-IP': clientIp } });
    const body = await res.text();
    return extractToken(body);
  }

  async function postRecovery(code, token, clientIp) {
    return fetch(`${baseUrl}/recovery`, {
      method: 'POST',
      redirect: 'manual',
      headers: { 'CF-Connecting-IP': clientIp, 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({ code, _token: token ?? '' }).toString(),
    });
  }

  logStep('seeding local D1 fixtures');
  seed();

  logStep(`starting local wrangler dev on ${baseUrl}`);
  dev = isolated.spawnDev(port);
  dev.stderr.on('data', (chunk) => {
    devStderr += chunk.toString();
  });
  await waitForServer(dev, () => devStderr);
  logStep('local wrangler dev is ready');

  // ── Unlink refused: only method, no fallback ───────────────────────────

  logStep('scenario: unlink refused — the only identity, no recovery credential fallback');
  const noFallbackToken = await getUnlinkToken(noFallbackSessionSecret, noFallbackIdentityId);
  const noFallbackPost = await postUnlink(noFallbackSessionSecret, noFallbackIdentityId, noFallbackToken.token);
  const noFallbackPostBody = await noFallbackPost.text();
  const noFallbackRowAfter = query(`SELECT status FROM user_identities WHERE id = '${noFallbackIdentityId}'`);
  results.push({
    name: 'unlink_refused_with_no_other_usable_method',
    observed: { confirmStatus: noFallbackToken.status, postStatus: noFallbackPost.status, rowAfter: noFallbackRowAfter },
    checks: {
      confirmationTokenPresent: Boolean(noFallbackToken.token),
      refusedGenerically: noFallbackPost.status === 200 && noFallbackPostBody.includes('連携を解除できませんでした'),
      rowUntouched: noFallbackRowAfter.length === 1 && noFallbackRowAfter[0]?.status === 'active',
    },
  });

  // ── Unlink succeeds: a second identity remains ─────────────────────────

  logStep('scenario: unlink succeeds — a second linked identity remains as the fallback');
  const twoIdToken = await getUnlinkToken(twoIdSessionSecret, twoIdIdentityAId);
  const twoIdPost = await postUnlink(twoIdSessionSecret, twoIdIdentityAId, twoIdToken.token);
  const twoIdRowA = query(`SELECT status FROM user_identities WHERE id = '${twoIdIdentityAId}'`);
  const twoIdRowB = query(`SELECT status FROM user_identities WHERE id = '${twoIdIdentityBId}'`);
  const twoIdActiveSessions = query(
    `SELECT id FROM sessions WHERE user_id = '${twoIdUserId}' AND revoked_at IS NULL AND expires_at > '${now}'`,
  );
  results.push({
    name: 'unlink_succeeds_with_a_second_identity_remaining',
    observed: { postStatus: twoIdPost.status, twoIdRowA, twoIdRowB, activeSessionCount: twoIdActiveSessions.length },
    checks: {
      redirectsToAccount: twoIdPost.status === 303 && twoIdPost.headers.get('location') === '/account',
      unlinkedIdentityRevoked: twoIdRowA[0]?.status === 'revoked',
      otherIdentityStillActive: twoIdRowB[0]?.status === 'active',
    },
  });

  // ── Unlink succeeds: only a recovery credential remains ────────────────

  logStep('scenario: unlink succeeds — an unconsumed recovery credential is the fallback');
  const credToken = await getUnlinkToken(credFallbackSessionSecret, credFallbackIdentityId);
  const credPost = await postUnlink(credFallbackSessionSecret, credFallbackIdentityId, credToken.token);
  const credRow = query(`SELECT status FROM user_identities WHERE id = '${credFallbackIdentityId}'`);
  results.push({
    name: 'unlink_succeeds_with_only_a_recovery_credential_remaining',
    observed: { postStatus: credPost.status, credRow },
    checks: {
      redirectsToAccount: credPost.status === 303 && credPost.headers.get('location') === '/account',
      identityRevoked: credRow[0]?.status === 'revoked',
    },
  });

  // ── The concurrency race, against real D1 ──────────────────────────────

  logStep('scenario: two unlinks race on a two-identity account — driven against real D1');
  const raceTokenA = await getUnlinkToken(raceSessionSecret, raceIdentityAId);
  const raceTokenB = await getUnlinkToken(raceSessionSecret, raceIdentityBId);
  const [raceResultA, raceResultB] = await Promise.all([
    postUnlink(raceSessionSecret, raceIdentityAId, raceTokenA.token),
    postUnlink(raceSessionSecret, raceIdentityBId, raceTokenB.token),
  ]);
  const raceActiveIdentities = query(
    `SELECT id FROM user_identities WHERE user_id = '${raceUserId}' AND status = 'active'`,
  );
  const raceSuccessCount = [raceResultA, raceResultB].filter((r) => r.status === 303).length;
  results.push({
    name: 'concurrent_unlinks_on_a_two_identity_account_leave_at_least_one_active',
    observed: {
      statusA: raceResultA.status,
      statusB: raceResultB.status,
      activeIdentityCount: raceActiveIdentities.length,
      successCount: raceSuccessCount,
    },
    checks: {
      atLeastOneIdentityStillActive: raceActiveIdentities.length >= 1,
      // The stronger, expected result: the guard's shared usable-method
      // definition means exactly one request should win, not both.
      exactlyOneSucceeded: raceSuccessCount === 1,
      exactlyOneActiveIdentityRemains: raceActiveIdentities.length === 1,
    },
  });

  // ── Unlink refused: Relink-provenance (community-scoped) session ──────

  logStep('scenario: unlink refused entirely for a Relink-provenance session');
  const relinkGet = await fetch(`${baseUrl}/account/unlink/${relinkIdentityId}`, { headers: cookieHeader(relinkSessionSecret) });
  const relinkPost = await postUnlink(relinkSessionSecret, relinkIdentityId, 'irrelevant');
  results.push({
    name: 'unlink_refused_for_relink_provenance_session',
    observed: { getStatus: relinkGet.status, postStatus: relinkPost.status },
    checks: {
      getIsNotFound: relinkGet.status === 404,
      postIsNotFound: relinkPost.status === 404,
    },
  });

  // ── Unlink refused: stale session redirected to re-authenticate ───────

  logStep('scenario: unlink from a stale session redirects to re-authenticate, not a dead end');
  const staleGet = await fetch(`${baseUrl}/account/unlink/${staleIdentityId}`, {
    redirect: 'manual',
    headers: cookieHeader(staleSessionSecret),
  });
  const staleRowBefore = query(`SELECT status FROM user_identities WHERE id = '${staleIdentityId}'`);
  results.push({
    name: 'unlink_refused_for_stale_session_redirects_to_reauthenticate',
    observed: { getStatus: staleGet.status, getLocation: staleGet.headers.get('location'), staleRowBefore },
    checks: {
      redirectsToReauthenticate: staleGet.status === 303 && staleGet.headers.get('location') === '/identity/start?action=sign_in',
      rowUntouched: staleRowBefore[0]?.status === 'active',
    },
  });

  // ── Consumption: valid code mints an account-tier, fresh session ──────

  logStep('scenario: consuming a valid code mints an account-tier, fresh session');
  const validToken = await getRecoveryToken(syntheticClientIp(0));
  const validRes = await postRecovery(consumeValidCode, validToken, syntheticClientIp(0));
  const validSessionSecret = /ciao_sid=([^;]+)/.exec(validRes.headers.get('set-cookie') ?? '')?.[1];
  const validSessionRows = validSessionSecret
    ? query(
        `SELECT provenance, scope_community_id, authenticated_at, user_id FROM sessions WHERE session_hmac = '${hmac(validSessionSecret)}'`,
      )
    : [];
  const validCredentialRow = query(`SELECT consumed_at FROM account_recovery_credentials WHERE id = '${consumeValidCredentialId}'`);
  results.push({
    name: 'consumption_of_valid_code_mints_account_tier_fresh_session',
    observed: {
      status: validRes.status,
      location: validRes.headers.get('location'),
      validSessionRows,
      validCredentialRow,
    },
    checks: {
      redirectsToAccount: validRes.status === 303 && validRes.headers.get('location') === '/account',
      sessionIssued: validSessionRows.length === 1 && validSessionRows[0]?.user_id === consumeValidUserId,
      sessionProvenanceIsAccountRecovery: validSessionRows[0]?.provenance === 'account_recovery',
      sessionIsUnscoped: !validSessionRows[0]?.scope_community_id,
      sessionIsFresh: validSessionRows[0]?.authenticated_at >= new Date(Date.now() - 60_000).toISOString(),
      credentialMarkedConsumed: validCredentialRow[0]?.consumed_at != null,
    },
  });

  // ── Consumption: consumed / revoked / expired / unknown — identical ───

  logStep('scenario: consumption failures (consumed, revoked, expired, unknown) are all identical');
  async function attemptConsumption(code, clientIp) {
    const token = await getRecoveryToken(clientIp);
    const res = await postRecovery(code, token, clientIp);
    const body = await res.text();
    return { status: res.status, hasSetCookie: Boolean(res.headers.get('set-cookie')), body };
  }
  const consumedAttempt = await attemptConsumption('h057-consumed-code', syntheticClientIp(1));
  const revokedAttempt = await attemptConsumption('h057-revoked-code', syntheticClientIp(2));
  const expiredAttempt = await attemptConsumption('h057-expired-code', syntheticClientIp(3));
  const unknownAttempt = await attemptConsumption('completely-unknown-code-value', syntheticClientIp(4));
  const allAttempts = [consumedAttempt, revokedAttempt, expiredAttempt, unknownAttempt];
  // Each render embeds a fresh single-use CSRF token in a hidden field —
  // stripped before comparing, since that value is *supposed* to differ
  // per response regardless of cause and would otherwise make every
  // comparison fail for a reason unrelated to what this check is for.
  const stripToken = (body) => body.replace(/name="_token" value="[^"]*"/, 'name="_token" value=""');
  results.push({
    name: 'consumption_failures_are_generic_and_identical_across_all_causes',
    observed: {
      statuses: allAttempts.map((a) => a.status),
      anySetCookie: allAttempts.some((a) => a.hasSetCookie),
    },
    checks: {
      allSameStatus: allAttempts.every((a) => a.status === consumedAttempt.status),
      // RFC-054 A1 (Handoff 060): the message no longer claims an expiry
      // recovery credentials cannot have — updated with the constant.
      allShowGenericInvalidMessage: allAttempts.every((a) => a.body.includes('このコードは使用できません。すでに使われているか、正しくない可能性があります')),
      noneIssuedASessionCookie: allAttempts.every((a) => !a.hasSetCookie),
      bodiesIdenticalApartFromTheirOwnCsrfToken: allAttempts.every(
        (a) => stripToken(a.body) === stripToken(consumedAttempt.body),
      ),
    },
  });

  // ── The full flow with JavaScript disabled, in a real browser ─────────
  //
  // Handoff 057 §7's own ordering (generate → unlink-refused → consume →
  // unlink-succeeds) can't be driven literally: generating a credential
  // *before* the first unlink attempt would make that attempt succeed,
  // not get refused, since the credential is itself a usable fallback.
  // This walkthrough covers the same four pieces of functionality in the
  // only order that is internally consistent: refused (no fallback yet)
  // → generate → succeeds (now there is one) → consume (the revealed
  // code, anonymously, confirming it actually works).

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

    on(method, cb) {
      this.events.set(method, [...(this.events.get(method) ?? []), cb]);
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
    value: noJsSessionSecret,
    domain: '127.0.0.1',
    path: '/',
    httpOnly: true,
    secure: false,
    sameSite: 'Strict',
  });
  await cdp.send('Emulation.setScriptExecutionDisabled', { value: true });
  await attachCspViolationCapture(cdp);

  logStep('no-JS: unlink refused (no fallback yet)');
  let loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/account/unlink/${noJsIdentityId}` });
  await withTimeout(loaded, 'no-JS unlink confirm navigation');
  await sleep(150);
  let submitLoaded = cdp.once('Page.loadEventFired');
  const firstSubmitted = await evalExpr(
    cdp,
    `(() => {
      const form = document.querySelector('form[action^="/account/unlink/"]');
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  await withTimeout(submitLoaded, 'no-JS unlink first submit');
  await sleep(150);
  const afterFirstUnlink = await evalExpr(cdp, `(() => ({ path: location.pathname, text: document.body.innerText }))()`);

  logStep('no-JS: generate a recovery credential');
  loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/account` });
  await withTimeout(loaded, 'no-JS /account navigation before generate');
  await sleep(150);
  submitLoaded = cdp.once('Page.loadEventFired');
  const generateSubmitted = await evalExpr(
    cdp,
    `(() => {
      const form = document.querySelector('form[action="/account/recovery/regenerate"]');
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  await withTimeout(submitLoaded, 'no-JS generate submit');
  await sleep(150);
  // The revealed code is read into this orchestrating process only long
  // enough to type it into the *next* page's form below — the same way
  // every other fetch-based scenario above already holds a session
  // secret in a local variable. It is never written into `observed`,
  // `results`, or any log statement; a page navigation resets the
  // browser's own JS context, so there is no way to carry it forward
  // *inside* the browser without exactly this kind of orchestrator-side
  // relay.
  const revealState = await evalExpr(
    cdp,
    `(() => ({
      path: location.pathname,
      hasReveal: Boolean(document.getElementById('recovery-code-reveal')),
      code: document.querySelector('.cz-account-recovery-code-display')?.textContent ?? '',
    }))()`,
  );
  const revealedCode = revealState.code;
  if (!/^[A-Z0-9-]+$/.test(revealedCode)) {
    throw new Error('revealed recovery code contained unexpected characters — refusing to interpolate it into a script expression');
  }

  logStep('no-JS: unlink succeeds now that a recovery credential exists');
  loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/account/unlink/${noJsIdentityId}` });
  await withTimeout(loaded, 'no-JS unlink confirm navigation (second)');
  await sleep(150);
  submitLoaded = cdp.once('Page.loadEventFired');
  const secondSubmitted = await evalExpr(
    cdp,
    `(() => {
      const form = document.querySelector('form[action^="/account/unlink/"]');
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  await withTimeout(submitLoaded, 'no-JS unlink second submit');
  await sleep(150);
  const afterSecondUnlink = await evalExpr(cdp, `(() => ({ path: location.pathname }))()`);

  logStep('no-JS: consume the revealed code anonymously to sign back in');
  loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Network.clearBrowserCookies');
  // A distinct synthetic client IP, same reasoning as `syntheticClientIp`
  // above — this browser-driven request must not share the fetch-based
  // consumption scenarios' abuse-limiter budget.
  await cdp.send('Network.setExtraHTTPHeaders', {
    headers: { 'CF-Connecting-IP': syntheticClientIp(5), 'Accept-Language': SMOKE_ACCEPT_LANGUAGE },
  });
  await cdp.send('Page.navigate', { url: `${baseUrl}/recovery` });
  await withTimeout(loaded, 'no-JS /recovery navigation');
  await sleep(150);
  submitLoaded = cdp.once('Page.loadEventFired');
  // The navigation above reset the browser's JS context, so the code
  // captured earlier (held only in this orchestrator process, never
  // logged) is typed into the fresh page's form here — validated above
  // as alphanumeric-plus-hyphen only, so direct interpolation into this
  // expression carries no injection risk.
  const consumeSubmitted = await evalExpr(
    cdp,
    `(() => {
      const form = document.querySelector('form[action="/recovery"]');
      const input = document.getElementById('code');
      if (!form || !input) return false;
      input.value = ${JSON.stringify(revealedCode)};
      form.requestSubmit();
      return true;
    })()`,
  );
  await withTimeout(submitLoaded, 'no-JS recovery consume submit');
  await sleep(150);
  const afterConsume = await evalExpr(cdp, `(() => ({ path: location.pathname }))()`);

  const cspViolations = await readCspViolations(cdp);
  const noJsIdentityFinalRow = query(`SELECT status FROM user_identities WHERE id = '${noJsIdentityId}'`);
  // Deliberately omits `revealedCode`/`revealState.code` — never written
  // into evidence, only `hasReveal` (a boolean) is.
  results.push({
    name: 'refused_then_generate_then_succeeds_then_consume_with_javascript_disabled',
    observed: {
      firstSubmitted,
      afterFirstUnlink: { path: afterFirstUnlink.path },
      generateSubmitted,
      revealPath: revealState.path,
      hasReveal: revealState.hasReveal,
      secondSubmitted,
      afterSecondUnlink,
      consumeSubmitted,
      afterConsume,
    },
    checks: {
      firstAttemptRefusedGenerically:
        firstSubmitted === true && afterFirstUnlink.text.includes('連携を解除できませんでした'),
      generateFormSubmittedAndRevealed: generateSubmitted === true && revealState.hasReveal === true,
      secondAttemptSucceeded: secondSubmitted === true && afterSecondUnlink.path === '/account',
      identityNowRevoked: noJsIdentityFinalRow[0]?.status === 'revoked',
      consumeLandedOnAccount: consumeSubmitted === true && afterConsume.path === '/account',
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
    note: 'RFC-081 §3 end-to-end proof (Handoff 057 §7): unlink refused with no fallback and row untouched, unlink succeeding with a second identity or a recovery credential as the fallback, the concurrency race against real D1 leaving exactly one identity active, unlink refused for a Relink-provenance session and redirected-to-reauthenticate for a stale one, consumption of a valid code minting an account-tier fresh session, all four consumption failure causes generic and byte-identical, and the refused/generate/succeeds/consume sequence navigating correctly with application JavaScript fully disabled.',
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
    console.error('[recovery-unlink-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[recovery-unlink-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  if (isolated) await isolated.cleanup();
}
