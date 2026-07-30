# ciao.zinnias Roadmap

## Status

**Current release:** 0.60.0.

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

## Architect Review Remediation Hold — Amended 2026-07-29

The 2026-07-14 architecture preparation review found production/pilot blockers
in one-time invite-code handling, hosted secret configuration, abuse controls,
and audit durability/redaction. The original hold froze product-feature work
until those blockers were "resolved and reviewed".

**Owner decision, 2026-07-29: the feature freeze is lifted; the deployment gates
are not.** The hold's original wording conflated two different things, and they
have now separated:

| | Status |
|---|---|
| **Source remediation** — the code that makes the service safe | **Complete.** B2 fully closed. B1, B3, and B5 have their source fixes implemented, reviewed, committed, and locally evidenced |
| **Hosted evidence** — proof against a frozen candidate | **Outstanding**, and only required before deploying |

The blockers' *code* is fixed. What remains is evidence for a deployment that is
not yet scheduled, so continuing to freeze product work on it would optimize the
wrong constraint — particularly while the service's user-facing function is the
weaker part of the product.

Accordingly:

- **product-feature work resumes** — RFC-073 and RFC-074 are complete and
  RFC-072 is in progress (see Active Themes);
- the RFC-050 hosted evidence campaign is **deferred, not cancelled**, and
  remains mandatory before any real community uses the service;
- persistent-incident-sink provisioning (Logpush → R2) is deferred with it, since
  its only consumer is that campaign;
- production, public-pilot, and first-real-community deployment remain **No-Go**;
- B1, B3, B4, and B5 remain **open**; nothing in this amendment closes a finding;
- controlled staging remains conditional on the open B1 and B3 findings being
  fixed or explicitly risk-accepted for an isolated, short-lived non-production
  environment; B2 is closed.

**What resuming feature work does not permit:** no reintroduction of a fail-open
path, no weakening of the fail-closed pepper, abuse-control, audit, or
form-token replay contracts, and no feature that would require hosted access to
verify. Security-sensitive changes still go through the RFC-071 threat-model and
form-security review gate.

When a pilot is actually scheduled, the RFC-050 campaign must run against the
candidate of that day — which will be later than today's tree. The tooling is
committed and gated, so drift is detectable, but the campaign is not
transferable evidence from an earlier commit.

### Tagging is not deploying — clarified 2026-07-30

The hold amendment separated **source remediation** from **hosted evidence** for
feature work. That same separation was never stated for *versioning*, and the
omission cost us: `0.59.0` was tagged 2026-07-11 and by 2026-07-30 the tree had
**49 unreleased commits across five RFCs** (070, 071, 073, 074, 072) with no
version boundary between them. Nothing was blocking a tag; nobody asked.

**A version tag is a source-control act. A deployment is a hosted act.** B1, B3,
B4, and B5 gate **deploying**, not **versioning**. Cutting a tag on `main`
violates no open finding, requires no hosted access, and closes nothing.

Concretely, the drift produced a real defect. `workers/ssr/static/app.js` gained
48 lines after `0.59.0` while its cache-buster
(`render/shell.rs`, `?v=0.59.0-…`) and the service worker's
`CACHE_VERSION = 'v0.59.0'` both stayed put — so a returning browser would serve
stale JavaScript and a returning PWA client's `shell-v0.59.0` cache would never
invalidate. It is **latent, not live**: nothing is deployed, so no real client
holds a stale cache. But it sits on the critical path, because RFC-050's
exact-candidate campaign depends on version metadata being truthful, and a
49-commit span covering a security remediation, three UX features, and a
localization refactor is a poor candidate boundary.

The RFC-044 service-worker gate did **not** catch this, and structurally cannot:
it asserts `sw.js CACHE_VERSION` equals the package version, and both were stale
at `0.59.0` together. It proves internal consistency, not freshness against the
tree.

**Standing rules from 2026-07-30:**

- **Tag when an RFC reaches `rfcs/done/`.** Release boundaries then map onto
  reviewed units, which this process already produces. The pre-hold rhythm of
  1–2 tags per day suited smaller units; 19 days for five RFCs is the opposite
  failure.
