# ciao.zinnias Roadmap

## Status

**Current release:** 0.62.0.

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

- **product-feature work resumes** — RFC-072, RFC-073, RFC-074, and RFC-075 are
  all complete (see Active Themes);
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

### Migration immutability begins at first deployment — owner decision 2026-08-10

Migrations are normally append-only, because a committed migration has already
run somewhere you cannot reach. **That premise does not hold yet.** The service
has never been deployed; no database outside a developer's machine has ever
applied any migration in this repository.

So, once and only once: **the initial schema may be corrected at source rather
than migrated forward.** This was decided to reach RFC-081 §1's membership
invariant, which is unreachable under D1 by migration — see RFC-081 §1.2a for
the five mechanisms tested and rejected. The cost is that every developer resets
their local database (`bun run reset:dev`).

**The exception expires at first deployment and must not be cited afterwards.**
From that moment a committed migration has run somewhere unreachable, editing one
silently diverges every deployed database from its history, and the only correct
move is a forward migration — however inconvenient.

Anyone reading this after a deployment has happened: the answer is no.

### The default language flips to English when Slice D completes — owner decision 2026-08-16, taken 2026-08-16

`Locale::default()` was Japanese when this decision was recorded
(`packages/contracts/src/locale.rs:43`), and migration
`0011_membership_ui_language.sql` was additive with no backfill, so **every
membership had `ui_language = NULL`** and resolved through that default. The
default was therefore not an edge case — it was what every member saw.

**Decided: an English-first default is a planned future advancement, deferred on
readiness, not on principle.** It was not taken immediately because English was at
the time the less complete surface: after RFC-083 Slice D1a, **203 render sites
still emitted Japanese regardless of locale** — 23 structurally unresolvable, 180
simply not yet converted (RFC-083 D1b, D1c, D2).

**The trigger was RFC-083 Slice D reaching completion**, measurable as
`LOCALIZATION_EXCEPTIONS` containing only the three structurally-unresolvable
entries (`render/errors.rs`, `handlers/calendar.rs`, `handlers/communities.rs`).
RFC-084 (`cf3baba`) closed the last convertible work and reached exactly that
state — the trigger fired for real, not by re-pinning a number.

**Taken, in three sequenced packages, exactly as this entry anticipated:**

1. **RFC-085** (accepted `26cc28b`, implemented `f50dd57`) split the ambient
   `Locale::default()` into two independently-named answers —
   `Locale::PRODUCT_DEFAULT` (this decision) and `Locale::FAIL_CLOSED` (RFC-072's
   SEC-5 safety answer for a corrupt stored value) — so the flip below could move
   one without the other, provably, not by inspection.
2. **Handoff 078** (`9c1a03b`) pinned every smoke fixture's `ui_language`, proven
   by running the whole suite with the flip applied temporarily before it was
   real — 25/25.
3. **Handoff 079** took the flip itself: `Locale::PRODUCT_DEFAULT` is now
   `Locale::En`. `Locale::FAIL_CLOSED` is unchanged at `Locale::Ja` — confirmed
   unmoved by a source-level gate, not assumed. Migration `0011`'s comment was
   corrected to describe the new resolution (schema and `CHECK` constraint
   untouched — this was never a migration edit in the immutability sense; there
   is no installed base to disrupt, since the service has never been deployed).

**The tripwire this entry said was owed
(`roadmap_english_default_tripwire_fires_when_slice_d_completes`, added by
Handoff 072) existed, fired when RFC-084 closed Slice D, and is retired by
Handoff 079** — the decision it existed to surface has been taken, and the
properties that remain (the exception table's shrink-only pin, the RFC-085
re-merge gate, the locale-blind-helper check) are all still enforced
independently of it.

