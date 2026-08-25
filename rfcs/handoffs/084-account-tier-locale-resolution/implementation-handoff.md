# RFC-084 — Implementation Handoff: Account-Tier Locale Resolution

**Handoff status:** **Authorized 2026-08-16 by nabbisen.** Implement as specified.
Authorization covers §3–§8 only — §14's exclusions still bind, and nothing here
authorizes a tag, deployment, or deletion of evidence.
**Prepared:** 2026-08-16 by the high-capability model
**Checkpoint:** `e094831` (pushed; `origin/main` in sync)
**Governing RFC:** `rfcs/accepted/084-account-tier-locale-resolution.md`, **§10's two
decisions in particular**. Per RFC-000, this handoff has no lifecycle state of its
own — its status is inherited from that RFC, which is **Accepted**.
**Was:** `.git-exclude/tasks/dev-team/077-rfc084-account-tier-locale-handoff.md`,
relocated 2026-08-16 on the owner's instruction to use `rfcs/handoffs/` for
RFC-companion handoffs.

---

## 1. Task title

Give the account tier a locale, and finish the localization programme.

## 2. Scope: three files, 31 sites

| File | `ja_count` | `bare_helper_calls` |
|---|---|---|
| `workers/ssr/src/handlers/account/mod.rs` | 20 | 1 |
| `workers/ssr/src/handlers/account/unlink.rs` | 6 | 1 |
| `workers/ssr/src/handlers/account/link.rs` | 5 | 1 |

**This is the last convertible localization work.** Everything else in
`LOCALIZATION_EXCEPTIONS` is structurally unresolvable (RFC-083 §4.4).

`account/link.rs` also carries the explicit `Locale::Ja` literal Handoff 075 passed
to `start_oidc_transaction` as a documented D2b placeholder. **Replace it with the
resolved locale**, and update `prompt_login_is_sent_for_link_and_reauthentication`,
which pins that argument's literal value (Handoff 075 §3.5 added that assertion for
exactly this moment).

## 3. The resolution rule

RFC-084 §4A, first match wins:

| | Source | Resolves when |
|---|---|---|
| 1 | Membership preference | **exactly one distinct non-NULL `ui_language`** across the member's present memberships |
| 2 | `Accept-Language` | rung 1 did not resolve |
| 3 | Japanese | neither above resolved |

### 3.1 A clarification of §4A's wording, and why

RFC-084 §4A says rung 1 resolves when *"every present membership carries the same
non-NULL `ui_language`."* Read strictly, a member with `en` in one community and
`NULL` in another would **fall through to the browser header** — ignoring an
explicit choice they did make.

**Implement the distinct-set rule instead:** collect the non-NULL values; resolve
if the set has exactly one element; fall through if it is empty or has more than one.

| Memberships | Resolves to |
|---|---|
| none | rung 2 |
| all NULL | rung 2 — NULL means no preference expressed |
| `en`, NULL | **`en`** — one expressed choice, unambiguous |
| `en`, `en` | `en` |
| `en`, `ja` | rung 2 — two choices, neither about this page |

This is **more faithful to the ladder's first principle**, not less: rung 1 must beat
a browser header wherever a stored preference exists, and one preference plus one
silence is a preference. Disclosed here as an architect's clarification; if the owner
prefers the strict reading, it is a one-line change.

## 4. The query design — do not widen the shared type

RFC-084 §7 requires that reading `ui_language` *"must not widen what that query
returns to any caller that does not need it."* That constraint has teeth:

**`list_communities_for_user` has 23 call sites.** Adding `ui_language` to
`CommunitySummary` would hand a language value to twenty-two callers that do not
want it — and those summaries reach render paths and, through smoke captures,
evidence artifacts. Given the evidence work of Handoffs 065–069, do not widen it.

**Add a sibling** in `workers/ssr/src/db/membership.rs` that runs the same SELECT
plus `m.ui_language` and returns its own type. Then:

- **`account/mod.rs:52`** calls the sibling **instead of**
  `list_communities_for_user` — one query, swapped, **not added**. It already
  renders from those rows; it renders from the sibling's rows instead.
- **`account/link.rs` and `account/unlink.rs`** call the sibling — **one new D1
  query each**, RFC-084 §10 decision 2.

If you find a shape that avoids duplicating the SQL while still not widening
`CommunitySummary`, propose it. The requirement is the property, not the mechanism.

## 5. Query counts — the decision must stay visible

RFC-084 §10 requires per-route counts to be **reported**, so the increase reads as a
deliberate cost:

- `account/mod.rs` — **must not change.** The sibling replaces an existing call.
- `account/link.rs`, `account/unlink.rs` — **+1 each**, expected and authorized.
- **Anywhere else — a rise is a defect**, not this decision. Report all three files
  and confirm nothing else moved.

## 6. This package ends with a FAILING test. That is success.

Removing the last three entries takes `LOCALIZATION_EXCEPTIONS` to exactly the three
structurally-unresolvable paths — which is precisely the trigger for
`roadmap_english_default_tripwire_fires_when_slice_d_completes`.

**It will fire, and it is meant to.** Its message says the English-default decision
in ROADMAP.md § *The default language flips to English when Slice D completes* is now
due.

