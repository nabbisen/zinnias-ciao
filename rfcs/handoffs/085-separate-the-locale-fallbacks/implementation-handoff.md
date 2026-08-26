# RFC-085 — Implementation Handoff: Separate the Locale Fallbacks

**Handoff status:** **Authorized 2026-08-16 by nabbisen.** Implement as specified.
Authorization covers §3–§7 only — §12's exclusions still bind, and nothing here
authorizes a tag, deployment, or deletion of evidence.
**Prepared:** 2026-08-16 by the high-capability model
**Checkpoint:** `26cc28b` (pushed; `origin/main` in sync)
**Governing RFC:** `rfcs/done/085-separate-the-locale-fallbacks.md`. Per RFC-000
this handoff has no lifecycle state of its own — its status is inherited from that
RFC, which is **Done (0.63.0)**.

---

## 1. Task title

Make `Locale`'s three fallbacks three named answers, so the default can be changed
without moving a safety behaviour.

## 2. The suite is already red, and stays red

`roadmap_english_default_tripwire_fires_when_slice_d_completes` has failed since
`cf3baba`, by design (RFC-084's package). **This package does not resolve it.**

- Expect **exactly one** failing test throughout: that one.
- **Do not re-pin, delete, or add an exception to it.** RFC-085 §7 sequences its
  retirement with the ROADMAP decision, not with this work.
- **Report passing and failing counts separately**, as RFC-084's package did, so the
  arithmetic is not confused with a regression.

Anything failing *besides* the tripwire is a stop condition.

## 3. Required implementation

### 3.1 Delete `impl Default for Locale`

`packages/contracts/src/locale.rs:37–44`. Removing it makes the conflation
untypeable — the same move as `resolve_safe_return`'s `&'static str` return and D1's
required `locale` parameters.

Measured at `cf3baba` and re-verify before starting: **two** sites reach it
(`workers/ssr/src/db/membership.rs:67`, `workers/ssr/src/authz.rs:180`), and **no
struct derives `Default` with a `Locale` field**. If your own count differs,
investigate before proceeding.

### 3.2 Split `resolve_locale` into its two jobs

`workers/ssr/src/db/membership.rs:67` is one line doing both:

```rust
fn resolve_locale(stored: Option<&str>) -> Locale {
    stored.and_then(Locale::parse).unwrap_or_default()
}
```

Three inputs must become three distinguishable outcomes:

| Stored | Meaning | Answer |
|---|---|---|
| `Some("ja")` / `Some("en")` | the member's preference | that locale |
| `None` (SQL `NULL`) | **no preference expressed** | the *product* default |
| `Some(other)` | **corrupt** — outside `0011`'s `CHECK` | a *named fail-closed* constant |

**The product default and the fail-closed constant must be separately named**, with
their own doc comments saying which question each answers and — for the fail-closed
one — that it does **not** move when the product default does. That sentence is the
package's whole purpose; write it deliberately.

Both are `Locale::Ja` today. **Do not change either value.**

### 3.3 Name the ladder's floor

`workers/ssr/src/authz.rs:180` ends RFC-083 §8.1's ladder with
`.unwrap_or_default()`. Replace it with the named product default so the ladder reads
as three stated rungs rather than two plus whatever `Default` said.

Its doc comment already describes rung 3 as `Locale::default()` — update it.

### 3.4 Gate the impl's absence

Add a gate asserting `impl Default for Locale` does not exist, so it cannot return
as a convenience and silently re-merge the three answers.

**Strip comments before scanning** — standing rule; six gates in this project have
matched their own explanatory prose, and this one will be surrounded by prose about
the impl it forbids.

Assert on the *impl*, not on the string `Default` — `#[derive(Debug, Clone, Copy,
PartialEq, Eq)]` and unrelated `Default` derives elsewhere must not trip it.

## 4. Explicit non-change scope

- **No value changes.** `Locale::default()`'s replacement is Japanese; the
  fail-closed constant is Japanese. **This package does not flip anything.**
- No new language; `Locale` stays `ja` | `en`.
- No migration. `0011`'s `CHECK` is correct and stays.
- No copy change, no route change, no authorization change.
- No change to RFC-083 §8.1's ladder *semantics* — only how its floor is written.
- **No touching the ROADMAP tripwire.**

## 5. Required tests

- **The corrupt-value path proven to not follow the product default.** RFC-085 §6.4
  requires this specifically: a test that would **fail if the two were re-merged**.
  Construct it so that if someone later points both at one constant, this test
  breaks. That is the package's central assertion.
- Unit tests for all three `resolve_locale` inputs: a valid code, `None`, and an
  unparseable `Some`.
- **The new gate demonstrated failing**: reintroduce `impl Default for Locale`
  temporarily, confirm the gate catches it, restore byte-identical. **Verify the
  mutation landed** before trusting the result.
- **No panic path introduced** (SEC-5). Confirm no `unwrap()`, `expect()`, or panic
  reachable from a render path was added — removing `Default` must not tempt one.
  Say what you checked.
- `cargo test --workspace --no-fail-fast` and `--features dev_fake_issuer`. Current:
  **659 passed / 1 failed / 660** and **662 / 1 / 663**. Report passing, failing, and
  total separately, and the arithmetic for what you added.
- `bun run smoke:all` at **25/25** — this package changes no rendered text, so a
  smoke failure means something unintended.
- `node scripts/test-evidence-leakage-baseline.mjs` green at **996**.
- clippy `-D warnings` both feature states, fmt, wasm check, `mdbook build docs`,
  `git diff --check`, `bun run build`.

## 6. Required documentation updates

`docs/src/tester/release-checklist.md` — add only: the three fallbacks now named
separately, which is which, and that the fail-closed answer does not move with the
product default.

## 7. Acceptance criteria

1. `impl Default for Locale` deleted; a comment-stripping gate prevents its return,
   demonstrated failing.
2. No `Locale::default()` or `unwrap_or_default()` on an `Option<Locale>` anywhere.
3. `resolve_locale` distinguishes `None` from `Some(unparseable)`; the fail-closed
   answer is a separately named constant with a doc comment stating it does not move
   with the product default.
4. A test that fails if the two constants are re-merged.
5. `authz.rs`'s rung 3 named explicitly; its doc comment updated.
6. No panic path introduced; no value changed.
7. Exactly one failing test — the ROADMAP tripwire — untouched.

## 8. Prohibited shortcuts

- No changing either constant's value.
- No re-pinning, deleting, or excepting the tripwire.
- No `unwrap()`/`expect()` to replace what `Default` provided.
- No single constant serving both the product default and the fail-closed answer
  "because they're both Japanese" — that is the defect.
- No gate that matches the bare word `Default`.

## 9. Security constraints

RFC-072's invariant, which this package exists to protect:

> **Locale selects rendered text and nothing else. A localized string may differ;
> the decision that produced it must not.**

Two properties:

1. **The fail-closed answer for a corrupt stored value must never move as a side
   effect of a product decision.** Today it would. After this it must not be able to
   — enforced by §5's re-merge test, not by review.
2. **No panic in a render path** (SEC-5). A corrupt `ui_language` reaching a render
   must resolve to a named locale, never panic.

Evidence must not retain any prohibited value.

## 10. Required review-request format

What you ran and what you observed, separately from what you concluded. Include: your
own count of sites reaching `Default` before starting; the two constants' names and
doc comments; the re-merge test and why it would fail if they were merged; the gate
demonstration; what you checked for panic paths; and passing/failing/total counts
reported separately.

**Label the checkpoint from `git log -1`.**

## 11. Stop conditions

Stop and escalate if: more than two sites reach `Default`; a struct derives `Default`
with a `Locale` field; removing the impl requires a panic or an `unwrap()` anywhere;
anything other than the tripwire fails; or a smoke's rendered text changes.

## 12. Not authorized by this handoff

No deployment, hosted action, secret access, remote D1, tag, RFC lifecycle movement,
finding closure, or release. **No change to any locale value. No migration. No
touching the ROADMAP tripwire.** B1, B3, B4, and B5 remain open; production,
public-pilot, and first-real-community deployment remain **No-Go**.