Recorded in **RFC-083 §8.2**.

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
5. ~~RFC-075 — Render style system and inline style reduction.~~ **Complete
   2026-08-01.** Design accepted at `96e3de5`; delivered across seven migration
   slices, a dead-code sweep, two English-leak packages, and a terminal CSP
   slice (`b5a88e1`…`f32ba3a`), every package architecture-reviewed and
   Approved; moved to `rfcs/done/`.

   **Inline `style=`: 486 → 0**, now asserted rather than ratcheted. Hardcoded
   hex in Rust: 383 → 25. `app.css` went from 96 lines of tokens that nothing
   consumed to a real component layer.

   **The security deliverable landed:** `style-src` no longer carries
   `'unsafe-inline'`, and a gate blocks reintroduction. That directive existed
   solely because the SSR templates used inline styles, and could only be
   dropped at zero — which is why the migration had to finish rather than stop
   at the interesting surfaces. Removal was proven safe with browser evidence:
   violation capture was built, shown to fire on a known-bad case, and all ten
   smokes report zero violations, with the three `app.js` CSSOM writes exercised
   under the strict header. It was never an active vulnerability — output is
   escaped throughout — so the honest framing is *a strict directive restored*.

   Along the way the work also closed three English leaks classes and replaced
   three hand-maintained gate lists with default-fail mechanisms.

6. ~~Cut `0.61.0`.~~ **Complete 2026-08-02** — tagged at `4764c2a`, triggered by
   RFC-075 reaching `rfcs/done/`. The cadence rule worked as intended: 21 commits
   and one RFC, not 49 commits and five. Tagged locally, **not pushed and not
   deployed**.
7. ~~**External identity foundation and membership suspension.**~~ **Complete
   2026-08-13** — RFC-080, RFC-081, and RFC-082 implemented across eight reviewed
   packages (`5d1ad94`…`ba37d60`) and moved to `rfcs/done/` at `d001d60`.
   RFC-081 §2 turned out to be a **live authorization gap**, not a precaution:
   one community's admin could already mint a session reaching every community a
   member belonged to. Found while reviewing the RFC's own design, closed in the
   first slice. **No provider is chosen, and that is the finished state** — Stage
   3 is deliberately outside RFC-080/081 and blocked on Stage 0 user research
   that does not exist.
8. ~~Cut `0.62.0`.~~ **Complete 2026-08-15** — tagged at `6c05b69`, triggered by
   three RFCs reaching `rfcs/done/`. Commit and annotated tag **pushed**; `main`
   and `origin/main` in sync. **Tagged, not deployed** — B1, B3, B4, and B5
   remain open and nothing in the release closes a finding.
9. ~~**Choose the next user-facing theme.**~~ **Complete 2026-08-15** —
   **RFC-054, Japanese UX copy review**, selected and accepted the same day after
   a substantial revision.

   The revision matters more than the selection. The RFC was written in June 2026
   against v0.36.0 and its premise had expired: it inventoried 143 strings in one
   `i18n.rs` (there are now **319 across 13 modules**), and the jargon it existed
   to remove — セッション, トークン, 同期 — appears in **no** Japanese string
   today, having been fixed incidentally by RFC-049 and RFC-072.

   What the review is actually for is the constants **no one has ever reviewed**,
   added by the external-identity track and RFC-082: the suspension paused page,
   the account surface, and the recovery credential. Those are read by a member
   who is confused or worried, which is when copy quality matters most.

   **Corrected 2026-08-15 (`03821bc`): that count is 180, not 54.** The "54" was a
   module-level estimate rather than a measurement, and correcting it moved the
   slice boundary — novelty no longer selects slice 1, consequence does.

   The original §4 blocker — *"requires a Japanese native speaker"* — was never
   about finding a person. It was that nobody had prepared the work so the native
   speaker's time went to judgement rather than sorting. The accepted split:
   architect prepares the inventory, scans, and a proposed rewrite for every
   finding; owner decides what sounds natural; developers apply.

10. **Execute RFC-054, then RFC-083.** Slice 1 finding A1 shipped at `6ea3765`.
    Findings B1–B4 are accepted and packaged with two copy-harmony items derived
    from a full comparison of all 632 `&str` constants: **H1**, one word for
    dismissing a form (three キャンセル → やめる, because キャンセル already means
    *an event was called off* in this product); **H2**, one word for the role
    (管理者/administrator — 運営者 appeared exactly once).

    **RFC-083 (localization Slice D) is accepted and follows, not overlaps.**
    Doing both at once produces a diff in which changed wording and changed
    plumbing are indistinguishable. Slice D1 (admin surfaces) carries the bulk at
    no extra D1 cost; **Slice D2 is blocked** on whether anonymous routes should
    honour `Accept-Language`.

