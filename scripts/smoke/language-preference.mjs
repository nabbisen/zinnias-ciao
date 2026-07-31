#!/usr/bin/env node
// Scenario smoke for RFC-072 Slice C: the language preference setting,
// linked from My Page, and its effect on the member-facing core. Local
// wrangler dev only.

import { prepareIsolatedWorkerTest } from "../lib/isolated-worker-test.mjs";

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
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${memberMembershipId}', '${communityId}', '${memberUserId}', 'member', '${esc(memberDisplayName)}', '${now}')`,
    `INSERT INTO community_memberships (id, community_id, user_id, role, display_name, joined_at) VALUES ('${otherMembershipId}', '${otherCommunityId}', '${memberUserId}', 'member', 'RFC072 Member Other', '${now}')`,
    `INSERT INTO sessions (id, user_id, session_hmac, created_at, expires_at, last_seen_at) VALUES ('sess_rfc072_member', '${memberUserId}', '${memberSessionHmac}', '${now}', '2099-12-31T23:59:59.000Z', '${now}')`,
    `INSERT INTO events (id, community_id, created_by_membership_id, title, location, description, status, repeat_rule, repeat_count, created_at, updated_at) VALUES ('${eventId}', '${communityId}', '${memberMembershipId}', 'RFC072 Visible Event', 'RFC072 Room', '', 'scheduled', 'none', NULL, '${now}', '${now}')`,
    // 2026-08-03T00:00:00Z is 09:00 JST on Monday, 3 Aug 2026 — matches the
    // native tz.rs/render.rs tests exercising the same date.
    `INSERT INTO event_days (id, event_id, community_id, seq, day_date, starts_at_utc, ends_at_utc, created_at, occurrence_status) VALUES ('${eventDayId}', '${eventId}', '${communityId}', 1, '2026-08-03', '2026-08-03T00:00:00.000Z', '2026-08-03T01:00:00.000Z', '${now}', 'scheduled')`,
  ];
  for (const statement of statements) sql(statement);
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
      this.events.set(method, [...(this.events.get(method) ?? []), cb]);
    });
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
      const fields = [...document.querySelectorAll('input[name], textarea[name], select[name]')];
      const links = [...document.querySelectorAll('a[href]')].map((a) => ({
        href: a.getAttribute('href'),
        text: a.innerText,
      }));
      const dayCell = document.querySelector('a[aria-label][href*="day="]');
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
        values: Object.fromEntries(fields.map((el) => [el.getAttribute('name'), el.value])),
        dayCellAriaLabel: dayCell ? dayCell.getAttribute('aria-label') : null,
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
      showsEnglishDateLabel: /\b(Mon|Tue|Wed|Thu|Fri|Sat|Sun), 3 Aug\b/.test(homeEnglish.text),
      noAllNumericDate: !/\b08\/03\/2026\b/.test(homeEnglish.text),
      noHorizontalScrollAt200Percent: homeEnglish.noHorizontalScroll,
    },
  });

  // The ICS feed settings page is intentionally out of RFC-072 scope and
  // stays Japanese-only regardless of the member's language preference —
  // confirming that invariant survived this package's styling-only
  // migration is as important as the layout itself not overflowing.
  logStep('confirming the calendar-feed settings page stays Japanese-only after switching to English (200% text screenshot)');
  await navigate(page, `/c/${communityId}/me/calendar`, { textScale: 2 });
  const calendarFeedAfterEnglishSwitch = await collect(page);
  results.push({
    name: 'calendar-feed-page-stays-japanese-only-after-english-switch',
    screenshotPath: await screenshot(page, 'calendar-feed-still-japanese-200-percent'),
    observed: calendarFeedAfterEnglishSwitch,
    checks: {
      htmlLangStaysJa: calendarFeedAfterEnglishSwitch.htmlLang === 'ja',
      noHorizontalScrollAt200Percent: calendarFeedAfterEnglishSwitch.noHorizontalScroll,
    },
  });

  logStep('confirming Calendar month view renders English (month header + day-cell aria-label)');
  await navigate(page, `/c/${communityId}/communities?month=2026-08`, { textScale: 2 });
  const calendarMonthEnglish = await collect(page);
  results.push({
    name: 'calendar-month-renders-english',
    screenshotPath: await screenshot(page, 'calendar-month-english-200-percent'),
    observed: calendarMonthEnglish,
    checks: {
      htmlLangEn: calendarMonthEnglish.htmlLang === 'en',
      monthHeaderIsEnglish: calendarMonthEnglish.monthHeaderText === 'August 2026',
      monthHeaderNotJapanese: calendarMonthEnglish.monthHeaderText !== '2026年8月',
      dayCellAriaLabelIsEnglish:
        calendarMonthEnglish.dayCellAriaLabel != null &&
        !calendarMonthEnglish.dayCellAriaLabel.includes('年') &&
        !calendarMonthEnglish.dayCellAriaLabel.includes('月') &&
        !calendarMonthEnglish.dayCellAriaLabel.includes('日'),
      noHorizontalScrollAt200Percent: calendarMonthEnglish.noHorizontalScroll,
    },
  });

  logStep('confirming Calendar list view renders English');
  await navigate(page, `/c/${communityId}/communities?month=2026-08&view=list`);
  const calendarListEnglish = await collect(page);
  results.push({
    name: 'calendar-list-renders-english',
    observed: calendarListEnglish,
    checks: {
      htmlLangEn: calendarListEnglish.htmlLang === 'en',
      showsEnglishDateLabel: /\b(Mon|Tue|Wed|Thu|Fri|Sat|Sun), 3 Aug\b/.test(calendarListEnglish.text),
    },
  });

  logStep('confirming Calendar matrix view renders English (month header + member column)');
  await navigate(page, `/c/${communityId}/communities?month=2026-08&view=matrix`);
  const calendarMatrixEnglish = await collect(page);
  results.push({
    name: 'calendar-matrix-renders-english',
    observed: calendarMatrixEnglish,
    checks: {
      htmlLangEn: calendarMatrixEnglish.htmlLang === 'en',
      monthHeaderIsEnglish: calendarMatrixEnglish.monthHeaderText === 'August 2026',
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
      showsEnglishDateLabel: /\b(Mon|Tue|Wed|Thu|Fri|Sat|Sun), 3 Aug\b/.test(eventDetailEnglish.text),
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
  if (chrome) chrome.kill('SIGTERM');
  if (dev) dev.kill('SIGTERM');
  await isolated.cleanup();
}
