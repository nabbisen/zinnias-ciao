# RFC-085 — Separate the Locale Fallbacks Before Changing the Default

**Status:** Accepted
**Author:** high-capability model
**Date:** 2026-08-16
**Accepted:** 2026-08-16 by nabbisen. Acceptance authorizes the separation only;
**the default's value is deliberately not changed here** — that remains ROADMAP.md's
open decision, and §7 states why the two are sequenced apart.
**Blocks:** ROADMAP.md § *The default language flips to English when Slice D
completes* — the decision now made due by
`roadmap_english_default_tripwire_fires_when_slice_d_completes`, which has been
failing since `cf3baba`.
**Depends on:** RFC-072 (member language preference), RFC-083 §8.1 (the ladder),
RFC-084 (account tier)

---

## 1. Why this exists

RFC-083 Slice D is complete. `LOCALIZATION_EXCEPTIONS` holds only the three
structurally-unresolvable files, and the tripwire planted for that moment is now
failing by design, saying the English-default decision is due.

The obvious response is one line: change `Locale::default()` from `Ja` to `En`.

**That line does more than it appears to**, and this RFC exists to make it do
exactly what it says before it is taken.

## 2. `impl Default for Locale` answers three questions with one value

Its own doc comment concedes the conflation:

> *"Japanese is the fallback when no membership preference is set, **and** the safe
> fallback when a stored value is outside the allow-list … a bad stored value
> reaching a render path would be an SEC-5 violation."*

Those are not one question:

| Site | Question | Nature |
|---|---|---|
| `db/membership.rs:67` | a member expressed **no preference** | product |
| `db/membership.rs:67` | a stored value is **corrupt** | **safety** |
| `authz.rs:180` | an anonymous visitor's `Accept-Language` matched nothing | product |

All three currently resolve to Japanese, so one constant serves them and the
difference is invisible. **Flipping the default flips all three**, including the
fail-closed one — a corrupt `ui_language` column would begin rendering English
because of a product decision about members who have expressed nothing.

RFC-072 made the safety answer explicit: *"a value outside the allow-list must fail
safe to Japanese rather than panicking — a panic in a render path is an SEC-5
violation."* It said **fail safe**, not *"fail to whatever the product default
happens to be."* Today those coincide. The flip is precisely the event that
separates them, and nothing in the code would notice.

### 2.1 `Default` makes the answer ambient

Beyond the two known sites, `impl Default` is reachable from anywhere: a bare
`Locale::default()`, any `unwrap_or_default()` on an `Option<Locale>`, and any
future `#[derive(Default)]` on a struct that gains a `Locale` field. None of those
states which question it is answering.

Measured at `cf3baba`: **exactly two sites** reach it, and **no struct derives
`Default` with a `Locale` field**. The conflation is cheap to remove *now* and gets
more expensive with every future call site.

## 3. Proposal

### 3.1 Delete `impl Default for Locale`

