# Post-MVP Scope Guardrails

**Project:** ciao.zinnias  
**Purpose:** Prevent future RFCs from accidentally bloating the MVP

---

## 1. Core Rule

The MVP is a private community schedule notice board. Any feature that turns it into a social network, chat service, enterprise calendar suite, analytics dashboard, or CRM must be rejected or deferred.

---

## 2. Do Not Pull Into MVP

Do not add the following before the MVP release gates pass:

- push notifications;
- recurring events;
- external calendar OAuth integration;
- email/SMS contact channels;
- subgroups;
- account recovery beyond simple new invite flow;
- analytics dashboards;
- member-to-member direct messaging;
- image/file uploads;
- complex role hierarchy;
- payments;
- public event discovery;
- AI summarization or recommendation.

---

## 3. Accept Future Work Only When

A future RFC may be accepted when:

- pilot use shows repeated need;
- the feature does not confuse the primary member flow;
- the security/privacy model is explicit;
- design team deliverables are complete;
- implementation has tests for community isolation;
- rollback/defer behavior is known.

---

## 4. Default Answer to Scope Pressure

When uncertain, prefer:

- one-off events over recurring events;
- explicit Save over magical auto-publish;
- simple list over dense calendar grid;
- admin-mediated recovery over automatic identity guessing;
- quiet digests over instant notifications;
- soft delete over hard delete;
- plain text over rich media;
- community boundary over convenience.
