# RFC 024 — Display Name Recovery and Admin-Mediated Account Relinking

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design. Kept as the invite-era lost-session recovery path; largely self-healing once OIDC (AD-2) lands, so revisit scope then.

**Status.** Proposed  
**Phase:** F3 / Identity and Recovery  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines a conservative recovery and relinking model. The MVP intentionally avoids email/password login, but users can lose browser sessions. Recovery must be possible without pretending the system can magically identify people.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Allow admins to relink a returning person to a previous membership when appropriate.
- Avoid self-service recovery that enables impersonation.
- Preserve privacy and auditability.
- Keep non-technical UX plain and honest.

---

## 4. Non-Goals

- No email/password accounts.
- No biometric identity requirement.
- No automatic merge based on display name alone.
- No recovery without admin confirmation unless a later strong identity RFC is accepted.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Lost phone/session | Member asks admin for help. |
| Admin relink | Admin selects old inactive membership and issues a relink code. |
| Member enters relink code | Member regains the previous membership identity on the new browser. |
| Audit trail | Relink action is recorded. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Add `membership_relink_codes` with one-time hashed codes, expiry, target membership, created_by_admin, used_at, and audit reference. Relinking should attach a new session/browser identity to the existing membership after admin confirmation. Old sessions may be revoked. The UI must clearly distinguish “join as new member” from “restore existing member”.

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

- Relink codes must be short-lived and single-use.
- Admins must see enough information to avoid mistakes, such as display name and recent event participation, but not private technical identifiers.
- Relinking must be auditable and reversible by support/admin policy.
- Do not infer identity from names alone.

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
