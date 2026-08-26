# RFC-086 — Derive the Destructive-Confirm Surface From the Schema

**Status:** Proposed
**Prepared:** 2026-08-26
**Author:** architect (for nabbisen's decision)
**Origin:** the slice 3 and slice 4 reviews of RFC-054; candidate theme for `0.64.0`

## 1. Why this exists

Three times in two days I have stated how many irreversible actions this product
has. Three different answers, each derived by a hand-written grep:

| Stated | Where | Basis | Wrong because |
|---|---|---|---|
| **four** | slice 3 decision list §1 | grep of constant-name suffixes `_KEEP\|_CONSEQUENCE\|_DELETE_BODY\|_CANCEL_EVENT_BODY` | missed `OCCURRENCE_CANCEL_HELPER` — a `_HELPER` |
| **five** | slice 4 handoff §0 | the above, plus the one I'd just missed | missed `ACCOUNT_UNLINK_BODY` entirely — it lives in `account.rs`, outside the surface I was grepping |
| **six+** | this RFC | `awk` over owning-constant, multi-line safe | found a seventh candidate **and a live defect** — see §2 |

Each count was produced carefully, cross-checked, and wrong. The pattern is not
carelessness; it is that **the surface has no name-level definition to grep for.**
A destructive confirm is defined by what the code *does* — a one-way write — not
by what its constant is called.

This RFC proposes deriving the surface from the schema, and pinning the mapping
with staleness assertions, in the shape this project already uses for
`LOCALIZATION_EXCEPTIONS`, `SMOKE_COVERAGE_EXCEPTIONS`, and Handoff 070's derived
parity gate.

## 2. The defect this derivation already found

`workers/ssr/src/handlers/admin/events/notes.rs:52-80` renders the **admin
hide-note** confirmation dialog. It POSTs to `.../notes/{mid}/hide` and sets
`hidden_by_admin_at` (`db/event_note.rs:123`). Its body is:

```rust
consequence = i18n::t(locale, i18n::ADMIN_REMOVE_CONSEQUENCE),
```

That is the **member-removal** consequence. An admin hiding one note currently
reads, in full:

> このメンバーはイベントやメモを見ることができなくなります。過去の参加状況やメモは
> 残ります。他のコミュニティへの参加には影響しません。この操作は取り消せません。
> あとで招待し直すことはできますが、新しいメンバーとしての参加になり、役割や表示名
> は引き継がれません。 〔メンバー名〕

None of that is true of hiding a note. Nobody loses access, nobody is re-invited,
no role or display name is involved.

Two aggravating details:

- **Slice 2 made it worse.** That constant's re-invitation sentences were added
  by RFC-054 slice 2, carefully reviewed *for the removal dialog*. Sharing the
  constant propagated them to a dialog nobody was looking at.
- **The title is also wrong.** The dialog's `<h1>` and submit button both use
  `NOTE_DELETE` 「メモを削除」/"Delete Note", but the action hides. `notes.rs:29`
  already documents that "hidden, not removed" — the flash was fixed for exactly
  this reason; the dialog was not.

It accidentally states the correct *fact* (`hidden_by_admin_at` is one-way, and
the borrowed text does say 取り消せません) with an entirely wrong explanation.

**This is a live, member-facing copy defect, not a hypothetical.** It is the
evidence for this RFC, and it is the reason I am proposing the gate rather than
a seventh hand-count.

## 3. Proposal

A derived release gate in `packages/contracts/tests/release_gates.rs`, in three
parts.

### 3.1 Derive the one-way transitions from the DB layer

Scan `workers/ssr/src/db/*.rs` for `UPDATE <table> SET <column>`. A
`(table, column)` pair is **one-way** when no statement anywhere sets that same
pair back — to `NULL` or to its prior value. This is mechanically derivable
today; I ran it while writing this RFC:

```
one-way (no clearing write):  events.status, event_days.occurrence_status,
                              community_memberships.removed_at,
                              event_notes.hidden_by_admin_at,
                              user_identities.status, invite_codes.used_at,
                              sessions.revoked_at, …

cleared somewhere:            community_memberships.suspended_at,
                              attendance.status, event_notes.note_deleted_at
```

Assert the derived one-way set equals a pinned `ONE_WAY_TRANSITIONS` table. A new
one-way write appearing without a table entry **fails the build**, with a message
telling the author to map it to a confirm body or document why it has none. That
is the half that would have caught §2 and both of my miscounts.

### 3.2 Assert the property on each mapped confirm body

For each `ONE_WAY_TRANSITIONS` entry that names a confirm-body constant, assert
both locale halves state irreversibility — 「取り消せません」 and "cannot be
undone". **Full conjugated form**, per Handoff 082: 取り消せ also prefixes
「取り消せます」, which claims the opposite.

### 3.3 Assert the negative, so the gate is two-directional

A `REVERSIBLE_ACTIONS` table for the confirms whose action *can* be undone —
today `ADMIN_SUSPEND_CONSEQUENCE`. Assert those contain 「取り消せます」 and do
**not** contain 「取り消せません」. Without this half, the gate cannot catch the
worst version of the bug: reversible copy on an irreversible action, or the
reverse.

Both tables carry stale-entry assertions — a pinned entry naming a
`(table, column)` or constant that no longer exists must fail, per the
`SMOKE_COVERAGE_EXCEPTIONS` precedent.

## 4. Three design pitfalls, already hit

Recording these because each one broke a naive version of §3.1 while I was
testing it, and the implementation must handle them:

1. **Column names are not unique across tables.** `attendance.status` is set to
   `NULL` when a member clears their answer (`db/attendance.rs:158`). A
   column-name-only derivation reads that as "`status` is clearable" and
   silently reclassifies **event cancellation** as reversible. The key must be
   `(table, column)`, never `column`.
2. **`event_notes.note_deleted_at` is cleared, yet the action is irreversible.**
   The upsert at `db/event_note.rs:79` sets it back to `NULL` — but overwrites
   `note` in the same statement, so the text never returns. The derivation will
   classify it reversible; it needs a pinned exception with a written reason, not
   a change to the copy, which is correct as it stands.
3. **One constant can serve two actions.** §2 is precisely this. The mapping must
   be keyed by *dialog*, and a constant appearing under two different transitions
   should be treated as a finding to investigate, not a shortcut to allow.

## 5. Non-goals

- Not a copy rewrite. §2's defect should be fixed **before** this gate lands, as
  its own small package — otherwise the gate's first run fails on a defect it did
  not cause, and the two changes become hard to review separately.
- Not a new exception table for the two observations carried since slice 3 (the
  jargon gate's hand-maintained 22-constant array; no smoke asserting any
  destructive-confirm wording). Both remain open and are worth their own
  packages; neither belongs here.
- Not a change to any one-way behaviour. Every transition stays exactly as
  one-way as it is today.

## 6. Security

The gate reads source at compile time and asserts on string contents. It grants
nothing, weakens nothing, and touches no runtime path.

One genuine security-adjacent benefit: several one-way transitions in §3.1 are
credential lifecycle writes — `invite_codes.used_at`, `sessions.revoked_at`,
`membership_relink_codes.revoked_at`, `account_recovery_credentials.consumed_at`.
Most have no confirm dialog and should be mapped as "no dialog, by design" with a
reason. Forcing that decision to be written down is worth more than the copy
check: it makes a future *reversible* rewrite of a single-use credential fail the
build rather than pass unnoticed. No fail-open path is introduced.

## 7. Acceptance criteria

1. The derived one-way set is computed from `workers/ssr/src/db/`, not enumerated
   by hand, and the count is reported by the gate rather than asserted by a human.
2. Every one-way transition either names a confirm body that states
   irreversibility in both locales, or carries a written reason for having none.
3. The suspend/unsuspend pair fails the gate if its copy is flipped in either
   direction.
4. Adding a new one-way write without a table entry fails the build, and the
   failure message says what to do.
5. Each pinned table fails on a stale entry.
6. **The gate is demonstrated failing in both directions before it is accepted** —
   per Handoff 082's standard: break it, watch it fail, reword-but-keep-correct,
   watch it pass.
7. §2's defect is fixed and committed **before** this gate lands.

## 8. Risks

- **Comment contamination.** This project has hit gates matching their own prose
  eight times, most recently in SQL. The SQL scan must strip `--` and `/* */`
  comments before matching, and the gate's own doc-comment must not contain a
  literal `UPDATE … SET …` that it would match.
- **Over-pinning.** §3.2 pins one sentence per dialog, not whole bodies. Pinning
  more would re-create the wording-lock that Handoff 082 spent a package undoing.
- **False confidence.** The derivation covers the `db/` layer only. A one-way
  write issued from anywhere else would be invisible; the gate should assert that
  no `UPDATE … SET` exists outside `workers/ssr/src/db/`, or state plainly that
  it does not cover that case.

## 9. Recommended sequencing

1. **Fix §2** — the hide-note dialog's body and title. Small, independent, and it
   is a live defect regardless of whether this RFC is accepted. Suggest RFC-054
   slice 5.
2. **Then this gate**, with §7.6's both-direction demonstration.
3. The two carried observations stay open for later packages.

## 10. Not authorized by this RFC

No deployment, hosted action, secret access, remote D1 access, tag, release, or
version bump. No finding closure — B1, B3, B4, and B5 remain open, and
production, public-pilot, and first-real-community deployment remain **No-Go**.
Nothing here is implemented until nabbisen accepts it.
