# RFC 021 — Post-MVP Notification Strategy and Reminder Digests

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Proposed  
**Phase:** F1 / Communication Without Noise  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines a cautious notification strategy for reminders and summaries. ciao.zinnias must not become noisy, addictive, or chat-like. Notifications should help members remember events and notice meaningful changes, while preserving the quiet notice-board character of the product.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Add opt-in reminder and digest capabilities after MVP.
- Avoid real-time chat semantics and noisy interruption patterns.
- Define notification preference UX for non-technical users.
- Keep notification content privacy-safe.
- Make delivery failure non-destructive.

---

## 4. Non-Goals

- No push notifications in MVP unless explicitly re-scoped.
- No typing indicators, live chat, or social feeds.
- No behavioral analytics-based notification ranking.
- No cross-community notification aggregation that leaks private context.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Event reminder | A member receives a simple reminder before an event they can access, if they opted in. |
| Event changed | A member is told that an event time/location was changed or cancelled. |
| Digest | A member receives a periodic “This week” summary, not instant updates for every small edit. |
| Admin reminder | Admins can see unsent/failed reminder status, but not private device internals. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Notification jobs should be modeled as derived work items, not as the source of truth. Store notification preferences per membership, not globally, because community expectations differ. Use a `notification_preferences` table and a `notification_deliveries` table with minimal metadata. Notification payloads should contain only the event title, time, and a generic call to open the app; avoid full comment text. Delivery must be idempotent by `(user_id, event_id, notification_kind, scheduled_for)`.

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

- Users must explicitly opt in.
- Notification text must not reveal private community data on shared lock screens beyond the minimum useful content.
- Admins must not be able to force personal push notifications without consent.
- Failure to send a reminder must never alter attendance status.

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
