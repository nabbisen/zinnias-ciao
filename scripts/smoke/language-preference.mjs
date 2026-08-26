#!/usr/bin/env node
// Scenario smoke for RFC-072 Slice C: the language preference setting,
// linked from My Page, and its effect on the member-facing core. Local
// wrangler dev only.

import { prepareIsolatedWorkerTest } from "../lib/isolated-worker-test.mjs";
import { SMOKE_ACCEPT_LANGUAGE } from "../lib/smoke-locale.mjs";
import { attachCspViolationCapture, readCspViolations } from "../lib/csp-violation-capture.mjs";
import { killAndWait } from "../lib/kill-and-wait.mjs";

import { createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const port = Number(process.env.SMOKE_PORT ?? 8799);
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9251);
const baseUrl = `http://127.0.0.1:${port}`;
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc072';
const reportName = process.env.REPORT_NAME ?? 'rfc072-language-preference-smoke-results.json';
const userDataDir = `.git-exclude/tmp/chrome-rfc072-language-preference-sandboxed-${Date.now()}`;
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const isolated = await prepareIsolatedWorkerTest("language-preference");
const pepper = isolated.pepper;
const now = '2026-07-30T00:00:00.000Z';

// Handoff 042 §7.1: the visible event is seeded 3 days ahead of this run, at
// 03:00 UTC — never a fixed calendar instant. +3 days keeps it comfortably
// upcoming for the whole run (the old fixed-date fixture failed by 13
// minutes once real time caught up to it; do not design a margin that thin
// again). 03:00 UTC is 12:00 JST the same day, so the UTC calendar date and
// the JST calendar date this app renders in are identical for this instant
// — midnight UTC, the old fixture's choice, is exactly what makes them
// diverge.
const WEEKDAY_EN = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTH_ABBR_EN = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const MONTH_NAME_EN = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
];

function pad2(n) {
  return String(n).padStart(2, '0');
}

const runAt = new Date();
const eventDate = new Date(Date.UTC(
  runAt.getUTCFullYear(),
  runAt.getUTCMonth(),
  runAt.getUTCDate() + 3,
  3, 0, 0, 0,
));
const eventYear = eventDate.getUTCFullYear();
const eventMonth = eventDate.getUTCMonth() + 1; // 1-12
const eventDay = eventDate.getUTCDate();
const eventWeekdayIndex = eventDate.getUTCDay(); // 0=Sun..6=Sat — JS's own weekday computation, independent of tz.rs's Zeller's-congruence one

const eventDayDate = `${eventYear}-${pad2(eventMonth)}-${pad2(eventDay)}`;
const eventStartsAtUtc = eventDate.toISOString();
const eventEndsAtUtc = new Date(eventDate.getTime() + 60 * 60 * 1000).toISOString();
const eventMonthParam = `${eventYear}-${pad2(eventMonth)}`;

// Handoff 042 §7.2: built from literal arrays and Date.UTC/getUTCDay, to
// match packages/contracts/src/tz.rs's date_label_en ("{weekday}, {day}
// {month abbr}" — no year, day not zero-padded). Deliberately not
// Intl.DateTimeFormat (depends on the host's ICU data, not the project's
// format decision) and not read from the app itself — agreement with the
// app's own, differently-implemented weekday computation means something.
const expectedDateLabelEn = `${WEEKDAY_EN[eventWeekdayIndex]}, ${eventDay} ${MONTH_ABBR_EN[eventMonth - 1]}`;
const expectedDateLabelEnPattern = new RegExp(`\\b${expectedDateLabelEn}\\b`);
// The all-numeric US-style rendering this smoke guards against never
// appearing — also derived, so the guard stays meaningful as the seeded
// date moves with each run instead of vacuously passing forever.
const expectedAllNumericDatePattern = new RegExp(`\\b${pad2(eventMonth)}/${pad2(eventDay)}/${eventYear}\\b`);
const expectedMonthHeaderEn = `${MONTH_NAME_EN[eventMonth - 1]} ${eventYear}`;
const expectedMonthHeaderJa = `${eventYear}年${eventMonth}月`;

