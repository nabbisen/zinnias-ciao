# RFC-083 — Localization Slice D: Admin, Anonymous, and Unresolvable Surfaces

**Status:** Accepted
**Author:** high-capability model
**Date:** 2026-08-16
**Accepted:** 2026-08-16 by nabbisen. Acceptance authorizes Slices D1 and D3;
**Slice D2 remains blocked on §8**, which acceptance did not settle.
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

Measured against the tree at `6ea3765`, by the same counting method the gate uses
(verified by reproducing four of its pinned counts exactly):

| Quantity | Count |
|---|---|
| Bare `i18n::JA_*` render sites | **409** |
| Localized `i18n::t(...)` sites | 176 |
| Distinct JA constants at bare sites | **224** |
| — of those, **already have an English half** | **216** |
| — of those, needing new English copy | **8** |
| Files in `LOCALIZATION_EXCEPTIONS` | **27** |

**The English copy for this work almost entirely already exists.** 216 of 224
constants are complete `EN_`/`JA_` pairs whose render site simply does not call
`t()`. This is plumbing, not translation. The eight exceptions are listed in §4.3.

The 224/216/8 split was derived twice by independent methods — a name-level
pairing comparison across all 632 `&str` constants, and a reference-level walk of
every bare render site — which agree exactly.

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

~21 files, the large majority of the 409 sites.

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

### 4.2 Anonymous routes — there is no membership to ask

`handlers/join.rs` (18), `handlers/relink.rs` (10), `handlers/recovery.rs` (10),
`handlers/identity/mod.rs` (6), and the account surfaces reached before a
community is chosen.

These run **before** the visitor has a membership, often before they have an
account at all. There is no stored preference to read. RFC-072 named the three
candidate sources and settled none of them:

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

## 8. The one decision required from the owner

**Slice D2 only. Slices D1 and D3 need nothing.**

> On anonymous routes (`/join`, `/relink`, recovery, identity sign-in), where no
> membership and therefore no stored preference exists — should the page honour the
> browser's `Accept-Language` header, or stay Japanese until the visitor has a
> membership?

Accept-Language is untrusted input arriving on the routes that redeem secrets. It
would be parsed into the closed `Locale` type and never echoed, which contains the
injection risk — but it remains a visitor-controlled input influencing what a
redemption page renders, and RFC-076 deliberately narrowed what those pages vary
on.

**My recommendation: stay Japanese for now, and revisit with Stage 0 user
research.** The community is Japanese-speaking and invite-only; an English-reading
visitor on `/join` is a hypothesis, not an observed user. Slice D1 delivers nearly
all the real value without touching the redemption path at all.

## 9. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A slice lands half-migrated and is claimed complete | **Was high, twice** | The exact defect RFC-072 corrected twice | §6.3 asserts on rendered output under both locales, not on source files |
| Copy revision and plumbing land in one diff | Medium | Reviewer cannot separate a translation error from a threading error | §7 sequencing after RFC-054 slice 1 |
| Locale threading adds a D1 query per admin render | Low | Query budget regression (RFC-044) | §4.1 — the value is already fetched and discarded; assert the await count does not move |
| Accept-Language parsing accepts something unexpected | Low | Untrusted input on the redemption path | Deferred entirely by §8's recommendation; if adopted, parse into the closed `Locale` type, never echo |
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
