# RFC-054 — Implementation Handoff: Slice 4, the Occurrence-Cancel Confirm

**Prepared:** 2026-08-26
**Checkpoint:** `77d12ec` — confirm with `git log -1` before starting.
**Origin:** §E of
`.git-exclude/reviewed/zinnias-ciao-main-2026-08-26-rfc054-slice3-copy-decision-list.md`
(findings S4-1 and S4-2), raised during the slice 3 review.
**Predecessor:** `rfcs/handoffs/054-japanese-ux-copy-review/slice-3-implementation-handoff.md`

## 0. Why this slice exists

Slice 3 derived its scope by grepping constant-name suffixes
(`_KEEP|_CONSEQUENCE|_DELETE_BODY|_CANCEL_EVENT_BODY`). The occurrence-cancel
confirm dialog names its body `OCCURRENCE_CANCEL_HELPER`, so it was invisible to
that pattern and went unexamined. **That was an error in my scoping, not in slice
3's delivery** — slice 3 implemented its handoff exactly.

Consequence: slice 3's headline ("four irreversible actions; three said so") is
off by one. There are **five**. This slice closes the fifth.

The lesson, for future slices: derive a surface from **the handlers that render
it**, not from constant-name suffixes.

## 1. Scope: one constant pair changed, one added, one handler line

```
packages/contracts/src/i18n/events.rs
  JA_OCCURRENCE_CANCEL_HELPER   EN_OCCURRENCE_CANCEL_HELPER   (changed)
  JA_OCCURRENCE_CANCEL_KEEP     EN_OCCURRENCE_CANCEL_KEEP     (new)
  OCCURRENCE_CANCEL_KEEP: super::Localized                     (new)

workers/ssr/src/handlers/admin/events/occurrence.rs
  line 58 — the decline link's label
```

Unlike slice 3, **this package touches a handler.** That is deliberate and is
the whole of S4-2; see §2.2 for why it is safe.

## 2. Required implementation

### 2.1 S4-1 — the helper must state permanence

Cancelling one occurrence sets `occurrence_status='cancelled'`
(`workers/ssr/src/db/event_write.rs:228`) and nothing restores it — the three
nearby `occurrence_status='scheduled'` matches are all `WHERE` guards on that
same statement, not a `SET`. Verify this yourself before writing copy that
claims it.

```
JA_OCCURRENCE_CANCEL_HELPER
- この日だけを中止します。同じ繰り返し予定の他の日はそのまま残ります。
+ この日だけを中止します。同じ繰り返し予定の他の日はそのまま残ります。この操作は取り消せません。

EN_OCCURRENCE_CANCEL_HELPER
- Only this date will be cancelled. Other dates in the series stay scheduled.
+ Only this date will be cancelled. Other dates in the series stay scheduled. This cannot be undone.
```

**No remedy sentence — and that omission is deliberate.** Slice 3's cancel bodies
end with "you can create a similar event afterwards" because a real button exists
on that surface. Here there is none:

- the recreate action renders only when the **event** is cancelled
  (`workers/ssr/src/handlers/event.rs:313` —
  `membership.is_admin() && event.status == "cancelled"`), which is not the case
  for a cancelled occurrence of a still-scheduled series; and
- a recurring event's schedule is **never** editable
  (`workers/ssr/src/handlers/admin/events/policy.rs:30` —
  `days.len() == 1 && !event_is_recurring(event)`), so the date cannot be added
  back by editing.

Naming a remedy that has no affordance on the surface would be the slice 1 F1
error class. **If you think a remedy sentence belongs here, say so in the review
request rather than adding one.**

**Keep the full conjugated 「取り消せません」** — the same shared-stem trap slice 3
carried: 取り消せ also prefixes 「取り消せます」 in `JA_ADMIN_SUSPEND_CONSEQUENCE`,
which claims the opposite.

### 2.2 S4-2 — the dialog has no decline label; give it one

