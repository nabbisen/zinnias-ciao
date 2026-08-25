# ciao.zinnias RFC Index

Folder is the source of truth for state (see [RFC 000](./done/000-rfc-lifecycle-policy.md)). This index mirrors the folders.

Lifecycle: `proposed/` is design under review; `accepted/` is reviewed and
owner-approved for implementation; `done/` is shipped. Only Accepted RFCs
authorize implementation.

> **Hold amended 2026-07-29:** The 2026-07-14 feature freeze is **lifted**.
> Source remediation for the architecture-review blockers is complete (B2
> closed; B1, B3, and B5 fixed, reviewed, committed, and locally evidenced), so
> product-feature work has resumed. What remains outstanding is **hosted
> evidence**, which is required only before deploying: B1, B3, B4, and B5 stay
> **open**, and production, public-pilot, and first-real-community deployment
> stay **No-Go**. The RFC-050 hosted campaign and persistent-incident-sink
> provisioning are deferred until a pilot is scheduled — deferred, not
> cancelled. See
> [ROADMAP.md](../ROADMAP.md#architect-review-remediation-hold--amended-2026-07-29).

RFC numbers and the tables below are stable identifiers and lifecycle indexes,
not execution priority. The active order is maintained in
[ROADMAP.md](../ROADMAP.md#active-remediation-and-release-sequence): RFC-072,
RFC-073, and RFC-074 shipped in `0.60.0` and RFC-075 in `0.61.0` (tagged
2026-08-02, neither deployed). **RFC-080, RFC-081, and RFC-082 shipped in
`0.62.0`** (tagged 2026-08-15, not deployed) — the external-identity foundation
and membership suspension. **RFC-054 (Japanese UX copy review) is the
active theme** as of 2026-08-15, with **RFC-083** accepted 2026-08-16 and its Slices D1 and D2a **complete**
(`57ffc37`), and **RFC-084** accepted the same day for the account tier — the last
convertible localization work.
Stage 3 (choosing an identity provider) remains
deliberately outside RFC-080/081 and blocked on user research that does not
exist. The RFC-050 hosted
campaign and its sink remain deferred to the point a pilot is scheduled.

## Accepted — approved for implementation

| ID | Title | Accepted | Note |
|----|-------|----------|------|
| 050 | [Exact-Candidate Hosted Staging Evidence and Pilot Gate](./accepted/050-staging-runtime-verification-evidence-pack.md) | 2026-07-28 | B4 remediation revision architecture-reviewed and owner-accepted; authorizes local tooling implementation (RFC-050 Tooling Slices) only, not hosted execution. Slices 1–7 committed at `15b9409`/`c55787a`; Slices 8–9 remain; B4 still open because local tooling is not evidence until run against a frozen candidate. **IPv6 client support is not confirmed/implemented for this deployment** — see RFC-078 § Dated owner risk acceptances and this document's E4a |
| 054 | [Japanese UX Copy Review](./accepted/054-japanese-ux-copy-review.md) | 2026-08-15 | **The active theme after `0.62.0`.** Substantially revised on acceptance: the original 143-string inventory (v0.36.0, one `i18n.rs`) is superseded — there are now **319 `JA_*` constants across 13 modules** and 94 `Localized` pairs. Its original headline problem is already solved: セッション/トークン/同期 appear in **no** Japanese string today. The real work is the **180 constants never reviewed by anyone**, spread across *every* module — corrected 2026-08-15 from an earlier "54", which was a module-level estimate rather than a measurement, and which moved the slice boundary: **novelty no longer selects slice 1, consequence does**. Acceptance authorizes the review; each slice's edits need their own package. Slice 1 finding A1 shipped at `6ea3765`; B1–B4 plus copy-harmony items H1/H2 are packaged and awaiting authorization |
| 076 | [One-Time Invite Code Response Isolation](./accepted/076-one-time-invite-code-response-isolation.md) | 2026-07-17 | Local implementation reviewed/accepted/committed at `b72f22b`; corrected isolated automation and bounded human no-JS/network evidence were architecture-reviewed and owner-accepted, closing criterion 8 locally. RFC-050 hosted evidence remains required before public/production B1 closure |
| 078 | [Fail-Closed Strongly Consistent Abuse Controls](./accepted/078-fail-closed-strongly-consistent-abuse-controls.md) | 2026-07-23 | Corrected architecture accepted; implemented and committed at `c991b82` on 2026-07-28, including the required I-B1 concurrency-burst evidence. RFC-050 exact-candidate hosted evidence remains pending, so B3 and all staging/public/production holds remain open |
| 079 | [Atomic Required Audits and Recursive Metadata Redaction](./accepted/079-atomic-required-audits-and-recursive-redaction.md) | 2026-07-15 | Local Packages 0A–8 and the Class A telemetry correction reviewed/committed. Persistent delivery and exact-candidate hosted B5 evidence remain required |
| 083 | [Localization Slice D: Admin, Anonymous, and Unresolvable Surfaces](./accepted/083-localization-slice-d-admin-anonymous-and-unresolvable-surfaces.md) | 2026-08-16 | Discharges the Slice D that **RFC-072** deferred as a future RFC. Measured (corrected 2026-08-16 — the first figures counted test-file references as render sites): **308** bare `i18n::JA_*` render sites over **191** constants, of which **183 already have an English half** — plumbing, not translation. **Slice D1 complete** (`8cce1de`, `34176d9`, `7806c5c`) and **D2a complete** (`57ffc37`) — 17 admin files, 210 sites, plus the four anonymous routes; `LOCALIZATION_EXCEPTIONS` has gone **27 entries / 308 sites → 6 / 54**. D1 cost no extra D1 query: `require_admin` already returns a `MembershipContext` carrying a resolved locale that handlers discard. **§8 settled 2026-08-16**: membership preference → `Accept-Language` → **Japanese** floor, for the four genuinely anonymous routes only. **D2b (account surfaces) split out to its own RFC — now RFC-084, accepted 2026-08-16.** What remains under this RFC is only D3, recommended (§4.4) to stay pinned. An **English-first default is a planned future advancement**, triggered by Slice D completing — see ROADMAP.md § *The default language flips to English when Slice D completes* |
| 084 | [Account-Tier Locale Resolution](./accepted/084-account-tier-locale-resolution.md) | 2026-08-16 | Discharges **RFC-083 §4.2's D2b** — the last convertible localization work: `account/mod.rs` (20 sites), `account/unlink.rs` (6), `account/link.rs` (5). The account tier is authenticated but **not community-scoped**, so a member arrives holding zero, one, or several disagreeing `ui_language` values. **Decided: option A** — rung 1 wins only when every present membership agrees on a non-NULL value, else the RFC-083 §8.1 ladder (`Accept-Language`, then Japanese). No migration; RFC-072's deliberate per-membership model stands (display name lives there too). Corrects RFC-083 §4.2's own reasoning: rung 2 does not override a choice when no stored choice *resolves* — it fills a hole. `link.rs`/`unlink.rs` take **one new D1 query each** (§10 decision 2); `mod.rs` is free, its `list_communities_for_user` call already reads the table. **Completing this fires the Slice D tripwire** — ROADMAP.md's English-default decision becomes due, by design, and must be resolved rather than re-pinned |

## Done — MVP core (001–019)

| ID | Title | Shipped in |
|----|-------|------------|
| 001 | [Project Bootstrap: Cloudflare Workers, Leptos SSR, D1](./done/001-project-bootstrap-cloudflare-leptos-d1.md) | v0.1.0 |
| 002 | [Data Model and D1 Migrations](./done/002-data-model-and-d1-migrations.md) | v0.1.0 |
| 003 | [Invite Redemption and Session Authentication](./done/003-invite-redemption-and-session-auth.md) | v0.2.0 |
| 004 | [Community Isolation and Authorization](./done/004-community-isolation-and-authorization.md) | v0.2.0 |
| 005 | [Member Home and Event Detail UI](./done/005-member-home-and-event-detail-ui.md) | v0.3.0 |
| 006 | [Participation Status Lifecycle](./done/006-participation-status-lifecycle.md) | v0.3.0 |
| 007 | [Notes and Comment Safety](./done/007-notes-and-comment-safety.md) | v0.3.0 |
| 008 | [Offline Cache, Mutation Queue, and Sync](./done/008-offline-cache-mutation-queue-and-sync.md) | v0.4.0 |
| 009 | [Admin Event Management](./done/009-admin-event-management.md) | v0.4.0 |
| 010 | [Admin Invite and Member Management](./done/010-admin-invite-and-member-management.md) | v0.4.0 |
| 011 | [Accessibility and Design System](./done/011-accessibility-and-design-system.md) | v0.5.0 |
| 012 | [Security Hardening and Abuse Controls](./done/012-security-hardening-and-abuse-controls.md) | v0.5.0 |
| 013 | [API Contracts, Error Model, and Idempotency](./done/013-api-contracts-error-model-and-idempotency.md) | v0.3.0 |
| 014 | [Observability, Audit, and Privacy Logging](./done/014-observability-audit-and-privacy-logging.md) | v0.5.0 |
| 015 | [Testing, QA, and Release Gates](./done/015-testing-qa-and-release-gates.md) | v0.5.0 |
| 016 | [Deployment Environments and Operations](./done/016-deployment-environments-and-operations.md) | v0.5.0 |
| 017 | [PWA Installability and Service Worker](./done/017-pwa-installability-and-service-worker.md) | v0.4.0 |
| 018 | [Time-Zone and Event Cutoff Policy](./done/018-timezone-and-event-cutoff-policy.md) | v0.7.0 |
| 019 | [Retention, Soft Delete, and Data Lifecycle](./done/019-retention-soft-delete-and-data-lifecycle.md) | v0.4.0 |
| 022 | [Recurring Events and Event Series](./done/022-recurring-events-and-event-series.md) | v0.17.0 |
| 023 | [Optional Calendar Export and ICS Interop](./done/023-optional-calendar-export-and-ics-interop.md) | v0.10.0 |
| 024 | [Display Name Recovery and Admin-Mediated Account Relinking](./done/024-display-name-recovery-and-admin-mediated-account-relinking.md) | v0.51.0 |
| 027 | [Import, Export, Human-Readable Backup, and Data Portability](./done/027-import-export-human-readable-backup-and-data-portability.md) | v0.15.0 |
| 028 | [Backup, Restore, and Disaster Recovery Operations](./done/028-backup-restore-and-disaster-recovery-operations.md) | v0.14.0 |
| 035 | [Support, Diagnostics, and User Help Without Data Leakage](./done/035-support-diagnostics-and-user-help-without-data-leakage.md) | v0.15.0 |
| 029 | [Scalability and Query Performance Discipline](./done/029-scalability-and-query-performance-discipline.md) | v0.12.0 |
| 025 | [Community Moderation, Abuse Response, and Member Safety](./done/025-community-moderation-abuse-response-and-member-safety.md) | v0.13.0 |
| 032 | [Event Templates and Quick Create for Non-Technical Admins](./done/032-event-templates-and-quick-create-for-non-technical-admins.md) | v0.16.0 |
| 030 | [Admin Onboarding, First Community Setup, and Empty States](./done/030-admin-onboarding-first-community-setup-and-empty-states.md) | v0.14.0 |
| 036 | [Public Release Readiness, Security Review, and Launch Runbook](./done/036-public-release-readiness-security-review-and-launch-runbook.md) | v0.15.0 |
| 026 | [Multi-Language and Plain-Language Localization](./done/026-multi-language-and-plain-language-localization.md) | v0.10.0 — EN/JA table complete; per-community lang selection deferred |

## Done — F7 stabilization and architect remediation

| ID | Title | Shipped in |
|----|-------|------------|
| 037 | [Token Subject Normalization and Form-Token Atomicity](./done/037-token-subject-and-form-token-atomicity.md) | v0.23.0 |
| 038 | [Session and Secret Binding Hardening](./done/038-session-and-secret-binding-hardening.md) | v0.23.0 |
| 039 | [Timezone-Correct Event Write Path](./done/039-timezone-correct-event-write-path.md) | v0.23.0 |
| 040 | [Event Edit Contract](./done/040-event-edit-contract.md) | v0.23.0 |
| 041 | [Atomic Invite Redemption](./done/041-atomic-invite-redemption.md) | v0.23.0 |
| 042 | [Pilot Offline and Private Cache Contract](./done/042-pilot-offline-and-private-cache-contract.md) | v0.23.0 |
| 077 | [Fail-Closed HMAC Pepper Configuration](./done/077-fail-closed-hmac-pepper-configuration.md) | `main` at `901855b`; hosted criteria 8–9 and B2 closure accepted 2026-07-22 |
| 043 | [Pilot UX Acceptance and Error Feedback](./done/043-pilot-ux-acceptance-and-error-feedback.md) | v0.23.0 / v0.24.0 |
| 046 | [Event-Bound Status Token](./done/046-event-bound-status-token.md) | v0.27.0 |
| 047 | [Japanese Date/Time Presentation](./done/047-japanese-date-time-presentation.md) | v0.27.0 |
| 048 | [Pilot Security Headers and Cache-Control Gate](./done/048-pilot-security-headers.md) | v0.30.0 |
| 049 | [Japanese-Language Pilot Rendering](./done/049-japanese-language-pilot-rendering.md) | v0.30.0 |
| 055 | [Offline Read-Only Contract](./done/055-offline-read-only-contract.md) | v0.31.0 |
| 052 | [Audit Retention and Operator Access Policy](./done/052-audit-retention-and-operator-access-policy.md) | v0.36.0 |

## Done — F8 workflow improvements

| ID | Title | Shipped in |
|----|-------|------------|
| 051 | [Multi-Day and Recurring Event Edit Semantics](./done/051-multi-day-and-recurring-event-edit-semantics.md) | v0.44.0 |
| 053 | [ICS Feed Privacy and Revocation UX](./done/053-ics-feed-privacy-and-revocation-ux.md) | v0.46.0 |
| 056 | [Calendar-Centered Home Dashboard](./done/056-calendar-centered-home-dashboard.md) | v0.40.0 |
| 057 | [Community Creation and Bootstrap Flow](./done/057-community-creation-and-bootstrap-flow.md) | v0.41.0 |
| 058 | [Calendar Month Navigation and Day Agenda](./done/058-calendar-month-navigation-and-day-agenda.md) | v0.42.0 |
| 059 | [Calendar Create Event From Day](./done/059-calendar-create-event-from-day.md) | v0.43.0 |
| 060 | [Cancel-and-Recreate Assistance](./done/060-cancel-and-recreate-assistance.md) | v0.45.0 |
| 061 | [Community Admin Member Management Navigation](./done/061-community-admin-member-management-navigation.md) | v0.48.0 |
| 062 | [Admin Role Transfer and Promotion](./done/062-admin-role-transfer-and-promotion.md) | v0.49.0 |
| 063 | [Member Removal, Re-add, and Suspension Policy](./done/063-member-removal-readd-and-suspension-policy.md) | v0.50.0 |
| 065 | [Recurrence v2 and Occurrence Exceptions](./done/065-recurrence-v2-and-occurrence-exceptions.md) | v0.54.0 |
| 066 | [Event Copy From Existing Event](./done/066-event-copy-from-existing-event.md) | v0.55.0 |
| 067 | [Monthly Attendance Matrix](./done/067-monthly-attendance-matrix.md) | v0.56.0 |
| 068 | [Calendar Matrix CSV Export](./done/068-calendar-matrix-csv-export.md) | v0.57.0 |
| 064 | [Rust Module and Crate Boundary Cleanup](./done/064-rust-module-and-crate-boundary-cleanup.md) | v0.52.0 / v0.53.0 / v0.58.0 |
| 069 | [Total Community Access Recovery](./done/069-total-community-access-recovery.md) | v0.59.0 |

## Done — shipped in 0.62.0

Tagged 2026-08-15 at `6c05b69`, triggered by three RFCs reaching `done/` — the
cadence rule working again: 25 commits and three RFCs, not 49 and five. Commit
and annotated tag pushed. **Tagged, not deployed**; B1, B3, B4, and B5 remain
open and nothing in this release closes a finding.

| ID | Title | Shipped in |
|----|-------|------------|
| 080 | [External Identity Foundation](./done/080-external-identity-foundation.md) | 0.62.0 (`5d1ad94`…`a31856f`) — seven packages across five slices, every one Approved. Namespaced identity keys, namespace-pinned JWT verification, the server-side authentication transaction, and a local fake issuer. **Chooses no provider**, which is the completed state: the whole contract is exercised with no provider account, no secrets, and no network |
| 081 | [Account Recovery and Membership Continuity](./done/081-account-recovery-and-membership-continuity.md) | 0.62.0 (`5d1ad94`…`9b121f1`) — amends **RFC-024** and **RFC-063**. §2 turned out to be a **live gap**: one community's admin could already mint a session reaching every community a member belonged to. Closed in Slice 1. **AD-2 holds** — the recovery credential cannot be removed while it is a member's last usable method, enforced in the same SQL statement as the unlink |
| 082 | [Membership Suspension](./done/082-membership-suspension.md) | 0.62.0 (`ba37d60`) — amends **RFC-063**, answering the suspension question it deferred. Two additive columns; the work was classifying **54** activeness decisions into two named predicates, three of them `PRESENT`, gated against inline use |

## Done — shipped in 0.61.0

Tagged 2026-08-02 at `4764c2a`, triggered by RFC-075 reaching `done/` — the
cadence rule working rather than another accumulation. **Tagged, not deployed**;
B1, B3, B4, and B5 remain open.

| ID | Title | Shipped in |
|----|-------|------------|
| 075 | [Render Style System and Inline Style Reduction](./done/075-render-style-system-and-inline-style-reduction.md) | 0.61.0 (`b5a88e1`…`f32ba3a`) — seven migration slices, a dead-code sweep, two English-leak packages, and the terminal CSP slice, every one Approved. **Inline `style=` 486 → 0**, asserted not ratcheted; hardcoded hex 383 → 25. **`style-src` no longer carries `'unsafe-inline'`**, gated against reintroduction and proven with browser evidence |

This release also carried RFC-072's three post-completion corrections
(`ced6ae4`, `e80fbe7`, `2d2be47`), the Event Detail 200%-text reflow fix, and
three hand-maintained gate lists replaced with default-fail mechanisms.

## Done — shipped in 0.60.0

Tagged 2026-07-30 after 49 unreleased commits had accumulated with no version
boundary. **Tagged, not deployed** — production, public-pilot, and
first-real-community deployment remain No-Go while B1, B3, B4, and B5 are open.

| ID | Title | Shipped in |
|----|-------|------------|
| 070 | [Self Display Name Editing](./done/070-self-display-name-editing.md) | 0.60.0 (`4bfe4f2`) |
| 071 | [Application Threat Model and Form Security Baseline](./done/071-application-threat-model-and-form-security-baseline.md) | 0.60.0 (`1b12d96`) — documentation baseline, active |
| 072 | [Member Language Preference and Runtime Localization](./done/072-member-language-preference-and-runtime-localization.md) | 0.60.0 (`b237788`/`93e25de`/`fcf84aa`) — Slices A–C. **Its three post-completion corrections shipped in `0.61.0`** — acceptance criterion 9 did not actually hold until then. Slice D is a future RFC |
| 073 | [Calendar Events List and Day Detail UX](./done/073-calendar-events-list-and-day-detail-ux.md) | 0.60.0 (`ed549be`) |
| 074 | [Community Switch Route Preservation](./done/074-community-switch-route-preservation.md) | 0.60.0 (`30e90c4`) |

## Also Done

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | implemented |

## Proposed — MVP remaining (020)

| ID | Title | File | Note |
|----|-------|------|------|
| 020 | Design Assets, Prototype, and Handoff | [./proposed/020-design-assets-prototype-and-handoff.md](./proposed/020-design-assets-prototype-and-handoff.md) | Wireframes + RFC spec delivered; mockups/tokens/icons pending |

## Proposed — post-MVP backlog (021–036, stubs)

| ID | Title | File |
|----|-------|------|
| 021 | Post-MVP Notification Strategy and Reminder Digests | [./proposed/021-post-mvp-notification-strategy-and-reminder-digests.md](./proposed/021-post-mvp-notification-strategy-and-reminder-digests.md) |
| 031 | Consentful Contact Channels and Privacy-Safe Messaging | [./proposed/031-consentful-contact-channels-and-privacy-safe-messaging.md](./proposed/031-consentful-contact-channels-and-privacy-safe-messaging.md) |
| 033 | Subgroups, Event Visibility, and Boundary Safety | [./proposed/033-subgroups-event-visibility-and-boundary-safety.md](./proposed/033-subgroups-event-visibility-and-boundary-safety.md) |
| 034 | Notification-Free Quiet Mode and Attention Design | [./proposed/034-notification-free-quiet-mode-and-attention-design.md](./proposed/034-notification-free-quiet-mode-and-attention-design.md) |

## Proposed — localization programme close-out (085)

| ID | Title | File | Note |
|----|-------|------|------|
| 085 | Separate the Locale Fallbacks Before Changing the Default | [./proposed/085-separate-the-locale-fallbacks.md](./proposed/085-separate-the-locale-fallbacks.md) | **Blocks** ROADMAP.md's English-default flip, now due — the Slice D tripwire has been failing by design since `cf3baba`. `impl Default for Locale` answers three questions with one value: no preference expressed (product), a **corrupt** stored value (safety, RFC-072's SEC-5 fail-safe), and the RFC-083 §8.1 ladder's floor (product). Flipping the default would move the fail-closed answer too. Deletes the impl so the ambiguity is untypeable, splits `resolve_locale`'s `None` from `Some(unparseable)`, and names rung 3 — two call sites, no migration. Does **not** flip the default; makes the flip a one-line change with provable boundaries |

## Proposed — F7/F8 follow-ups (044–054)

| ID | Title | File | Note |
|----|-------|------|------|
| 044 | D1 Query-Budget Gate and Integration Test Harness | [./proposed/044-d1-query-budget-gate-and-integration-test-harness.md](./proposed/044-d1-query-budget-gate-and-integration-test-harness.md) | **Narrowed 2026-07-29.** Harness and all four deferred regression tests discharged by RFC-050 local tooling at `c55787a`; only the runtime query-counting shim remains (gates beta) |
| 045 | Pre-Pilot Runtime Verification Matrix | [./proposed/045-pre-pilot-runtime-verification-matrix.md](./proposed/045-pre-pilot-runtime-verification-matrix.md) | Source verification discharged; staging-runtime matrix pending environment |

## Archive

_None._
