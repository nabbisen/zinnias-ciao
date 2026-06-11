# RFC 011 — Accessibility and Design System

**Status.** Proposed  
**Phase:** M5 / UX and Release Hardening  
**Project:** ciao.zinnias  
**Date:** June 11, 2026  
**Related roadmap milestone:** M5 / UX and Release Hardening  

---

## 1. Summary

This RFC defines the minimum design system and accessibility rules. Accessibility is not a later polish task; it is required because the target users include non-technical people using mobile devices in ordinary environments.

---

## 2. Goals

- Define semantic design tokens.
- Ensure status is conveyed by color + icon + label.
- Enforce touch target and spacing rules.
- Support 200% font scaling.
- Support reduced motion.
- Provide reusable accessible components.

---

## 3. Non-Goals

- No full multi-theme system in MVP.
- No custom animation-heavy design language.
- No color-only status UI.
- No icon-only critical actions.

---

## 4. External Behavior

Core components:

- `PrimaryButton`;
- `DangerButton`;
- `StatusButtonGroup`;
- `EventCard`;
- `ParticipantRow`;
- `NoteEditor`;
- `BottomNav`;
- `Banner`;
- `ConfirmDialog`.

Status examples:

```text
Going: check icon + “Going” + blue token
No Go: cross icon + “No Go” + red token
Attended: badge/flag icon + “Attended” + green token
No answer: dash/question icon + “No answer” + grey token
```

---

## 5. Internal Design

Design tokens should be represented as CSS variables and optionally generated from design-token JSON.

Example:

```css
--cz-color-status-going: #007AFF;
--cz-color-status-not-going: #FF3B30;
--cz-color-status-attended: #34C759;
--cz-color-status-unknown: #8E8E93;
--cz-touch-target-min: 44px;
```

Reduced motion:

- disable bounce/scale animations;
- replace slide-heavy transitions with instant or fade transitions;
- do not rely on blur for comprehension.

---

## 6. Data and API Design

No backend API required.

Frontend component APIs should make unsafe usage difficult:

```rust
StatusChip { status, label_required: true }
IconButton { aria_label: NonEmptyString }
ConfirmDialog { title, body, confirm_label, cancel_label }
```

---

## 7. Security, Privacy, and Safety

Accessibility is a safety issue: users must not accidentally cancel events, remove members, or publish notes because controls are small, ambiguous, or color-only.

All user-generated content must be displayed in text elements that escape content safely.

---

## 8. Acceptance Criteria

- All important controls are at least 44 x 44 px.
- Status remains understandable in grayscale.
- Event Detail works at 200% text scaling.
- Reduced-motion setting is respected.
- Critical icon buttons have accessible names.
- Focus ring is visible for keyboard users.

---

## 9. Test Plan

- Automated accessibility checks.
- Manual large-text phone test.
- Manual color-blind/grayscale review.
- Keyboard navigation smoke test.
- Screen reader label smoke test.

---

## 10. Open Questions / Decisions

Decision: design system remains small. Add components only when repeated across at least two flows or required for safety.