- **Do not re-pin it.** Do not delete it. Do not add an exception to silence it.
- **Report it as the expected outcome**, with its full message, and stop there.
- The owner resolves the ROADMAP decision; the tripwire is deleted or rewritten as
  part of *that* work, not this.

If the tripwire does **not** fire after your table edit, something is wrong —
that is a stop condition.

Everything else in the suite must pass. A green tripwire and a red anything-else is
the failure mode to report.

## 7. Tables

- **`LOCALIZATION_EXCEPTIONS`**: remove all three entries. **6 → 3 entries,
  54 → 23 `ja` sites, 3 → 0 `bare_helper_calls`.** Re-pin all three aggregates.
- **`EN_JA_PARITY_EXCEPTIONS`** stays empty. **If a constant would make it non-empty,
  stop** — that means new copy, which this package does not authorize.
- The derived helper set stays `{page}`; no helper is removed by this package.

## 8. Rendered-output assertions

An English-locale render assertion for **at least two** of the three pages, one of
which must be `account/mod.rs` (the largest, and the one listing communities).
Include the `Locale::Ja` discriminating half.

Plus **unit tests for the resolution rule itself** — it is the new logic. Cover every
row of §3.1's table, including the mixed `en`/NULL case and the disagreeing case.

## 9. Explicit non-change scope

- **No Japanese wording changed; no new constant.** RFC-054 owns copy.
- **No migration.** RFC-084 chose option A precisely to avoid one.
- No change to `require_account_surface`'s authorization behaviour, or to
  `CommunitySummary`.
- No D3 change; `render/errors.rs` untouched.
- No change to the evidence baseline or the other pinned tables.

## 10. Security constraints

The account tier is where a member **unlinks an identity** and **regenerates a
recovery credential**. RFC-084 §7 names two properties; verify both against the
implementation and report what you checked:

- **RFC-081's last-credential guard.** An unlink refused because it would leave no
  sign-in method must stay refused **in both languages**, with the same generic
  message. Enumerate the refusal branches and confirm each still funnels into one
  constant, then read both halves of that constant — the method D2a used for
  RFC-081 §3.2.
- **The one-time reveal.** `ACCOUNT_RECOVERY_REVEAL_WARNING` is shown once. Localizing
  it must not change what is revealed, or for how long.

`require_account_surface`'s result must still be honoured on every path — adding a
query before or after it must not turn an early return into a fall-through. It
returns `Result<()>`, so its `?` is the whole guard; **read every call site.**

Evidence must not retain display names, community names, identity subjects, recovery
codes, session identifiers, or any other prohibited value.

## 11. Required tests

- `cargo test --workspace` (**653**) / `--features dev_fake_issuer` (**656**) — both
  rise. **Report the arithmetic**, and report the tripwire failure separately from
  the totals so the two are not confused.
- Both table gates pass with the three files removed; all three aggregates re-pinned.
- **The render assertion demonstrated failing** in both classes — a bare `i18n::JA_`
  site and a locale-blind helper.
- **The resolution rule demonstrated failing**: break one §3.1 row deliberately,
  confirm it is caught, restore byte-identical. **Verify the mutation landed first.**
- Per-route query counts per §5.
- `smoke:account-surface`, `smoke:account-link-reauth`,
  `smoke:account-recovery-unlink`, plus **`bun run smoke:all`**. All smokes pin
  `Accept-Language: ja` since Handoff 076, so their existing Japanese assertions
  should hold — **if one flips, say so; that would mean rung 1 is not resolving where
  it should.**
- `node scripts/test-evidence-leakage-baseline.mjs` green at **996**.
- clippy `-D warnings` both feature states, fmt, wasm check, `mdbook build docs`,
  `git diff --check`, `bun run build`.

## 12. Required documentation updates

`docs/src/tester/release-checklist.md` — add only: the three files, the resolution
rule including §3.1's clarification, the two new queries, the three re-pins, and
**that the tripwire now fires and why that is the expected end state.**

## 13. Required review-request format

What you ran and what you observed, separately from what you concluded. Include: the
sibling query's shape and confirmation `CommunitySummary` is unchanged; per-route
query counts for all three files; the tripwire's **full failure message**, reported
as the expected outcome; both render-failure demonstrations and the resolution-rule
one; §10's two verifications with what you checked; and the test-count arithmetic
separated from the tripwire failure.

**Label the checkpoint from `git log -1`.**

## 14. Stop conditions

Stop and escalate if: the tripwire does **not** fire after the table edit; anything
other than the tripwire fails; a query count rises outside `link.rs`/`unlink.rs`;
`CommunitySummary` would have to change; a constant would make the parity table
non-empty; localizing would split a generic refusal into distinguishable causes; or a
pinned smoke assertion flips language.

## 15. Not authorized by this handoff

No deployment, hosted action, secret access, remote D1, tag, RFC lifecycle movement,
finding closure, or release. **No copy revision. No migration. No re-pinning or
deleting the tripwire.** B1, B3, B4, and B5 remain open; production, public-pilot,
and first-real-community deployment remain **No-Go**.
