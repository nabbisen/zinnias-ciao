# RFC 022 — Recurring Events and Event Series

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Implemented (v0.17.0)
**Phase:** F2 / Scheduling Power Carefully  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines post-MVP recurring events. Recurrence is useful for clubs and neighborhood groups, but it can easily create confusing edit and cancellation behavior. This feature must be added only after one-off events are stable.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Support simple recurrence patterns for admins.
- Preserve clear editing semantics: one occurrence, future occurrences, or full series.
- Represent generated occurrences without corrupting historical attendance.
- Keep member UI simple: members see concrete event instances, not abstract rules.

---

## 4. Non-Goals

- No complex calendar-rule editor in the first recurrence release.
- No importing arbitrary external RRULE complexity.
- No automatic recurrence repair after timezone policy changes without admin review.
- No recurring polls or availability voting.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Create weekly event | Admin creates “Every Saturday at 10:00” with a clear end condition. |
| Cancel one instance | Admin cancels only this week without deleting the series. |
| Edit future events | Admin changes location for future occurrences only. |
| Member view | Member sees each occurrence as a normal event card. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Introduce `event_series` and `event_occurrences`. A series stores the recurrence rule, owner community, admin-created template fields, timezone, and active/cancelled state. Each visible occurrence should have a stable ID once materialized. Attendance and notes must attach to occurrence IDs, never to the abstract series. Materialize a limited horizon, for example the next few months, and extend it with a scheduled job. Editing semantics require a split point when applying changes to future occurrences.

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

- A cancelled occurrence must preserve attendance history if the event had already happened.
- Members must not lose their notes because an admin edited the series title.
- Recurrence expansion must be deterministic in the community timezone.
- Recurring event UI must explain what will change before confirmation.

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
