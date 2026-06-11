# RFC 028 — Backup, Restore, and Disaster Recovery Operations

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Proposed  
**Phase:** F6 / Data Stewardship and Operations  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines operational backup and restore requirements. It is separate from user-facing export. The goal is to recover service data after operator mistake, migration failure, or platform incident.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Define backup cadence and restore rehearsal.
- Protect backups as sensitive data.
- Make migrations reversible or recoverable.
- Document operator actions for incident response.

---

## 4. Non-Goals

- No enterprise-grade multi-region active-active architecture in this RFC.
- No user-facing backup download as a substitute for operator backup.
- No manual database edits as normal recovery workflow.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Pre-migration backup | Operator creates or verifies backup before applying migrations. |
| Restore rehearsal | Team periodically restores to staging. |
| Incident recovery | Operator follows a documented runbook. |
| Backup access control | Only authorized operators can access backups. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

The system should maintain a backup inventory with timestamp, environment, schema version, and retention class. Restore must be tested against staging before production use where practical. Schema migrations need preconditions and postconditions. If D1 export or platform backup mechanisms are used, wrap them in project runbooks rather than relying on ad hoc console actions.

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

- Backups include private community data and must be encrypted or protected by platform controls.
- Production restore can overwrite newer data; require explicit approval.
- Store no plaintext invite codes or session secrets in backups beyond hashed records.
- Document data-loss windows honestly.

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