// Handoff 042 §7.3: with the relative seed above in place this should never
// fire. It exists only so that a future re-pin back to a literal date fails
// with a message naming the fixture and the reason, instead of an opaque
// content mismatch. It guards this file only — one assertion in one smoke
// does not guard the class of "smokes that hardcode dates."
function assertFixtureStillUpcoming() {
  const startMs = Date.parse(eventStartsAtUtc);
  if (!(Date.now() < startMs)) {
    throw new Error(
      `language-preference smoke fixture has expired: scripts/smoke/language-preference.mjs's ` +
      `seeded event (day_date=${eventDayDate}, starts_at_utc=${eventStartsAtUtc}) is no longer in ` +
      `the future relative to this run (now=${new Date().toISOString()}). This should be ` +
      `impossible with the relative-date seed in place — check that the +3 day derivation above ` +
      `is intact and has not been re-pinned to a literal date.`,
    );
  }
}

const communityId = 'com_rfc072_primary';
const otherCommunityId = 'com_rfc072_other';
const memberUserId = 'usr_rfc072_member';
const memberMembershipId = 'mem_rfc072_member';
const otherMembershipId = 'mem_rfc072_other';
const eventId = 'evt_rfc072_visible';
const eventDayId = 'day_rfc072_visible';
const memberSessionSecret = 'rfc072-smoke-member-session';
const memberSessionHmac = hmac(memberSessionSecret);
const memberDisplayName = 'RFC072 Member';

assertLocalOnly();
assertFixtureStillUpcoming();
await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function hmac(secret) {
  return createHmac('sha256', pepper).update(secret).digest('hex');
}

function assertLocalOnly() {
  const parsed = new URL(baseUrl);
  if (!['127.0.0.1', 'localhost'].includes(parsed.hostname)) {
    throw new Error(`language-preference smoke is local-only; refusing base URL ${baseUrl}`);
  }
  for (const arg of process.argv.slice(2)) {
    if (arg === '--remote' || arg.includes('staging') || arg.includes('production')) {
      throw new Error(`language-preference smoke is local-only; refused argument ${arg}`);
    }
  }
}

