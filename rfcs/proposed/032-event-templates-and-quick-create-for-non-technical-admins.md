# RFC 032 — Event Templates and Quick Create for Non-Technical Admins

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Proposed  
**Phase:** F2 / Scheduling Power Carefully  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines event templates and quick-create patterns. Many communities repeat similar event types even before full recurrence is justified.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Reduce repeated admin typing.
- Keep one-off event creation simple.
- Avoid recurrence complexity when a template is enough.
- Support safe defaults and validation.

---

## 4. Non-Goals

- No full recurring event engine in this RFC.
- No complex template marketplace.
- No user-specific template scripting.
- No auto-generated events without admin confirmation.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Save as template | Admin saves title/location/duration as a template. |
| Quick create | Admin picks template and chooses date/time. |
| Edit before publish | Admin reviews details before event is visible. |
| Delete template | Admin removes obsolete template. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Add `event_templates` scoped to community. A template stores title, optional location, optional description, default duration, and active flag. Creating an event from a template copies fields into a normal event; later template edits do not mutate existing events. Templates are admin-only and should be listed in the create-event screen after the standard blank form option.

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

- Templates must not create hidden recurring commitments.
- Template names and descriptions are community-private.
- Validation must run on the final event, not only the template.
- Admin preview must make publish/cancel distinction clear.

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
