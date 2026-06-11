# RFC 033 — Subgroups, Event Visibility, and Boundary Safety

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Proposed  
**Phase:** F7 / Boundary-Safe Growth  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines optional subgroups and event visibility. It is useful when one community contains teams or committees, but it can undermine the simple community boundary if done carelessly.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Allow events visible only to selected subgroups if strongly needed.
- Preserve default community-wide visibility.
- Make visibility clear to admins before publishing.
- Ensure non-members cannot infer hidden events.

---

## 4. Non-Goals

- No arbitrary per-field permissions.
- No nested organization hierarchy.
- No public/private hybrid events in the first version.
- No visibility based on social graph or algorithmic grouping.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Create subgroup | Admin creates “Choir”, “Cleanup Team”, or similar. |
| Assign member | Admin adds members to subgroup. |
| Subgroup event | Admin creates event visible only to subgroup. |
| Member view | Member sees only events they can access. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Introduce `subgroups`, `subgroup_memberships`, and optional `event_visibility_scope`. The default scope is `community`. A subgroup event must still belong to a community and authorization must check both community membership and subgroup membership. Event detail, sync, export, calendar feed, and audit views must all respect visibility. Admin UI should show a prominent visibility label during create/edit.

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

- Hidden events must not appear in counts for unauthorized members.
- Export and notifications must not leak subgroup-only events.
- Admin mistakes are likely; confirmation must show who can see the event.
- Subgroup removal must define historical visibility behavior.

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
