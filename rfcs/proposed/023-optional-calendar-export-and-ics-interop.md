# RFC 023 — Optional Calendar Export and ICS Interop

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design. A basic ICS download route already exists in the codebase; this RFC formalizes the per-membership hashed-token feed.

**Status.** Proposed  
**Phase:** F2 / Scheduling Power Carefully  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines optional calendar export. Some members may want to see community events in their phone calendar, but ciao.zinnias should not require Google, Apple, or Outlook accounts and should not become dependent on external calendar platforms.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Provide privacy-safe read-only event export where appropriate.
- Keep export optional per member and per community policy.
- Avoid writing into external calendars in the first release.
- Define revocation behavior for exported feeds.

---

## 4. Non-Goals

- No two-way sync.
- No OAuth integration with Google or Microsoft in this RFC.
- No exposing comments or participant lists in calendar feeds by default.
- No public unguessable URL treated as full authorization for sensitive data without revocation.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Subscribe to feed | Member copies/subscribes to a personal feed URL. |
| Revoke feed | Member regenerates or disables the feed URL. |
| Admin disable | Admin disables calendar export for a community. |
| Minimal event data | Calendar entry shows title, time, and location only. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Use per-membership export tokens stored as hashed secrets. A feed request authenticates by token and returns only events visible to that membership. The feed must not include comments, participant names, invite codes, or admin-only metadata. Feed tokens should be revocable and rotatable. Generated calendar output should be derived from canonical event data; the feed itself is not a storage authority.

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

- Export URLs are bearer secrets and must be displayed with clear privacy wording.
- Revocation must immediately invalidate old feed tokens.
- Event cancellation should be visible as cancellation or title prefix depending on supported client behavior.
- Feeds must be rate-limited.

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