11. **When a pilot is scheduled** — and not before — the RFC-050 hosted evidence
    campaign, unchanged. Stage 3 of the external-identity track (choosing a
    provider) also waits on Stage 0 user research that does not exist.
8. **When a pilot is scheduled** — and not before — provision the persistent
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
RFC-075 left it on acceptance 2026-07-30 and is now complete. None of the four
is paused proposed
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

   **Hold-lift recorded — owner decision, 2026-08-09.** The gating sentence
   above predates the 2026-07-29 amendment and contradicts this section's own
   header: the freeze over this list was lifted, but this entry still told the
   reader to wait for that same lift. It was written when the whole list was
   frozen and was never reconciled. **The 2026-07-29 lift covers this track like
   the rest of the list**, and this entry is selected as the active theme.

   What that does **not** change. The consultation's own staging is unaffected:
   Stage 0 (user research and provider due diligence) → Stage 1 (identity
   foundation RFC, which chooses no provider) → Stage 2 (recovery and
   membership-continuity policy) → Stage 3 (first provider rollout). Stages 1
   and 2 must both be accepted before any provider rollout begins.

   Still not authorized by this entry: an RFC number, any implementation, any
   provider registration, secret provisioning, hosted callback, or external data
   collection. B1, B3, B4, and B5 remain open; production, public-pilot, and
   first-real-community deployment remain **No-Go**. Nothing here closes a
   finding.

   **Why now rather than after a pilot.** Adding external identity later would
   require a first-link ceremony for every existing member, and the consultation
   forbids treating an existing 30-day cookie as sufficient step-up for a
   permanent external link — so it would mean a bounded migration ceremony for
   legacy sessions with no provenance, on live data. Settling the foundation
   before any real community joins avoids that entirely. That window closes when
   the first one does.

   **Stages 1 and 2 implemented; moved to `rfcs/done/` 2026-08-13.** RFC-080
   *External Identity Foundation* and RFC-081 *Account Recovery and Membership
   Continuity*, delivered across **seven packages in five slices**
   (`5d1ad94`…`9b121f1`), each architecture-reviewed and Approved. Neither
   chooses a provider, and that is the completed state rather than an omission:
   the whole contract is exercised against a local fake issuer with no provider
   account, no secrets, and no network — with a feature gate proven from the
   compiled artifact that a production build cannot reach that issuer.

   **The theme is finished. No next theme is selected.**

   **Stage 3 (first provider rollout) still requires the Stage 0 user research,
   which does not exist**, and its checklist now carries four items — three found
   during implementation rather than anticipated: the `aud`-array limitation,
   whether a provider honours `prompt=login` (a selection criterion, since one
   that ignores it cannot support step-up), LINE's `sub` scope, and Google's PKCE
   support.

   **Under the standing rule, this triggers `0.62.0`** — see *Tagging is not
   deploying*. RFC-082 *Membership Suspension* (`ba37d60`) moved to `done/` in
   the same pass and belongs to the same release.

   RFC-081 amends two shipped RFCs: **RFC-024** (relink and help-signin sessions
   become bound to the granting community, because a stable `users.id` would
   otherwise let one community admin mint a session reaching every community that
   person belongs to) and **RFC-063** (`UNIQUE(community_id, user_id)` becomes a
   partial unique index on `removed_at IS NULL`, so a removed member can return as
   the same principal without reactivating the old membership).

   Owner expectation for eventual providers, recorded 2026-08-09: **Google Account
   and LINE, at least** — which makes account linking a day-one concern rather
   than a later one, since one person may hold both and auto-linking by email or
   name is prohibited.

   **One open owner decision**, deliberately deferred: whether to expire all
   pre-cutover sessions at implementation (RFC-081 §11.4). Its answer depends on
   whether a real community has used the service by then — free if not, a real
   cost if so. To be answered in the implementation handoff, not assumed.

   Governing records:
   `.git-exclude/reviewed/zinnias-ciao-main-2026-07-17-external-identity-pre-rfc-architect-consultation.md`,
   `.git-exclude/research/2026-08-09-stage0-provider-due-diligence.md`

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

10. ~~**RFC-075: Render Style System and Inline Style Reduction**~~ — **done**
    (`f32ba3a`, 2026-08-01). Retained here for ordering context only; it is no
    longer a candidate.
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
