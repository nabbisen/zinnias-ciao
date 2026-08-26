# RFC-054 — Implementation Handoff: Slice 3, Destructive-Confirm Copy

**Prepared:** 2026-08-26
**Checkpoint:** `7be273c` — confirm with `git log -1` before starting.
**Decision list:**
`.git-exclude/reviewed/zinnias-ciao-main-2026-08-26-rfc054-slice3-copy-decision-list.md`
**Predecessor:** `rfcs/handoffs/054-japanese-ux-copy-review/slice-2-implementation-handoff.md`

## 0. Status of the wording in this handoff

Every string below is a **proposal**, drafted against the decision list's §D
recommendations. Part 3.4 (A4) is **separable** — if the owner declines it,
drop §3.4 entirely and the rest stands unchanged; nothing else depends on it.

Do not begin until nabbisen has confirmed §D. If any wording arrives changed,
the changed wording wins over the text here.

---

## 1. Task title

Make the event-cancellation confirm dialog tell the truth about permanence,
and finish the decline-button convergence slice 1 started.

## 2. Scope: eight constants, two files

```
packages/contracts/src/i18n/events.rs
  JA_ADMIN_CANCEL_EVENT_BODY            EN_ADMIN_CANCEL_EVENT_BODY
  JA_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS   EN_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS
  JA_ADMIN_CANCEL_EVENT_KEEP            EN_ADMIN_CANCEL_EVENT_KEEP

packages/contracts/src/i18n/notes.rs
  JA_NOTE_KEEP_ACTION                   (EN half unchanged — already "Keep note")
  JA_NOTE_DELETE_BODY                   (§3.4 only; EN half unchanged)
```

**5 JA + 3 EN = 8.** Not four pairs — A4 and `NOTE_KEEP_ACTION` are
Japanese-only, because the English halves are already correct.

Locate constants **by name**, not by the line numbers in the decision list.
Editing multi-line string constants shifts every number below it.

## 3. Required implementation

### 3.1 A1 + A2 — both cancel bodies

Cancellation is one-way: `cancelled_at` is written at
`workers/ssr/src/db/event_write.rs:407` and there is no clearing path. The
current body says only that members will still see the cancellation — nothing
about permanence. Fix both bodies to the shape slice 2 settled on:
**consequence → irreversibility → what you can do instead.**

```
JA_ADMIN_CANCEL_EVENT_BODY
- メンバーにはキャンセルされたことが引き続き表示されます。
+ メンバーにはキャンセルされたことが引き続き表示されます。参加の回答も、これ以上変更できなくなります。この操作は取り消せません。あとで似た内容の新しいイベントを作成することはできます。

EN_ADMIN_CANCEL_EVENT_BODY
- Members will still see that it was cancelled.
+ Members will still see that it was cancelled. Attendance answers can no longer be changed. This cannot be undone. You can create a similar event afterwards.

JA_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS
- このイベントのすべての日程をキャンセルします。参加の回答も、これ以上変更できなくなります。
+ このイベントのすべての日程をキャンセルします。参加の回答も、これ以上変更できなくなります。この操作は取り消せません。あとで似た内容の新しいイベントを作成することはできます。

EN_ADMIN_CANCEL_EVENT_BODY_ALL_DAYS
- All dates for this event will be cancelled. Attendance answers can no longer be changed.
+ All dates for this event will be cancelled. Attendance answers can no longer be changed. This cannot be undone. You can create a similar event afterwards.
```

After this, the two bodies differ **only in their first sentence** — the scope,
which is the only thing that actually differs between the two actions.

**On A2 (why the single-event body gains the attendance sentence):**
`workers/ssr/src/handlers/admin/events/cancel.rs:82-85` picks the body by
`whole_event_scope` but writes the same `status='cancelled'` either way, and
`workers/ssr/src/handlers/event.rs:140` and `:447` disable the attendance form
on that status. The fact was always equally true of both paths; only one dialog
said it.

### 3.2 Two wording traps in §3.1 — read before editing

**Trap 1 — do not rename the recreate button in prose.** The button reads
「似た内容で新しいイベントを作成」 / "Create similar event"
(`packages/contracts/src/i18n/events.rs`, `*_ADMIN_RECREATE_EVENT_ACTION`). My
first draft of the sentence above said 「同じ内容で作り直す」, which would have
described a button by a name it does not have — the exact defect slice 1's F1
found. The wording above matches the real label; **if you change it, keep it
matching.** Re-grep the label before you finish, in case it has moved.

