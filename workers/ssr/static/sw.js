// ciao.zinnias Service Worker — RFC-017 / RFC-008
// Read-only caching only. No mutation queue. No IndexedDB private store.
'use strict';

const CACHE_VERSION   = 'v0.5.0';
const SHELL_CACHE     = 'shell-' + CACHE_VERSION;
const PAGE_CACHE      = 'pages-' + CACHE_VERSION;
const OFFLINE_URL     = '/offline';

// Static shell assets — cache-first, versioned.
const SHELL_ASSETS = [
  '/static/app.css',
  '/static/app.js',
  '/manifest.webmanifest',
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
// Delete all caches from previous versions (deploy cache-bust, RFC-017 §5).
self.addEventListener('activate', function(e) {
  e.waitUntil(
    caches.keys().then(function(keys) {
      return Promise.all(
        keys.filter(function(k) {
          return k !== SHELL_CACHE && k !== PAGE_CACHE;
        }).map(function(k) { return caches.delete(k); })
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

  // Never cache: POSTs, session-sensitive API paths, or cross-origin.
  if (url.origin !== self.location.origin) return;
  if (url.pathname.startsWith('/healthz') ||
      url.pathname.startsWith('/version')) return;

  // Shell assets: cache-first.
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

  // Authorized GET pages (home, event detail, join): network-first, fall back to cache.
  // Never cache /join/profile (contains a live form token) or static icons.
  if (url.pathname.startsWith('/c/') ||
      url.pathname === '/' ||
      url.pathname === '/join') {
    e.respondWith(
      fetch(req)
        .then(function(resp) {
          // Only cache successful HTML responses — never cache redirects or errors.
          if (resp.ok && resp.headers.get('content-type') &&
              resp.headers.get('content-type').includes('text/html')) {
            const clone = resp.clone();
            caches.open(PAGE_CACHE).then(function(c) { c.put(req, clone); });
          }
          return resp;
        })
        .catch(function() {
          // Offline: serve from cache or the offline fallback.
          return caches.match(req).then(function(cached) {
            return cached || caches.match(OFFLINE_URL) ||
              new Response('<html><body><p>You are offline. Open again when connected.</p></body></html>',
                { headers: { 'Content-Type': 'text/html' } });
          });
        })
    );
    return;
  }
});

// ── Message: PURGE_PRIVATE ────────────────────────────────────────────────
// Triggered by app.js before logout to clear private page cache (RFC-017 §7).
self.addEventListener('message', function(e) {
  if (e.data && e.data.type === 'PURGE_PRIVATE') {
    caches.delete(PAGE_CACHE).catch(function() {});
  }
});
