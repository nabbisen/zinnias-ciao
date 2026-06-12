# RFC 043 — Pilot UX Acceptance and Error Feedback

**Status.** Proposed
**Phase:** F7 / Stabilization (architect deep-review remediation)
**Project:** ciao.zinnias
**Date:** June 12, 2026
**Relationship:** Stabilization RFC. Closes deep-review finding P1-3 (partially shipped in v0.23.0) and consolidates the remaining pilot-UX acceptance gates (deep review §6.2, §9). Refines RFC-011 (accessibility) and RFC-030 (onboarding/empty states).

> **Proposed.** Error-banner rendering (P1-3) shipped in v0.23.0; the remaining
> items — no-JS destructive confirmations, device QA at 200% scaling, and a
> standardized flash/err convention — are not yet complete. This RFC tracks the
> whole acceptance surface so the pilot has a single checklist.

---

## 1. Summary

This RFC defines the user-experience acceptance bar for a pilot with
non-technical users: every action gives visible feedback, no control silently
fails, destructive actions are protected even without JavaScript, and the core
flows survive 200% text scaling on a real phone. Part of it (visible error
banners for create-event and event-detail) shipped in v0.23.0; the rest is
proposed work.

---

## 2. Motivation

Deep review §6.2 and §9 stress that for IT-averse users, silent failure is
worse than a clear error, and that controls relying on JavaScript (`confirm`,
`onchange`) must degrade safely. Three concrete gaps remain after v0.23.0:

1. **Inconsistent flash/err handling.** v0.23.0 fixed the invisible `?err=`
   redirects on create-event and event-detail, but the project still lacks a
   single convention for success (`flash`) vs error (`err`) rendering across
   all routes.
2. **JS-dependent destructive confirmations.** Some destructive actions (e.g.
   cancel event, remove member, delete note) rely on a JavaScript `confirm()`
   dialog. Without JS, the action either proceeds unguarded or the affordance is
   unclear.
3. **Unverified large-text / real-device behavior.** The design mandates 200%
   text scaling and 44px targets (RFC-011), but this has not been verified on a
   real phone for the core flows.

---

## 3. Goals

- A single, documented convention: `flash=…` for success banners, `err=…` for
  error banners, both rendered as visible, `role`-appropriate banners with
  plain-language copy, extracted uniformly at every route entry point.
- No-JS-safe confirmation for every destructive/access-changing action
  (route-backed confirmation step, not a JS `confirm`).
- A device-QA pass at 200% scaling on a real phone for the core flows, recorded
  as an acceptance artifact.
- No control that fails to persist is ever shown (carrying forward RFC-040's
  principle).

---

## 4. Non-Goals

- No visual redesign; this is about feedback and degradation, not aesthetics.
- No new toast/animation framework; banners are server-rendered HTML.
- No client-side form validation requirement (server validation remains
  authoritative; client hints are optional enhancement).

---

## 5. External Behavior

| Action | Required feedback / protection |
|---|---|
| Any successful mutation | Visible success banner (`flash`), plain language. |
| Any rejected mutation (validation, permission, stale state) | Visible error banner (`err`), `role="alert"`, plain language, no technical terms. |
| Cancel event | Route-backed confirmation page/step that works without JS. |
| Remove member | Route-backed confirmation; consequence stated; last-admin block enforced. |
| Delete note (own or admin) | Route-backed confirmation. |
| Any screen at 200% text | Core flow remains operable; lists/cards reflow; nav labels visible. |

---

## 6. Internal Design (proposed)

### 6.1 Flash/err convention

Introduce a small shared helper (e.g. `render::banner(flash, err)`) that every
page calls, and ensure every route's GET handler extracts both query params and
passes them through. Audit all redirects to use `flash=` / `err=` consistently.
(v0.23.0 already did this for create-event and event-detail; this generalizes
it.)

### 6.2 No-JS destructive confirmations

Replace JS `confirm()`-gated POSTs with a two-step, route-backed pattern:
`GET …/confirm` renders a confirmation page with an explicit form, `POST`
performs the action. Each confirmation form carries its own purpose-bound form
token (AD-4). This is the same pattern already used elsewhere; the work is to
apply it everywhere a destructive action currently leans on JS.

### 6.3 Device QA

A scripted manual pass (checklist in the launch runbook) on at least one real
iOS and one real Android phone at 200% system text, covering: join, set status,
save note, offline status, admin create event, generate invite, remove member.

---

## 7. Data Model Notes

None. Confirmation steps reuse existing tables and the form-token table.

---

## 8. API and UI Contract Notes

- Adds `GET …/confirm` companion routes for destructive actions (exact paths to
  be specified during implementation).
- Banner rendering becomes a shared contract used by all pages.

---

## 9. Security, Privacy, and Safety

- No-JS confirmations protect non-technical admins from accidental destructive
  actions even with scripting disabled or broken.
- Each confirmation POST remains CSRF-protected by a single-use form token.
- Error banners must continue to use generic, non-leaking copy (no resource
  existence disclosure, no stack/SQL detail).

---

## 10. Acceptance Criteria

1. Every state-changing route renders a visible success or error banner; none
   fail silently. (Pre-pilot gate #9.)
2. Cancel event, remove member, and delete note each have a no-JS confirmation
   step.
3. Core flows pass manual QA at 200% scaling on a real phone. (Pre-pilot
   gate #10.)
4. No non-persisting control is shown (re-verifies RFC-040).

Item 1 is partially met (create-event, event-detail shipped in v0.23.0);
items 2–4 are open.

---

## 11. Test Plan

- **Manual device QA** as in §6.3, recorded as an acceptance artifact.
- **Route audit** (could be a simple script per RFC-044): every redirect uses
  `flash`/`err`; every destructive POST has a `…/confirm` GET companion.
- Existing unit tests unaffected.

---

## 12. Rollout Plan

Implement after the v0.23.0 P0/P1 fixes are validated. Banner generalization and
no-JS confirmations can ship in a single patch release; device QA gates the
pilot go/no-go.

---

## 13. Open Decisions

- Exact `…/confirm` route naming and whether to use a shared confirmation
  template vs per-action pages.
- Whether to add lightweight client-side enhancement (disable submit after first
  click) as defense-in-depth atop the server-side idempotency (RFC-037).
- Minimum device matrix for the QA gate (which OS versions / screen sizes).
