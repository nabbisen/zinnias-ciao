# RFC 026 — Multi-Language and Plain-Language Localization

> **Stub (backlog).** Direction only. Sections 7–13 are shared scaffolding to be detailed when this RFC is accepted; do not treat as finished design.

**Status.** Proposed  
**Phase:** F5 / Internationalization and Plain Language  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Relationship:** Continuation RFC; not required for the first MVP unless explicitly accepted

---

## 1. Summary

This RFC defines localization and plain-language strategy. The MVP may ship in one language, but the product should be prepared for communities whose members are more comfortable in another language.

---

## 2. Motivation

The first release of ciao.zinnias must stay small and trustworthy. However, implementation choices made during the MVP can either enable or block this future capability. This RFC records the desired future shape now so that the core design avoids dead ends without prematurely implementing the feature.

---

## 3. Goals

- Prepare UI text for localization.
- Keep terminology simple and culturally adaptable.
- Support longer translated labels without breaking layouts.
- Allow community-specific terminology only where safe.

---

## 4. Non-Goals

- No machine translation of user comments.
- No per-user mixed-language administration complexity in the first release.
- No user-generated HTML or markdown for localized copy.
- No locale-specific business rules unless separately designed.

---

## 5. External Behavior

| Scenario | Required behavior |
|---|---|
| Japanese deployment | Core UI strings can be translated cleanly. |
| Long text stress | Buttons and cards remain usable with longer translated labels. |
| Community term | A community may call events “Meetups” or “Practice” if configured later. |
| Fallback | Missing translation falls back predictably. |

The visible language must remain plain and calm. The user should understand what happened, what they can do next, and whether the action is optional.

---

## 6. Internal Design

Externalize all user-facing strings into message catalogs. Components must be designed with text expansion in mind. Status values should have stable internal codes and localized labels. Avoid embedding English strings in API error payloads; send stable error codes plus frontend-localized messages. For community-specific wording, store controlled label overrides such as `event_label_singular`, not arbitrary UI templates.

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

- Translations must preserve safety meaning, especially errors and destructive confirmations.
- Do not localize internal enum values.
- Text truncation must not hide important warnings.
- Accessibility labels must be localized alongside visible text.

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