Today the decline affordance is a back-link labelled with `EVENT_TITLE_HEADER` —
literally 「イベント」 / "Event"
(`workers/ssr/src/handlers/admin/events/occurrence.rs:58`). That names a noun,
not a choice. It is a worse decline label than the 「戻る」 slice 3 replaced,
and it had no `_KEEP` constant for slice 3's A3 to converge.

Add the pair, placed with its family (after `OCCURRENCE_CANCEL_SUBMIT`'s
`Localized` at `packages/contracts/src/i18n/events.rs:507`, and with the JA/EN
string constants beside their siblings at `:64` and `:131`):

```rust
pub const EN_OCCURRENCE_CANCEL_KEEP: &str = "Keep this date";
pub const JA_OCCURRENCE_CANCEL_KEEP: &str = "やめる";

pub const OCCURRENCE_CANCEL_KEEP: super::Localized = super::Localized {
    ja: JA_OCCURRENCE_CANCEL_KEEP,
    en: EN_OCCURRENCE_CANCEL_KEEP,
};
```

Then in `occurrence.rs`:

```rust
- back = i18n::t(locale, i18n::EVENT_TITLE_HEADER),
+ keep = i18n::t(locale, i18n::OCCURRENCE_CANCEL_KEEP),
```

**Rename the format binding `back` → `keep` and its `{back}` placeholder too.**
A variable named `back` bound to a keep-label is precisely the drift this
programme removes. It is a local format argument; nothing outside this format
call reads it.

**Why this is safe — verify each before editing:**

- `EVENT_TITLE_HEADER` keeps its legitimate use at
  `workers/ssr/src/handlers/event.rs:355` (the event page header). **Do not touch
  that one.**
- The gate that pins it (`packages/contracts/tests/release_gates.rs:3748`) reads
  `EVENT_HANDLER_SRC`, which is `include_str!(".../handlers/event.rs")`
  (`release_gates.rs:257`) — **not** `occurrence.rs`. This change is outside that
  gate's reach. Confirm by reading line 257 yourself.
- The link's `href` (`/c/{cid}/events/{eid}`) is **unchanged** — same destination
  as the event-cancel dialog's keep link. Only the label changes.

**Do not rename the CSS class `cz-admin-occurrence-back-link`.** That is a
stylesheet change with cache-buster consequences and is out of scope.

## 3. Explicit non-change scope

- 中止 vs キャンセル — **untouched, and confirmed correct.** The occurrence surface
  uses 中止 throughout (`OCCURRENCE_CANCEL_ACTION`/`_TITLE`/`_SUBMIT`/
  `_CANCELLED_BADGE`) and its helper says the other dates remain; the split is
  scope-based and deliberate. `scripts/smoke/recurrence-v2.mjs` asserts
  「この日は中止です」.
- `OCCURRENCE_CANCEL_TITLE`, `_ACTION`, `_SUBMIT`, `_CANCELLED_BADGE` — untouched.
- Everything slice 3 changed — untouched.
- `EVENT_TITLE_HEADER`'s value — untouched. Only one of its two call sites moves.
- No schema, route, `href`, authorization, or version change. No CSS.
- No new gate. If you think S4-1's fact deserves pinning, say so in the review
  request; do not add one.

## 4. Required tests

- Take your own pre-package baseline first, then compare:
  `cargo test --workspace --no-fail-fast`, default and `--features dev_fake_issuer`
  (665/0 and 668/0 as of slice 3 — confirm, do not assume).
- **`en_ja_parity_is_derived_from_the_constants_themselves` in isolation.** This
  package **adds a stem**, which is the case that gate exists for. It should pass
  with no exception entry: `OCCURRENCE_CANCEL_KEEP` has both halves, and
  「やめる」 vs "Keep this date" are not identical. **If it demands an entry in
  `EN_JA_PARITY_EXCEPTIONS` or `EN_JA_IDENTICAL_EXCEPTIONS`, stop and report** —
  do not add one.
