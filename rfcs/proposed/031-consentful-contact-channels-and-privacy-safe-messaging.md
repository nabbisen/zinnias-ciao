# RFC 031 — Consentful Contact Channels and Privacy-Safe Messaging

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Proposed  
**Phase:** F1 / Communication Without Noise  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines optional contact channels such as email or SMS-like delivery, if the project later decides that browser-only PWA reminders are insufficient. This must be privacy-first and consent-based.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Support optional contact methods only with explicit consent.
- Keep contact data minimal and separable from membership display data.
- Allow users to remove contact methods.
- Avoid turning the product into messaging infrastructure.

---

## 4. Non-Goals

- No member-to-member direct messaging.
- No marketing emails.
- No importing phone address books.
- No contact method required for MVP use.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Add email | Member optionally adds an email for reminders. |
| Verify contact | System verifies ownership before use. |
| Remove contact | Member removes contact and reminders stop. |
| Admin view | Admin sees that reminders are enabled, not necessarily raw contact details unless policy allows. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Introduce `contact_methods` with user/membership scope, type, redacted display value, verification state, and consent timestamp. Store provider identifiers separately. Reminder delivery references contact method IDs, not raw addresses in business tables. Verification flows must be simple and must not block core app use.

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

- Contact data is personal data and must have stricter retention.
- Admins should not gain unnecessary access to member contact details.
- All outbound messages must include context and avoid sensitive note contents.
- Failed delivery should not leak whether a person belongs to a community.

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
