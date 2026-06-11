# RFC 017 — PWA Installability and Service Worker

**Status.** Proposed
**Phase:** M3 / Read-only Offline + PWA
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** AD-1 — the service worker is read-only caching only. It is plain JS (not hydration) and is compatible with SSR. It hosts no mutation queue (that was removed from RFC-008).

---

## 1. Summary

Defines PWA installability and a minimal service worker: cache the static shell and recently viewed authorized pages, provide an honest offline fallback, and clear private cache on logout. No private write storage.

## 2. Goals

- Web app manifest, icons, theme colors (from design tokens).
- SW caches the static shell and visited GET pages safely.
- Offline fallback for uncached navigations (RFC-008 states).
- Cache cleared/locked on logout/session expiry; cache-busted on deploy.

## 3. Non-Goals

- No native app, no push notifications (MVP), no background write-sync, no caching of private API responses outside the controlled page cache.

## 4. External Behavior

Installable to home screen where supported; opens to last route or Home; shows the offline banner/fallback from RFC-008. The install prompt is not pushy.

## 5. Internal Design

- **Manifest**: name `ciao.zinnias`, `display: standalone`, start URL `/`, token-driven theme/background, design-team icons.
- **SW caching**: static shell assets via cache-first with versioned cache names; visited authorized GET pages via network-first (falling back to cache when offline); an offline fallback document for uncached navigations. Never cache POST responses or anything carrying secrets.
- **Lifecycle**: on `activate`, delete old cache versions (deploy cache-bust). On logout/expiry, the page triggers a cache purge of private entries.
- The SW is the *only* client JS required for MVP; it does not hydrate or mutate.

## 6. Data and API Design

No backend changes. Static assets are served outside the Worker CPU budget (AD-3).

## 7. Security, Privacy, and Safety

- SW never caches cookies/secrets; private page cache scoped to the session and purged on logout.
- A deploy must not strand stale private pages; versioned caches handle this.
- Install prompt and offline behavior must not imply data is saved when it is not (RFC-008).

## 8. Acceptance Criteria

- Manifest validates; app installs.
- Shell loads offline after first visit; visited pages open offline; uncached pages show the fallback.
- No private API/POST data in the SW cache.
- Logout purges private cached pages; deploy cleans old caches.

## 9. Test Plan

- Manifest/PWA validation.
- Offline first-load vs second-load tests.
- SW cache inspection (no secrets/POSTs).
- Logout purge + deploy cache-bust tests.

## 10. Open Questions / Decisions

Decision: the SW is read-only. Push and background sync are out of MVP (RFC-021/008 futures).