Removing it makes the ambiguity **untypeable**. Every site must name its answer, and
a future `unwrap_or_default()` cannot silently acquire one. This is the same move as
`resolve_safe_return`'s `&'static str` return and D1's required `locale` parameters:
the wrong thing stops being expressible.

### 3.2 Split `resolve_locale` into its two jobs

`db/membership.rs:67` is one line doing both:

```rust
fn resolve_locale(stored: Option<&str>) -> Locale {
    stored.and_then(Locale::parse).unwrap_or_default()
}
```

Three inputs, currently indistinguishable:

| Stored | Meaning | Answer |
|---|---|---|
| `Some("ja")` / `Some("en")` | the member's preference | that locale |
| `None` (SQL `NULL`) | **no preference expressed** | the *product* default |
| `Some(other)` | **corrupt** — outside the `CHECK` allow-list | a *named fail-closed* constant |

The third is only reachable by manual repair — migration `0011`'s
`CHECK(ui_language IN ('ja','en') OR ui_language IS NULL)` prevents it through the
application. That is exactly why it must stay explicit: it is the path nobody
exercises and therefore the one that quietly acquires the wrong behaviour.

**The fail-closed constant does not move when the product default moves.** That is
the whole point of separating them.

### 3.3 Name the ladder's floor

`authz.rs:180` ends RFC-083 §8.1's ladder with `.unwrap_or_default()`. It should name
rung 3 explicitly, so the ladder reads as three stated rungs rather than two plus
whatever `Default` says.

### 3.4 Then, and only then, the flip

With the three separated, changing the product default is a one-line change to one
named constant, and the fail-closed path is **provably** unaffected — by
construction, not by inspection.

This RFC does not itself change the default. It makes the change safe to make and
reviewable on its own merits.

## 4. Non-goals

- **No new language.** `Locale` stays `ja` | `en`.
- **No migration.** `0011`'s `CHECK` is correct and stays.
- **No copy change**, no route change, no authorization change.
- **This RFC does not flip the default.** That remains ROADMAP.md's decision; §7
  states what changes about how it is taken.
- No change to RFC-083 §8.1's ladder *semantics* — only to how its floor is written.

## 5. Security

The property RFC-072 states and this RFC preserves:

> **Locale selects rendered text and nothing else. A localized string may differ;
> the decision that produced it must not.**

Two invariants:

1. **The fail-closed answer for a corrupt stored value must never move as a side
   effect of a product decision.** Today it would. After this RFC it cannot, because
   the two are different constants with different names and different reasons.
2. **No panic in a render path** (SEC-5). Removing `Default` must not tempt anyone
   toward `expect()` or `unwrap()`; every site returns a named locale.

A gate should assert that `impl Default for Locale` does not exist, so it cannot be
reintroduced as a convenience and quietly re-merge the three answers.

## 6. Acceptance criteria

1. `impl Default for Locale` is deleted, and a gate prevents its return.
2. No `Locale::default()` or `unwrap_or_default::<Locale>()` remains anywhere.
3. `resolve_locale` distinguishes `None` from `Some(unparseable)`, with the
   fail-closed answer a **separately named constant** from the product default.
4. The corrupt-value path has a test proving it does **not** follow the product
   default — the test that would fail if the two were re-merged.
5. `authz.rs`'s ladder names rung 3 explicitly.
6. No panic path introduced; SEC-5 preserved.
7. Suite green except the ROADMAP tripwire, which stays failing until §7 is
   resolved.

## 7. What this changes about the ROADMAP decision

After this RFC, ROADMAP.md § *The default language flips to English when Slice D
completes* becomes a **one-line change to a single named product default**, with the
fail-closed behaviour untouched by construction.

**The tripwire stays red until that decision is taken.** This RFC deliberately does
not resolve it — a red suite is a poor steady state, and the honest sequence is:
separate the fallbacks, then decide the default, then retire the tripwire as part of
*that* work.

## 8. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The flip is taken before this lands | Medium | Fail-closed behaviour silently becomes English | §7 — this RFC blocks that decision |
| `Default` is reintroduced for convenience | Medium | The three answers re-merge invisibly | §5 — a gate asserts its absence |
| Removing `Default` tempts `unwrap()` | Low | SEC-5 panic in a render path | §6.6; every site returns a named locale |
| The corrupt-value path is never exercised | **High** | It is only reachable by manual repair — which is why §6.4 requires a test rather than trusting review |
| Treated as churn with no user behind it | Medium | Nothing is deployed and no member is mis-served today | True, and stated: this buys a design that explains itself before the flip, not a fix for an observed defect |

## 9. Decision taken, 2026-08-16

> **Accepted by nabbisen, 2026-08-16.** Implement before the flip.

There were no options to weigh: the alternative was taking the flip with the
conflation intact, which is correct behaviour but leaves a safety fallback and a
product default sharing one value for no stated reason.

**The ROADMAP English-default decision remains open and unmade.** This RFC makes it
a one-line change to a single named product default whose boundaries are provable;
it does not take it.

## 10. Not authorized by this RFC

No implementation until accepted and a package authorized. **No change to
`Locale::default()`'s value** — that is ROADMAP.md's decision, deliberately left
open. No deployment, hosted action, secret access, remote D1, tag, or release. B1,
B3, B4, and B5 remain open; production, public-pilot, and first-real-community
deployment remain **No-Go**.
