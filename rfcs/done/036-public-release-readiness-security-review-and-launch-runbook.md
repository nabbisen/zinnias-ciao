# RFC 036 — Public Release Readiness, Security Review, and Launch Runbook

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Implemented (v0.15.0)
**Phase:** F6 / Data Stewardship and Operations  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines the final release-readiness process before public or broader production use. It turns engineering completion into an explicit launch decision.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Define release candidate criteria.
- Require security, privacy, accessibility, and usability review.
- Define rollback and incident communication procedure.
- Prevent accidental launch without owner confirmation.

---

## 4. Non-Goals

- No marketing launch plan.
- No paid plan/billing scope.
- No automatic production promotion from CI without human approval.
- No relaxing MVP release gates for schedule pressure.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Release candidate | Team tags a candidate and deploys to staging. |
| Review checklist | Security/a11y/usability/ops checks are completed. |
| Go/no-go | Project owner approves production launch explicitly. |
| Rollback | Operator can revert to previous version and database state policy is known. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Create a launch runbook with environment versions, migration status, known limitations, rollback steps, monitoring checks, contact points, and go/no-go signoff. Release candidates should freeze schema migrations unless a blocker fix is required. Production launch must require explicit confirmation by the project owner. Post-launch, collect pilot feedback without enabling invasive analytics.

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

- Security review must include community isolation tests.
- Privacy review must check exports, logs, cache, and support diagnostics.
- Accessibility review must include large text and reduced motion.
- Rollback must not silently lose accepted user mutations without documented decision.

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
