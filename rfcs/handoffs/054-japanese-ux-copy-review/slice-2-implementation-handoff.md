# RFC-054 — Implementation Handoff: Slice 2, Admin Destructive-Action Copy

**Handoff status:** **Authorized 2026-08-16 by nabbisen** — all four findings
accepted. Implement as specified. Authorization covers §3–§6 only; §11's exclusions
still bind, and nothing here authorizes a tag, deployment, version bump, or release.
**Prepared:** 2026-08-16 by the high-capability model
**Checkpoint:** `6472965` (pushed; `origin/main` in sync)
**Governing RFC:** `rfcs/accepted/054-japanese-ux-copy-review.md` §6 Slice 2. Per
RFC-000 this handoff inherits its status from that RFC, which is **Accepted**.
**Decision list:**
`.git-exclude/reviewed/zinnias-ciao-main-2026-08-16-rfc054-slice2-copy-decision-list.md`
— findings **A1, A2, B1, B2**, all accepted.

---

## 1. Task title

Make the irreversible admin action say so, and converge two strays.

## 2. Scope: three constants, six changes

**B2 merged into A1's constant rather than adding a sentence**, so the accepted four
findings land on **three** `Localized` pairs, both halves each:

| Constant | Findings |
|---|---|
| `ADMIN_REMOVE_CONSEQUENCE` | **A1** + **B2** + B1's English half |
| `ADMIN_LAST_ADMIN_DEMOTE` | **A2** |
| `ADMIN_DEMOTE_CONSEQUENCE` | **B1** |

All in `packages/contracts/src/i18n/admin.rs`. **No new constant** — RFC-054 §7
forbids it and none is needed.

### 2.1 On B2, which I recommended leaving

You accepted it. My objection was that a fourth sentence would make A1 harder to
read — and merging it into the **existing** history clause avoids that entirely,
while making removal structurally parallel to suspension: *what stops / what is
unaffected / whether it can be undone.* **The objection does not survive the
composition**, and the result is better than either finding alone.

## 3. Required implementation

### 3.1 A1 + B2 — `ADMIN_REMOVE_CONSEQUENCE`

Removal is **one-way**: `removed_at` is only ever `SET` (`db/membership.rs:641`) and
no path clears it. Suspension says 「この操作は取り消せます。」; removal says nothing.
Silence beside an explicit reassurance reads as "probably similar."

Migration `0001`'s partial unique index — *"any number of removed historical rows for
the same pair is permitted"* — is what makes an accurate second half possible: the
**person** can be invited again, the **membership** cannot be restored.

```
JA_ADMIN_REMOVE_CONSEQUENCE
現在  このメンバーはイベントやメモを見ることができなくなります。過去の参加状況やメモは残ります。
提案  このメンバーはイベントやメモを見ることができなくなります。過去の参加状況やメモは残り、
      他のコミュニティへの参加には影響しません。この操作は取り消せません。あとで招待し直すことは
      できますが、新しいメンバーとしての参加になり、役割や表示名は引き継がれません。

EN_ADMIN_REMOVE_CONSEQUENCE
現在  They will no longer be able to see events or notes. Past attendance and notes remain.
提案  This member will no longer be able to see events or notes. Past attendance and notes
      remain, and other communities are unaffected. This cannot be undone. They can be
      invited again later, but as a new member — their role and display name will not
      carry over.
```

**The English subject changes too** — see §3.3; `They` is B1's second English outlier.

**This wording is a proposal.** The Japanese for A1 was in the decision list; the
merged B2 clause and both English halves are new. **Flag them in the review request
for the owner rather than treating them as settled** — but do not block on it.

### 3.2 A2 — `ADMIN_LAST_ADMIN_DEMOTE`

Two of three last-admin refusals name the remedy; this one stops at the refusal, and
the remedy is identical in all three cases.

```
JA_ADMIN_LAST_ADMIN_DEMOTE
現在  最後の管理者はメンバーに戻せません。
提案  最後の管理者はメンバーに戻せません。先に管理者権限を移譲してください。

EN_ADMIN_LAST_ADMIN_DEMOTE
現在  Cannot make the last admin a member.
提案  Cannot make the last admin a member. Transfer the admin role first.
```

**Both second sentences are copied verbatim from the siblings** —
`JA_ADMIN_LAST_ADMIN` / `EN_ADMIN_LAST_ADMIN`. This is convergence, not new wording;
do not paraphrase.

### 3.3 B1 — the subject, in both languages

Measured across `admin.rs`:

```
Japanese   このメンバー  5     この人  1  ← DEMOTE_CONSEQUENCE
English    This member   3     This person 1 ← DEMOTE   /   They 1 ← REMOVE
```

**English has two outliers, Japanese one.** The decision list only measured the
Japanese; the parity criterion (RFC-054 §3) means both halves converge.

```
JA_ADMIN_DEMOTE_CONSEQUENCE
現在  この人はイベントの作成、メンバー管理、招待コードの作成ができなくなります。過去の参加状況やメモは残ります。
提案  このメンバーはイベントの作成、メンバー管理、招待コードの作成ができなくなります。過去の参加状況やメモは残ります。

EN_ADMIN_DEMOTE_CONSEQUENCE
現在  This person will no longer be able to create events, manage members, or generate invite codes. Past attendance and notes remain.
提案  This member will no longer be able to create events, manage members, or generate invite codes. Past attendance and notes remain.
```

`REMOVE`'s English subject is fixed in §3.1, where that string is already changing.

## 4. The cross-check, already run — verify it, do not repeat it blind

