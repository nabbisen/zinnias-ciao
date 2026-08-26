# RFC-084 — Account-Tier Locale Resolution

**Status:** Done (0.63.0)
**Author:** high-capability model
**Date:** 2026-08-16
**Accepted:** 2026-08-16 by nabbisen. **Option A**, with `link.rs`/`unlink.rs`
resolving via one new query each — see §10, where both decisions are recorded.
**Shipped:** `cf3baba`, which also closed RFC-083 Slice D in full.
**Discharges:** RFC-083 §4.2's **D2b**, deferred there because *"`ui_language` is a
column on `community_memberships`, not on the user … an account-level page has no
single correct answer."*
**Depends on:** RFC-072 (member language preference), RFC-083 Slice D2a (the ladder,
settled 2026-08-16)

---

## 1. What is left, and why it was deferred

RFC-083 Slice D converted 17 admin files and the four anonymous routes. Three files
remain convertible, all at the **account tier**:

| File | `ja_count` | `bare_helper_calls` |
|---|---|---|
| `workers/ssr/src/handlers/account/mod.rs` | 20 | 1 |
| `workers/ssr/src/handlers/account/unlink.rs` | 6 | 1 |
| `workers/ssr/src/handlers/account/link.rs` | 5 | 1 |
| **Total** | **31** | **3** |

Every other entry in `LOCALIZATION_EXCEPTIONS` is structurally unresolvable
(RFC-083 §4.4). **These three are the last convertible work in the localization
programme.**

They were deferred because they are neither of the two cases Slice D solved. Admin
surfaces have a membership and therefore a stored preference. Anonymous routes have
nothing, so RFC-083 §8.1's rung 2 reads `Accept-Language`. **The account tier is
authenticated but not community-scoped** — `authz::require_account_surface` checks
session provenance and eligibility, never a community.

So a signed-in member reaches these pages holding zero, one, or several
`ui_language` values, possibly disagreeing.

## 2. The storage location was a deliberate decision, not an accident

RFC-072 chose `community_memberships` with a stated reason:

> *"The preference should apply to the current signed-in member's experience in the
> current community. This matches the current membership-centered data model:
> display name, role, access, calendar feed, and member settings are all tied to
> `community_memberships`."*

**Display name is per-membership too.** A user genuinely presents differently in
different communities in this product; language sitting beside display name is
consistent with that model, not a mistake to correct.

**This RFC does not propose overturning it lightly.** Any option that moves the
column is proposing that language is a property of a *person* while display name
remains a property of a *membership* — a real asymmetry that needs arguing, not
assuming.

## 3. The reframe: rung 1 outranks rung 2 only when it *resolves*

RFC-083 §4.2 ruled `Accept-Language` out for D2b on this ground:

> *"These members have made an explicit choice; letting a browser header override it
> breaks the ladder's first principle."*

**That reasoning is incomplete, and I wrote it.** It holds when a stored preference
*applies*. At the account tier it may not:

- a member with **no** memberships has expressed nothing;
- a member whose memberships are **all NULL** has expressed nothing — NULL means
  "no preference", and RFC-072's Japanese fallback is a *rendering* default for a
  community page, not an expressed choice;
- a member whose memberships **disagree** has expressed two things, neither of which
  is about this page.

In none of those cases does rung 2 override a choice. **It fills a hole**, which is
exactly what RFC-083 §8.1 designed it to do. The ladder's first principle — a stored
preference always beats a browser header — is preserved by requiring rung 1 to
*resolve unambiguously* before it wins.

## 4. Options

### A — Unambiguous membership agreement, else the existing ladder *(recommended)*

Rung 1 resolves **only** when every present membership carries the same non-NULL
`ui_language`. Otherwise fall to rung 2 (`Accept-Language`), then rung 3 (Japanese).

- No migration. No schema change. No new preference for a member to manage.
- A member who set English everywhere gets English — the case that actually matters.
- A member who deliberately differs by community is not silently assigned one of
  their choices; their browser decides, and their per-community settings are
  untouched.
- Consistent with the ladder D2a already implements, rather than a fourth rule.

**Cost is uneven, and §5 is where this option is decided.**

### B — Promote `ui_language` to the user *(migration)*

Move the column to `users`; the account tier reads it directly and every community
page reads it too.

- Simplest resolution everywhere, permanently.
- **Contradicts RFC-072's stated reasoning** and makes display name the odd one out.
- **Removes a capability**: a member who wants Japanese in one community and English
  in another can no longer have it. Nobody has asked for that, but nobody has asked
  for anything — there are no members.
- A migration. Migration immutability begins at first deployment (ROADMAP, owner
  decision 2026-08-10) and the service has never been deployed, so the cost is a
  local `bun run reset:dev`, not a forward migration.

### C — A separate account-tier preference *(migration + UI)*

A new column expressing the account surface's own language, distinct from any
membership's.

