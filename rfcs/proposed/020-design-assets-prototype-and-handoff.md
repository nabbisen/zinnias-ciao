# RFC 020 — Design Assets, Prototype, and Handoff

**Status.** Proposed  
**Phase:** M7 / Design Team Handoff  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Related roadmap milestone:** M7 / Design Team Handoff  

---

## 1. Summary

This RFC defines design-team deliverables and handoff acceptance. It ensures that visual design, accessibility, component implementation, and product scope stay aligned.

---

## 2. Goals

- Define required wireframes, mockups, prototype, tokens, and icons.
- Make design deliverables testable by developers and QA.
- Prevent visual design from reintroducing complexity.
- Ensure admin and offline states are designed, not improvised.

---

## 3. Non-Goals

- No final marketing site design.
- No native app store artwork.
- No post-MVP feature mockups as implementation commitments.
- No design-only change that contradicts requirements/RFCs.

---

## 4. External Behavior

Design deliverables must cover member, admin, empty, error, offline, and accessibility stress states.

Design team must not provide only happy-path screens. Non-technical users need safe failure and confirmation states.

---

## 5. Internal Design

Required files:

```text
design/
  wireframes/
  mockups/
  prototype-link.md
  tokens/ciao-zinnias.tokens.json
  icons/*.svg
  copy/ux-copy.md
```

Design tokens should map to frontend constants/CSS variables. Icon filenames should be stable and semantic.

Prototype paths:

- Join -> Display Name -> Home;
- Home -> Event Detail -> Set Status;
- Event Detail -> Save Note;
- Offline -> Queued -> Synced;
- Admin -> Create Event;
- Admin -> Generate Invite;
- Admin -> Cancel Event;
- Admin -> Remove Member.

---

## 6. Data and API Design

No backend API changes required. However, design must align with API capability flags and error states.

Design QA should include a traceability table:

| Screen/component | Requirement/RFC | Mockup link | Implementation component |
|---|---|---|---|

---

## 7. Security, Privacy, and Safety

- Designs must not show plaintext used invite codes to members.
- Destructive admin actions require confirmation.
- Status must include icon + label + color.
- Offline and sync failure states must be visible and understandable.
- UX copy must not expose private resource existence.

---

## 8. Acceptance Criteria

- Wireframes cover all required screens.
- High-fidelity mockups cover normal, large text, offline, and error states.
- Interactive prototype covers core member and admin flows.
- Token JSON is delivered.
- SVG icon set is delivered.
- UX copy sheet is delivered.

---

## 9. Test Plan

- Design review against ROADMAP checklist.
- Developer review for implementability.
- Accessibility review for touch target, contrast, color-independent meaning.
- QA review of error/empty/offline states.
- Product owner review for scope boundaries.

---

## 10. Open Questions / Decisions

Decision: design handoff is part of MVP readiness, not optional polish. Missing admin/offline/error designs block release candidate.