**Trap 2 — 取り消せ is a shared stem.** Handoff 082 pinned
`JA_ADMIN_REMOVE_CONSEQUENCE.contains("取り消せません")` as the **full conjugated
form**, precisely because 取り消せ is also a prefix of 「取り消せます」 in
`JA_ADMIN_SUSPEND_CONSEQUENCE`, which claims the opposite. Use
「取り消せません」 exactly. Do not abbreviate, and do not touch the suspend
constant.

**Do not repeat the carry-over detail.** `ADMIN_RECREATE_EVENT_HELPER` already
explains what a recreated event does and does not inherit, at the point of use.
The confirm body deliberately stops at "you can create a similar event."

### 3.3 A3 — the decline buttons

```
JA_ADMIN_CANCEL_EVENT_KEEP    "戻る"       → "やめる"
JA_NOTE_KEEP_ACTION           "メモを保持"  → "やめる"
EN_ADMIN_CANCEL_EVENT_KEEP    "Back"       → "Keep event"

JA_ADMIN_REMOVE_KEEP  unchanged (already "やめる")
EN_NOTE_KEEP_ACTION   unchanged (already "Keep note")
EN_ADMIN_REMOVE_KEEP  unchanged (already "Keep Member")
```

Two facts that make this safe, both verified — **re-verify, do not take on
trust:**

1. **やめる is already the product's decline word in seven places** —
   `JA_IDENTITY_SIGN_IN_CANCEL`, `JA_ME_DISPLAY_NAME_EDIT_CANCEL`,
   `JA_ME_LANGUAGE_CANCEL`, `JA_ACCOUNT_LINK_CANCEL`, `JA_ACCOUNT_UNLINK_CANCEL`,
   `JA_COMMUNITY_CREATE_CANCEL`, `JA_ADMIN_REMOVE_KEEP`. This takes 7 to 9.
2. **Making two JA constants identical is not a gate problem.** The
   identical-pair gate (`EN_JA_IDENTICAL_EXCEPTIONS`,
   `packages/contracts/tests/release_gates.rs:3285`) compares the **EN and JA
   halves of one stem** — it exists to catch untranslated copy-paste — not two
   JA constants against each other. The existing seven-way share proves it does
   not object. **No exception entry is needed; do not add one.**

**Why EN diverges from JA here, deliberately.** Japanese names the abandoned
action (やめる); English names the preserved outcome (Keep X). "Back" was
originally chosen for the cancel dialog because its confirm button is already
"Cancel Event" and a "Cancel" decline would have been ambiguous — sound
reasoning. "Keep event" avoids that collision equally well **and** restores the
parallel with "Keep Member" / "Keep note". The EN/JA parity gate keys on
constant-name stems, not meaning, so the divergence is mechanically safe.

### 3.4 A4 — one Japanese phrasing for irreversibility *(separable; drop if declined)*

```
JA_NOTE_DELETE_BODY
- このメモは削除されます。元に戻すことはできません。
+ このメモは削除されます。この操作は取り消せません。
```

`EN_NOTE_DELETE_BODY` is **unchanged** — English already says "This cannot be
undone." in both places, so there is no English divergence to fix.

This is the weakest item in the slice and was recommended only weakly. If it is
declined, delete this subsection; nothing in §3.1–§3.3 depends on it.

**Do not touch `NOTE_HIDDEN_FLASH` or the comment at
`packages/contracts/src/i18n/notes.rs:27`.** That comment documents why the
admin-hide flash says 非表示 while the member's own action says 削除 — they are
different actions on different columns. It is correct and must stay accurate.

## 4. The cross-checks, already run — verify them, do not repeat them blind

Re-run each and **report what you observe**, even where it matches:

| Check | What I found | What it means for you |
|---|---|---|
| Gates pinning the *text* of any changed constant | **none** | `rfc060_cancelled_event_recreate_is_admin_only_and_details_only` references `ADMIN_CANCEL_EVENT_BODY_ALL_DAYS` **by name**, asserting the handler mentions it. Wording-agnostic. Do not rename or remove the constant. |
| `scripts/` for 「戻る」 | one hit: `scripts/smoke/member-management.mjs:474` | It is 「メンバー管理へ戻る」, a **navigation link**, not this button. **Do not "fix" it.** |
| `scripts/` for 「メモを保持」 | no hits | — |
| `scripts/` for 「メンバーにはキャンセルされたことが」 | no hits | — |
| `scripts/` for 「元に戻すことはできません」 and "cannot be undone" | **no hits**, either locale | §3.4 is safe. |
| Jargon gate (`rfc054_member_facing_japanese_copy_avoids_technical_jargon`) | none of slice 3's constants are in its `reviewed` list; none of the proposed words are in `forbidden` | No conflict. Do not add constants to that list in this package. |

**Expected `.mjs` edits: zero — and here is the real reason.** No smoke asserts
the text of *any* of the eight constants in §2. `smoke:admin-event-forms` renders
the cancel confirm page but checks layout only; nothing checks the words. Note
that `SMOKE_COVERAGE_EXCEPTIONS` does **not** contradict this — that gate checks
every smoke *file* is runnable by a `package.json` name, not that any copy is
covered.

So the smokes will not validate this change; the gates and your own rendering
check must. If you find a `.mjs` hit anyway, stop and report rather than editing
a smoke to match new copy.

**Report, do not fix:** whether this copy-coverage gap deserves a future package.
It is not slice 3's job.

## 5. Explicit non-change scope

- `ADMIN_SUSPEND_CONSEQUENCE`, `ADMIN_REMOVE_CONSEQUENCE`,
  `ADMIN_DEMOTE_CONSEQUENCE`, `ADMIN_PROMOTE_CONSEQUENCE`,
  `ADMIN_UNSUSPEND_CONSEQUENCE`, `ADMIN_HELP_SIGNIN_CONSEQUENCE` — **untouched.**
  All six state their reversibility correctly already, or correctly say nothing
  because the action is not destructive.
- `JA_RELINK_INVALID` / `JA_RECOVERY_INVALID` — **untouched, and the carried item
  is closed unactioned.** Slice 1 resolved it deliberately (relink codes expire,
  recovery codes never do) and Handoff 082 wrote that reasoning into
  `packages/contracts/tests/release_gates.rs:1825-1836` as a gate comment plus an
  assertion. Nothing left to do.
- 中止 vs キャンセル (`JA_OCCURRENCE_CANCELLED_BADGE` 「この日は中止です」 against
  キャンセル elsewhere) — **untouched.** The split is by scope and reads
  deliberate. `scripts/smoke/recurrence-v2.mjs` asserts the 中止 string.
- `NOTE_DELETE`, `NOTE_HIDDEN_FLASH`, `NOTE_SAVED_FLASH` — **untouched.**
- No new constant added, removed, or renamed. Only literal values change.
- No behaviour, route, handler, gate, schema, or version change.
- No new gate. If you believe one of these facts deserves pinning, say so in the
  review request; do not add it.

**Observed but explicitly out of scope:** the jargon gate's `reviewed` array is a
hand-maintained list of 24 constants — the enumerate-don't-derive pattern this
project has been removing elsewhere. Not slice 3's job. Mention it in the review
request; do not act on it.

## 6. Required tests

- `cargo test --workspace --no-fail-fast`, both default and
  `--features dev_fake_issuer`. **Take your own pre-package baseline first and
  report both numbers** — it was 665/0 and 668/0 at the Handoff 082 review, but
  confirm rather than assume. Expected **unchanged**: every gate touching these
  constants compares dynamically or by name.
- **Run the derived parity gate in isolation**, not merely inferred from the
  full-suite pass:
  `cargo test -p zinnias-ciao-contracts --test release_gates en_ja_parity_is_derived_from_the_constants_themselves`
- **Run `rfc060_cancelled_event_recreate_is_admin_only_and_details_only` in
  isolation** — it is the one gate that names a changed constant.
- **Grep every `.mjs` for each old string before editing**, and report findings
  against §4's table.
- Targeted smokes — **derived, and the derivation matters** (my first draft of
  this handoff named `smoke:recurrence-v2`, which is neither a real script name
  nor the right path; corrected below):
  - **`smoke:admin-event-forms`** — the only smoke that renders the event-level
    cancel confirmation page (`scripts/smoke/rfc075-slice4-admin-event-forms.mjs:339-343`,
    case `cancel-event-confirmation`). It asserts **layout only**
    (`denseRowSelector: '.cz-admin-confirm-actions'`), not copy — so it catches a
    rendering break, not a wording one.
  - **`smoke:recurrence`** (file `recurrence-v2.mjs` — the script name differs
    from the filename) — covers **occurrence**-level cancel via `/days/…/cancel`,
    a *different* path that does not use the constants in §2. Run it as a
    regression check; do not expect it to exercise the changed copy.
  - **`smoke:language`** — the only smoke touching the note route at all.
  - Then `bun run smoke:all` at **25/25**.
