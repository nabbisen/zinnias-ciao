# RFC 054 — Japanese UX Copy Review

**Status.** Accepted — **substantially revised 2026-08-15 and owner-accepted the
same day**, selected as the active theme after `0.62.0`. The original text
(June 2026, written against v0.36.0) is superseded below: its inventory covered
143 strings in a single `i18n.rs`, and its headline problem has since been fixed
incidentally. See §1.

**Acceptance authorizes the review, not a blanket string edit.** The deliverable
is a decision list (§5); each slice's accepted changes then need their own
implementation package.

**Phase:** Pre-pilot hardening
**Relationship:** Follows RFC-049 (Japanese rendering) and RFC-072 (member
language preference and runtime localization). Required before a public pilot
with non-technical users.

---

## 1. What changed since this RFC was written, and why it needed revising

Measured at `5248a51`, not carried forward from the original text:

| | Original (v0.36.0) | Now (`0.62.0`) |
|---|---|---|
| `JA_*` constants | 143 | **319** |
| Location | one `i18n.rs` | **13 modules** under `i18n/` |
| `Localized{ja,en}` pairs | did not exist | **94** |

**The original RFC's headline problem is largely already solved.** Its §3 named
three jargon terms to eliminate — セッション, トークン, 同期 — and a scan of the
current tree finds **none of them in any Japanese string**. `JA_SESSION_EXPIRED`,
its worked example, now reads:

> 時間がたったため、もう一度入る必要があります。管理者から受け取った招待コード、
> またはサインインし直すためのコードを使ってください。

That is the RFC's own recommendation, already applied — arrived at through
RFC-049's English-leak work and RFC-072's localization pass rather than through
this RFC.

The remaining jargon in Japanese copy is **two strings**: `JA_TZ_ERROR`'s
タイムゾーン and `JA_OFFLINE_BANNER`'s オフライン — both of which the original text
itself flagged as probably acceptable.

**So this RFC is no longer primarily a jargon hunt.** Keeping it pointed at a
solved problem would waste the one reviewer who can do the work.

## 2. What the review is actually for now

Three things, in descending order of value:

### 2.1 The strings no one has ever reviewed

> **Corrected 2026-08-15, immediately after acceptance.** This section originally
> said "the 54 strings no one has ever reviewed" and attributed them to three
> whole modules. **That was a module-level estimate, not a measurement, and it was
> wrong in both directions.** Diffing the superseded inventory's constant names
> against the current tree gives **180 constants that were never in it**, spread
> across *every* module — and `access.rs` contributes 12, not 23, because its
> eleven `JA_JOIN_*` strings *were* reviewed.
>
> Measured breakdown: `events.rs` 35, `admin.rs` 29, `calendar.rs` 29,
> `account.rs` 26, `community.rs` 16, `access.rs` 12, `me.rs` 12, `general.rs` 5,
> `home.rs` 5, `recovery.rs` 5, `templates.rs` 3, `notes.rs` 2, `export.rs` 1.
>
> This is the same failure this project keeps catching in handoffs — counting a
> list instead of deriving it — and it is worth recording rather than silently
> fixing, because it moved the slice boundary. **Novelty no longer selects slice
> 1; consequence does.** See the corrected §6.

The external-identity track and RFC-082 added member-facing copy that has never
been through any copy review:

| Module | Strings | What it says |
|---|---|---|
| `i18n/account.rs` | 26 | the account surface, linking, unlink confirmation |
| `i18n/access.rs` | 23 | the suspension paused page, access refusals |
| `i18n/recovery.rs` | 5 | the recovery credential — issuance, reveal, consumption |

**This is the highest-value part of the review and the original RFC could not
have anticipated it.** These strings carry the most consequential moments in the
product: a member being told their access is paused, and a member being shown a
recovery code once and told to keep it. Both are read by someone who is confused
or worried, which is exactly when copy quality matters most and when jargon does
the most damage.

### 2.2 Cross-module consistency

With one file, consistency was self-evident. With 13 modules it is not. The
review should check that the same concept uses the same word everywhere —
コミュニティ vs グループ, 参加 vs 出席, メモ vs ノート, 管理者 vs 運営者 (both
currently appear) — and that register does not drift between modules written
months apart.

