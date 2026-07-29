# ciao.zinnias Roadmap

## Status

**Current release:** 0.59.0.

The RFC folder is the source of truth for implementation state:

- Detailed RFC index: [rfcs/README.md](./rfcs/README.md)
- Accepted RFCs: [rfcs/accepted/](./rfcs/accepted/)
- Implemented RFCs: [rfcs/done/](./rfcs/done/)
- Proposed RFCs: [rfcs/proposed/](./rfcs/proposed/)

Recent workflow releases focused on calendar-centered use, community bootstrap,
member administration, admin role transfer, member lifecycle policy,
admin-mediated help sign-in, Rust module boundary cleanup, recurrence v2 for
Calendar workflows, admin event-copy creation assistance, the monthly
attendance matrix, admin-only matrix CSV export, contracts i18n boundary
cleanup, and operator-only total community access recovery.

## Architect Review Remediation Hold

The 2026-07-14 architecture preparation review found production/pilot blockers
in one-time invite-code handling, hosted secret configuration, abuse controls,
and audit durability/redaction. Until those blockers are resolved and reviewed:

- do not implement new product-feature breadth from the proposed RFC backlog,
  including RFC-072 through RFC-075;
- remediation design and implementation take priority over the candidate order
  below;
- RFC-050 hosted evidence work and RFC-054 Japanese copy review may continue,
  because they close existing release evidence rather than add feature scope;
- production, public-pilot, and first-real-community deployment remain No-Go;
- controlled staging remains conditional on the architecture review's open B1
  and B3 findings being fixed or explicitly risk-accepted for an isolated,
  short-lived non-production environment; B2 is closed.

The RFC lifecycle now separates Proposed design from Accepted implementation.
Remediation design and accepted implementation take priority. Feature work
resumes only after the remediation hold is explicitly lifted in this roadmap.

## Active Remediation and Release Sequence

RFC numbers are stable identifiers, not execution-order numbers. Steps 1–4 of
the prior sequence are now complete; the live head is provisioning the
persistent incident sink (renumbered 1 below). A fuller milestone/dependency
breakdown with entry/exit criteria is maintained alongside this roadmap for
internal planning (programme roadmap and developer execution guide); this
section remains the tracked summary of record.

**Complete:**

1. Reconciled RFC-079 and the release checklist with the completed Class A
   failure-telemetry implementation review and committed implementation.
2. Prepared the RFC-078 implementation handoff, including native SSR CI,
   workspace/all-target Clippy, retention of the WASM check, and focused test
   modules that do not further enlarge the monolithic release-gate file,
   applying the already-committed RFC-071 threat-model and form-security
   baseline.
3. Implemented RFC-078 — including the required I-B1 concurrency-burst
   evidence and every non-blocking review item — and committed it at
   `c991b82` on 2026-07-28. This was the immediate code priority and the
   remaining B3 source remediation; B3 itself remains open pending RFC-050
   exact-candidate hosted evidence.
4. Architecture-reviewed and owner-accepted RFC-050 on 2026-07-28, moving it to
   `rfcs/accepted/`. The owner also risk-accepted deferring the native IPv6
   `/64` hosted-sharing proof on the same date — recorded in RFC-078 § Dated
   owner risk acceptances and RFC-050's E4a; **IPv6 client support is not
   confirmed/implemented for this deployment**, and this service must be
   treated as IPv4-only in practice until hosted evidence says otherwise.
   Acceptance authorizes local RFC-050 tooling implementation only, not any
   hosted action.
5. Implemented RFC-050 Tooling Slices 1–7 — version metadata and the strict
   `/version` schema, the candidate manifest with mechanical redaction,
   exact-identity runtime smoke, E3 flow collection, E4 concurrency and
   postcondition tooling, E5 negative-configuration fixtures, and the manual
   evidence templates with artifact hashing and leakage scanning — committed at
   `15b9409` and `c55787a`. Slices 8 (tracked attestation template and gate
   rules) and 9 (documentation reconciliation) remain. All of this is **local
   tooling only**: none of it is B4 evidence until the same tooling runs against
   a frozen exact candidate, and it closes no finding.
6. Fixed a shipped form-token replay-detection defect discovered by Slice 5's
   concurrency evidence, committed at `c55787a`. A compatibility wrapper
   collapsed `ConsumeResult::Proceed` and `Replay(None)` into one value, so 21
   call sites across 17 handler files could not detect a replayed single-use
   token; only display-name editing, which stores a `result_ref`, was unaffected.
   CSRF protection was never weakened — invalid and absent tokens were always
   rejected — but replay protection was, so a replayed token could re-execute
   non-idempotent actions such as calendar-token regeneration and community
   export authorization. The wrapper was removed, every call site migrated to
   match `ConsumeResult` explicitly, a contract gate added so the pattern can no
   longer compile, a replay regression test added for two non-idempotent
   actions, and `docs/src/tester/release-checklist.md` corrected from a false
   claim under an explicit "Corrected 2026-07-28" marker. Per the owner's
   decision this was handled as a bounded remediation rather than a numbered
   RFC; this entry is its durable record.

