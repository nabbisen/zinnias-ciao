# RFC 035 — Support, Diagnostics, and User Help Without Data Leakage

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Proposed  
**Phase:** F4 / Community Safety and Moderation  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines support and diagnostics that help solve user problems without leaking private community data or technical secrets.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Give users plain self-help for common issues.
- Allow admins/operators to diagnose session, sync, and invite problems safely.
- Generate support codes that reveal minimal context.
- Avoid exposing logs or internal IDs directly to normal users.

---

## 4. Non-Goals

- No live support chat built into the product.
- No remote control of user devices.
- No raw database viewer for admins.
- No logging of full notes for diagnostics.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Invite problem | User sees “Invalid or expired code” plus “Ask your admin for a new code”. |
| Sync problem | User can copy a short support code. |
| Admin diagnosis | Admin sees invite code status without seeing plaintext used codes. |
| Operator diagnosis | Operator traces request by safe correlation ID. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Add a support-info panel under Me with app version, sync state, last successful sync time, and a generated support reference. The support reference maps to server-side minimal diagnostic records with correlation IDs, not raw event/comment content. Admin diagnostic screens can show invite counts, expiry, and member state but should not expose session secrets or raw hashed values.

Implementation should be behind an explicit feature gate, migration step, or product decision. No future RFC may silently change MVP behavior.

---

## 7. Data Model Notes

Detailed schema should be finalized when the RFC is accepted, but the following principles apply:

- all records must be scoped to community, membership, or user as appropriate;
- foreign keys must preserve community boundaries;
- soft deletion should be preferred where history and audit matter;
- secret-like values must be hashed or stored through a platform secret mechanism;
- created/updated timestamps must be available for audit and sync reasoning.

---

## 8. API and UI Contract Notes

APIs must expose stable error codes and avoid English-only backend messages. UI must remain mobile-first, accessible, and compatible with large text. If the feature introduces a new user choice, the default must be conservative and privacy-preserving.

---

## 9. Security, Privacy, and Safety

- Support codes must expire or be harmless if shared.
- Diagnostics must not include note text by default.
- Admins should not see other communities.
- Help copy must be written for non-technical users.

---

## 10. Acceptance Criteria

- The feature can be enabled without changing unrelated MVP behavior.
- Community isolation tests cover the new data and API paths.
- The UI has empty, loading, failure, and permission-denied states.
- Admin-only actions are auditable.
- Documentation explains the feature in plain language.
- The feature can be disabled or deferred without breaking existing data.

---

## 11. Test Plan

- Unit tests for new domain rules.
- API authorization tests across at least two communities.
- UI state tests for mobile viewport and large text.
- Offline or retry tests where relevant.
- Manual usability check with non-technical wording.
- Security review of secrets, tokens, export data, or personal data if applicable.

---

## 12. Rollout Plan

1. Keep disabled in production until accepted.
2. Implement schema migrations in staging.
3. Add internal/admin-only preview if useful.
4. Run focused usability and security checks.
5. Enable for one pilot community.
6. Review support burden before broader rollout.

---

## 13. Open Decisions

- Whether this RFC is needed before the first public launch.
- Whether the feature should be community-configurable or globally enabled.
- What support documentation is needed for non-technical admins.
- Whether the feature introduces new privacy notice requirements.