- **Tagging requires no hosted authorization**; deploying, provisioning, and
  publishing still require explicit per-instance owner approval, unchanged.
- A release must re-truth every version-bearing artifact, including the two
  cache keys above — not just `package.json` and `Cargo.toml`.

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

**Live sequence — reordered 2026-07-29 (owner decision):**

1. ~~RFC-073 — Calendar events list and day detail UX.~~ **Complete
   2026-07-29.** Design accepted at `342ad2c`, implemented at `ed549be`,
   implementation review Approved, and moved to `rfcs/done/`. The Calendar page
   now has three route-backed tabs with an always-present day-detail section, so
   a date click no longer returns the user to the top of the page. All nine
   acceptance criteria met locally; no hosted-evidence precondition. Real-device
   200%-scaling confirmation remains a separate pre-pilot gate below.
2. ~~RFC-074 — Community switch route preservation.~~ **Complete
   2026-07-30.** Design accepted at `5b901df`, implemented at `30e90c4`, both
   review points Approved, and moved to `rfcs/done/`. Changing community from
   the header now keeps the user on the same kind of page when the target
   community supports it, through a closed `next` token grammar with
   target-side authorization, no open redirect, no preserved community-scoped
   identifiers, and no fragment. All nine acceptance criteria met locally; no
   hosted-evidence precondition. It also **closed observation O1** from the
   RFC-073 review: RFC-067's matrix contract is now proven behaviorally in
   `ssr`, with the `release_gates.rs` source literal retained as a secondary
   tripwire.
3. ~~RFC-072 — Member language preference and runtime localization.~~
   **Complete 2026-07-30.** Design accepted at `1b49070`; Slices A
   (`b237788`), B (`93e25de`), and C (`fcf84aa`) each architecture-reviewed and
   Approved; moved to `rfcs/done/`. Members now choose Japanese or English per
   membership, and the member-facing UI, dates, and screen-reader labels follow
   it with `html lang` matching. The measurement that shaped it: all 254 English
   string constants already existed under a parity gate and **not one was
   reachable** — a seam refactor, not a translation project. The switcher stayed
   unreachable until Slice C so members were never offered a half-honoured
   language. All ten acceptance criteria met locally; no hosted-evidence
   precondition. **Slice D** — admin surfaces, `/join`, `/relink`, static
   offline HTML, `Accept-Language`, community default — remains a future RFC.
4. ~~Cut `0.60.0`.~~ **Complete 2026-07-30** — tagged at `0f679ae`, the first
   tag since 2026-07-11, covering RFC-070, 071, 072, 073, and 074. It re-truthed
   every version-bearing artifact including the stale `app.js` cache-buster and
   service-worker `CACHE_VERSION`, dropped the unmaintained RFC-number accretion
   from the cache-buster, and added `cached_asset_content_matches_pinned_hash`,
   which fires when cached-asset content changes without the pinned digest
   moving — the drift the existing `CACHE_VERSION`-equals-package-version check
   structurally could not catch, since both had gone stale together.
   **Tagged locally, not pushed and not deployed;** the deployment posture is
   unchanged. Carried follow-up: that gate *prompts* the cache-key bump rather
   than *enforcing* it — the cheapest way to green it is re-pinning the digest
   alone. Hashing only `app.js`/`app.css` and requiring the cache-buster to
   embed a prefix of that digest would make the invariant structural.
5. **RFC-075 — Render style system and inline style reduction.** Owner-selected
   2026-07-30 as the theme following RFC-072, and accepted the same day. The
   measurements that shaped the design: **477 inline `style=` occurrences, 356
   hardcoded hex colours, and zero `--cz-*` token references from Rust** — the
   35-token design system in `app.css` is decorative, consumed by nothing that
   renders the UI. Inline styling has also nearly **doubled** since `lib.rs`'s
   CSP comment recorded "~272 occurrences", so this is accumulating, not static.
   It carries a **security deliverable** the original draft did not name:
   `style-src 'unsafe-inline'` exists solely because of these inline styles and
   can be dropped only at **zero**, which makes that the terminal slice rather
   than a nice-to-have. Migration is per-surface starting with Calendar, guarded
   by two ratchets (inline-style count and hardcoded-colour count may only
   decrease) rather than an arguable threshold. Hard constraint: the Calendar
   accessibility gate at `release_gates.rs:2378` must be **re-expressed against
   the rendered class set, never deleted**.