Slice 1's F1 was **prose naming a button whose label had changed**, which my §5 then
did not tell anyone to look for. I ran the equivalent check before writing this:

| Searched | Result |
|---|---|
| 「メンバーから外す」 | **1** — `ADMIN_REMOVE_CONFIRM`, the button label itself. **No prose names it.** |
| 「メンバーに戻す」 | **1** — `ADMIN_DEMOTE_ACTION`, the label itself |
| 「管理者権限を移譲」 | **2** — the two siblings A2 converges on |
| 「この人」 | **1** — only the constant being changed |
| 「この操作は取り消せ」 | **2** — `ACCOUNT_UNLINK_BODY` (…ません) and `ADMIN_SUSPEND_CONSEQUENCE` (…ます) |

**Two consequences.**

**No button-naming risk this time** — nothing describes these buttons in running
prose, so slice 1's F1 class is checked and clear.

**A1 converges rather than invents.** `JA_ACCOUNT_UNLINK_BODY` already reads
「この操作は取り消せません。」 for the identity-unlink action. The proposed sentence is
existing project usage for an irreversible action, not a new phrase.

**Re-derive both checks yourself before editing** and report what you find. Taking a
cross-check on trust is how slice 1's F1 happened.

## 5. Explicit non-change scope

- **`ADMIN_SUSPEND_CONSEQUENCE` is not touched.** Its 「この操作は取り消せます。」 is
  true and useful; the asymmetry is fixed by making the silent one speak.
- No new constant; none removed or renamed.
- No behaviour, route, gate, or schema change.
- **No other `admin.rs` string** — the decision list's §C names what was read and
  deliberately left, including `ADMIN_INVITES_REVOKE`, `ADMIN_SUSPENDED_BADGE`, and
  the four `〜しますか？` titles.
- No version bump; this touches no cached asset.

## 6. Required tests

- `cargo test --workspace --no-fail-fast` — **665 / 0** today, and **expected
  unchanged**: the derived EN/JA parity gate, the identical-pair gate, and the
  placeholder check all compare values dynamically and need no edit. **If a count
  moves, explain before re-pinning anything.**
- **Confirm the derived parity gate still passes** — these three pairs stay paired,
  and none becomes identical to another constant. That gate is the reason no
  hand-maintained list needs touching.
- **Grep every `.mjs` for each old string before editing**, and report what you find.
  Slice 1's assertions moved with their constants; these may too.
- `smoke:admin-member-management` and `smoke:admin-tools-onboarding` specifically —
  both render these pages — plus **`bun run smoke:all` at 25/25**.
- `node scripts/test-evidence-leakage-baseline.mjs` green at **996**.
- clippy `-D warnings` both feature states, fmt, wasm check, `mdbook build docs`,
  `git diff --check`, `bun run build`.

## 7. Required documentation updates

`docs/src/tester/release-checklist.md` — add only: the three constants, that removal
now states it cannot be undone while suspension states it can, and that the last-admin
refusals now all name the same remedy.

## 8. Acceptance criteria

1. Three `Localized` pairs changed, **both halves each**; no other constant touched.
2. `ADMIN_REMOVE_CONSEQUENCE` states it cannot be undone **and** that re-invitation
   creates a new membership without the prior role or display name.
3. `ADMIN_LAST_ADMIN_DEMOTE`'s second sentence is **verbatim** its siblings'.
4. Both subject outliers fixed — Japanese この人, English `This person` and `They`.
5. `ADMIN_SUSPEND_CONSEQUENCE` untouched.
6. Your own cross-check re-run and reported; smoke `.mjs` grep reported.
7. Suite unchanged at 665/0; `smoke:all` 25/25; baseline 996; no version bump.
8. The proposed wording flagged as a proposal, not presented as settled.

## 9. Prohibited shortcuts

- No find-and-replace on any old string — §4 exists because two of them appear
  elsewhere, correctly.
- No paraphrasing A2's borrowed sentence; verbatim or it is not convergence.
- No touching `ADMIN_SUSPEND_CONSEQUENCE` to "balance" the pair.
- No new constant, however tempting a missing message looks — RFC-054 §7 makes that a
  separate finding, not a rewrite.
- No English half that drops the irreversibility statement to stay short.

## 10. Security constraints

**These strings gate nothing; they describe.** No authorization, no refusal logic, no
predicate changes — `MEMBERSHIP_ACTIVE`/`MEMBERSHIP_PRESENT` and the last-admin guards
are untouched.

One property to preserve: **the last-admin refusals must stay refusals.** A2 adds a
remedy sentence to a message shown when an action was *denied*; it must not read as
though the action succeeded, and the guard that produced it must not change.

RFC-082's suspension semantics and RFC-063's removal semantics are described here, not
altered. **If the accurate copy and the actual behaviour disagree, stop** — that is a
product finding, not a copy one.

Evidence must not retain display names, community names, or any other prohibited
value.

## 11. Required review-request format

What you ran and what you observed, separately from what you concluded. Include: your
own re-run of §4's cross-check; the `.mjs` grep; confirmation that the derived parity
and identical-pair gates still pass without edits; the test counts; and the proposed
wording flagged for the owner.

**Label the checkpoint from `git log -1`.**

## 12. Not authorized by this handoff

No deployment, hosted action, secret access, remote D1, tag, RFC lifecycle movement,
finding closure, release, or version bump. **No slice 3 work** — RFC-054 §6 defers it,
and its one carried item (the divergent `JA_RELINK_INVALID` / `JA_RECOVERY_INVALID`
messages) stays carried. B1, B3, B4, and B5 remain open; production, public-pilot, and
first-real-community deployment remain **No-Go**.