- `node scripts/test-evidence-leakage-baseline.mjs` green at **996**.
- clippy `-D warnings` both feature states, `cargo fmt --all -- --check`,
  `cargo check --target wasm32-unknown-unknown -p zinnias-ciao-ssr`,
  `mdbook build docs`, `git diff --check`, `bun run build`.

## 7. Required documentation updates

`docs/src/tester/release-checklist.md` — add only: that cancelling an event now
states it cannot be undone and points to creating a similar event; that the
attendance-freeze is now stated for single-date cancellation as well as
all-days; and that every destructive confirm's decline button now reads
「やめる」 in Japanese.

## 8. Acceptance criteria

1. All four irreversible actions in the product state that they are
   irreversible. Before this package: three of four.
2. Both cancel bodies differ only in their first sentence.
3. Every Japanese decline button in a destructive confirm reads 「やめる」 — nine
   constants, verified by grep, not by inspection of the three you edited.
4. No gate edited, no exception-table entry added, no `.mjs` edited.
5. Test counts unchanged from the baseline you took in §6, or the movement
   explained before anything is re-pinned.
6. The recreate button's label and the body's reference to it still match.

## 9. Prohibited shortcuts

- Do not edit a gate or a smoke assertion to accommodate new copy. If copy and a
  gate disagree, **stop and report** — that is what slice 2 did when
  「残ります」 broke, and the fix was a punctuation change to the copy, not to the
  gate.
- Do not add an entry to `EN_JA_IDENTICAL_EXCEPTIONS` or
  `EN_JA_PARITY_EXCEPTIONS`. §3.3 explains why none is needed. A second entry in
  the identical table is a **stop condition** (Handoff 070 §12), not a row to add.
- Do not implement §3.1's wording byte-for-byte if it breaks a gate. Report the
  break, propose the minimal fix, and flag the deviation explicitly — as slice 2
  did.
- No `--force`, `--allow`, or `--skip` flag on the leakage scanner, and none
  should be added.
- Do not extend the slice. The eight constants in §2 are the whole package.

## 10. Security constraints

These strings describe; they gate nothing. Confirm via `git diff` that no
change reaches `MEMBERSHIP_ACTIVE`/`MEMBERSHIP_PRESENT`, the cancel
authorization path, `event_can_seed_recreate`, the recreate admin check, or any
refusal logic — only string literals in two `i18n/` files.

RFC-060's cancellation semantics are being **described** accurately by the new
copy, not altered: `cancelled_at` remains one-way (confirm by inspection —
`workers/ssr/src/db/event_write.rs:407` sets it, and nothing clears it), and the
recreate flow remains admin-only and details-only. The copy's claim "you can
create a similar event" must stay true of what `get_recreate_event` actually
permits; if that changes, the copy is wrong.

B1, B3, B4, and B5 remain open. Production, public-pilot, and
first-real-community deployment remain **No-Go**. No release closes a finding.

## 11. Required review-request format

Write to `.git-exclude/review-request/`, following slice 2's structure —
**what was run vs. observed, separately from what was concluded.** Include:

1. §4's cross-checks, re-run by you, with the observed counts.
2. The `.mjs` grep results, including the ones with no hits.
3. Every constant changed, both halves, old and new.
4. **Any deviation from §3's proposed wording, flagged explicitly**, with whether
   it was mechanical (a gate) or a taste judgment.
5. Pre- and post-package test counts, both feature states.
6. The two isolated gate runs from §6.
7. Confirmation of §5's non-change list, by diff.
8. Whether §3.4 was included or dropped.
9. The jargon-gate observation from §5 and the copy-coverage gap from §4, if you
   agree either is worth a future package.

## 12. Not authorized by this handoff

No commit, deployment, hosted action, secret access, remote D1 access, tag, RFC
lifecycle movement, finding closure, release, or version bump. No slice 4 work.
Await review before committing.
