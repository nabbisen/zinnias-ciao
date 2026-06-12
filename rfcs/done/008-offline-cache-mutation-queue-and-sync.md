# RFC 008 — Offline Read Behavior and Honest State

**Status.** Implemented (v0.4.0)
**Phase:** M3 / Read-only Offline + PWA
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** AD-1 — offline is **read-only**. The client mutation queue, IndexedDB write store, and optimistic sync states of the original RFC-008 are removed. (Title kept for traceability; scope is now read-only.)

---

## 1. Summary

Defines what the app does without a network. Under SSR with no client write path (AD-1), offline support is limited to viewing pages already fetched plus an honest "you are offline" state. There is no offline write queue. The seam to add background write-sync later (a "small end" option) is noted but explicitly out of MVP.

## 2. Goals

- Let a member re-open recently viewed Home/Event Detail pages while offline.
- Show an honest offline banner and a clear fallback when no cached page exists.
- Make it obvious that changes require a connection.

## 3. Non-Goals

- No offline status/note writes, no client mutation queue, no optimistic UI, no IndexedDB private store, no background sync in MVP.

## 4. External Behavior

| Situation | What the user sees |
|---|---|
| Offline, page was visited before | The cached page renders with an "Offline — showing last loaded" banner. |
| Offline, page not cached | "You are offline. Open again when connected." |
| Offline, tries to submit a form | The action does not pretend to succeed; the offline banner explains a connection is needed. |
| Back online | Banner clears on the next successful load. |

There is deliberately no "Saved on this phone / will sync" state, because nothing is queued.

## 5. Internal Design

Caching is handled entirely by the service worker (RFC-017): cache the static shell + GET responses for recently visited authorized pages, scoped so they are cleared on logout/expiry. No private mutation state on the client. Forms always target the server; offline submission simply fails to reach it and the SW serves the offline fallback for the navigation.

Future seam (out of scope): if write-sync is ever justified, it would be a new RFC introducing an explicit, server-revalidated queue — it must not be retrofitted silently, and would reuse the form-token idempotency contract (AD-4).

## 6. Data and API Design

No new endpoints, tables, or DTOs. SW cache versioning/cleanup is in RFC-017.

## 7. Security, Privacy, and Safety

- Cache only authorized GET responses for the current session; never session secrets.
- Clear/lock the private cache on logout/session expiration (RFC-017).
- After a shared-device logout, no prior user's cached pages remain readable.

## 8. Acceptance Criteria

- A previously viewed Home/Event Detail opens offline with the offline banner.
- An unvisited page shows the offline fallback, not an error trace.
- No write appears to succeed while offline.
- Logout clears private cached pages.

## 9. Test Plan

- Offline-after-visit render test; offline-unvisited fallback test.
- Offline form-submit shows the offline state (no false success).
- Shared-device logout cache-clear test.

## 10. Open Questions / Decisions

Decision: MVP offline is read-only. Write-sync is a deliberately deferred future RFC, not a silent enhancement.
