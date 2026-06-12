# RFC 015 — Testing, QA, and Release Gates

**Status.** Implemented (v0.5.0)
**Phase:** M5 / UX and Release Hardening  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Reconciled:** drops the offline-write-queue gate; adds per-day status and read-only-offline tests (AD-1).
**Related roadmap milestone:** M5 / UX and Release Hardening  

---

## 1. Summary

This RFC defines the required test strategy and release gates for MVP. The product is simple, but community isolation, session safety, offline sync, and accessibility require deliberate tests.

---

## 2. Goals

- Define automated tests for core business logic.
- Define integration tests for trust boundaries.
- Define UI and accessibility tests.
- Define manual non-technical user acceptance checks.
- Define release-blocking criteria.

---

## 3. Non-Goals

- No full formal verification requirement.
- No exhaustive browser/device lab for MVP.
- No performance testing beyond MVP targets.
- No enterprise compliance certification.

---

## 4. External Behavior

Release readiness should be evaluated against user flows:

- join community;
- set display name;
- view Home;
- open event;
- set status;
- save note;
- recover from offline;
- admin create/cancel event;
- admin invite/remove member.

---

## 5. Internal Design

Test layers:

1. Unit tests: status transitions, invite validation, note length, authorization helpers.
2. Integration tests: API + D1 behavior.
3. UI component tests: rendering, disabled states, error states.
4. End-to-end smoke tests: core flows.
5. Security tests: XSS/CSRF/cross-community.
6. Accessibility tests: labels, contrast, touch targets, large text manual review.
7. Offline tests: cached-page read offline; form submit shows offline state (no false success); form-token replay idempotency.

CI gates:

- format;
- lint/clippy equivalent;
- unit tests;
- migration tests;
- security dependency audit where practical;
- frontend build;
- backend build.

---

## 6. Data and API Design

Required release gate matrix:

| Gate | Required for MVP |
|---|---|
| Invite onboarding | Yes |
| Session/logout | Yes |
| Cross-community denial | Yes |
| Member status/note | Yes |
| Admin create/cancel/invite/remove | Yes |
| Read-only offline (cached page opens; no false write success) | Yes |
| XSS/CSRF tests | Yes |
| A11y manual review | Yes |
| Performance smoke | Yes |

---

## 7. Security, Privacy, and Safety

Security/privacy tests are release-blocking. A UI that appears correct but leaks another community's data is a failed release.

Test fixtures must not use real member data, real invite codes, or production secrets.

---

## 8. Acceptance Criteria

- CI runs core test suite.
- Release checklist exists and is filled for release candidate.
- Non-technical user can join and mark attendance within 2 minutes in usability trial.
- No critical security bug remains open.
- Per-day status transitions, form-token idempotency, and read-only offline behavior are tested.

---

## 9. Test Plan

- Automated tests described above.
- Manual QA script for phone viewport.
- Manual offline airplane-mode test (read-only: cached page opens; submitting shows the offline state).
- Manual screen reader/large text smoke test.
- Manual admin workflow test.

---

## 10. Open Questions / Decisions

Decision: code coverage target should emphasize core business/security logic. A raw percentage alone must not override missing critical-path tests.
