#!/usr/bin/env node
// Scenario smoke for self display-name editing. Local wrangler dev only.

import { prepareIsolatedWorkerTest } from "../lib/isolated-worker-test.mjs";
import { attachCspViolationCapture, readCspViolations } from "../lib/csp-violation-capture.mjs";

import { createHmac } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8799);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9251);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc070';
const reportName = process.env.REPORT_NAME ?? 'rfc070-self-display-name-editing-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-self-display-name-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const isolated = await prepareIsolatedWorkerTest("self-display-name-editing");
const pepper = isolated.pepper;
const now = '2026-07-11T00:00:00.000Z';

const communityId = 'com_rfc070_primary';
const otherCommunityId = 'com_rfc070_other';
const memberUserId = 'usr_rfc070_member';
const otherUserId = 'usr_rfc070_other';
const memberMembershipId = 'mem_rfc070_member';
const otherMembershipId = 'mem_rfc070_other';
const eventId = 'evt_rfc070_visible';
const eventDayId = 'day_rfc070_visible';
const attendanceId = 'att_rfc070_member';
const memberSessionSecret = 'rfc070-smoke-member-session';
const memberSessionHmac = hmac(memberSessionSecret);
const originalDisplayName = 'RFC070 Original Member';
const updatedDisplayName = 'RFC070 Updated Member';
const afterInvalidDisplayName = 'RFC070 After Invalid';