**Live sequence:**

1. Decide, provision, and document the owner-approved persistent incident
   sink (Logpush → R2, per the owner's decision), retention, and access
   boundary before canonical candidate upload.
2. Build and identify one exact immutable release candidate, then execute the
   frozen RFC-050 hosted evidence campaign for the remaining B1, B3, B4, and B5
   claims. The same campaign must include E7 canary delivery, retrieval,
   health, retention, and access proof for the provisioned persistent sink;
   console output and `wrangler tail` are not sufficient.
3. Reassess the remediation hold and v0.60.0 readiness from the integrated
   evidence. Lifting one finding does not implicitly lift another.
4. Resume product-feature implementation only after this roadmap explicitly
   lifts the hold.

RFC-050 procedures, tooling, and persistent-sink preparation may proceed where
doing so reduces risk, but the sink must be provisioned before canonical upload
and none of that preparation is final B4/B5 evidence until the frozen campaign
proves it against the reviewed RFC-078 exact candidate. RFC-044 and RFC-045 are
not automatically prerequisites: their remaining scopes must first be checked
for overlap with or supersession by the revised RFC-050.

## Architecture Remediation Work

| RFC | Theme | Current note |
|-----|-------|--------------|
| 050 | Exact-candidate hosted staging evidence and pilot gate | B4 remediation revision, architecture-reviewed and owner-accepted 2026-07-28, moving it to `rfcs/accepted/`. A 2026-07-28 reconciliation review found the design needed a new E4a ingress/topology evidence item plus E1/E9/gate-invalidation extensions to match what shipped, and a re-review accepted the applied edits (Accept/Go). **IPv6 client support is not confirmed/implemented for this deployment** — the document carries a 2026-07-28 owner risk acceptance deferring the native IPv6 `/64` hosted-sharing proof; treat this service as IPv4-only in practice until hosted evidence says otherwise. Acceptance authorizes local RFC-050 tooling implementation only, not hosted execution. Tooling Slices 1–7 are implemented, reviewed, and committed at `15b9409` and `c55787a`; Slices 8–9 remain. That tooling is local only — it becomes B4 evidence solely when run against a frozen exact candidate, and B4 remains open. |
| 076 | One-time invite code response isolation | Local implementation reviewed, owner-accepted, and committed at `b72f22b`. Corrected isolated automated evidence and bounded human no-JS/network observation were architecture-reviewed and owner-accepted on 2026-07-21, closing criterion 8 locally; RFC-050 exact-candidate hosted evidence remains required before B1 closes for a public or production pilot. |
| 077 | Fail-closed HMAC pepper configuration | Implemented at `901855b`. Corrected disposable hosted evidence was architecture-reviewed and owner-accepted on 2026-07-22: criteria 8–9 are satisfied and B2 is closed. This does not close unrelated RFC-050, B1, B3, B5, production, public-pilot, real-device, performance, persistent-observability, or release gates. |
| 078 | Fail-closed strongly consistent abuse controls | Corrected architecture was reviewed and owner-accepted on 2026-07-23. Implementation was reviewed, owner-accepted, and committed at `c991b82` on 2026-07-28; the required I-B1 concurrency-burst evidence and every non-blocking item were re-reviewed and accepted in that same round. B3 remains open pending the RFC-050 exact-candidate hosted evidence campaign. All controlled-staging, public-pilot, production, and release holds remain unchanged. |
| 079 | Atomic required audits and recursive metadata redaction | Architecture reviewed and owner-accepted on 2026-07-15. Local Packages 0A–8 and the Class A failure-telemetry correction are reviewed and committed. RFC-050 exact-candidate hosted evidence, persistent incident delivery, and every public/production pilot gate remain open. |

## Implemented Lifecycle Reconciliation

RFC-070 and RFC-071 are already implemented on `main`; they are not future
implementation candidates. On 2026-07-23 the owner explicitly authorized their
direct lifecycle correction from `proposed/` to `done/`. Their Status fields,
index entries, and inbound links move together while their remaining evidence
qualifications and non-blocking notes remain visible.

| RFC | Implemented baseline | Remaining qualification |
|-----|----------------------|-------------------------|
| 070 | Self display-name editing committed at `4bfe4f2`; implementation review accepted with notes. | Remaining browser/hosted evidence is release evidence, not unimplemented source scope. |
| 071 | Threat-model documentation committed at `1b12d96`; implementation review accepted with notes. | The mandatory baseline is already in use. Preserve precise checklist-versus-observed-evidence labeling as non-blocking documentation cleanup. |

## Paused Proposed Work

The paused proposed backlog is:

| RFC | Theme | Current note |
|-----|-------|--------------|
| 020 | Design assets, prototype, and handoff | Non-code design deliverable remains. |
| 021 | Notification strategy and reminder digests | Requires product and infrastructure design before implementation. |
| 031 | Consentful contact channels and privacy-safe messaging | Depends on consent and notification policy decisions. |
| 033 | Subgroups, event visibility, and boundary safety | High-impact authorization and privacy design work. |
| 034 | Notification-free quiet mode and attention design | Should be considered with RFC-021. |
| 044 | D1 query-budget gate and integration test harness | Runtime/integration hardening candidate. |
| 045 | Pre-pilot runtime verification matrix | Runtime evidence and operator verification candidate. |
| 054 | Japanese UX copy review | Needs native-speaker review and copy-quality pass. |
| 072 | Member language preference and runtime localization | Design remains proposed; implementation is paused by the architect remediation hold. |
| 073 | Calendar events list and day detail UX | Design remains proposed; implementation is paused by the architect remediation hold. |
| 074 | Community switch route preservation | Design remains proposed; implementation is paused by the architect remediation hold. |
| 075 | Render style system and inline style reduction | Design remains proposed; implementation is paused by the architect remediation hold. |

## Post-Hold Feature Candidates

The feature candidates below are paused while the architect-review remediation
hold is active. Their relative order is preserved for reconsideration after the
hold is explicitly lifted; this list is not implementation authorization.

1. **External identity/OIDC pre-RFC consultation**
   Revisit the parked consultation only after the remediation hold is
   explicitly lifted, applying the current RFC-071 security baseline.
   Resolve account linking, codlet-to-OIDC transition, provider identity,
   recovery, session, and lockout semantics before drafting an implementation
   RFC. This roadmap entry does not authorize an RFC or implementation.

2. **RFC-054: Japanese UX Copy Review**
   Recent releases added sensitive recovery and member-management flows. Copy
   quality is part of usability and safety, but the full native-speaker copy
   review should wait until the user-facing surface is more stable.

3. **RFC-021 and RFC-034: Notifications and Quiet Mode**
   These should be designed together to avoid adding reminders without a clear
   attention and opt-out policy.

4. **RFC-031: Consentful Contact Channels**
   Useful after notification policy is clear. This should remain privacy-first
   and consent-bound.

5. **RFC-033: Subgroups and Event Visibility**
   Large feature area touching authorization, event visibility, and community
   boundaries. It should start with design review, not direct implementation.

6. **RFC-044 and RFC-045: Remaining Runtime Harness Work**
   Reassess their unimplemented portions after RFC-050. Retain only work that
   is not already discharged or superseded by the exact-candidate evidence
   contract.

7. **RFC-072: Member Language Preference and Runtime Localization**
   The i18n scaffold exists, but runtime UI language selection does not. This
   should start with design review around membership-vs-user scope, locale
   precedence, settings UX, `html lang`, and tests before schema or handler
   work.

8. **RFC-073: Calendar Events List and Day Detail UX**
   The Calendar grid, monthly event list, and attendance matrix now carry
   different user jobs. Split them into explicit tabs and make selected-day
   detail appear under the calendar without sending users back to the top.

9. **RFC-074: Community Switch Route Preservation**
    Header switching should preserve the current page family where safe, such
    as Calendar or My Page, and fall back only when the target community lacks
    permission or no equivalent route exists.

10. **RFC-075: Render Style System and Inline Style Reduction**
    Inline styling across server-rendered Rust strings is now a maintenance
    risk. Start with a reviewed CSS/class strategy and migrate by surface,
    likely beginning with Calendar.

## Before First Pilot Deployment

These are the remaining gates before the first real community can use the
service.

### Operator Tasks

- [ ] Apply the exact candidate's complete D1 migration ledger to the target environment, through at least `0010_audit_integrity.sql` at the current baseline; rehearse rollback/forward recovery and RFC-079's metadata-reset, backup-sensitivity, and privacy boundary.
- [ ] Set required secrets per environment without printing or committing real values.
- [ ] Configure required KV/D1 bindings per environment.
- [ ] Configure `SESSION_COOKIE_DOMAIN` as a non-secret variable only when a shared cookie domain is required.
- [ ] Decide and provision the owner-approved persistent incident sink before canonical candidate upload; document retention and access, then prove E7 canary delivery/retrieval in the frozen RFC-050 campaign.
- [x] Consolidate and review the application threat model and form-security baseline. *(committed at `1b12d96`; implementation review accepted with notes)*
- [ ] Apply the threat-model/form-security review gate to every security-sensitive change in the exact candidate and retain precise checklist-versus-observed-evidence labels.
- [ ] Run security review against the release checklist.

### Browser and Device QA

- [ ] Core join-and-mark-attendance flow under 2 minutes on a real phone.
- [ ] Calendar, event detail, and admin member flows remain usable at 200% system text scaling.
- [ ] No-JS destructive confirmations work without scripting.
- [ ] Recovery/help-signin flow works on the target hosted environment.

### Release Gate

- **Do not deploy to production** or tag a public pilot release without explicit confirmation from nabbisen.

## After First Pilot

Once a pilot community has been running for at least 4 weeks, revisit:

1. **RFC-033: Subgroups** if privacy or visibility boundaries emerge from real usage.
2. **RFC-021: Notifications** if sync-based checking proves insufficient.
3. **RFC-031: Contact channels** if admins need direct member communication and consent rules are clear.

The guiding principle remains: add only what is needed. Every feature added is a
feature that must be maintained, explained, and trusted.
