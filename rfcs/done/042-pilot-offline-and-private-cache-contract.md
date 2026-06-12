# RFC 042 — Pilot Offline and Private Cache Contract

**Status.** Implemented (v0.23.0)
**Phase:** F7 / Stabilization (architect deep-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes deep-review finding P0/P1 (service-worker private caching). Refines RFC-017 (PWA installability and service worker) and RFC-008 (offline cache). Tightens the offline policy for the pilot.

---

## 1. Summary

The service worker cached authenticated HTML (`/`, `/c/*`, `/join`) in a page
cache that was purged only by a best-effort JavaScript message, and its
`CACHE_VERSION` was stale relative to the release. On a shared device this could
leave one member's community pages, names, and notes visible to the next user.
This RFC sets the pilot policy to **static assets only**: authenticated HTML is
never cached; it is network-only with a static offline fallback.

---

## 2. Motivation

Deep-review P0/P1. The prior `sw.js`:

- cached `/c/`, `/`, and `/join` responses in `PAGE_CACHE` (network-first, but
  the successful HTML was written to cache);
- purged that cache only via a `PURGE_PRIVATE` message from `app.js` at logout —
  best-effort, dependent on the JS path completing, and useless if the browser
  was closed or the message dropped;
- declared `CACHE_VERSION = 'v0.19.0'` inside a v0.23.0 build, so deploy-time
  cache-busting did not occur.

Caching private, community-scoped HTML on a device that may be shared (an
explicit security assumption in requirements §4.4) is a privacy leak that a
best-effort purge does not close. For a pilot, the simplest safe policy is to
not cache private pages at all.

---

## 3. Goals

- **Never** store authenticated HTML in any cache.
- Cache only static, non-sensitive assets (CSS, JS, manifest) and a static
  `/offline` fallback page.
- On network failure for an authenticated route, serve the static offline page —
  not a stale private page.
- Clean up any legacy page cache from prior versions at service-worker
  activation (defensive, for devices upgrading from an older build).
- Keep `CACHE_VERSION` in step with the release and add a release-checklist gate
  to verify it.

---

## 4. Non-Goals

- **No offline writes / mutation queue.** RFC-008's queued-mutation vision is
  not part of the pilot SW; the app is online-first for writes (forms POST to
  the server, AD-1). Offline write support, if ever pursued, is a separate RFC.
- No caching of read-only private data for offline viewing in the pilot. (A
  future stricter private-cache design could revisit this with encryption /
  per-session scoping; explicitly out of scope now.)
- No background sync, push, or periodic sync.

---

## 5. External Behavior

| Scenario | Behavior |
|---|---|
| Online, normal use | Pages served fresh from network. |
| Offline, opens an authenticated page | Static offline fallback page shown ("You are offline. Open again when connected."). |
| Offline, requests a cached static asset | Served from the shell cache. |
| Shared device, second user | No previous user's private HTML is retrievable from cache. |
| App upgrade from older build | Legacy page cache deleted at activation. |
| Logout | All non-shell caches cleared (defensive; no private cache should exist anyway). |

---

## 6. Internal Design

### 6.1 Caches

- `SHELL_CACHE = 'shell-' + CACHE_VERSION` holds `SHELL_ASSETS`:
  `/static/app.css`, `/static/app.js`, `/manifest.webmanifest`, `/offline`.
- No `PAGE_CACHE`. There is no cache for HTML routes.

### 6.2 Fetch routing

- Non-GET, cross-origin, and `/healthz` / `/version` are not intercepted.
- Shell assets: cache-first.
- `/`, `/c/*`, `/join*`: **network-only**, with `.catch()` falling back to the
  cached `/offline` page (or an inline minimal offline HTML if `/offline`
  itself is missing). Nothing from these routes is written to a cache.

### 6.3 Activation cleanup

`activate` deletes every cache key except the current `SHELL_CACHE`, removing
any `pages-*` cache left by versions ≤ 0.19.0.

### 6.4 Versioning + gate

`CACHE_VERSION = 'v0.23.0'`. A release-checklist item (and, per RFC-044, an
optional automated gate) verifies the SW version matches the package version at
release time.

### 6.5 `PURGE_PRIVATE`

Retained for compatibility with `app.js`; it now clears all non-shell caches as
a defensive no-op (there is no private cache to purge under this policy).

---

## 7. Data Model Notes

Not applicable (client-side caching policy). Relates to requirements §11.5
(local cache lifecycle) and §16.3 (local storage boundaries): the SW now holds
no private data, satisfying "must not cache data for communities after the user
is removed" and "private cache cleared on logout" trivially.

---

## 8. API and UI Contract Notes

- A `/offline` route serving a static, non-sensitive page must exist (it does;
  it is in `SHELL_ASSETS`).
- No change to application routes or forms.

---

## 9. Security, Privacy, and Safety

- **Primary win:** eliminates the shared-device private-HTML leak. This directly
  satisfies the security-acceptance criterion "private cache is cleared or
  locked on logout/session expiration" — by never creating it.
- The SW never caches unauthorized or authenticated responses across sessions
  (RFC-017 §requirement).
- Offline fallback is a static, content-free page; it reveals nothing about any
  community.

---

## 10. Acceptance Criteria

1. No authenticated HTML is present in any cache after browsing
   (`caches.keys()` shows only `shell-v0.23.0`). (Pre-pilot gate #8.)
2. Offline navigation to `/c/*` yields the offline page, not a prior view.
3. `CACHE_VERSION` equals the release version.
4. Activation removes legacy `pages-*` caches.

All met in v0.23.0.

---

## 11. Test Plan

- **Manual (pre-pilot gate):** in a browser, log in, browse, go offline, attempt
  to revisit a community page, confirm the offline page appears; inspect
  `caches` to confirm only the shell cache exists. On a second profile/user,
  confirm no private page is retrievable.
- **Release gate:** SW version check (RFC-044).

---

## 12. Rollout Plan

Shipped in v0.23.0. On next load, the new SW activates, deletes legacy caches,
and installs the shell-only cache. Users upgrading from an older build have any
cached private pages purged at activation.

---

## 13. Open Decisions

- **Future read-only private offline cache.** If offline *reading* of recent
  events becomes a requirement, design a stricter scheme (per-session scoping,
  short TTL, explicit "offline copy" labelling, possibly encryption). Out of
  scope for the pilot; would be its own RFC superseding this policy.