assertLocalOnly();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function hmac(secret) {
  return createHmac('sha256', pepper).update(secret).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`self-display-name smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`self-display-name smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) {
    throw new Error('self-display-name smoke refuses remote D1 operations');
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

function clean() {
  sql(`DELETE FROM attendances WHERE event_day_id = '${eventDayId}' OR membership_id IN ('${memberMembershipId}','${otherMembershipId}')`);
  sql(`DELETE FROM event_notes WHERE event_id = '${eventId}'`);
  sql(`DELETE FROM event_days WHERE event_id = '${eventId}' OR community_id IN ('${communityId}','${otherCommunityId}')`);
  sql(`DELETE FROM events WHERE id = '${eventId}' OR community_id IN ('${communityId}','${otherCommunityId}')`);
  sql(`DELETE FROM audit_log WHERE community_id IN ('${communityId}','${otherCommunityId}')`);
  sql(`DELETE FROM form_tokens WHERE user_id IN ('${memberUserId}','${otherUserId}')`);
  sql(`DELETE FROM sessions WHERE session_hmac IN ('${memberSessionHmac}') OR user_id IN ('${memberUserId}','${otherUserId}')`);
  sql(`DELETE FROM community_memberships WHERE id IN ('${memberMembershipId}','${otherMembershipId}') OR community_id IN ('${communityId}','${otherCommunityId}')`);
  sql(`DELETE FROM users WHERE id IN ('${memberUserId}','${otherUserId}')`);
  sql(`DELETE FROM communities WHERE id IN ('${communityId}','${otherCommunityId}')`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'RFC070 Primary', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${otherCommunityId}', 'RFC070 Other', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${otherUserId}', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${communityId}', '${memberUserId}', 'member', '${esc(originalDisplayName)}', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${otherMembershipId}', '${otherCommunityId}', '${otherUserId}', 'admin', 'RFC070 Other Admin', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc070_member', '${memberUserId}', '${memberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${eventId}', '${communityId}', '${memberMembershipId}', 'RFC070 Visible Event', 'RFC070 Room', '', 'scheduled', 'none', NULL, '${now}', '${now}')`,
    `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, occurrence_status) VALUES ('${eventDayId}', '${eventId}', '${communityId}', 1, '2026-07-20', '2026-07-20T01:00:00.000Z', '2026-07-20T02:00:00.000Z', '${now}', 'scheduled')`,
    `INSERT INTO attendances (id, event_day_id, membership_id, status, updated_at) VALUES ('${attendanceId}', '${eventDayId}', '${memberMembershipId}', 'going', '${now}')`,
  ];
  for (const statement of statements) sql(statement);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[self-display-name-smoke] ${message}`);
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

async function newPage(sessionSecret) {
  const target = await json(`http://127.0.0.1:${remotePort}/json/new`, {
    method: 'PUT',
  });
  const cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  await attachCspViolationCapture(cdp);
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
    mobile: options.mobile ?? true,
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

async function setInputValue(cdp, name, value) {
  const changed = await evalExpr(
    cdp,
    `(() => {
      const input = document.querySelector(${JSON.stringify(`input[name="${name}"]`)});
      if (!input) return false;
      input.value = ${JSON.stringify(value)};
      input.dispatchEvent(new Event('input', { bubbles: true }));
      return true;
    })()`,
  );
  if (!changed) throw new Error(`Input not found: ${name}`);
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
}

async function openDetailsContaining(cdp, text) {
  const opened = await evalExpr(
    cdp,
    `(() => {
      const details = [...document.querySelectorAll('details')].find((item) => item.innerText.includes(${JSON.stringify(text)}));
      if (!details) return false;
      details.open = true;
      return true;
    })()`,
  );
  if (!opened) throw new Error(`Details section not found: ${text}`);
  await sleep(150);
}

async function postDisplayName(token, displayName, cid = communityId) {
  const body = new URLSearchParams();
  body.set('_token', token);
  body.set('display_name', displayName);
  return await fetch(`${baseUrl}/c/${cid}/me/display-name`, {
    method: 'POST',
    headers: {
      Cookie: `ciao_sid=${memberSessionSecret}`,
      'Content-Type': 'application/x-www-form-urlencoded',
    },
    body,
    redirect: 'follow',
  });
}

function membershipName(id = memberMembershipId) {
  return query(`SELECT display_name FROM community_memberships WHERE id='${esc(id)}'`)[0]?.display_name;
}

function auditCount() {
  return Number(
    query(
      `SELECT COUNT(*) AS n FROM audit_log WHERE community_id='${communityId}' AND action='membership.display_name_updated'`,
    )[0]?.n ?? 0,
  );
}

function tokenRow(rawToken) {
  const tokenHmac = hmac(rawToken);
  return query(
    `SELECT consumed_at, result_ref FROM form_tokens WHERE token_hmac='${tokenHmac}' LIMIT 1`,
  )[0] ?? {};
}

function auditMetadataRows() {
  return query(
    `SELECT metadata_json FROM audit_log WHERE community_id='${communityId}' AND action='membership.display_name_updated' ORDER BY created_at ASC`,
  ).map((row) => row.metadata_json);
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
  chrome = spawn(chromium, flags, {
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  chrome.stderr.on('data', (chunk) => {
    chromeStderr += chunk.toString();
  });
  await waitForDebugger(() => chromeStderr);
  logStep('sandboxed incognito Chromium is ready');

  const page = await newPage(memberSessionSecret);

  logStep('checking Me entry point at mobile 200% text');
  await navigate(page, `/c/${communityId}/me`, { textScale: 2 });
  const meBefore = await collect(page);
  results.push({
    name: 'me-page-shows-current-name-and-edit-link',
    screenshotPath: await screenshot(page, 'me-page-shows-current-name-and-edit-link'),
    observed: meBefore,
    checks: {
      noHorizontalScroll: meBefore.noHorizontalScroll,
      showsCurrentName: meBefore.text.includes(originalDisplayName),
      showsEditLink: meBefore.hrefs.includes(`/c/${communityId}/me/display-name`),
      noSuccessBeforeMutation: !meBefore.text.includes('表示名を変更しました'),
    },
  });

  logStep('checking edit form and browser submit');
  await navigate(page, `/c/${communityId}/me/display-name`, { textScale: 2 });
  const editForm = await collect(page);
  const firstToken = editForm.values._token;
  await setInputValue(page, 'display_name', updatedDisplayName);
  await submitFormByAction(page, `/c/${communityId}/me/display-name`);
  const meAfterEdit = await collect(page);
  const eventPagePath = `/c/${communityId}/events/${eventId}`;
  await navigate(page, eventPagePath, { textScale: 2 });
  await openDetailsContaining(page, '参加予定者');
  const eventAfterEdit = await collect(page);
  await postDisplayName(firstToken, 'RFC070 Altered Replay');
  const firstReplayRow = tokenRow(firstToken);
  results.push({
    name: 'valid-edit-updates-member-visible-surfaces-and-replay-is-benign',
    screenshotPath: await screenshot(page, 'event-detail-shows-updated-display-name'),
    observed: {
      meAfterEdit,
      eventAfterEdit,
      membershipName: membershipName(),
      auditCount: auditCount(),
      firstReplayRow,
      auditMetadataRows: auditMetadataRows(),
    },
    checks: {
      editRedirectedToMe: meAfterEdit.path === `/c/${communityId}/me?flash=display_name_updated`,
      successFeedbackVisible: meAfterEdit.text.includes('表示名を変更しました'),
      meShowsUpdatedName: meAfterEdit.text.includes(updatedDisplayName),
      eventShowsUpdatedName: eventAfterEdit.text.includes(updatedDisplayName),
      replayDidNotAlterName: membershipName() === updatedDisplayName,
      replayDidNotAddAudit: auditCount() === 1,
      tokenStoresUpdatedResult: firstReplayRow.result_ref === 'display_name_updated',
      metadataMinimal: auditMetadataRows().every((metadata) => (
        metadata.includes('"changed_fields":["display_name"]') &&
        !metadata.includes('membership_id') &&
        !metadata.includes('community_id')
      )),
      noHorizontalScrollOnMe: meAfterEdit.noHorizontalScroll,
    },
  });

  logStep('checking same-value no-op and altered replay');
  await navigate(page, `/c/${communityId}/me/display-name`, { textScale: 2 });
  const sameValueForm = await collect(page);
  const sameValueToken = sameValueForm.values._token;
  await submitFormByAction(page, `/c/${communityId}/me/display-name`);
  const afterSameValue = await collect(page);
  await postDisplayName(sameValueToken, 'RFC070 Same Token Altered Replay');
  const sameValueRow = tokenRow(sameValueToken);
  results.push({
    name: 'same-value-noop-stores-result-and-replay-does-not-mutate',
    observed: {
      afterSameValue,
      membershipName: membershipName(),
      auditCount: auditCount(),
      sameValueRow,
    },
    checks: {
      sameValueRedirectedQuietly: afterSameValue.path === `/c/${communityId}/me`,
      sameValueStoredResult: sameValueRow.result_ref === 'display_name_unchanged',
      sameValueReplayDidNotAlterName: membershipName() === updatedDisplayName,
      sameValueDidNotAddAudit: auditCount() === 1,
    },
  });

  logStep('checking invalid input does not consume original token');
  await navigate(page, `/c/${communityId}/me/display-name`, { textScale: 2 });
  const invalidForm = await collect(page);
  const invalidToken = invalidForm.values._token;
  const invalidResponse = await postDisplayName(invalidToken, 'bad\u0001name');
  const invalidRow = tokenRow(invalidToken);
  await postDisplayName(invalidToken, afterInvalidDisplayName);
  results.push({
    name: 'invalid-input-does-not-consume-token',
    observed: {
      invalidStatus: invalidResponse.status,
      invalidRow,
      membershipName: membershipName(),
      auditCount: auditCount(),
      finalTokenRow: tokenRow(invalidToken),
    },
    checks: {
      invalidRequestReturnedForm: invalidResponse.status === 200,
      invalidTokenNotConsumed: !invalidRow.consumed_at,
      originalTokenStillUsable: membershipName() === afterInvalidDisplayName,
      validAfterInvalidAddedOneAudit: auditCount() === 2,
      finalTokenStoresUpdatedResult: tokenRow(invalidToken).result_ref === 'display_name_updated',
    },
  });

  logStep('checking cross-community direct URL cannot edit another community');
  await navigate(page, `/c/${communityId}/me/display-name`, { textScale: 2 });
  const crossForm = await collect(page);
  const crossToken = crossForm.values._token;
  const otherBefore = membershipName(otherMembershipId);
  const crossResponse = await postDisplayName(crossToken, 'RFC070 Cross Changed', otherCommunityId);
  const otherAfter = membershipName(otherMembershipId);
  results.push({
    name: 'cross-community-url-does-not-edit-other-membership',
    screenshotPath: await screenshot(page, 'cross-community-source-form-mobile-200-percent'),
    observed: {
      crossStatus: crossResponse.status,
      otherBefore,
      otherAfter,
      sourceMembershipName: membershipName(),
    },
    checks: {
      crossRequestNotSuccessfulRedirect: crossResponse.status !== 200 || otherAfter === otherBefore,
      otherMembershipUnchanged: otherAfter === otherBefore,
      sourceMembershipUnchanged: membershipName() === afterInvalidDisplayName,
    },
  });

  const cspViolations = await readCspViolations(page);
  results.push({
    name: 'no-csp-violations',
    observed: { cspViolations },
    checks: { zeroCspViolations: cspViolations.length === 0 },
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
    console.error('[self-display-name-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[self-display-name-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  await isolated.cleanup();
}