- Honest about the account tier being its own scope.
- Most expensive: migration, a settings surface, and a third thing to keep coherent.
- **Not recommended** — it solves a problem no observed member has.

### D — Leave the three pinned, permanently

Record them as a fourth structurally-unresolvable case and stop.

- Zero cost, and defensible if the account tier is considered marginal.
- But it leaves a signed-in English-reading member with a Japanese account page,
  which is precisely the defect RFC-072 exists to remove.

## 5. The cost question that decides option A

`account/mod.rs:52` already calls `list_communities_for_user`, which reads
`community_memberships` — **adding `m.ui_language` to that SELECT costs no
additional query.**

`account/link.rs` and `account/unlink.rs` make **zero** membership calls today.
Resolving a locale there means **one new D1 query per route**, against RFC-044's
query budget.

Three ways out, and this RFC does not choose:

1. **Accept one query** on two low-traffic, human-paced routes (a member links or
   unlinks an identity rarely).
2. **Resolve once at the session boundary** and carry it, so all three pages share a
   single lookup — larger change, touches session handling.
3. **Split the tier**: `mod.rs` uses the full ladder; `link.rs`/`unlink.rs` use
   rung 2 only. Cheapest, and **inconsistent within one tier** — I would rather pay
   the query than explain why two pages next to each other resolve differently.

**This is the decision the RFC needs, and it is an owner decision** because it trades
a D1 query budget against internal consistency.

## 6. Non-goals

- No change to what any string says — RFC-054 owns copy.
- No new language; `Locale` stays `ja` | `en`.
- No change to `require_account_surface`'s authorization behaviour. This RFC is
  about language, never about who may see the page.
- No change to RFC-083 §8.1's ladder for the routes it already governs.

## 7. Security constraints

The account tier is where a member **unlinks an identity** and **regenerates a
recovery credential**. Two properties must survive whatever is chosen:

- **RFC-081's last-credential guard** — an unlink refused because it would leave no
  sign-in method must stay refused, in both languages, with the same generic
  message. A translation must not split one refusal into distinguishable causes.
- **The one-time reveal** — `ACCOUNT_RECOVERY_REVEAL_WARNING` is shown once and
  never again. Localizing it must not change what is revealed or for how long.

If option A is chosen, reading `ui_language` alongside an existing membership query
must not widen what that query returns to any caller that does not need it.

## 8. Acceptance criteria

1. All three files resolve a locale; `LOCALIZATION_EXCEPTIONS` reaches **3 entries /
   23 sites / 0 bare helper calls** — only the structurally-unresolvable set.
2. **The Slice D completion tripwire fires**, and is resolved rather than re-pinned:
   ROADMAP.md's *The default language flips to English when Slice D completes* becomes
   due at that moment. That is by design.
3. Rung 1 outranks rung 2 wherever a stored preference resolves unambiguously —
   asserted, not assumed.
4. A member with disagreeing memberships is handled by a stated rule, with a test.
5. Per-route query counts reported; any increase is the §5 decision, taken
   deliberately.
6. §7's two properties verified against the implementation.
7. No copy change, no authorization change.

## 9. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| A stored preference is overridden by a header | Low | Breaks the ladder's first principle | §3 — rung 1 wins whenever it resolves; only genuine ambiguity falls through |
| Option B silently removes per-community language | Medium | A capability disappears with no member to consult | §4B states it plainly; the owner decides with it visible |
| A new D1 query lands on a budgeted route | Medium | RFC-044 budget regression | §5 makes it an explicit decision, not a side effect |
| Localizing splits a generic refusal | Low | RFC-081 §3.2 disclosure property | §7, verified against the implementation as D2a did |
| The tripwire fires and is re-pinned to silence it | **Medium** | The English-default decision is lost again | §8.2 — it is *meant* to fire; resolving it is the work |

## 10. The decisions, taken 2026-08-16

> **Decided by nabbisen, 2026-08-16.**

**1. Option A** — unambiguous membership agreement, else the existing ladder. No
migration; RFC-072's per-membership model stands; per-community language stays
available; the resolution rule is the one D2a already proved.

**2. `link.rs` and `unlink.rs` resolve via one new D1 query each** — §5's option 1,
the architect's stated preference, accepted with the RFC.

The alternative was rung 2 only on those two pages, which is cheaper and leaves two
adjacent pages in one tier resolving differently. **Internal consistency was
preferred over one query on two human-paced routes** — a member links or unlinks an
identity rarely, and RFC-044's budget is not under pressure from either.

The implementing package must still **report the per-route query counts**, so the
increase is visible as a deliberate cost rather than absorbed silently. A rise
anywhere other than these two routes is a defect, not this decision.

## 11. Not authorized by this RFC

No implementation until this RFC is accepted and a slice authorized. No deployment,
hosted action, secret access, remote D1, tag, or release. B1, B3, B4, and B5 remain
open; production, public-pilot, and first-real-community deployment remain **No-Go**.
