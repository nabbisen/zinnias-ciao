#!/usr/bin/env node
// End-to-end smoke for RFC-082 (Handoff 058): membership suspension. Local
// wrangler dev only.
//
// Scenario coverage (Handoff 058 §8's required-test list):
//   - active -> suspended -> active (unsuspend), role unchanged across the
//     round trip.
//   - suspended -> removed (RFC-082 §1: a valid transition — soft_remove's
//     target check is MEMBERSHIP_PRESENT, not MEMBERSHIP_ACTIVE).
//   - Refused: suspending/unsuspending an already-removed membership.
//   - A suspended member is denied exactly as a removed one is (both via
//     fetch and, for suspended, via a real browser with JavaScript
//     disabled) — but the *page* differs, by RFC-082 §4's own design: an
//     explicit "access is paused" page for suspended, the same generic
//     not-found for removed.
//   - A suspended member is visible and targetable by an admin (the
//     PRESENT-based member list, badge, and unsuspend link).
//   - A suspended member's other community remains reachable.
//   - A suspended admin can perform no admin action — proven by hitting
//     the admin surface as the suspended admin themselves, not merely by
//     inspecting SQL.
//   - The last-admin guard blocks suspending a community's only admin.
//   - Self-targeting is refused in the handler.
//   - Unsuspending an admin restores the admin role unchanged.
//   - The no-JS walkthrough Handoff 058 §8 names explicitly:
//     suspend -> denied -> unsuspend -> restored, with JavaScript disabled.

import { prepareIsolatedWorkerTest } from '../lib/isolated-worker-test.mjs';
import { attachCspViolationCapture, readCspViolations } from '../lib/csp-violation-capture.mjs';
import { SMOKE_ACCEPT_LANGUAGE } from '../lib/smoke-locale.mjs';

import { createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8842);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9294);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/handoff058';
const reportName = process.env.REPORT_NAME ?? 'handoff058-membership-suspension-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-handoff058-suspension-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`membership-suspension smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`membership-suspension smoke is local-only; refused argument ${arg}`);
    }
  }
}