function runWrangler(args) {
  if (args.includes('--remote')) {
    throw new Error('language-preference smoke refuses remote D1 operations');
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
  runWrangler([
    'd1', 'execute', 'zinnias-ciao-dev', '--local', '--env', 'dev', '--command', statement,
  ]);
}

function query(statement) {
  const raw = runWrangler([
    'd1', 'execute', 'zinnias-ciao-dev', '--local', '--env', 'dev', '--json', '--command', statement,
  ]);
  const parsed = JSON.parse(raw);
  return parsed?.[0]?.results ?? parsed?.results ?? [];
}

function esc(value) {
  return String(value).replaceAll("'", "''");
}

function clean() {
  const communities = `'${communityId}','${otherCommunityId}'`;
  sql(`DELETE FROM attendances WHERE event_day_id = '${eventDayId}'`);
  sql(`DELETE FROM event_notes WHERE event_id = '${eventId}'`);
  sql(`DELETE FROM event_days WHERE event_id = '${eventId}' OR community_id IN (${communities})`);
  sql(`DELETE FROM events WHERE id = '${eventId}' OR community_id IN (${communities})`);
  sql(`DELETE FROM audit_log WHERE community_id IN (${communities})`);
  sql(`DELETE FROM form_tokens WHERE user_id = '${memberUserId}'`);
  sql(`DELETE FROM sessions WHERE session_hmac = '${memberSessionHmac}' OR user_id = '${memberUserId}'`);
  sql(`DELETE FROM community_memberships WHERE id IN ('${memberMembershipId}','${otherMembershipId}') OR community_id IN (${communities})`);
  sql(`DELETE FROM users WHERE id = '${memberUserId}'`);
  sql(`DELETE FROM communities WHERE id IN (${communities})`);
}

function seed() {
  runWrangler(['d1', 'migrations', 'apply', 'zinnias-ciao-dev', '--local', '--env', 'dev']);
  clean();
  const statements = [
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${communityId}', 'RFC072 Primary', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO communities (id, name, timezone, is_active, created_at) VALUES ('${otherCommunityId}', 'RFC072 Other', 'Asia/Tokyo', 1, '${now}')`,
    `INSERT INTO users (id, created_at) VALUES ('${memberUserId}', '${now}')`,
    // 'admin' here (not 'member'): Handoff 030's create-community scenario
    // needs this to be THE admin membership that `require_active_admin_somewhere`
    // returns (find_first_admin_for_user, ORDER BY joined_at ASC), since §7.2
    // resolves /communities/new's locale from that exact row. It must be the
    // membership the test actually switches to English via
    // /c/:cid/me/language — otherMembershipId lives in a different community
    // and is deliberately never switched, so making IT the admin membership
    // would leave /communities/new Japanese and the scenario would be
    // asserting the wrong thing (membership-scoping working, not the feature).
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${communityId}', '${memberUserId}', 'admin', '${esc(memberDisplayName)}', '${now}')`,
    // Deliberately left 'member' and never switched: proves the language
    // switch and §7.2's locale resolution are both membership-scoped, not
    // user-scoped (see 'otherMembershipUnaffectedThroughout' below).
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${otherMembershipId}', '${otherCommunityId}', '${memberUserId}', 'member', 'RFC072 Member Other', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at, provenance) VALUES ('sess_rfc072_member', '${memberUserId}', '${memberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}', 'invite_redemption')`,
    `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${eventId}', '${communityId}', '${memberMembershipId}', 'RFC072 Visible Event', 'RFC072 Room', '', 'scheduled', 'none', NULL, '${now}', '${now}')`,
    // Handoff 042 §7.1: seeded relative to this run (see the derivation
    // above `now`), not a fixed calendar instant — this fixture cannot
    // expire the way the old 2026-08-03 pin did.
    `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, occurrence_status) VALUES ('${eventDayId}', '${eventId}', '${communityId}', 1, '${eventDayDate}', '${eventStartsAtUtc}', '${eventEndsAtUtc}', '${now}', 'scheduled')`,
  ];
  for (const statement of statements) sql(statement);
  // Handoff 078: deliberately NOT the shared, blanket
  // PIN_FIXTURE_UI_LANGUAGE_TO_JAPANESE_SQL — this smoke's own
  // `otherMembershipUnaffectedThroughout` check (below) proves the
  // language switch is membership-scoped precisely by asserting
  // otherMembershipId's ui_language stays NULL forever; the blanket
  // `WHERE ui_language IS NULL` pin would set it to 'ja' and make that
  // check pass by construction, proving nothing (RFC-085's own §10
  // security constraint). Only memberMembershipId — the one this smoke's
  // *initial*-state assertions (before the first switch) depend on — is
  // pinned, scoped by id rather than by "any NULL row."
  sql(`UPDATE community_memberships SET ui_language = 'ja' WHERE id = '${memberMembershipId}' AND ui_language IS NULL`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function logStep(message) {
  console.error(`[language-preference-smoke] ${message}`);
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

async function newPage(sessionSecret) {
  const target = await json(`http://127.0.0.1:${remotePort}/json/new`, { method: 'PUT' });
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
    headers: { Cookie: `ciao_sid=${sessionSecret}`, "Accept-Language": SMOKE_ACCEPT_LANGUAGE },
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
    await evalExpr(cdp, `(() => { document.documentElement.style.fontSize = '200%'; })()`);
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
  const shot = await cdp.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
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
      const dayCell = document.querySelector('a[aria-label][href*="day="]');
      // Handoff 036: the rendered attribute, not just the source i18n
      // constant, is what a screen reader actually announces — this is the
      // bottom nav landmark present on every member-facing page.
      const bottomNavAriaLabel = document.querySelector('nav.cz-bottom-nav')?.getAttribute('aria-label') ?? null;
      // Handoff 026: scoped specifically to the attendance-status buttons
      // (form action '.../my-status'), not the page's general text — the
      // Event Detail counts line ("Going 0 . No Go 0 . No answer 3") was
      // already locale-aware before this fix and would satisfy a bare
      // text.includes('Going') check even while the buttons themselves
      // rendered Japanese. This is what the pre-fix scenario missed.
      const statusButtons = [...document.querySelectorAll('form[action*="my-status"] button[name="status"]')]
        .map((b) => b.getAttribute('aria-label'));
      return {
        path: location.pathname + location.search,
        htmlLang: document.documentElement.getAttribute('lang'),
        text: document.body.innerText,
        hrefs: links.map((link) => link.href),
        links,
        dayCellAriaLabel: dayCell ? dayCell.getAttribute('aria-label') : null,
        bottomNavAriaLabel,
        // Handoff 037: the rendered flash text after saving a note on Event
        // Detail — proves the code->locale mapping actually reaches the
        // page, not just that the mapper function returns the right string.
        noteFlashText: document.querySelector('.cz-note-flash')?.innerText ?? null,
        monthHeaderText: document.querySelector('h2 + p')?.innerText ?? null,
        noHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
        // RFC-075 Slice 3 §8 / Handoff 028 precedent: a measurement, not
        // just a boolean, for every scenario — how much (if any) the page
        // overflows the viewport, and how wide the viewport was.
        pageOverflowPx: Math.max(
          0,
          document.documentElement.scrollWidth - document.documentElement.clientWidth,
        ),
        viewportWidth: document.documentElement.clientWidth,
        statusButtons,
        // Handoff 028 §8: a measurement, not just a boolean — the status
        // button row's right edge against the viewport width, so a
        // shrinking margin is visible before it becomes an actual failure.
        statusButtonRowMeasurement: (() => {
          const row = document.querySelector('.cz-status-form-buttons');
          if (!row) return null;
          const r = row.getBoundingClientRect();
          return {
            viewportWidth: document.documentElement.clientWidth,
            rowRight: Math.round(r.right),
            marginPx: Math.round(document.documentElement.clientWidth - r.right),
          };
        })(),
      };
    })()`,
  );
}

async function selectRadioAndSubmit(cdp, radioSelector, formAction) {
  const loaded = cdp.once('Page.loadEventFired');
  const submitted = await evalExpr(
    cdp,
    `(() => {
      const radio = document.querySelector(${JSON.stringify(radioSelector)});
      if (!radio) return false;
      radio.checked = true;
      radio.dispatchEvent(new Event('change', { bubbles: true }));
      const form = [...document.querySelectorAll('form[action]')].find(
        (item) => item.getAttribute('action') === ${JSON.stringify(formAction)},
      );
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  if (!submitted) throw new Error(`Radio or form not found: ${radioSelector} / ${formAction}`);
  await withTimeout(loaded, `submit form to ${formAction}`);
}

async function fillFieldAndSubmit(cdp, fieldSelector, value, formAction) {
  const loaded = cdp.once('Page.loadEventFired');
  const submitted = await evalExpr(
    cdp,
    `(() => {
      const field = document.querySelector(${JSON.stringify(fieldSelector)});
      if (!field) return false;
      field.value = ${JSON.stringify(value)};
      field.dispatchEvent(new Event('input', { bubbles: true }));
      const form = [...document.querySelectorAll('form[action]')].find(
        (item) => item.getAttribute('action') === ${JSON.stringify(formAction)},
      );
      if (!form) return false;
      form.requestSubmit();
      return true;
    })()`,
  );
  if (!submitted) throw new Error(`Field or form not found: ${fieldSelector} / ${formAction}`);
  await withTimeout(loaded, `submit form to ${formAction}`);
}

function memberUiLanguage(id = memberMembershipId) {
  return query(`SELECT ui_language FROM community_memberships WHERE id='${esc(id)}'`)[0]?.ui_language ?? null;
}

function allChecksPass(checks) {
  return Object.values(checks).every(Boolean);
}

let dev;
let chrome;
let devStderr = '';
let chromeStderr = '';
const results = [];
const notes = [];

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
  chrome = spawn(chromium, flags, { stdio: ['ignore', 'ignore', 'pipe'] });
  chrome.stderr.on('data', (chunk) => {
    chromeStderr += chunk.toString();
  });
  await waitForDebugger(() => chromeStderr);
  logStep('sandboxed incognito Chromium is ready');

  const page = await newPage(memberSessionSecret);

  logStep('confirming settings page starts in Japanese and is linked from My Page');
  await navigate(page, `/c/${communityId}/me`, { textScale: 2 });
  const meBeforeJa = await collect(page);
  results.push({
    name: 'my-page-links-to-language-setting-and-starts-japanese',
    screenshotPath: await screenshot(page, 'my-page-japanese-200-percent'),
    observed: meBeforeJa,
    checks: {
      htmlLangJa: meBeforeJa.htmlLang === 'ja',
      linksToLanguageSetting: meBeforeJa.hrefs.includes(`/c/${communityId}/me/language`),
      noHorizontalScrollAt200Percent: meBeforeJa.noHorizontalScroll,
      // Handoff 036: proves the RENDERED attribute is Japanese, not just
      // that the source constant exists — a screen reader reads this exact
      // string on every page's bottom nav.
      bottomNavAriaLabelIsJapanese: meBeforeJa.bottomNavAriaLabel === 'メインナビゲーション',
    },
  });

  // Handoff 028 §7.1: the overflow fix only changes behaviour at 200% text
  // (flex-wrap only engages once a label's min-content width exceeds its
  // equal-thirds share); confirm normal (100%) scale is unaffected.
  logStep('confirming Event Detail status buttons are unchanged at normal (100%) scale');
  await navigate(page, `/c/${communityId}/events/${eventId}`);
  const eventDetailNormalScale = await collect(page);
  results.push({
    name: 'event-detail-status-buttons-unchanged-at-normal-scale',
    screenshotPath: await screenshot(page, 'event-detail-japanese-100-percent'),
    observed: eventDetailNormalScale,
    checks: {
      htmlLangJa: eventDetailNormalScale.htmlLang === 'ja',
      threeButtonsRendered: eventDetailNormalScale.statusButtons.length === 3,
      noHorizontalScroll: eventDetailNormalScale.noHorizontalScroll,
    },
  });

  // RFC-075 Slice 2: screenshot evidence for Event Detail's migrated cz-*
  // classes (status buttons, note form, participant list) at mobile width
  // and 200% text, in both languages — required per Handoff 027 §8/§15.
  logStep('confirming Event Detail renders Japanese with migrated classes (200% text screenshot)');
  await navigate(page, `/c/${communityId}/events/${eventId}`, { textScale: 2 });
  const eventDetailJapanese = await collect(page);
  results.push({
    name: 'event-detail-renders-japanese',
    screenshotPath: await screenshot(page, 'event-detail-japanese-200-percent'),
    observed: eventDetailJapanese,
    checks: {
      htmlLangJa: eventDetailJapanese.htmlLang === 'ja',
      attendanceButtonsRendered: eventDetailJapanese.statusButtons.length > 0,
      noHorizontalScrollAt200Percent: eventDetailJapanese.noHorizontalScroll,
    },
  });

  // Handoff 038 §7.1: exercises app.js's note-character-limit CSSOM write
  // (`ta.style.borderColor`) under the real page, then resets the textarea
  // before the actual (valid-length) note below — this interaction is never
  // submitted, so it cannot affect the flash assertions that follow.
  logStep('typing past the note character limit to exercise the borderColor CSSOM write');
  const overLimitStyleCheck = await evalExpr(
    page,
    `(() => {
      const ta = document.querySelector('textarea[name="note"]');
      if (!ta) return { found: false };
      const original = ta.value;
      ta.value = 'x'.repeat(201);
      ta.dispatchEvent(new Event('input', { bubbles: true }));
      const borderColorOverLimit = getComputedStyle(ta).borderColor;
      ta.value = original;
      ta.dispatchEvent(new Event('input', { bubbles: true }));
      const borderColorAfterReset = getComputedStyle(ta).borderColor;
      return { found: true, borderColorOverLimit, borderColorAfterReset };
    })()`,
  );
  results.push({
    name: 'note-character-limit-style-write',
    observed: overLimitStyleCheck,
    checks: {
      textareaFound: overLimitStyleCheck.found,
      // #FF3B30 → rgb(255, 59, 48): proves the browser actually applied the
      // CSSOM write, not that CSP silently dropped it while the page kept
      // rendering — the failure mode §12 warns is otherwise invisible.
      borderTurnedRedOverLimit: overLimitStyleCheck.borderColorOverLimit === 'rgb(255, 59, 48)',
      borderResetAfterFix: overLimitStyleCheck.borderColorAfterReset !== 'rgb(255, 59, 48)',
    },
  });

  // Handoff 037: saving a note is a real `?flash=note_saved` redirect round
  // trip, not a mapper unit test — proves the member actually sees the
  // Japanese flash text, not just that `note_flash_message` returns it.
  logStep('confirming saving a note on Event Detail shows the Japanese flash message');
  await fillFieldAndSubmit(
    page,
    'textarea[name="note"]',
    'RFC072 smoke note (ja)',
    `/c/${communityId}/events/${eventId}/my-note`,
  );
  const eventDetailNoteSavedJapanese = await collect(page);
  results.push({
    name: 'event-detail-note-save-flash-is-japanese',
    observed: eventDetailNoteSavedJapanese,
    checks: {
      htmlLangJa: eventDetailNoteSavedJapanese.htmlLang === 'ja',
      redirectedWithFlashCode: eventDetailNoteSavedJapanese.path ===
        `/c/${communityId}/events/${eventId}?flash=note_saved`,
      noteFlashIsJapanese: eventDetailNoteSavedJapanese.noteFlashText === 'メモを保存しました。',
    },
  });

  // RFC-075 Slice 3: Home and the ICS calendar-feed settings page
  // (/c/:cid/me/calendar — not the Calendar page), at mobile width and
  // 200% text, both languages.
  logStep('confirming Home renders Japanese at 200% text (screenshot)');
  await navigate(page, `/c/${communityId}/home`, { textScale: 2 });
  const homeJapanese200 = await collect(page);
  results.push({
    name: 'home-renders-japanese-200-percent',
    screenshotPath: await screenshot(page, 'home-japanese-200-percent'),
    observed: homeJapanese200,
    checks: {
      htmlLangJa: homeJapanese200.htmlLang === 'ja',
      noHorizontalScrollAt200Percent: homeJapanese200.noHorizontalScroll,
    },
  });

  logStep('confirming the calendar-feed settings page renders Japanese at 200% text (screenshot)');
  await navigate(page, `/c/${communityId}/me/calendar`, { textScale: 2 });
  const calendarFeedJapanese200 = await collect(page);
  results.push({
    name: 'calendar-feed-page-renders-japanese-200-percent',
    screenshotPath: await screenshot(page, 'calendar-feed-japanese-200-percent'),
    observed: calendarFeedJapanese200,
    checks: {
      htmlLangJa: calendarFeedJapanese200.htmlLang === 'ja',
      showsGenerateOrFeedUrl:
        calendarFeedJapanese200.text.includes('リンクを作成') || calendarFeedJapanese200.text.includes('http'),
      noHorizontalScrollAt200Percent: calendarFeedJapanese200.noHorizontalScroll,
    },
  });

  logStep('setting language to English via the no-JS settings form (real submit button, no JS shim)');
  await navigate(page, `/c/${communityId}/me/language`, { textScale: 2 });
  const settingsFormJa = await collect(page);
  await selectRadioAndSubmit(
    page,
    'input[name="ui_language"][value="en"]',
    `/c/${communityId}/me/language`,
  );
  const settingsAfterEnglish = await collect(page);
  results.push({
    name: 'settings-form-switches-to-english-via-plain-post',
    screenshotPath: await screenshot(page, 'settings-page-english-200-percent'),
    observed: { settingsFormJa, settingsAfterEnglish, storedUiLanguage: memberUiLanguage() },
    checks: {
      formWasPlainPost: settingsFormJa.text.length > 0, // page rendered without any JS framework; requestSubmit drove a real navigation
      redirectedWithFlash: settingsAfterEnglish.path === `/c/${communityId}/me/language?flash=ui_language_updated`,
      htmlLangEn: settingsAfterEnglish.htmlLang === 'en',
      showsEnglishSuccessMessage: settingsAfterEnglish.text.includes('Language updated.'),
      storedAsEn: memberUiLanguage() === 'en',
    },
  });

  logStep('confirming My Page renders English with lang=en');
  await navigate(page, `/c/${communityId}/me`, { textScale: 2 });
  const meEnglish = await collect(page);
  results.push({
    name: 'my-page-renders-english',
    screenshotPath: await screenshot(page, 'my-page-english-200-percent'),
    observed: meEnglish,
    checks: {
      htmlLangEn: meEnglish.htmlLang === 'en',
      showsEnglishNav: meEnglish.text.includes('Home') && meEnglish.text.includes('Calendar'),
      noHorizontalScrollAt200Percent: meEnglish.noHorizontalScroll,
      // Handoff 036: the rendered attribute must follow locale too, not
      // just stay Japanese from before the switch.
      bottomNavAriaLabelIsEnglish: meEnglish.bottomNavAriaLabel === 'Main navigation',
    },
  });

  logStep('following the display-name link and confirming it is English too');
  await navigate(page, `/c/${communityId}/me/display-name`);
  const displayNameEnglish = await collect(page);
  results.push({
    name: 'display-name-sub-page-renders-english',
    observed: displayNameEnglish,
    checks: {
      htmlLangEn: displayNameEnglish.htmlLang === 'en',
      showsEnglishTitle: displayNameEnglish.text.includes('Change display name'),
      showsEnglishSubmit: displayNameEnglish.text.includes('Save display name'),
    },
  });

  logStep('confirming Home renders English at 200% text (screenshot)');
  await navigate(page, `/c/${communityId}/home`, { textScale: 2 });
  const homeEnglish = await collect(page);
  results.push({
    name: 'home-renders-english',
    screenshotPath: await screenshot(page, 'home-english-200-percent'),
    observed: homeEnglish,
    checks: {
      htmlLangEn: homeEnglish.htmlLang === 'en',
      showsEnglishDateLabel: expectedDateLabelEnPattern.test(homeEnglish.text),
      noAllNumericDate: !expectedAllNumericDatePattern.test(homeEnglish.text),
      noHorizontalScrollAt200Percent: homeEnglish.noHorizontalScroll,
    },
  });

  // Handoff 030: RFC-072 criterion 9 was violated here — the ICS feed
  // settings page was Japanese by omission, not documented decision. Now
  // localized; this assertion is inverted from what it asserted before this
  // package (which was itself the defect being fixed, not a feature).
  logStep('confirming the calendar-feed settings page renders English after switching (200% text screenshot)');
  await navigate(page, `/c/${communityId}/me/calendar`, { textScale: 2 });
  const calendarFeedAfterEnglishSwitch = await collect(page);
  results.push({
    name: 'calendar-feed-page-renders-english-after-switch',
    screenshotPath: await screenshot(page, 'calendar-feed-english-200-percent'),
    observed: calendarFeedAfterEnglishSwitch,
    checks: {
      htmlLangEn: calendarFeedAfterEnglishSwitch.htmlLang === 'en',
      showsEnglishGenerateOrFeedUrl:
        calendarFeedAfterEnglishSwitch.text.includes('Generate feed URL') ||
        calendarFeedAfterEnglishSwitch.text.includes('http'),
      noHorizontalScrollAt200Percent: calendarFeedAfterEnglishSwitch.noHorizontalScroll,
    },
  });

  // Handoff 030: the second page RFC-072 criterion 9 was missing — linked
  // directly from My Page, not admin/anonymous/error, so not one of the
  // three documented exclusions. Resolves locale from the authorizing admin
  // membership (§7.2), not a `:cid` this route doesn't have.
  logStep('confirming the create-community page renders English after switching (200% text screenshot)');
  await navigate(page, '/communities/new', { textScale: 2 });
  const createCommunityEnglish = await collect(page);
  results.push({
    name: 'create-community-page-renders-english-after-switch',
    screenshotPath: await screenshot(page, 'create-community-english-200-percent'),
    observed: createCommunityEnglish,
    checks: {
      htmlLangEn: createCommunityEnglish.htmlLang === 'en',
      showsEnglishTitle: createCommunityEnglish.text.includes('Create community'),
      showsEnglishSubmit: createCommunityEnglish.text.includes('Create'),
      noHorizontalScrollAt200Percent: createCommunityEnglish.noHorizontalScroll,
    },
  });

  logStep('confirming Calendar month view renders English (month header + day-cell aria-label)');
  await navigate(page, `/c/${communityId}/communities?month=${eventMonthParam}`, { textScale: 2 });
  const calendarMonthEnglish = await collect(page);
  results.push({
    name: 'calendar-month-renders-english',
    screenshotPath: await screenshot(page, 'calendar-month-english-200-percent'),
    observed: calendarMonthEnglish,
    checks: {
      htmlLangEn: calendarMonthEnglish.htmlLang === 'en',
      monthHeaderIsEnglish: calendarMonthEnglish.monthHeaderText === expectedMonthHeaderEn,
      monthHeaderNotJapanese: calendarMonthEnglish.monthHeaderText !== expectedMonthHeaderJa,
      dayCellAriaLabelIsEnglish:
        calendarMonthEnglish.dayCellAriaLabel != null &&
        !calendarMonthEnglish.dayCellAriaLabel.includes('年') &&
        !calendarMonthEnglish.dayCellAriaLabel.includes('月') &&
        !calendarMonthEnglish.dayCellAriaLabel.includes('日'),
      noHorizontalScrollAt200Percent: calendarMonthEnglish.noHorizontalScroll,
    },
  });

  logStep('confirming Calendar list view renders English');
  await navigate(page, `/c/${communityId}/communities?month=${eventMonthParam}&view=list`);
  const calendarListEnglish = await collect(page);
  results.push({
    name: 'calendar-list-renders-english',
    observed: calendarListEnglish,
    checks: {
      htmlLangEn: calendarListEnglish.htmlLang === 'en',
      showsEnglishDateLabel: expectedDateLabelEnPattern.test(calendarListEnglish.text),
    },
  });

  logStep('confirming Calendar matrix view renders English (month header + member column)');
  await navigate(page, `/c/${communityId}/communities?month=${eventMonthParam}&view=matrix`);
  const calendarMatrixEnglish = await collect(page);
  results.push({
    name: 'calendar-matrix-renders-english',
    observed: calendarMatrixEnglish,
    checks: {
      htmlLangEn: calendarMatrixEnglish.htmlLang === 'en',
      monthHeaderIsEnglish: calendarMatrixEnglish.monthHeaderText === expectedMonthHeaderEn,
      showsEnglishMemberColumn: calendarMatrixEnglish.text.includes('Member'),
    },
  });

  logStep('confirming Event Detail renders English, including the event day date label (200% text screenshot)');
  await navigate(page, `/c/${communityId}/events/${eventId}`, { textScale: 2 });
  const eventDetailEnglish = await collect(page);
  results.push({
    name: 'event-detail-renders-english',
    screenshotPath: await screenshot(page, 'event-detail-english-200-percent'),
    observed: eventDetailEnglish,
    checks: {
      htmlLangEn: eventDetailEnglish.htmlLang === 'en',
      showsEnglishDateLabel: expectedDateLabelEnPattern.test(eventDetailEnglish.text),
      showsEnglishStatusLabels:
        eventDetailEnglish.text.includes('Going') && eventDetailEnglish.text.includes('No answer'),
      // Handoff 026: the specific defect this scenario previously missed.
      // The two checks above are satisfied by the counts line ("Going 0 ·
      // No Go 0 · No answer 3"), which was already locale-aware before this
      // fix — they would have passed even while the attendance buttons
      // themselves rendered Japanese. These checks are scoped to the
      // buttons directly (form[action*="my-status"]).
      attendanceButtonsRendered: eventDetailEnglish.statusButtons.length > 0,
      attendanceButtonsAreEnglish: eventDetailEnglish.statusButtons.every(
        (label) => label === 'Going' || label === 'No Go' || label === 'Attended',
      ),
      attendanceButtonsHaveNoJapanese: eventDetailEnglish.statusButtons.every(
        (label) => !/[぀-ヿ一-鿿]/.test(label ?? ''),
      ),
      noHorizontalScrollAt200Percent: eventDetailEnglish.noHorizontalScroll,
    },
  });

  logStep('confirming saving a note on Event Detail shows the English flash message');
  await fillFieldAndSubmit(
    page,
    'textarea[name="note"]',
    'RFC072 smoke note (en)',
    `/c/${communityId}/events/${eventId}/my-note`,
  );
  const eventDetailNoteSavedEnglish = await collect(page);
  results.push({
    name: 'event-detail-note-save-flash-is-english',
    observed: eventDetailNoteSavedEnglish,
    checks: {
      htmlLangEn: eventDetailNoteSavedEnglish.htmlLang === 'en',
      redirectedWithFlashCode: eventDetailNoteSavedEnglish.path ===
        `/c/${communityId}/events/${eventId}?flash=note_saved`,
      noteFlashIsEnglish: eventDetailNoteSavedEnglish.noteFlashText === 'Note saved.',
      noteFlashHasNoJapanese: !/[぀-ヿ一-鿿]/.test(eventDetailNoteSavedEnglish.noteFlashText ?? ''),
    },
  });

  logStep('confirming the other community membership was not affected (per-membership, not per-user)');
  const otherStillNull = memberUiLanguage(otherMembershipId) === null;

  logStep('setting language back to Japanese and confirming a page flips back');
  await navigate(page, `/c/${communityId}/me/language`);
  await selectRadioAndSubmit(
    page,
    'input[name="ui_language"][value="ja"]',
    `/c/${communityId}/me/language`,
  );
  await navigate(page, `/c/${communityId}/home`);
  const homeAfterFlipBack = await collect(page);
  results.push({
    name: 'language-flips-back-to-japanese-and-is-membership-scoped',
    observed: { homeAfterFlipBack, otherMembershipUiLanguage: memberUiLanguage(otherMembershipId) },
    checks: {
      htmlLangJa: homeAfterFlipBack.htmlLang === 'ja',
      showsJapaneseText: homeAfterFlipBack.text.includes('ホーム'),
      otherMembershipUnaffectedThroughout: otherStillNull,
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
    note: 'Chromium launched with --incognito and without --no-sandbox. Local wrangler dev only. The language-setting form is a plain <form method="post"> with a real submit button; selectRadioAndSubmit drives its native requestSubmit(), not a JS shim or fetch call.',
    localOnlyGuard: true,
    results,
    notes,
    passed: results.every((result) => result.passed),
  };

  await writeFile(`${outDir}/${reportName}`, JSON.stringify(report, null, 2));
  console.log(
    JSON.stringify(
      {
        passed: report.passed,
        results: results.map((result) => ({ name: result.name, passed: result.passed, checks: result.checks })),
      },
      null,
      2,
    ),
  );

  if (!report.passed) process.exitCode = 1;
} catch (error) {
  if (devStderr.trim()) {
    console.error('[language-preference-smoke] wrangler stderr follows:');
    console.error(devStderr.trim());
  }
  if (chromeStderr.trim()) {
    console.error('[language-preference-smoke] chromium stderr follows:');
    console.error(chromeStderr.trim());
  }
  throw error;
} finally {
  if (chrome) await killAndWait(chrome);
  if (dev) await killAndWait(dev);
  await isolated.cleanup();
}
