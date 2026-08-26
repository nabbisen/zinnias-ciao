# RFC-037 — Implementation Handoff: Restore the Form-Token Replay Test

**Prepared:** 2026-08-26
**Checkpoint:** `3f19078` — confirm with `git log -1` before starting.
**Governing RFC:** `rfcs/done/037-token-subject-and-form-token-atomicity.md` (AD-4)
**Origin:** F1 of
`.git-exclude/reviewed/zinnias-ciao-main-2026-08-26-slice5-review-and-ad4-replay-test-finding.md`

## 0. Why this exists

Handoff 037 converted redirect flash values from raw English prose to
snake_case codes, and added a gate
(`rfc072_flash_query_values_are_lowercase_snake_case_codes_not_prose`) whose own
motivating example is the literal string `"...?flash=Note+removed"`.

The conversion reached the handlers. It never reached the two scripts that
**verify AD-4** — they still assert the pre-conversion value. The gate could not
see them: it scans `handlers_and_render_files()`, and these live in `scripts/`.

So the package that eliminated a string left the only test of single-use form
tokens asserting on it.

## 1. The defect, precisely

The handler emits `flash=note_hidden`
(`workers/ssr/src/handlers/admin/events/notes.rs:130`).

### 1.1 `scripts/test-form-token-replay-rejected.mjs` — two defects, one dangerous

```js
:115  assert(first.location.includes('flash=Note+removed'),  …)
:122  assert(!replay.location.includes('flash=Note+removed'), …)
```

- **Line 115 is false today**, so the test throws before reaching its subject.
- **Line 122 is the replay-rejection assertion, and it now passes
  unconditionally** — no location contains that string any more, so it holds
  whether or not replay protection works. **If line 115 is "fixed" by deleting
  it, this test goes green while verifying nothing.** That outcome is worse than
  the current loud failure, and avoiding it is this package's real purpose.

### 1.2 `scripts/collect-evidence-e4-concurrency.mjs:341-342`

```js
const hideWinners = hideResults.filter((r) => … r.location.includes('flash=Note+removed'));
const hideReplays = hideResults.filter((r) => … !r.location.includes('flash=Note+removed'));
```

`hideWinners` is always empty. The artifact it emits reports that **zero**
legitimate submissions succeeded and **all** were replays — the inverse of the
truth, in an RFC-050 evidence artifact. Wrong evidence is worse than none.

### 1.3 Provenance — verify, do not take from here

`git log -S"flash=note_hidden" -- workers/ssr/src/handlers/admin/events/notes.rs`
→ `2d2be47`. `git log -S"Note+removed" -- scripts/test-form-token-replay-rejected.mjs`
→ `c55787a`, the script's original addition. The scripts were never revisited.

### 1.4 What the code actually does — establish before asserting on it

- **Success:** `redirect("/c/{cid}/events/{eid}?flash=note_hidden")`
  (`notes.rs:130`).
- **Replay:** `return redirect("/c/{cid}/events/{eid}")` — the bare path, **no
  query string at all** (`notes.rs`, the `consume_token` failure branch ~`:109`).
- **No audit row is written** for hide-note — confirmed by grep; there is no
  audit table on this path. The redirect is therefore the *only* observable
  difference between a winner and a replay. Re-verify this yourself: if it is
  wrong, §2.2's design needs revisiting.

## 2. Required implementation

### Part A — derive the flash code; never retype it

A literal `'note_hidden'` in the scripts is the same defect one rename later.
Add a helper — suggested `scripts/lib/flash-code.mjs` — that **reads the handler
source and extracts the code the handler actually emits**, keyed by the function
that emits it:

```js
// throws loudly if the function is not found, or emits no ?flash= at all
export function flashCodeEmittedBy(handlerRelPath, fnName) { … }
```

Then both scripts obtain the expected value through it. No flash-code string
literal may remain in either script — that is the acceptance test for Part A.

Note there is **no existing precedent** for programmatic Rust-source derivation
in `scripts/` (`smoke-fixture-locale.mjs` and `language-preference.mjs` only
*reference* Rust files in comments). You are establishing the pattern; keep it
small and make its failure mode loud.

### Part B — make the replay assertion positive and falsifiable

Replace line 122's `!includes(...)`. Assert what a rejected replay **is**, not
what it is not:

```js
assert(
  replay.location === eventDetailPath,   // exact, no query string
  `replayed token must redirect to the bare event page, got "${replay.location}"`,
);
```

This fails when replay protection fails, which the current form cannot. Keep the
winner assertion positive too, using Part A's derived code.

Apply the same correction to `collect-evidence-e4-concurrency.mjs`'s
winner/replay partition, and re-check the narrative string at `:346` — it
describes what the partition found, so a wrong partition made it wrong prose too.

### Part C — both-direction demonstration *(mandatory; this is the point)*

Per Handoff 082's standard, and because this test has apparently never done
either:

1. **Break replay protection deliberately** — make `consume_token` accept a
   replayed token — and confirm the restored test **FAILS**, quoting the exact
   message.
2. **Restore it** (`git checkout --`, confirm byte-identical via `git diff
   --stat`) and confirm it **passes**.
3. **Rename the flash code** in the handler to something else, confirm the test
   still passes (proving Part A's derivation works), then restore.

A restored test that has not been shown to fail is not a test. Report all three.

### Part D — close the run-set blind spot

`SMOKE_COVERAGE_EXCEPTIONS` exists because "eight of twenty-four smoke scripts
went unrun without anyone noticing" — but it scans `scripts/smoke/` only. These
two scripts live one directory up, are runnable by name
(`test:form-token-replay-rejected`, `evidence:e4-concurrency`), and nothing
asserts they are ever run.

Extend the coverage gate to `scripts/*.mjs`, or add a pinned table naming each
top-level script with a written reason for having no run-set membership. Same
shape as its siblings: explicit paths, stale-entry assertion, default-fail.

**If this turns out to need redesign rather than extension, stop and report
rather than half-doing it** — Parts A–C stand on their own.

## 3. Explicit non-change scope

- **No production Rust changes.** `consume_token`, `token_purpose::ADMIN_HIDE_NOTE`,
  the hide handler, and every flash value stay exactly as they are. Part C's
  mutations are demonstrations, applied and reverted, never committed.
- No copy changes. No RFC-054 work. No RFC-086 work — it stays **Proposed**.
- `flash=Code+revoked` at `workers/ssr/src/handlers/admin/members.rs:743` is
  **synthetic test input** to `invite_get_preflight`, not a live redirect. I
  checked. Leave it alone.
- No regenerated evidence artifacts. Fixing the script does not mean re-running
  the campaign — that is an owner decision under RFC-050.

## 4. Required tests

- Pre-package baseline first: `cargo test --workspace --no-fail-fast`, both
  feature states (665/0, 668/0 as of `3f19078` — confirm, do not assume).
  Expected unchanged unless Part D adds a gate, in which case the count moves by
  exactly the number of tests added and you say so.
- `bun run test:form-token-replay-rejected` — **must now actually pass**, and
  must have been demonstrated failing per Part C.
- `bun run evidence:e4-concurrency` — run it and report the winner/replay counts.
  They should be non-degenerate; if `hideWinners` is still 0, Part B is not done.
- `bun run smoke:all` at **25/25**.
- `node scripts/test-evidence-leakage-baseline.mjs` — expected unchanged at
  **996**. This package edits scripts, not evidence artifacts; if the count
  moves, stop and explain before re-pinning.
- clippy `-D warnings` both feature states, fmt, wasm check, `mdbook build docs`,
  `git diff --check`, `bun run build`.

## 5. Acceptance criteria

1. No flash-code string literal remains in either script — the value is derived.
2. The replay assertion is positive and has been **demonstrated failing** when
   replay protection is broken.
3. Renaming the handler's flash code does not break the test (Part A works).
4. `evidence:e4-concurrency` produces a non-degenerate partition.
5. No production Rust file differs from `3f19078` — confirmed by `git diff`.
6. Part D either lands, or is reported with a specific reason it needs redesign.

## 6. Prohibited shortcuts

- **Do not delete line 115 to make the test green.** That produces a test that
  passes while checking nothing — the outcome §1.1 exists to prevent.
- Do not hardcode `'note_hidden'`.
- Do not weaken or delete the replay assertion; strengthen it.
- Do not change the handler to match the scripts. The handler is correct.
- Do not regenerate evidence artifacts.
- No `--force`/`--allow`/`--skip` on the leakage scanner, and none should be added.

## 7. Security constraints

This restores the only automated verification of AD-4's single-use form tokens.
Treat a green test as meaningless until Part C has shown it red.

Nothing in this package may weaken a guard to make a test pass. If the restored
test fails against **unmodified** production code, that is not a test bug to
work around — **stop immediately and report it**, because it would mean AD-4's
replay protection does not hold, which no part of this handoff assumes.

No fail-open path may be introduced. B1, B3, B4, B5 remain open; production,
public-pilot, and first-real-community deployment remain **No-Go**.

## 8. Required review-request format

Write to `.git-exclude/review-request/`, following slice 5's structure — what was
run vs. observed, separately from what was concluded. Include: §1.3's provenance
re-verified; §1.4's replay-branch and no-audit findings confirmed or corrected;
Part C's three demonstrations with exact failure messages; the
`evidence:e4-concurrency` counts before and after; whether Part D landed; and
whether any other script asserts on a value the handlers no longer emit — **that
class is the actual lesson here, and this package fixes only the two instances
already found.**

## 9. Not authorized by this handoff

No deployment, hosted action, secret access, remote D1 access, tag, RFC lifecycle
movement, finding closure, release, version bump, or evidence regeneration.
Await review before committing.
