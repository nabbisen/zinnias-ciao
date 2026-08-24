# RFC-083 — Localization Slice D: Admin, Anonymous, and Unresolvable Surfaces

**Status:** Accepted
**Author:** high-capability model
**Date:** 2026-08-16
**Accepted:** 2026-08-16 by nabbisen. Acceptance authorized Slices D1 and D3.
**§8 was settled 2026-08-16**, unblocking **D2a**; **D2b** is deferred to its own
RFC (§4.2).
**Amends:** RFC-072 (member language preference and runtime localization), which
defers Slice D explicitly: *"Admin surfaces, anonymous routes (`/join`,
`/relink`), static offline HTML, `Accept-Language`, and a community default — is
deliberately out of scope and remains a future RFC."*
**Depends on:** RFC-072 Slices A–C (done)

---

## 1. Context

RFC-072 gave members a language preference and migrated the member-facing core to
honour it. It drew the boundary at Slice D and named the surfaces left behind.
This RFC is that future RFC.

Nothing here is a newly discovered defect. Every unlocalized site is already
pinned, by exact count, in the `LOCALIZATION_EXCEPTIONS` table in
`packages/contracts/tests/release_gates.rs`, each with a written reason. The gate
`rfc072_every_handler_and_render_file_is_localized_or_documented_exception`
walks every file under `handlers/` and `render/` and fails on anything neither
localized nor listed. **The current state is honest and enforced.** What this RFC
proposes is closing the gap the table documents.

### 1.1 Measured scope

> **Corrected 2026-08-16, before any implementation.** The figures below
> originally read **409** render sites over **224** constants, **216** paired.
> **Those counts included `i18n::JA_*` references inside test files** — 101 of
> them — which are assertions, not render sites. The corrected render-site count is
> **308**, which reconciles exactly with the sum of the gate's own pinned
> `ja_count` values (27 entries, 308 total). The **8** needing English is
> unchanged, having come from the independent name-pairing method.
>
> This is the same failure the project keeps catching: a count stated as measured
> that a second method would have caught. Reproducing the gate's per-file pins is
> what surfaced it, and §6 of this RFC is the reason to keep doing that.

Measured against the tree at `2799250`, by the same counting method the gate uses
and reconciled against the sum of its pinned per-file counts:

| Quantity | Count |
|---|---|
| Bare `i18n::JA_*` render sites (excluding tests) | **308** |
| Distinct JA constants at bare sites | **191** |
| — of those, **already have an English half** | **183** |
| — of those, needing new English copy | **8** |
| Files in `LOCALIZATION_EXCEPTIONS` | **27** |

**The English copy for this work almost entirely already exists.** 183 of 191
constants are complete `EN_`/`JA_` pairs whose render site simply does not call
`t()`. This is plumbing, not translation. The eight exceptions are listed in §4.3.

The 8 was derived twice by independent methods — a name-level pairing comparison
across all 632 `&str` constants, and a reference-level walk of every bare render
site — which agree exactly on the same eight names.

### 1.2 The three buckets, by the gate's own table

| Bucket | Files | Sites |
|---|---|---|
| **D1 — admin** (§4.1) | 17 | **210** |
| **D2 — no community scope** (§4.2) | 7 | **75** |
| **D3 — unresolvable** (§4.4) | 3 | **23** |
| | **27** | **308** |

## 2. Goals

1. A member or admin who selects English sees English on every surface where a
   language preference is knowable.
2. Where it is **not** knowable, the choice of language is a written, defended
   decision rather than a default nobody chose.
3. The exception table shrinks monotonically, and cannot silently regrow.

## 3. Non-goals

- No new language. `Locale` stays `ja` | `en` (RFC-072 Slice A).
- No runtime message catalogue. RFC-072 rejected it so a missing translation stays
  a compile error; that holds.
- No change to what any Japanese string *says*. Copy revision is RFC-054's
  business, and the two must not be done in one pass — RFC-072's history shows why
  (§7).
- No change to authorization, routing, or D1 query budgets.

## 4. The three surfaces, which are not one problem

The 27 exception entries fall into three groups with genuinely different
difficulty. Treating them as one slice is the main risk this RFC exists to avoid.

### 4.1 Admin surfaces — the locale is already fetched and thrown away

17 files, **210** of the 308 sites (§1.2) — the large majority.

`require_admin` returns a `MembershipContext`, and `db/membership.rs` defines
admin membership rows that carry a **resolved `locale`** field. Admin handlers
already call it and discard the result:

```rust
let _membership = require_admin(env, &auth, community_id, rid).await?;
```

So the locale is present at essentially every admin render site, at **zero
additional D1 cost** — which matters, because RFC-044's query budget gate would
otherwise make this expensive. The change is to bind the value instead of
discarding it, and pass it to `t()` and to `render::page_localized`.

This is the cheapest slice and covers the most ground. It should go first.

### 4.2 No community scope — two different problems, not one

> **Corrected 2026-08-16.** This section originally called all seven files
> "anonymous routes" that run "before they have an account at all." **That is true
> of only four of them.** The account surfaces are authenticated; they simply are
> not community-scoped. The distinction changes the answer, so the two are split
> below.

Seven files, 75 sites, sharing one symptom — `db/membership.rs::find_active` is
*"the only trustworthy source of a page's locale"* and it requires a community —
but for two different reasons.

**D2a — genuinely anonymous** (44 sites): `handlers/join.rs` (18),
`handlers/relink.rs` (10), `handlers/recovery.rs` (10), `handlers/identity/mod.rs`
(6). No session, no account, nothing stored to read. This is the group §8 is about.

**D2b — authenticated, not community-scoped** (31 sites):
`handlers/account/mod.rs` (20), `handlers/account/unlink.rs` (6),
`handlers/account/link.rs` (5). These run behind `require_account_surface`, so the
member *is* signed in and may hold several memberships — each with its own stored
preference.

**`Accept-Language` is the wrong answer for D2b, and §8 must not be read as
covering it.** These members have made an explicit choice; letting a browser header
override it breaks the ladder's first principle exactly as it would on any
community page. The real obstacle is different, and it is an architecture question:

> **`ui_language` is a column on `community_memberships`** (migration
> `0011_membership_ui_language.sql`), not on the user. A member of two communities
> can hold two different preferences, so an account-level page has **no single
> correct answer** to "what language is this member reading in?"

That is a schema-shaped question, not a plumbing one. The options — resolve from
the most recently active membership, require agreement across memberships, or
promote the preference to the user — differ in cost and in what they mean, and one
of them is a migration. **D2b is therefore deferred to its own RFC** and is out of
scope here. Recorded so it is a known question rather than a surprise.

For D2a, RFC-072 named three candidate sources and settled none:

- **`Accept-Language`** — available, costs nothing, and is the browser's own
  answer. But it is attacker-controlled input on routes that redeem secrets, so it
  must be parsed into the closed `Locale` type and never echoed.
- **A community default** — would require resolving the community *before*
  redemption, which on `/join` means touching a community from an unauthenticated
  request. That has an abuse surface and interacts with RFC-076's response
  isolation.
- **A query parameter** — explicit, but becomes a reflected input on exactly the
  routes where reflection is most dangerous.

**This RFC does not settle it.** §8 asks the owner one question, because the
choice is a security decision about untrusted input on the redemption path, not a
localization preference.

### 4.3 The eight constants with no English half

```
JA_ADMIN_ATTENDANCE_SAVED_FLASH          JA_ADMIN_TEMPLATE_SAVED_FLASH
JA_ADMIN_INVITE_REVOKED_FLASH            JA_ADMIN_TEMPLATE_DELETED_FLASH
JA_ADMIN_USE_TEMPLATE_LINK               JA_ADMIN_TEMPLATE_TITLE_REQUIRED_FLASH
JA_ADMIN_ATTEND_MEMBER_ARIA_LABEL        JA_ADMIN_EXPORT_SUMMARY_COUNTS
```

All eight are admin-surface, so they belong to §4.1's slice. Two deserve note:

- `JA_ADMIN_ATTEND_MEMBER_ARIA_LABEL` is an **aria-label**. An unlocalized one is
  an accessibility defect, not a cosmetic one — a screen reader announces it in the
  wrong language while `html lang` claims another. RFC-072 flagged exactly this
  drift as "an accessibility defect, not cosmetic."
- `JA_ADMIN_EXPORT_SUMMARY_COUNTS` carries `{events}` / `{members}` placeholders
  substituted by `.replace()`. Its English half must keep both placeholders, and
  English word order will not match Japanese — so this is the one constant where
  the English is a rewrite rather than a translation.

### 4.4 Structurally unresolvable — 23 sites that should stay pinned

| File | Sites | Why no locale exists |
|---|---|---|
| `render/errors.rs` | 20 | the functions take no arguments — no membership, nothing to resolve |
| `handlers/calendar.rs` | 2 | `get_ics_feed` is an unauthenticated bearer-token route with no membership lookup |
| `handlers/communities.rs` | 1 | `post_matrix_export_audit`'s pre-auth 401 branch rejects before any membership exists |

