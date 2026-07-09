# Extended Design Team Deliverables Checklist

**Project:** ciao.zinnias  
**Scope:** Core MVP plus continuation RFCs  
**Use:** Handoff checklist for design, implementation, QA, and product review

---

## 1. Core MVP Screens

The design team must deliver mobile-first wireframes and high-fidelity mockups for:

- invite-code entry;
- first successful join;
- Home / Today / Upcoming event card list;
- community switcher or selected-community state;
- Event Detail route-backed sheet/page;
- status button group;
- participant list grouped by status;
- note editor with explicit save;
- Communities screen;
- Me screen;
- admin create event;
- admin edit/cancel event;
- admin generate invite code;
- admin member list;
- admin remove member confirmation;
- admin attendance correction after event;
- admin delete/hide note confirmation.

---

## 2. Required UI States

For every core screen and reusable component, provide:

- default state;
- loading skeleton;
- empty state;
- offline state;
- local-only/queued state;
- syncing state;
- sync failed state;
- rejected mutation state;
- expired session state;
- permission denied state;
- destructive-action confirmation;
- success confirmation;
- large text / 200% font state;
- reduced-motion behavior;
- color-blind-safe status representation.

---

## 3. Design Tokens

Deliver design tokens as JSON or equivalent source-of-truth file:

- colors;
- typography scale;
- spacing;
- radii;
- shadows/elevation;
- touch target sizes;
- focus ring;
- status icons and semantic labels;
- reduced-motion timings;
- breakpoints for mobile and narrow desktop.

---

## 4. Prototype Requirements

The click-through prototype must demonstrate:

1. join with invite code;
2. view upcoming event;
3. update status;
4. save note;
5. experience offline queued state;
6. sync success;
7. admin create event;
8. admin generate invite code;
9. admin cancel event;
10. logout/session expired recovery message.

---

## 5. Continuation RFC-Specific Deliverables

If future RFCs are accepted, add the following designs.

### Notifications / Reminders

- opt-in screen;
- reminder preference screen;
- quiet-mode screen;
- notification permission denied state;
- reminder delivery failure state.

### Recurring Events / Templates

- recurring event create form;
- edit one occurrence vs future occurrences vs full series confirmation;
- cancelled occurrence state;
- template selection and preview.

### Calendar Export

- export enable/disable screen;
- feed URL copy screen;
- revoke/regenerate confirmation;
- privacy warning copy.

### Account Recovery / Relinking

- admin relink initiation;
- member relink-code entry;
- relink success/failure;
- old session revoked message.

### Moderation and Support

- hidden note state;
- removed member state;
- admin moderation history;
- support code panel;
- diagnostic summary screen with no private content leakage.

### Localization

- string table review;
- long-label stress mockups;
- right-to-left feasibility note if future target locales require it;
- localized accessibility labels.

### Export / Backup / Launch

- export confirmation;
- export generated state;
- sensitive download warning;
- launch checklist dashboard or document view if implemented.

---

## 6. Handoff Quality Gate

Design handoff is complete only when:

- every component maps to an implementation component name;
- every screen maps to a route or modal/sheet state;
- every user-facing error has approved copy;
- every color-coded meaning also has icon/text representation;
- admin-only actions are visually separated from member actions;
- destructive actions are confirmation-protected;
- large text and reduced motion have been reviewed;
- implementation can trace requirements to designs and RFCs.
