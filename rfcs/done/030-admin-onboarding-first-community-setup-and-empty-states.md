# RFC 030 — Admin Onboarding, First Community Setup, and Empty States

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Implemented (v0.14.0)
**Phase:** F8 / First-Run Admin Success  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines first-run admin experience. The best backend is useless if a non-technical admin cannot create the first community, first event, and first invite without help.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Guide admins through first setup with minimal steps.
- Make empty states actionable.
- Prevent accidental misconfiguration.
- Reduce support burden during pilots.

---

## 4. Non-Goals

- No complex tutorial system.
- No gamification.
- No mandatory video walkthrough.
- No role hierarchy beyond existing admin/member model.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| First community | Admin names the community and sees a clear next action. |
| First event | Admin creates an event using a simple form. |
| First invite | Admin generates one code and receives sharing guidance. |
| No events | Home shows “No events yet” with admin create action. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Add an onboarding state derived from actual data, not a separate fragile progress flag. If a community has no events, show event creation prompt to admins and a plain “No events yet” message to members. If there are no members besides admin, show invite generation prompt. Admin forms should use safe defaults: community timezone, one-off event, no recurring, clear title/time/location validation.

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

- Invite-sharing guidance must not expose admin-only links to members.
- Empty states must not reveal hidden communities.
- Do not pressure admins to invite many people before testing.
- All setup actions must be auditable.

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