These are correctly reasoned in the existing table and **stay**. `render/errors.rs`
could in principle be threaded, but doing so means giving every error path a locale
argument, and error paths are precisely where a fail-open or a panic is least
acceptable. **Recommendation: leave all 23 pinned, and say so in the table's
reason text so a future reader knows it was decided rather than missed.**

## 5. Proposed slicing

- **Slice D1 — admin surfaces.** §4.1 plus the eight new English strings in §4.3.
  Largest coverage, lowest risk, no new query cost, no unresolved design question.
- **Slice D2 — anonymous routes.** Blocked on §8's decision. Do not start before it.
- **Slice D3 — the table's closure.** Rewrite the remaining exception reasons to
  state the decision, and add the shrink-only gate from §6.

Static offline HTML and a per-community default, both named in RFC-072's Slice D
sketch, are deferred again and explicitly out of scope here.

## 6. The gate

RFC-072 claimed completion twice while pages were half-migrated. Both times the
gate's unit was the **file** while the property was about the **rendered page**.
That history is the reason for the following requirement:

1. **The exception table may only shrink.** A gate asserts
   `LOCALIZATION_EXCEPTIONS.len()` against a pinned number that decreases with each
   slice. Adding a file back is a test failure requiring an explicit re-pin.
2. **Exact counts stay exact.** The existing per-file `ja_count` assertions are
   equality, not a ceiling — a partial edit to an excluded file fails. Keep that.
3. **Assert on rendered output, not on source files**, for at least one admin page
   per slice: request it under both locales and assert no Japanese codepoint appears
   in the English render. This is the check whose absence caused both prior false
   claims.
4. **Strip comments before scanning.** Standing project rule — three prior gates
   matched their own explanatory prose.

## 7. Sequencing against RFC-054