### 2.3 Register and politeness across 319 strings

The original criteria still stand and are restated in §3. What is new is scale:
this is now a review twice the original size, and it should be sliced rather than
attempted in one sitting.

## 3. Criteria

Unchanged from the original, and still right:

- **Clarity** — would an IT-averse Japanese user understand what to do?
- **Politeness** — ですます style, appropriate register for general community use.
- **No technical jargon** in member-facing copy.
- **Action labels** matching community norms.
- **Error messages** should say what to do, not what failed.

One criterion added, from RFC-072's structure:

- **JA/EN parity of register.** For the 94 `Localized` pairs, the two languages
  should say the same thing at the same level of formality. A polite Japanese
  string paired with a terse English one is a defect in the pair, not in either
  half.

## 4. Who does what — the original §4 "blocker", resolved

The original said: *"Requires a Japanese native speaker familiar with the target
community type."* That is still true, and the owner is one. The blocker was never
finding a person; it was that nobody had prepared the work so the native speaker's
time went to judgement rather than clerical sorting.

**Split accordingly:**

**Prepared by the architect** — mechanical, checkable, and not requiring native
judgement:

- the full current inventory, grouped by surface and by module;
- a jargon scan against §3's list, with every hit in context;
- a cross-module consistency report: every concept expressed more than one way;
- a register scan: strings that are imperative where the rest are suggestive,
  or plain where the rest are ですます;
- `Localized` pairs whose two halves differ in formality;
- error strings that describe a failure rather than an action;
- **a proposed rewrite for every finding**, so the reviewer is accepting,
  rejecting, or amending concrete text rather than composing from scratch.

**Decided by the owner** — and by nobody else:

- whether a proposed rewrite actually sounds natural to the target community;
- register calls that depend on knowing the audience;
- anything where the honest answer is "a native speaker would wince at this."

**Applied by the developers** — once decisions exist: a mechanical edit of the
constants, with the existing parity gate and localization gates proving nothing
was missed.

## 5. Deliverable

A decision list: constant name, current text, proposed text, and the owner's
verdict. Then one implementation package per slice applying the accepted changes.

**This RFC changes no behaviour.** It changes strings, and every string it changes
is already covered by the `en_ja_parity` gate and RFC-072's localization gates —
so the risk is not correctness but tone, which is precisely why the decision half
cannot be delegated.

## 6. Slicing

Not one review of 319 strings. **Corrected 2026-08-15** — sliced by consequence,
since §2.1's correction showed novelty spans every module and therefore selects
nothing.

**Slice 1 — the strings a member reads at their worst moment (44).** Someone
locked out, suspended, or holding a recovery code reads these while confused or
worried, which is when jargon does the most damage and when a misunderstanding is
most expensive:

- `general.rs::JA_MEMBERSHIP_SUSPENDED` — the paused page, the single string a
  suspended member sees;
- `recovery.rs` (5) — the recovery credential's issuance, reveal, and
  consumption;
- `account.rs` (26) — the account surface, linking, and unlink confirmation;
- `access.rs`'s 12 never-reviewed — relink and external sign-in failure.

**Slice 2 — admin destructive-action copy.** `admin.rs`'s removal, suspension,
and last-admin refusals: read by a volunteer about to do something irreversible
to another person.

**Slice 3 — everything else**, by module.

Slices 2 and 3 may be deferred indefinitely without blocking a pilot. **Slice 1
should not be.**

## 7. Non-goals

- No behavioural change, no route change, no schema change.
- No English copy review — the EN strings exist for RFC-072's locale switch and
  are not the pilot audience's language.
- No new strings. If the review finds a *missing* message, that is a separate
  finding, not a rewrite.
- No change to `Localized`'s structure or the parity gate.

## 8. Superseded

The original §3 suggestion table and §5 string inventory (143 strings, v0.36.0)
are superseded by this revision and by the measured inventory the architect will
prepare. They are retained in git history rather than reproduced here, because a
stale inventory in a live document is worse than none — the same reasoning that
made Handoff 058 re-derive its site count instead of trusting RFC-082's.
