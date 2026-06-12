# RFC 050 — Staging Runtime Verification Evidence Pack

**Status.** Proposed
**Phase:** F8 / Pre-pilot hardening
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Completes RFC-045 §6 (staging-runtime verification). Supersedes the "staging pending" items in the release checklist. Blocks production-pilot approval.

## 1. Summary

RFC-045 discharged the source-verification half of the architect's checklist. This RFC is the staging-runtime half: deploy to Cloudflare staging and collect evidence artifacts that the handoff claims are true in a live environment.

## 2. Deliverables

- **Route/header matrix:** curl or browser-DevTools screenshots showing `Cache-Control: no-store`, CSP, and other security headers on Home, Event Detail, Join, and export.
- **Asia/Tokyo timezone round-trip:** create event at 09:00 JST, view detail, edit to 13:00, confirm display, download ICS, check DTSTART/DTEND.
- **Concurrent invite redemption:** two simultaneous POSTs against the same invite code; confirm exactly one membership created.
- **Concurrent form-token double-submit:** two POSTs with the same `SET_STATUS` or `SAVE_NOTE` token; confirm exactly one mutation.
- **No-JS confirmations:** cancel event, remove member, delete note, admin hide note with browser scripting disabled.
- **200% text scaling:** screenshots on real mobile browser for join, home, event detail, note edit, admin create event, destructive confirmation.
- **Logpush/observability:** admin action appears in audit log; Logpush delivers to R2/S3.
- **Workers CPU:** no consistent Free-plan CPU limit errors on standard routes; if present, document plan to move to Standard/Paid.

## 3. Blocker

Requires Cloudflare staging deployment (`worker-build` + `wrangler deploy --env staging`, staging D1, KV, and `HMAC_PEPPER` secret set). Cannot be done in-repo.

## 4. Gate

Production pilot not approved until this RFC's deliverables are attached to the release checklist.
