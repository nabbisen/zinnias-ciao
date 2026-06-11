# RFC 034 — Notification-Free Quiet Mode and Attention Design

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Proposed  
**Phase:** F1 / Communication Without Noise  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines a quiet-mode and attention design philosophy. The service should help community coordination without becoming another demanding app.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Support a low-noise default experience.
- Provide quiet hours or quiet mode if notifications are added.
- Avoid red badges and urgency patterns except for true problems.
- Make attention cues meaningful and sparse.

---

## 4. Non-Goals

- No streaks, gamification, or engagement metrics.
- No infinite feed.
- No aggressive unread counters.
- No pressure tactics such as “everyone is waiting for you”.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Quiet default | Member sees schedule when opening app, not constant prompts. |
| Quiet hours | Member suppresses reminders during configured hours. |
| Important change | Cancellation or time change still gets clear treatment. |
| Admin view | Admin sees who has not answered without shaming language. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Attention states should be modeled semantically: `needs_answer`, `event_changed`, `sync_failed`, `admin_action_needed`. UI badges should use calm labels such as “Please answer” or “Changed” rather than alarming counters. If notification preferences exist, quiet mode should suppress reminders except critical event changes if the user opted into those.

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

- Do not expose private pressure tactics.
- Do not rank people publicly by responsiveness.
- Quiet mode must not hide sync failure for the current user.
- Accessibility users should not rely on color-only attention cues.

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
