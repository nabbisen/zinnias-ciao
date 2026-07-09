// ciao.zinnias Service Worker — RFC-017 / RFC-008 / RFC-042
// Pilot caching policy: STATIC ASSETS ONLY.
//
// Authenticated HTML (/, /c/*, /join) is NEVER cached. Community pages contain
// member names, notes, and event data; caching them risks leaving private data
// on shared devices, and a JS-message purge is best-effort only. For the pilot
// we cache just the static shell and an offline fallback page. A stricter
// private-cache design may be revisited in a later RFC.
'use strict';

// Keep in sync with the release version. A release gate verifies this matches
// the package version (see docs/src/tester/release-checklist.md).
const CACHE_VERSION = 'v0.53.1';
const SHELL_CACHE   = 'shell-' + CACHE_VERSION;
const OFFLINE_URL   = '/offline';

// Static, non-sensitive assets — safe to cache. Includes the offline page.
const SHELL_ASSETS = [
  '/static/app.css',
  '/static/app.js',
  '/manifest.webmanifest',
  OFFLINE_URL,
];

// ── Install ───────────────────────────────────────────────────────────────
self.addEventListener('install', function(e) {
  e.waitUntil(
    caches.open(SHELL_CACHE)
      .then(function(c) { return c.addAll(SHELL_ASSETS); })
      .then(function() { return self.skipWaiting(); })
  );
});

// ── Activate ─────────────────────────────────────────────────────────────
// Delete all caches from previous versions, including any legacy page cache
// from earlier builds that cached authenticated HTML (privacy cleanup).
self.addEventListener('activate', function(e) {
  e.waitUntil(
    caches.keys().then(function(keys) {
      return Promise.all(
        keys.filter(function(k) { return k !== SHELL_CACHE; })
            .map(function(k) { return caches.delete(k); })
      );
    }).then(function() { return self.clients.claim(); })
  );
});

// ── Fetch ─────────────────────────────────────────────────────────────────
self.addEventListener('fetch', function(e) {
  const req = e.request;
  const url = new URL(req.url);

  // Never intercept non-GET requests — forms POST to the server (AD-1).
  if (req.method !== 'GET') return;
  // Same-origin only.
  if (url.origin !== self.location.origin) return;
  // Never touch health/version probes.
  if (url.pathname.startsWith('/healthz') ||
      url.pathname.startsWith('/version')) return;

  // Static shell assets: cache-first.
  if (SHELL_ASSETS.includes(url.pathname) ||
      url.pathname === '/manifest.webmanifest') {
    e.respondWith(
      caches.open(SHELL_CACHE).then(function(c) {
        return c.match(req).then(function(cached) {
          return cached || fetch(req);
        });
      })
    );
    return;
  }

  // Authenticated HTML (/, /c/*, /join): NETWORK-ONLY, never cached.
  // On network failure, serve the static offline page — not a stale private page.
  if (url.pathname.startsWith('/c/') ||
      url.pathname === '/' ||
      url.pathname.startsWith('/join')) {
    e.respondWith(
      fetch(req).catch(function() {
        return caches.match(OFFLINE_URL).then(function(off) {
          return off || new Response(
            '<html><body><p>You are offline. Open again when connected.</p></body></html>',
            { headers: { 'Content-Type': 'text/html' } });
        });
      })
    );
    return;
  }
});

// ── Message: PURGE_PRIVATE ────────────────────────────────────────────────
// No private page cache exists in this policy, but app.js still sends this on
// logout. We clear everything except the static shell as a defensive measure.
self.addEventListener('message', function(e) {
  if (e.data && e.data.type === 'PURGE_PRIVATE') {
    caches.keys().then(function(keys) {
      keys.filter(function(k) { return k !== SHELL_CACHE; })
          .forEach(function(k) { caches.delete(k).catch(function() {}); });
    }).catch(function() {});
  }
});