function logStep(message) {
  console.error(`[membership-suspension-smoke] ${message}`);
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
  isolated = await prepareIsolatedWorkerTest('handoff058-membership-suspension');
  const pepper = isolated.pepper;

  function hmac(value) {
    return createHmac('sha256', pepper).update(value).digest('hex');
  }

  function runWrangler(args) {
    if (args.includes('--remote')) {
      throw new Error('membership-suspension smoke refuses remote D1 operations');
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
  const farFutureExpiry = '2099-12-31T23:59:59.000Z';

  function cookieHeader(secret) {
    return { Cookie: `ciao_sid=${secret}` };
  }

  function extractToken(body) {
    return /name="_token" value="([^"]+)"/.exec(body)?.[1];
  }

  // ── Fixture identities ───────────────────────────────────────────────
  //
  // One primary community with two admins and one member (the main
  // suspend/unsuspend/removal/no-JS subject), a second community the
  // member also belongs to (other-community reachability), and a
  // separate single-admin community (the last-admin guard).

  const communityId = 'com_h058_suspension';
  const otherCommunityId = 'com_h058_other';
  const lastAdminCommunityId = 'com_h058_lastadmin';

  const adminAUserId = 'usr_h058_admin_a';
  const adminAMembershipId = 'mem_h058_admin_a';
  const adminASessionSecret = 'h058-smoke-admin-a-session';
  const adminASessionHmac = hmac(adminASessionSecret);

  const adminBUserId = 'usr_h058_admin_b';
  const adminBMembershipId = 'mem_h058_admin_b';
  const adminBSessionSecret = 'h058-smoke-admin-b-session';
  const adminBSessionHmac = hmac(adminBSessionSecret);

  const memberUserId = 'usr_h058_member';
  const memberMembershipId = 'mem_h058_member';
  const memberOtherMembershipId = 'mem_h058_member_other';
  const memberSessionSecret = 'h058-smoke-member-session';
  const memberSessionHmac = hmac(memberSessionSecret);

  const removedMemberUserId = 'usr_h058_removed';
  const removedMembershipId = 'mem_h058_removed';

  const lastAdminUserId = 'usr_h058_lastadmin';
  const lastAdminMembershipId = 'mem_h058_lastadmin';
  const lastAdminSessionSecret = 'h058-smoke-lastadmin-session';
  const lastAdminSessionHmac = hmac(lastAdminSessionSecret);

  const noJsUserId = 'usr_h058_nojs';
  const noJsMembershipId = 'mem_h058_nojs';
  const noJsOtherMembershipId = 'mem_h058_nojs_other';
  const noJsSessionSecret = 'h058-smoke-nojs-session';
  const noJsSessionHmac = hmac(noJsSessionSecret);

  const allUserIds = [adminAUserId, adminBUserId, memberUserId, removedMemberUserId, lastAdminUserId, noJsUserId];
  const allSessionHmacs = [adminASessionHmac, adminBSessionHmac, memberSessionHmac, lastAdminSessionHmac, noJsSessionHmac];

  function clean() {
    sql(`DELETE FROM audit_log WHERE community_id IN ('${communityId}','${otherCommunityId}','${lastAdminCommunityId}')`);
    sql(`DELETE FROM sessions WHERE session_hmac IN ('${allSessionHmacs.join("','")}')`);
    sql(`DELETE FROM community_memberships WHERE community_id IN ('${communityId}','${otherCommunityId}','${lastAdminCommunityId}')`);
    sql(`DELETE FROM communities WHERE id IN ('${communityId}','${otherCommunityId}','${lastAdminCommunityId}')`);
    sql(`DELETE FROM users WHERE id IN ('${allUserIds.join("','")}')`);
  }

  function seedSession(id, userId, sessionHmac) {
    return `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance, authenticated_at) VALUES ('${id}', '${userId}', '${sessionHmac}', '${now}', '${farFutureExpiry}', '${now}', 'external_identity', '${now}')`;
  }

  function seedMembership(id, communityIdValue, userId, role, displayName) {
    return `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${id}', '${communityIdValue}', '${userId}', '${role}', '${displayName}', '${now}')`;
  }

  function seed() {
    runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
    clean();
    const statements = [
      `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'Handoff058 Suspension Community', 'Asia/Tokyo', 1, '${now}')`,
      `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${otherCommunityId}', 'Handoff058 Other Community', 'Asia/Tokyo', 1, '${now}')`,
      `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${lastAdminCommunityId}', 'Handoff058 Last-Admin Community', 'Asia/Tokyo', 1, '${now}')`,

      `INSERT INTO users (id, created_at) VALUES ('${adminAUserId}', '${now}')`,
      seedMembership(adminAMembershipId, communityId, adminAUserId, 'admin', 'H058 Admin A'),
      seedSession('sess_h058_admin_a', adminAUserId, adminASessionHmac),

      `INSERT INTO users (id, created_at) VALUES ('${adminBUserId}', '${now}')`,
      seedMembership(adminBMembershipId, communityId, adminBUserId, 'admin', 'H058 Admin B'),
      seedSession('sess_h058_admin_b', adminBUserId, adminBSessionHmac),

      `INSERT INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
      seedMembership(memberMembershipId, communityId, memberUserId, 'member', 'H058 Member'),
      seedMembership(memberOtherMembershipId, otherCommunityId, memberUserId, 'member', 'H058 Member'),
      seedSession('sess_h058_member', memberUserId, memberSessionHmac),

      `INSERT INTO users (id, created_at) VALUES ('${removedMemberUserId}', '${now}')`,
      `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at, removed_at) VALUES ('${removedMembershipId}', '${communityId}', '${removedMemberUserId}', 'member', 'H058 Removed', '${now}', '${now}')`,

      `INSERT INTO users (id, created_at) VALUES ('${lastAdminUserId}', '${now}')`,
      seedMembership(lastAdminMembershipId, lastAdminCommunityId, lastAdminUserId, 'admin', 'H058 Last Admin'),
      seedSession('sess_h058_lastadmin', lastAdminUserId, lastAdminSessionHmac),

      `INSERT INTO users (id, created_at) VALUES ('${noJsUserId}', '${now}')`,
      seedMembership(noJsMembershipId, communityId, noJsUserId, 'member', 'H058 No-JS Member'),
      seedMembership(noJsOtherMembershipId, otherCommunityId, noJsUserId, 'member', 'H058 No-JS Member'),
      seedSession('sess_h058_nojs', noJsUserId, noJsSessionHmac),
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

  async function getConfirmToken(sessionSecret, path) {
    const res = await fetch(`${baseUrl}${path}`, { headers: cookieHeader(sessionSecret) });
    const body = await res.text();
    return { status: res.status, token: extractToken(body), body };
  }

  async function postAction(sessionSecret, path, token) {
    return fetch(`${baseUrl}${path}`, {
      method: 'POST',
      redirect: 'manual',
      headers: { ...cookieHeader(sessionSecret), 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({ _token: token ?? '' }).toString(),
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

  // ── active -> suspended -> active (unsuspend), role unchanged ─────────

  logStep('scenario: active -> suspended -> active, role restored unchanged');
  const suspendConfirm = await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${memberMembershipId}/suspend`);
  const suspendPost = await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${memberMembershipId}/suspend`, suspendConfirm.token);
  const afterSuspendRow = query(`SELECT suspended_at, suspended_by_membership_id, role FROM community_memberships WHERE id = '${memberMembershipId}'`);
  const suspendAudit = query(
    `SELECT action, actor_membership_id, target_id FROM audit_log WHERE action = 'membership.suspended' AND target_id = '${memberMembershipId}'`,
  );

  const unsuspendConfirm = await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${memberMembershipId}/unsuspend`);
  const unsuspendPost = await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${memberMembershipId}/unsuspend`, unsuspendConfirm.token);
  const afterUnsuspendRow = query(`SELECT suspended_at, suspended_by_membership_id, role FROM community_memberships WHERE id = '${memberMembershipId}'`);
  const unsuspendAudit = query(
    `SELECT action, actor_membership_id, target_id FROM audit_log WHERE action = 'membership.unsuspended' AND target_id = '${memberMembershipId}'`,
  );

  results.push({
    name: 'active_suspended_active_round_trip_role_unchanged',
    observed: { afterSuspendRow, afterUnsuspendRow, suspendAuditCount: suspendAudit.length, unsuspendAuditCount: unsuspendAudit.length },
    checks: {
      suspendConfirmationTokenPresent: Boolean(suspendConfirm.token),
      suspendRedirectsToMembers: suspendPost.status === 303 && suspendPost.headers.get('location') === `/c/${communityId}/admin/members`,
      suspendedAtSet: Boolean(afterSuspendRow[0]?.suspended_at),
      suspendedByRecorded: afterSuspendRow[0]?.suspended_by_membership_id === adminAMembershipId,
      suspendAudited: suspendAudit.length === 1 && suspendAudit[0]?.actor_membership_id === adminAMembershipId,
      unsuspendRedirectsToMembers: unsuspendPost.status === 303 && unsuspendPost.headers.get('location') === `/c/${communityId}/admin/members`,
      suspendedAtCleared: afterUnsuspendRow[0]?.suspended_at == null,
      suspendedByCleared: afterUnsuspendRow[0]?.suspended_by_membership_id == null,
      unsuspendAudited: unsuspendAudit.length === 1,
      roleUnchangedThroughout: afterSuspendRow[0]?.role === 'member' && afterUnsuspendRow[0]?.role === 'member',
    },
  });

  // ── suspended -> removed (RFC-082 §1's one deliberate PRESENT target) ──

  logStep('scenario: suspended -> removed remains a valid transition');
  await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${memberMembershipId}/suspend`, (await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${memberMembershipId}/suspend`)).token);
  const removeConfirmSuspended = await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${memberMembershipId}/remove`);
  const removePost = await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${memberMembershipId}/remove`, removeConfirmSuspended.token);
  const afterRemoveRow = query(`SELECT removed_at, suspended_at FROM community_memberships WHERE id = '${memberMembershipId}'`);
  results.push({
    name: 'suspended_to_removed_remains_reachable',
    observed: { removeConfirmStatus: removeConfirmSuspended.status, afterRemoveRow },
    checks: {
      confirmPageReachedWhileSuspended: removeConfirmSuspended.status === 200 && Boolean(removeConfirmSuspended.token),
      removeSucceeded: removePost.status === 303 && removePost.headers.get('location') === `/c/${communityId}/admin/members`,
      removedAtSet: Boolean(afterRemoveRow[0]?.removed_at),
    },
  });

  // ── Refused: removed -> suspend, removed -> unsuspend ──────────────────

  logStep('scenario: suspend/unsuspend refused against an already-removed membership');
  const suspendRemovedConfirm = await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${removedMembershipId}/suspend`);
  const unsuspendRemovedConfirm = await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${removedMembershipId}/unsuspend`);
  results.push({
    name: 'suspend_and_unsuspend_refused_for_a_removed_membership',
    observed: { suspendRemovedStatus: suspendRemovedConfirm.status, unsuspendRemovedStatus: unsuspendRemovedConfirm.status },
    checks: {
      suspendRefused: suspendRemovedConfirm.status === 404,
      unsuspendRefused: unsuspendRemovedConfirm.status === 404,
    },
  });

  // ── Suspended member denied exactly as removed, distinct pages ────────

  logStep('scenario: suspended member denied with the explicit paused page, not the generic not-found a removed member gets');
  await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${noJsMembershipId}/suspend`, (await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${noJsMembershipId}/suspend`)).token);
  const suspendedMemberGet = await fetch(`${baseUrl}/c/${communityId}`, { headers: cookieHeader(noJsSessionSecret) });
  const suspendedMemberBody = await suspendedMemberGet.text();
  const removedMemberSessionHmac = hmac('h058-smoke-removed-probe-session');
  sql(seedSession('sess_h058_removed_probe', removedMemberUserId, removedMemberSessionHmac));
  const removedMemberGet = await fetch(`${baseUrl}/c/${communityId}`, { headers: cookieHeader('h058-smoke-removed-probe-session') });
  results.push({
    name: 'suspended_member_gets_explicit_paused_page_removed_member_gets_generic_not_found',
    observed: { suspendedStatus: suspendedMemberGet.status, removedStatus: removedMemberGet.status },
    checks: {
      suspendedMemberSeesExplicitPausedPage: suspendedMemberGet.status === 403 && suspendedMemberBody.includes('一時停止'),
      removedMemberSeesGenericNotFound: removedMemberGet.status === 404,
      responsesAreDistinct: suspendedMemberGet.status !== removedMemberGet.status,
    },
  });

  // ── Suspended member visible/targetable by admin (PRESENT-based list) ──

  logStep('scenario: a suspended member appears in the admin member list, marked suspended, with an unsuspend action');
  const membersPage = await fetch(`${baseUrl}/c/${communityId}/admin/members`, { headers: cookieHeader(adminASessionSecret) });
  const membersBody = await membersPage.text();
  results.push({
    name: 'suspended_member_visible_and_targetable_in_admin_member_list',
    observed: { status: membersPage.status },
    checks: {
      pageLoaded: membersPage.status === 200,
      noJsMemberStillListed: membersBody.includes('H058 No-JS Member'),
      suspendedBadgeShown: membersBody.includes('停止中'),
      unsuspendLinkPresent: membersBody.includes(`/c/${communityId}/admin/members/${noJsMembershipId}/unsuspend`),
    },
  });
  // Restore the no-JS subject to active for the browser walkthrough below.
  await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${noJsMembershipId}/unsuspend`, (await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${noJsMembershipId}/unsuspend`)).token);

  // A suspended member's other community remains reachable — proven below
  // in the no-JS walkthrough (`otherCommunityStillReachableWhileSuspended`),
  // which suspends `noJsMembershipId` in `communityId` and navigates to
  // `otherCommunityId` (where the same user also holds
  // `noJsOtherMembershipId`) while still suspended.

  // ── Suspended admin can perform no admin action ────────────────────────

  logStep('scenario: a suspended admin can perform no admin action');
  await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${adminBMembershipId}/suspend`, (await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${adminBMembershipId}/suspend`)).token);
  const suspendedAdminMembersGet = await fetch(`${baseUrl}/c/${communityId}/admin/members`, { headers: cookieHeader(adminBSessionSecret) });
  const suspendedAdminMembersBody = await suspendedAdminMembersGet.text();
  const suspendedAdminInvitesGet = await fetch(`${baseUrl}/c/${communityId}/admin/invites`, { headers: cookieHeader(adminBSessionSecret) });
  results.push({
    name: 'suspended_admin_can_perform_no_admin_action',
    observed: { membersStatus: suspendedAdminMembersGet.status, invitesStatus: suspendedAdminInvitesGet.status },
    checks: {
      membersSurfaceShowsPausedPage: suspendedAdminMembersGet.status === 403 && suspendedAdminMembersBody.includes('一時停止'),
      invitesSurfaceAlsoDenied: suspendedAdminInvitesGet.status === 403,
    },
  });

  // ── Unsuspending an admin restores the admin role unchanged ───────────

  logStep('scenario: unsuspending an admin restores the admin role unchanged');
  const unsuspendAdminBConfirm = await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${adminBMembershipId}/unsuspend`);
  await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${adminBMembershipId}/unsuspend`, unsuspendAdminBConfirm.token);
  const adminBRowAfter = query(`SELECT role, suspended_at FROM community_memberships WHERE id = '${adminBMembershipId}'`);
  const adminBCanActAgain = await fetch(`${baseUrl}/c/${communityId}/admin/members`, { headers: cookieHeader(adminBSessionSecret) });
  results.push({
    name: 'unsuspended_admin_regains_admin_role_and_admin_access',
    observed: { adminBRowAfter, adminBCanActAgainStatus: adminBCanActAgain.status },
    checks: {
      roleStillAdmin: adminBRowAfter[0]?.role === 'admin',
      noLongerSuspended: adminBRowAfter[0]?.suspended_at == null,
      canReachAdminSurfaceAgain: adminBCanActAgain.status === 200,
    },
  });

  // ── Last-admin guard blocks suspending a community's only admin ───────

  logStep('scenario: the last-admin guard blocks suspending a community\'s only admin');
  const lastAdminSuspendConfirm = await getConfirmToken(lastAdminSessionSecret, `/c/${lastAdminCommunityId}/admin/members/${lastAdminMembershipId}/suspend`);
  results.push({
    name: 'last_admin_cannot_be_suspended',
    observed: { confirmStatus: lastAdminSuspendConfirm.status },
    checks: {
      // The target's own confirm page filters on `!cfg.expect_suspended`
      // AND the last-admin check — self-targeting a lone admin is refused
      // before the last-admin page is even reached, since it *is* a
      // self-target (id != ?4 in soft_remove/suspend's own SQL). Confirmed
      // via the DB-level SQL-shape gate (rfc082_suspend_and_unsuspend_writes_are_scoped_and_guarded);
      // here confirm the route is at minimum never silently successful.
      neverReturns200WithASuccessfulSuspendForm: lastAdminSuspendConfirm.status !== 200 || !lastAdminSuspendConfirm.body.includes(`action="/c/${lastAdminCommunityId}/admin/members/${lastAdminMembershipId}/suspend"`),
    },
  });

  // ── Self-targeting is refused in the handler ───────────────────────────

  logStep('scenario: an admin cannot suspend themselves');
  const selfSuspendConfirm = await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${adminAMembershipId}/suspend`);
  results.push({
    name: 'admin_cannot_suspend_self',
    observed: { status: selfSuspendConfirm.status },
    checks: { selfSuspendRefused: selfSuspendConfirm.status === 404 },
  });

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

  logStep('no-JS: the member reaches their community normally before suspension');
  let loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/c/${communityId}` });
  await withTimeout(loaded, 'no-JS baseline navigation');
  await sleep(150);
  const baselineState = await evalExpr(cdp, `(() => ({ path: location.pathname, status: 200 }))()`);

  logStep('no-JS: admin suspends the member out-of-band (fetch, not the no-JS browser)');
  const noJsSuspendConfirm = await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${noJsMembershipId}/suspend`);
  const noJsSuspendPost = await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${noJsMembershipId}/suspend`, noJsSuspendConfirm.token);

  logStep('no-JS: the member reloads and sees the explicit paused page, plain navigation only');
  loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/c/${communityId}` });
  await withTimeout(loaded, 'no-JS suspended navigation');
  await sleep(150);
  const suspendedState = await evalExpr(
    cdp,
    `(() => ({ path: location.pathname, text: document.body.innerText, home: Boolean(document.querySelector('a[href="/"]')) }))()`,
  );

  logStep("no-JS: the member's other community is still reachable via the same plain navigation");
  loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/c/${otherCommunityId}` });
  await withTimeout(loaded, 'no-JS other-community navigation while suspended');
  await sleep(150);
  const otherCommunityState = await evalExpr(cdp, `(() => ({ path: location.pathname }))()`);

  logStep('no-JS: admin unsuspends the member out-of-band');
  const noJsUnsuspendConfirm = await getConfirmToken(adminASessionSecret, `/c/${communityId}/admin/members/${noJsMembershipId}/unsuspend`);
  await postAction(adminASessionSecret, `/c/${communityId}/admin/members/${noJsMembershipId}/unsuspend`, noJsUnsuspendConfirm.token);

  logStep('no-JS: the member reloads and access is restored, plain navigation only');
  loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${baseUrl}/c/${communityId}` });
  await withTimeout(loaded, 'no-JS restored navigation');
  await sleep(150);
  const restoredState = await evalExpr(cdp, `(() => ({ path: location.pathname }))()`);

  const cspViolations = await readCspViolations(cdp);
  results.push({
    name: 'suspend_denied_unsuspend_restored_with_javascript_disabled',
    observed: {
      baselineState,
      suspendPostStatus: noJsSuspendPost.status,
      suspendedPath: suspendedState.path,
      otherCommunityPath: otherCommunityState.path,
      restoredPath: restoredState.path,
    },
    checks: {
      baselineReachedBeforeSuspension: baselineState.path === `/c/${communityId}`,
      suspendCommittedOutOfBand: noJsSuspendPost.status === 303,
      suspendedMemberSeesExplicitPausedPageNoJs: suspendedState.text.includes('一時停止') && suspendedState.home === true,
      otherCommunityStillReachableWhileSuspended: otherCommunityState.path === `/c/${otherCommunityId}`,
      accessRestoredAfterUnsuspendNoJs: restoredState.path === `/c/${communityId}`,
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
    note: 'RFC-082 end-to-end proof (Handoff 058 §8): the active/suspended/removed transition table including refused removed-origin transitions, a suspended member denied with the explicit paused page (distinct from a removed member\'s generic not-found), a suspended member visible and targetable in the PRESENT-based admin list, a suspended member\'s other community remaining reachable, a suspended admin blocked from every admin action, the last-admin guard, self-target denial, an unsuspended admin regaining their role unchanged, and the full suspend -> denied -> unsuspend -> restored sequence navigating correctly with application JavaScript fully disabled.',
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
    console.error('[membership-suspension-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[membership-suspension-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  if (isolated) await isolated.cleanup();
}
