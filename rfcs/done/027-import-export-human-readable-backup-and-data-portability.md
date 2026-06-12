# RFC 027 — Import, Export, Human-Readable Backup, and Data Portability

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Implemented (v0.15.0)
**Phase:** F6 / Data Stewardship and Operations  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines data export and portability. Even a small community service should let administrators preserve essential records in a comprehensible form.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Allow admins to export community data in human-readable formats.
- Separate operational backup from user-facing export.
- Avoid exporting session secrets, invite secrets, or private technical logs.
- Make export behavior predictable and auditable.

---

## 4. Non-Goals

- No full analytics dashboard.
- No bulk import from arbitrary calendar systems in this RFC.
- No export of hidden/deleted content except in admin/legal archive mode.
- No self-service member export beyond membership-scoped data unless designed separately.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Community export | Admin downloads events, attendance, and notes for their community. |
| Member-safe export | Export labels former/deleted members appropriately. |
| Audit record | Export action is logged. |
| Redacted export | Sensitive operational details are omitted. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Create an export service that reads canonical data and emits structured JSON plus optional CSV summaries. The export should include community metadata, active members, events, participation statuses, visible notes, cancellation state, and timestamps. It must exclude session tokens, invite code hashes, internal mutation IDs unless needed for support, and raw logs. Exports should be generated on demand with authorization and rate limits.

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

- Exports contain personal data and must be treated as sensitive downloads.
- Only admins should export whole-community data.
- Export links should expire quickly if asynchronous export is introduced.
- Redaction rules must be tested.

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
