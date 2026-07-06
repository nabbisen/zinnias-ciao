import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';

const args = process.argv.slice(2);
const rawBaseUrl = argValue('--base-url') ?? args.find((arg) => !arg.startsWith('--'));
if (!rawBaseUrl) {
  console.error('Usage: node scripts/runtime-smoke.mjs <url>');
  console.error('Example: node scripts/runtime-smoke.mjs http://127.0.0.1:8787');
  process.exit(2);
}

const baseUrl = normalizeBaseUrl(rawBaseUrl);
const expectedVersion = process.env.EXPECTED_VERSION ?? defaultExpectedVersion(baseUrl);
const outDir = process.env.EVIDENCE_DIR ?? '.git-exclude/evidence/rfc050-prototype';
const chromium = process.env.CHROMIUM ?? '/usr/bin/chromium';
const remotePort = Number(process.env.CHROME_REMOTE_PORT ?? 9250);
const userDataDir = `.git-exclude/tmp/chrome-rfc050-runtime-sandboxed-${Date.now()}`;

await mkdir(outDir, { recursive: true });
await rm(userDataDir, { recursive: true, force: true });

function argValue(name) {
  const index = args.indexOf(name);
  if (index === -1) return undefined;
  return args[index + 1];
}

function normalizeBaseUrl(raw) {
  const url = new URL(raw);
  url.pathname = url.pathname.replace(/\/+$/, '');
  url.search = '';
  url.hash = '';
  return url;
}

function defaultExpectedVersion(url) {
  if (url.hostname === 'localhost' || url.hostname === '127.0.0.1' || url.hostname === '[::1]') {
    return 'dev';
  }
  return 'staging';
}

function urlFor(path) {
  return new URL(path, baseUrl).toString();
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchText(path) {
  const res = await fetch(urlFor(path), { redirect: 'manual' });
  return {
    path,
    status: res.status,
    redirected: res.status >= 300 && res.status < 400,
    location: res.headers.get('location'),
    contentType: res.headers.get('content-type'),
    cacheControl: res.headers.get('cache-control'),
    csp: res.headers.get('content-security-policy'),
    referrerPolicy: res.headers.get('referrer-policy'),
    permissionsPolicy: res.headers.get('permissions-policy'),
    frameOptions: res.headers.get('x-frame-options'),
    contentTypeOptions: res.headers.get('x-content-type-options'),
    body: await res.text(),
  };
}

async function fetchJson(path) {
  const result = await fetchText(path);
  try {
    result.json = JSON.parse(result.body);
  } catch (error) {
    result.jsonError = error.message;
  }
  return result;
}

function hasSecurityHeaders(result) {
  return Boolean(
    result.csp?.includes("default-src 'self'")
      && result.csp?.includes("form-action 'self'")
      && result.referrerPolicy
      && result.frameOptions === 'DENY'
      && result.contentTypeOptions === 'nosniff'
      && result.permissionsPolicy?.includes('camera=()'),
  );
}

function htmlRouteOk(result, expectedStatus = 200) {
  return result.status === expectedStatus
    && result.contentType?.includes('text/html')
    && result.cacheControl?.includes('no-store')
    && hasSecurityHeaders(result);
}

async function json(url, init) {
  const res = await fetch(url, init);
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}: ${url}`);
  return await res.json();
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

async function newPage() {
  const target = await json(`http://127.0.0.1:${remotePort}/json/new`, { method: 'PUT' });
  const cdp = new Cdp(target.webSocketDebuggerUrl);
  await cdp.open();
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  return cdp;
}