- **`rfc024_...`-style locale gates and the localization exception table**:
  confirm `LOCALIZATION_EXCEPTIONS` still holds **3 entries / 23 ja / 0
  bare_helper_calls**. `occurrence.rs` is not among the three exempt files, so it
  must stay locale-clean — the new call keeps the `i18n::t(locale, …)` form.
- Grep `scripts/` for 「イベント」 as a decline-link assertion before editing, and
  report. I did not run this one; do not assume it is empty.
- Smokes: **`smoke:recurrence`** — this is the one that actually exercises the
  occurrence-cancel path (`/days/…/cancel`), so unlike slice 3 there **is** real
  coverage here. Then `bun run smoke:all` at **25/25**.
- `node scripts/test-evidence-leakage-baseline.mjs` green at **996**.
- clippy `-D warnings` both feature states, fmt, wasm check
  (`cargo check --target wasm32-unknown-unknown -p zinnias-ciao-ssr` — this
  package changes a worker file, so this one matters more than usual),
  `mdbook build docs`, `git diff --check`, `bun run build`.

## 5. Required documentation updates

`docs/src/tester/release-checklist.md` — add only: that occurrence cancellation
now states it cannot be undone; that the dialog's decline link is now labelled as
a choice rather than 「イベント」; and **the correction that there are five
irreversible actions, not the four slice 3's entry claims.** Correct slice 3's
entry rather than leaving two counts in the file.

## 6. Acceptance criteria

1. All **five** irreversible actions state that they are irreversible.
2. Every destructive confirm in the product has a decline affordance labelled as
   a choice — verified by reading the handlers that render confirms, **not** by
   grepping constant names. That is the mistake that created this slice.
3. Japanese decline labels reading 「やめる」 go from 9 to **10**, confirmed by grep.
4. No gate edited, no exception-table entry added, no CSS touched.
5. `EVENT_TITLE_HEADER` still serves `event.rs:355`, and `release_gates.rs:3748`
   still passes.
6. Test counts unchanged from your baseline, or the movement explained before
   anything is re-pinned.

## 7. Prohibited shortcuts

- Do not add a remedy sentence to S4-1 without raising it first (§2.1).
- Do not edit a gate, exception table, or smoke assertion to accommodate copy.
  If copy and a gate disagree, stop and report.
- Do not rename the CSS class, change the link's `href`, or touch
  `EVENT_TITLE_HEADER`'s other call site.
- No `--force`, `--allow`, or `--skip` on the leakage scanner, and none should be
  added.

## 8. Security constraints

The copy describes; it gates nothing. The **handler** change is a label
substitution in a rendered string — confirm via `git diff` that
`occurrence.rs`'s form `action`, its `_token` hidden input, the `require_admin`
path, and the SQL guards at `event_write.rs:228` are all untouched. The
single-use form-token flow (AD-4) must be byte-identical.

S4-1's claim must stay true of what the code does: if a restore path for
`occurrence_status` is ever added, this copy becomes a lie.

B1, B3, B4, and B5 remain open. Production, public-pilot, and
first-real-community deployment remain **No-Go**.

## 9. Required review-request format

Write to `.git-exclude/review-request/`, following slice 3's structure — what was
run vs. observed, separately from what was concluded. Include: the one-way
verification of `occurrence_status`; the `EVENT_HANDLER_SRC` line-257 check; the
`scripts/` grep for 「イベント」; the isolated parity-gate run **and whether adding
a stem needed anything**; pre/post test counts both feature states; `smoke:recurrence`
results specifically; confirmation of §3 by diff; and whether you think S4-1
warrants a remedy sentence or a gate.

## 10. Not authorized by this handoff

No deployment, hosted action, secret access, remote D1 access, tag, RFC lifecycle
movement, finding closure, release, or version bump. Await review before
committing.
