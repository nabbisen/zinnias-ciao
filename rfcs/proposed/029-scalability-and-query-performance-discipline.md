# RFC 029 — Scalability and Query Performance Discipline

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Proposed  
**Phase:** F6 / Data Stewardship and Operations  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines performance discipline for a small but durable service. The product should not over-engineer for massive scale, but it must avoid query patterns that fail as communities and events accumulate.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Define query shapes for Home, Event Detail, Admin lists, and sync.
- Require indexes for authorization and event-range queries.
- Prevent N+1 frontend/backend patterns.
- Establish performance budgets and measurement practice.

---

## 4. Non-Goals

- No distributed database redesign.
- No premature sharding.
- No real-time fanout architecture.
- No heavy analytics workloads on the operational database.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Home load | Load upcoming event cards with status summary in bounded queries. |
| Event detail | Fetch event, participants, and notes without per-member round trips. |
| Admin list | Paginate members/events. |
| Sync | Replay queued mutations without scanning whole tables. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Define repository-level query functions with explicit input scopes. Home queries should use `(community_id, start_time)` indexes and bounded date windows. Event detail should left-join memberships with participation rows to preserve no-answer users. Avoid dynamic ad hoc SQL in UI handlers. Add query-plan review to migrations. Use pagination for admin history and audit views. Cache derived counts only when measurement proves it is needed.

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

- All queries must derive access from session membership.
- Performance shortcuts must not bypass authorization.
- Do not log full query payloads containing notes.
- Slow query handling should fail safely with plain user text.

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