async function navigate(cdp, path, options = {}) {
  await cdp.send('Emulation.setDeviceMetricsOverride', {
    width: options.width ?? 390,
    height: options.height ?? 900,
    deviceScaleFactor: 1,
    mobile: true,
  });
  await cdp.send('Emulation.setScriptExecutionDisabled', {
    value: Boolean(options.disableJavaScript),
  });
  const loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: urlFor(path) });
  await loaded;
  if (options.textScale === 2) {
    await evalExpr(cdp, `document.documentElement.style.fontSize = '200%'`);
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

async function collectPage(cdp) {
  return await evalExpr(
    cdp,
    `(() => ({
      path: location.pathname + location.search,
      title: document.title,
      text: document.body.innerText,
      htmlLang: document.documentElement.lang,
      noHorizontalScroll: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
      formCount: document.forms.length,
      hasInviteInput: Boolean(document.querySelector('input[name="code"]')),
      hasSubmit: Boolean(document.querySelector('button[type="submit"], input[type="submit"]')),
    }))()`,
  );
}

function allChecksPass(checks) {
  return Object.values(checks).every(Boolean);
}

function buildDiagnostics(routeResults, browserResults) {
  const diagnostics = [];
  const health = routeResults.find((result) => result.name === 'healthz-ok');
  const version = routeResults.find((result) => result.name === 'version-ok');
  const join = routeResults.find((result) => result.name === 'join-html');
  if (health?.passed && version?.passed && join?.observed?.status === 500) {
    diagnostics.push({
      area: 'join-route',
      summary: '/join returned 500 while health/version/static routes passed.',
      likelyCause: 'The deployed Worker is reachable, but the first route that issues a D1-backed form token failed. Check staging D1 binding and migrations, especially the form_tokens table.',
      checks: [
        'Confirm wrangler.staging.local.toml [env.staging] DB database_id is the real staging D1 id.',
        'Run bun run migrate:staging after creating/replacing the staging D1 database. The script uses wrangler d1 migrations apply --remote.',
        'Verify remote migrations with: bunx wrangler d1 migrations list zinnias-ciao-staging --remote --env staging',
        'Verify form_tokens exists with: bunx wrangler d1 execute zinnias-ciao-staging --remote --env staging --command "SELECT name FROM sqlite_master WHERE type=\'table\' AND name=\'form_tokens\'"',
        'If remote D1 checks pass, run bunx wrangler tail --env staging while requesting /join to capture the Worker exception.',
        'Confirm HMAC_PEPPER is set for staging before testing real join flows.',
      ],
    });
  }

  const joinBrowser = browserResults.find((result) => result.name === 'join-renders-at-mobile-200-percent');
  if (joinBrowser && !joinBrowser.checks.inviteForm) {
    diagnostics.push({
      area: 'join-browser',
      summary: 'The browser reached /join but saw no invite form.',
      likelyCause: 'The page rendered the generic error page instead of the join form; inspect the join-route diagnostic first.',
    });
  }

  return diagnostics;
}

const routeResults = [];
const browserResults = [];
let chrome;
let chromeStderr = '';

try {
  const health = await fetchJson('/healthz');
  routeResults.push({
    name: 'healthz-ok',
    observed: {
      status: health.status,
      json: health.json,
      cacheControl: health.cacheControl,
    },
    checks: {
      status: health.status === 200,
      jsonOk: health.json?.ok === true,
      noStore: health.cacheControl?.includes('no-store'),
      securityHeaders: hasSecurityHeaders(health),
    },
  });

  const version = await fetchJson('/version');
  routeResults.push({
    name: 'version-ok',
    observed: {
      status: version.status,
      json: version.json,
      expectedVersion,
      cacheControl: version.cacheControl,
    },
    checks: {
      status: version.status === 200,
      jsonOk: version.json?.ok === true,
      version: version.json?.version === expectedVersion,
      noStore: version.cacheControl?.includes('no-store'),
      securityHeaders: hasSecurityHeaders(version),
    },
  });

  const routes = [
    { path: '/join', name: 'join-html' },
    { path: '/offline', name: 'offline-html' },
  ];
  for (const route of routes) {
    const result = await fetchText(route.path);
    routeResults.push({
      name: route.name,
      observed: {
        status: result.status,
        contentType: result.contentType,
        cacheControl: result.cacheControl,
        bodyExcerpt: result.body.slice(0, 200),
      },
      checks: {
        html: htmlRouteOk(result),
        hasJapaneseHtml: result.body.includes('<html lang="ja">'),
      },
    });
  }

  const manifest = await fetchText('/manifest.webmanifest');
  routeResults.push({
    name: 'manifest-cache-and-security',
    observed: {
      status: manifest.status,
      contentType: manifest.contentType,
      cacheControl: manifest.cacheControl,
    },
    checks: {
      status: manifest.status === 200,
      contentType: manifest.contentType?.includes('application/manifest+json'),
      publicCache: manifest.cacheControl?.includes('public'),
      securityHeaders: hasSecurityHeaders(manifest),
    },
  });

  const serviceWorker = await fetchText('/sw.js');
  routeResults.push({
    name: 'service-worker-no-cache-and-security',
    observed: {
      status: serviceWorker.status,
      contentType: serviceWorker.contentType,
      cacheControl: serviceWorker.cacheControl,
    },
    checks: {
      status: serviceWorker.status === 200,
      contentType: serviceWorker.contentType?.includes('javascript'),
      noCache: serviceWorker.cacheControl?.includes('no-cache'),
      cacheVersion: serviceWorker.body.includes('CACHE_VERSION'),
      securityHeaders: hasSecurityHeaders(serviceWorker),
    },
  });

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

  const join = await newPage();
  await navigate(join, '/join', { textScale: 2, width: 390 });
  const joinPage = await collectPage(join);
  browserResults.push({
    name: 'join-renders-at-mobile-200-percent',
    screenshotPath: await screenshot(join, 'join-mobile-200-percent'),
    observed: joinPage,
    checks: {
      japaneseDocument: joinPage.htmlLang === 'ja',
      inviteForm: joinPage.hasInviteInput && joinPage.hasSubmit,
      noHorizontalScroll: joinPage.noHorizontalScroll,
      noTechnicalError: !/SQL|panic|stack|D1Error|FOREIGN KEY/i.test(joinPage.text),
    },
  });
  join.close();

  const noJs = await newPage();
  await navigate(noJs, '/join', { disableJavaScript: true, width: 390 });
  const noJsPage = await collectPage(noJs);
  browserResults.push({
    name: 'join-renders-with-javascript-disabled',
    screenshotPath: await screenshot(noJs, 'join-no-js-mobile'),
    observed: noJsPage,
    checks: {
      inviteForm: noJsPage.hasInviteInput && noJsPage.hasSubmit,
      noHorizontalScroll: noJsPage.noHorizontalScroll,
      noTechnicalError: !/SQL|panic|stack|D1Error|FOREIGN KEY/i.test(noJsPage.text),
    },
  });
  noJs.close();

  const offline = await newPage();
  await navigate(offline, '/offline', { textScale: 2, width: 390 });
  const offlinePage = await collectPage(offline);
  browserResults.push({
    name: 'offline-page-renders-at-mobile-200-percent',
    screenshotPath: await screenshot(offline, 'offline-mobile-200-percent'),
    observed: offlinePage,
    checks: {
      japaneseDocument: offlinePage.htmlLang === 'ja',
      offlineCopy: offlinePage.text.includes('オフライン'),
      noHorizontalScroll: offlinePage.noHorizontalScroll,
      noTechnicalError: !/SQL|panic|stack|D1Error|FOREIGN KEY/i.test(offlinePage.text),
    },
  });
  offline.close();

  for (const result of [...routeResults, ...browserResults]) {
    result.passed = allChecksPass(result.checks);
  }

  const report = {
    generatedAt: new Date().toISOString(),
    baseUrl: baseUrl.toString(),
    expectedVersion,
    chromium,
    userDataDir,
    chromeFlags: flags,
    note: 'Prototype RFC-050/RFC-045 runtime evidence collector. Chromium launches with --incognito and without --no-sandbox. The script takes a URL to an already-running Worker and does not start, deploy, seed, or mutate D1.',
    routeResults,
    browserResults,
    diagnostics: buildDiagnostics(routeResults, browserResults),
    pendingManualEvidence: [
      'Seeded authenticated admin/member flows',
      'Asia/Tokyo event create/edit/ICS round-trip',
      'Concurrent invite and form-token race checks',
      'Real phone 200% text scaling',
      'Logpush delivery to R2/S3',
      'Workers CPU/runtime dashboard review',
    ],
    passed: [...routeResults, ...browserResults].every((result) => result.passed),
  };

  await writeFile(
    `${outDir}/rfc050-runtime-smoke-results.json`,
    JSON.stringify(report, null, 2),
  );
  console.log(JSON.stringify({
    passed: report.passed,
    evidence: `${outDir}/rfc050-runtime-smoke-results.json`,
    routeResults: routeResults.map((result) => ({
      name: result.name,
      passed: result.passed,
      checks: result.checks,
    })),
    browserResults: browserResults.map((result) => ({
      name: result.name,
      passed: result.passed,
      checks: result.checks,
      screenshotPath: result.screenshotPath,
    })),
    diagnostics: report.diagnostics,
    pendingManualEvidence: report.pendingManualEvidence,
  }, null, 2));

  if (!report.passed) process.exitCode = 1;
} finally {
  if (chrome) chrome.kill('SIGTERM');
}