RFC-054 slice 1 is changing Japanese copy right now, including two admin constants
(`JA_ADMIN_INVITES_REVEAL_WARNING`, and `JA_TZ_ERROR`'s 運営者 → 管理者).

**Slice D1 must not begin until RFC-054 slice 1 has landed.** Doing both at once
means a diff in which changed wording and changed plumbing are indistinguishable,
and the reviewer cannot tell a translation error from a threading error. This is
cheap to honour — RFC-054 slice 1 is a small package — and expensive to unwind.

## 8. Locale resolution where no membership exists — decided

> **Decided 2026-08-16 by nabbisen.** The ladder below is accepted. The floor is
> **Japanese for now**; an English-first default is **planned as a future
> advancement**, recorded in §8.2. Applies to **D2a** — the genuinely anonymous
> routes. Slices D1 and D3 needed no decision.

> On anonymous routes (`/join`, `/relink`, recovery, identity sign-in), where no
> membership and therefore no stored preference exists — should the page honour the
> browser's `Accept-Language` header, or stay Japanese until the visitor has a
> membership?

Accept-Language is untrusted input arriving on the routes that redeem secrets. It
would be parsed into the closed `Locale` type and never echoed, which contains the
injection risk — but it remains a visitor-controlled input influencing what a
redemption page renders, and RFC-076 deliberately narrowed what those pages vary
on.

### 8.1 The accepted rule

A three-rung ladder, first match wins:

| | Source | Applies when |
|---|---|---|
| 1 | Membership preference | a membership exists — **always outranks the rest** |
| 2 | `Accept-Language` | no membership exists |
| 3 | **Japanese** | nothing above resolved |

**Rung 1 outranks rung 2 unconditionally.** `Accept-Language` is the browser's
guess; a stored preference is a member's decision. If a header could override it,
a member who chose Japanese would be shown English because their laptop is set to
English, and the setting would silently stop working. On anonymous routes there is
no stored preference to conflict with, so the header fills a hole rather than
beating anything.

Requirements on rung 2:

- Parse into the closed `Locale` type; anything unparseable falls to rung 3.
- **Never echo the raw header**, anywhere.
- **No `Vary: Accept-Language` needed** — `workers/ssr/src/lib.rs:281` already
  defaults every response without an explicit header to `Cache-Control: no-store`,
  and only `handlers/static_files.rs` opts into public caching. Nothing caches
  these pages. Pin that with a gate rather than relying on it; a future
  `Cache-Control` change would otherwise reintroduce a wrong-language-from-cache
  failure silently.

This is **not** an oracle. The render varies on the visitor's own request header,
not on server state, so it cannot reveal whether a code is valid, who owns it, or
whether a community exists. That is a different concern from the one RFC-076
addresses.

### 8.2 The Japanese floor is provisional, and why

> **Correction, 2026-08-16.** An earlier draft of this section argued for a
> Japanese floor on the grounds that *"the community is Japanese-speaking and
> invite-only; an English-reading visitor on `/join` is a hypothesis, not an
> observed user."* **The first half of that was not evidence.** With no production
> use there are no members at all, so Japanese-speaking members are exactly as
> hypothetical as English-speaking ones. The premise was inferred from the
> codebase's history and the owner's own language and stated as though observed —
> and it applied to Japanese a standard the same sentence applied to English.
> Withdrawn.

What survives is a fact about the artifact, not about users, and it is about
**timing rather than direction**:

**English is currently the less complete surface.** After Slice D1a, **203 render
sites still emit Japanese regardless of locale** — 23 structurally unresolvable
(§4.4) and **180 simply not yet converted** (D1b, D1c, D2). Six constants have no
English half at all. A default user meeting the product today would meet a
substantially Japanese admin surface.

**Decision: Japanese remains the floor for now. An English-first default is
planned as a future advancement**, to be taken when Slice D's remaining 180 sites
are converted and the product can actually serve it.

The change itself is trivial when that time comes — `Locale::default()` at
`packages/contracts/src/locale.rs:43`, one line, no migration, no data touched:
`ui_language` stays NULL everywhere and simply resolves differently. Migration
`0011_membership_ui_language.sql`'s comment (*"NULL means Japanese fallback"*)
would need updating with it, as prose describing intent rather than a constraint.

**Nothing blocks it but readiness.** There is no installed base to disrupt.

## 9. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A slice lands half-migrated and is claimed complete | **Was high, twice** | The exact defect RFC-072 corrected twice | §6.3 asserts on rendered output under both locales, not on source files |
| Copy revision and plumbing land in one diff | Medium | Reviewer cannot separate a translation error from a threading error | §7 sequencing after RFC-054 slice 1 |
| Locale threading adds a D1 query per admin render | Low | Query budget regression (RFC-044) | §4.1 — the value is already fetched and discarded; assert the await count does not move |
| Accept-Language parsing accepts something unexpected | Low | Untrusted input on the redemption path | §8.1 — parse into the closed `Locale` type, never echo, unparseable falls to the floor |
| A cache serves a wrong-language anonymous page | Low | A visitor sees another visitor's language | §8.1 — responses are `no-store` today; pin it with a gate so a future `Cache-Control` change cannot reintroduce this silently |
| The English-first flip lands before Slice D completes | Medium | The default user meets a substantially Japanese product | §8.2 — the flip is sequenced after the remaining 180 sites convert |
| The exception table quietly regrows | Medium | The gap reopens without anyone deciding to reopen it | §6.1 shrink-only assertion |
| `EXPORT_SUMMARY_COUNTS` loses a placeholder in translation | Medium | A render showing a literal `{events}` | Assert both placeholders survive in both halves |

## 10. Acceptance criteria

1. Every admin render site resolves its locale from the `MembershipContext` that
   `require_admin` already returns; none discards it.
2. The eight constants in §4.3 have English halves; the aria-label and the
   placeholder-bearing constant are verified specifically.
3. `LOCALIZATION_EXCEPTIONS` contains only the entries of §4.4, each with a reason
   stating the decision was made rather than deferred.
4. The shrink-only assertion of §6.1 exists and is pinned.
5. At least one admin page per slice is requested under both locales in a smoke,
   with no Japanese codepoint in the English render.
6. The awaited-query count for admin routes is unchanged.
7. No Japanese string's wording changed by this RFC — verified by diffing the JA
   constants against `6ea3765` plus RFC-054 slice 1's known edits.

## 11. Documentation

`docs/src/tester/release-checklist.md` gains a section per slice. RFC-072's
narrative correction quotes `render/errors.rs` at 17 sites; the pinned table now
reads 20, the file having legitimately grown. Add a dated note there so a future
reader takes the count from the table, which is the source of truth, rather than
from the prose.

## 12. Not authorized by this RFC

No deployment, hosted action, secret access, remote D1, tag, push, or release. No
implementation until the owner accepts this RFC and authorizes a slice. B1, B3,
B4, and B5 remain open; production, public-pilot, and first-real-community
deployment remain **No-Go**.