6. **Choose the next user-facing theme** after RFC-075 — an owner decision.
7. **When a pilot is scheduled** — and not before — provision the persistent
   incident sink (Logpush → R2), then build and freeze one exact immutable
   release candidate and execute the RFC-050 hosted evidence campaign for the
   remaining B1, B3, B4, and B5 claims. The campaign must include E7 canary
   delivery, retrieval, health, retention, and access proof for the provisioned
   sink; console output and `wrangler tail` are not sufficient.
4. Reassess the deployment gates and version readiness from the integrated
   evidence. Lifting one finding does not implicitly lift another.

Steps 3 and 4 are deferred, not cancelled. No real community may use the service
until they complete.

RFC-050 procedures, tooling, and persistent-sink preparation may proceed where
doing so reduces risk, but the sink must be provisioned before canonical upload
and none of that preparation is final B4/B5 evidence until the frozen campaign
proves it against the reviewed RFC-078 exact candidate. RFC-044 and RFC-045 are
not automatically prerequisites, and the overlap check this paragraph used to
require **has now been performed** for both — RFC-045 during RFC-050 Tooling
Slice 9, and RFC-044 on 2026-07-29 (recorded in that RFC's §6.4). Neither is a
prerequisite for the RFC-050 campaign. RFC-044's harness and all four of its
deferred regression tests were discharged by the RFC-050 local tooling committed
at `c55787a`; only its runtime query-counting shim remains, and that gates beta,
not the first pilot.

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
| 044 | D1 query-budget gate and integration test harness | Narrowed 2026-07-29 after the RFC-050 overlap check: harness and all four deferred regression tests discharged at `c55787a`. Only the runtime query-counting shim remains; gates beta, not the first pilot. |
| 045 | Pre-pilot runtime verification matrix | Runtime evidence and operator verification candidate. |
| 054 | Japanese UX copy review | Needs native-speaker review and copy-quality pass. |

RFC-073, RFC-074, and RFC-072 left this table and shipped in `0.60.0`;
RFC-075 left it on acceptance 2026-07-30. None of the four is paused proposed
work any more, and this table now holds only genuinely unscheduled backlog.

## Post-Hold Feature Candidates

The feature freeze over this list was **lifted on 2026-07-29**, so these are an
ordered backlog rather than paused work. Their relative order is preserved as
the default sequence, but theme selection is an owner decision taken one theme
at a time. **This list is not implementation authorization** — only an Accepted
RFC is.

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
   The reassessment this entry called for is **done** — RFC-045 under RFC-050
   Tooling Slice 9, RFC-044 on 2026-07-29 (see its §6.4). What survives is a
   single narrow package: the runtime D1 query-counting shim, closing the band
   between a route's exact budget and the 2× ceiling the existing static gate
   enforces. It gates beta.

7. ~~**RFC-072: Member Language Preference and Runtime Localization**~~ —
   **done** (`fcf84aa`, 2026-07-30). Retained here for ordering context only;
   it is no longer a candidate.

8. ~~**RFC-073: Calendar Events List and Day Detail UX**~~ — **done**
   (`ed549be`, 2026-07-29). Retained here for ordering context only; it is no
   longer a candidate.

9. ~~**RFC-074: Community Switch Route Preservation**~~ — **done**
   (`30e90c4`, 2026-07-30). Retained here for ordering context only; it is no
   longer a candidate.

10. ~~**RFC-075: Render Style System and Inline Style Reduction**~~ —
    **accepted** (2026-07-30), implementation pending. Retained here for
    ordering context only; it is no longer a candidate.
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
