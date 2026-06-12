# RFC 055 — Offline Read-Only Contract

**Status.** Implemented (v0.31.0)
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Closes the ambiguity left open by RFC-042 (service worker). Addresses architect review v0.30.0 §5.2.

## 1. Summary

The service worker caches static assets only. Authenticated HTML is never cached. When offline, a user can view already-loaded pages from the browser's in-memory state, but no new data loads and no writes succeed.

This RFC documents that contract precisely and ensures the UI reflects it honestly.

## 2. Proposed contract

- **What works offline:** pages already rendered in the current browser session are readable until the tab is closed.
- **What does not work offline:** loading new pages, setting status, saving notes, admin actions — all require network.
- **User-visible behavior:** the offline banner appears. Status/note submit buttons are disabled or show "オフラインです。保存はできません。" on attempt.
- **No write queue:** this app does not queue offline writes. Users are expected to act when connected.

## 3. Small progressive-enhancement change needed

Disable or warn on submit buttons when the browser reports offline (navigator.onLine). Already possible with the existing minimal JS in app.js. Low-risk one-liner.

## 4. Implementation notes

The offline submit-disabling enhancement was added to  alongside the
existing  function. Status, note, and attendance submit
buttons are disabled when  is false, with tooltip
. The button state is restored on reconnect.
AD-1 is preserved: without JS, forms still work normally (the server returns
a network error on offline POST, which is acceptable for no-JS users).
