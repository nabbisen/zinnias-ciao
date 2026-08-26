# RFC-054 — Implementation Handoff: Slice 5, the Admin Hide-Note Confirm

**Prepared:** 2026-08-26
**Checkpoint:** `f74395d` — confirm with `git log -1` before starting.
**Origin:** §2 of `rfcs/proposed/086-derive-the-destructive-confirm-surface.md`
**Predecessor:** `rfcs/handoffs/054-japanese-ux-copy-review/slice-4-implementation-handoff.md`

## 0. Independent of RFC-086

RFC-086 is **Proposed**, not accepted. This package does **not** depend on it and
must not implement any part of it — no derived gate, no `ONE_WAY_TRANSITIONS`
table, no new gate at all. RFC-086 merely found this defect; the defect is real
whether or not the RFC is ever accepted, and fixing it first is that RFC's own
§9.1 recommendation.

## 1. The defect

`workers/ssr/src/handlers/admin/events/notes.rs:52-80` renders the confirmation
an admin sees before hiding **another member's** note. Two things are wrong.

**The body is another dialog's copy.** Line 80 reads:

```rust
consequence = i18n::t(locale, i18n::ADMIN_REMOVE_CONSEQUENCE),
```

That constant describes **removing a member from the community**. An admin
hiding one note is currently told the member will lose access to events and
notes, that they can be re-invited later but as a new member, and that their role
and display name will not carry over. None of it is true here. RFC-054 slice 2
added those re-invitation sentences *for the removal dialog*; sharing the
constant propagated them to a dialog nobody was reviewing.

**The title and button say "delete".** Both use `NOTE_DELETE`
「メモを削除」/"Delete Note", but the form POSTs to `.../notes/{mid}/hide` and
sets `hidden_by_admin_at` (`workers/ssr/src/db/event_note.rs:123`). The file's
own comment at `packages/contracts/src/i18n/notes.rs:29` already records that
"hidden, not removed" — the flash was corrected for exactly this reason; this
dialog was missed.

**The member's own delete dialog is separate and correct** —
`workers/ssr/src/handlers/event.rs:587,611` uses `NOTE_DELETE` and
`NOTE_DELETE_BODY`, where "delete" is accurate. **Do not touch it.** Only the
admin dialog borrows wrongly.

## 2. What the action actually does — verify before writing copy

- Sets `hidden_by_admin_at`, guarded `AND note_deleted_at IS NULL AND
  hidden_by_admin_at IS NULL` (`db/event_note.rs:123-125`).
- **One-way.** No unhide path exists — confirm with a grep for
  `hidden_by_admin_at = NULL` / `unhide` across `workers/ssr/src/`.
- **Hidden from everyone, including the author.** Both read paths filter
  `hidden_by_admin_at IS NULL` (`db/event_note.rs:26` and `:48`) — confirm these
  are the only two SELECTs against that table before relying on it.
- The member's membership, access, and other notes are untouched — confirm no
  membership column is written on this path.

## 3. Required implementation

### 3.1 Three new constant pairs

Add to `packages/contracts/src/i18n/notes.rs`, beside the existing note
constants, each with its `super::Localized` pair following the file's pattern:

```rust
pub const EN_ADMIN_HIDE_NOTE_TITLE: &str = "Hide this note?";
pub const JA_ADMIN_HIDE_NOTE_TITLE: &str = "メモを非表示にしますか？";

pub const EN_ADMIN_HIDE_NOTE_CONSEQUENCE: &str = "This note will no longer be shown to anyone, including the member who wrote it. Their membership and other notes are unaffected. This cannot be undone.";
pub const JA_ADMIN_HIDE_NOTE_CONSEQUENCE: &str = "このメモは誰にも表示されなくなります。書いた本人にも表示されません。メンバーの参加やほかのメモには影響しません。この操作は取り消せません。";

pub const EN_ADMIN_HIDE_NOTE_CONFIRM: &str = "Hide note";
pub const JA_ADMIN_HIDE_NOTE_CONFIRM: &str = "メモを非表示にする";
```

Three notes on the wording:

- **「〜しますか？」 for the title** follows the established confirm-title pattern
  (`ADMIN_CANCEL_EVENT_TITLE`, `ADMIN_DEMOTE_TITLE`, and the others slice 2 read
  and deliberately left alone).
- **「非表示」, not 「削除」**, matches `NOTE_HIDDEN_FLASH`
  「メモを非表示にしました。」 — the dialog and its own result message will finally
  agree.
- **Full conjugated 「取り消せません」** — the shared-stem trap carried since
  Handoff 082: 取り消せ also prefixes 「取り消せます」, which claims the opposite.

**Do not add an unhide claim, and do not say the member is or is not notified** —
nothing in the code establishes notification behaviour either way, and inventing
it is the slice 1 F1 error class.

### 3.2 The handler

In `workers/ssr/src/handlers/admin/events/notes.rs`:

- `nd` currently serves four roles — `<h1>`, the header, the page title, and the
  submit button. Split it: title/header/page-title take
  `ADMIN_HIDE_NOTE_TITLE`; the submit button takes `ADMIN_HIDE_NOTE_CONFIRM`.
- Replace `consequence = i18n::t(locale, i18n::ADMIN_REMOVE_CONSEQUENCE)` with
  `ADMIN_HIDE_NOTE_CONSEQUENCE`.
- **Fix the dangling name.** The body is currently
  `<p class="cz-confirm-body">{consequence} {name}</p>` — the member's name
  trails the sentence with a bare space. Give it its own line above the
  consequence, matching the pattern `cancel.rs` already uses for the event title:

```
<p class=\"cz-confirm-body\"><strong>{name}</strong></p>\
<p class=\"cz-confirm-body\">{consequence}</p>\
```

  **Reuse the existing `cz-confirm-body` class for both.** Do not introduce a new
  class — that would be a stylesheet change with cache-buster consequences, and
  it is out of scope.
- `keep = i18n::t(locale, i18n::NOTE_KEEP_ACTION)` — **unchanged.** Slice 3
  already converged it on 「やめる」, which is correct here too.

## 4. Explicit non-change scope

- `ADMIN_REMOVE_CONSEQUENCE` itself — **untouched.** Its text is correct; the bug
  is that a second dialog borrowed it. After this package it must have exactly
  **one** call site.
- `NOTE_DELETE`, `NOTE_DELETE_BODY` and the member's own delete dialog at
  `event.rs:587,611` — untouched.
- `NOTE_HIDDEN_FLASH`, `NOTE_SAVED_FLASH`, and the comment at
  `packages/contracts/src/i18n/notes.rs:29` — untouched. That comment stays
  accurate and becomes *more* so.
- The form `action`, the `_token` input, `token_purpose::ADMIN_HIDE_NOTE`, and
  every authorization check — untouched.
- No CSS, schema, route, `href`, or version change.
- **No new gate**, and no work from RFC-086. If you think this fact deserves
  pinning, say so in the review request.

## 5. Required tests

- Pre-package baseline first, then compare: `cargo test --workspace
  --no-fail-fast`, default and `--features dev_fake_issuer` (665/0 and 668/0 as
  of slice 4 — confirm, do not assume).
- **`en_ja_parity_is_derived_from_the_constants_themselves` in isolation.** This
  package adds **three** stems. Expect no exception entry in either table; **if
  it demands one, stop and report** rather than adding it.
- **`rfc072_every_handler_and_render_file_is_localized_or_documented_exception`
  in isolation** — this package edits a handler. `LOCALIZATION_EXCEPTIONS` must
  still hold **3 entries / 23 ja / 0 bare_helper_calls**, and
  `admin/events/notes.rs` must not join it: keep the `i18n::t(locale, …)` form.
- **The form-token gate covering `token_purpose::ADMIN_HIDE_NOTE`**
  (`packages/contracts/tests/release_gates.rs:103`) — run it in isolation and
  confirm this package leaves AD-4 intact.
- Grep `scripts/` for the old strings before editing and report — including the
  Japanese removal text, in case any script asserts it on this page.
- Smokes: **there is no smoke covering this dialog** — the two `scripts/` hits
  for `hide` are the form-token replay and concurrency evidence scripts, not UI
  copy. Run `bun run smoke:all` at **25/25** as a regression check and say
  plainly in the review request that no smoke exercises the changed surface.
- `node scripts/test-evidence-leakage-baseline.mjs` green at **996**.
- clippy `-D warnings` both feature states, fmt, `cargo check --target
  wasm32-unknown-unknown -p zinnias-ciao-ssr` (a worker file changes),
  `mdbook build docs`, `git diff --check`, `bun run build`.

## 6. Required documentation updates

`docs/src/tester/release-checklist.md` — add only: that the admin hide-note
confirmation no longer shows member-removal copy; that its title and button now
say hide rather than delete, matching the flash; and that
`ADMIN_REMOVE_CONSEQUENCE` is back to a single call site.

## 7. Acceptance criteria

1. `ADMIN_REMOVE_CONSEQUENCE` has exactly **one** call site — verified by grep
   across `workers/ssr/src/`, not by inspecting the line you changed.
2. The admin hide-note dialog's title, body, and button all describe **hiding a
   note**, and agree with `NOTE_HIDDEN_FLASH`.
3. The member's own delete dialog is byte-identical.
4. No gate edited, no exception-table entry added, no CSS touched, no RFC-086
   work.
5. Test counts unchanged from your baseline, or the movement explained before
   anything is re-pinned.

## 8. Prohibited shortcuts

- Do not "fix" this by giving `ADMIN_REMOVE_CONSEQUENCE` wording generic enough
  to serve both dialogs. That is how the defect arose. Two actions, two constants.
- Do not edit a gate, exception table, or smoke to accommodate copy. If copy and
  a gate disagree, stop and report.
- Do not implement any part of RFC-086.
- No `--force`, `--allow`, or `--skip` on the leakage scanner, and none should be
  added.

## 9. Security constraints

The copy describes; it gates nothing. Confirm via `git diff` that the form's
`action`, its `_token` hidden input, `token_purpose::ADMIN_HIDE_NOTE`, the
`require_admin` path, and the SQL guards at `db/event_note.rs:123-125` are all
byte-identical. The single-use form-token flow (AD-4) must be unchanged.

The new copy's claims must stay true of the code: the note is hidden from
everyone including its author (both read paths filter it), the action is one-way,
and membership is unaffected. If any of those changes, the copy becomes a lie.

B1, B3, B4, and B5 remain open. Production, public-pilot, and
first-real-community deployment remain **No-Go**.

## 10. Required review-request format

Write to `.git-exclude/review-request/`, following slice 4's structure — what was
run vs. observed, separately from what was concluded. Include: the one-way and
read-path verifications from §2; the `ADMIN_REMOVE_CONSEQUENCE` call-site count
before and after; the three isolated gate runs from §5 **and whether adding three
stems needed anything**; the `scripts/` grep; pre/post test counts both feature
states; confirmation of §4 by diff; and whether you think this dialog warrants a
gate or a smoke.

## 11. Not authorized by this handoff

No deployment, hosted action, secret access, remote D1 access, tag, RFC lifecycle
movement (RFC-086 stays **Proposed**), finding closure, release, or version bump.
Await review before committing.
