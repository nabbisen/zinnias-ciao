# Changelog

All notable changes to ciao.zinnias are documented here.

## [0.36.1] — 2026-06-13

Security: two invite-code generation defects fixed (identified in codlet extraction review).

### Fixed

- **§7.1 — Fail-closed randomness.** `generate_invite_code()` previously called
  `getrandom::getrandom(&mut bytes).unwrap_or_default()`. On an RNG failure the
  buffer stayed zeroed, producing the deterministic code `"AAAAAA"`. The function
  now returns `worker::Result<String>` and propagates the error with `?`, matching
  the discipline of `crypto::random_token()` which already used `.expect()`.
  The call site in `post_generate_invite` propagates via `?`, returning HTTP 500
  rather than issuing a weak code.

- **§7.2 — Rejection sampling replaces modulo bias.** The alphabet has 31
  characters. `256 % 31 = 8`, so the previous `b % 31` map made the first eight
  characters (`A B C D E F G H`) appear with probability `9/256` vs `8/256` for
  the remaining 23. The new generator discards bytes `>= 248` (the biased tail)
  and resamples. Expected extra draws per character ≈ 0.03 — negligible overhead.

### Added

- **3 property tests in `packages/domain/src/invite.rs`:**
  - `rejection_sampling_ceiling_is_correct` — verifies `248 = 256 - (256 % 31)`.
  - `all_accepted_bytes_map_to_alphabet` — every byte `< 248` produces a valid
    alphabet character.
  - `alphabet_excludes_ambiguous_characters` — `0`, `1`, `O`, `I`, `L` are absent.

- **2 release gates in `packages/contracts/tests/release_gates.rs`:**
  - `invite_code_generator_does_not_use_unwrap_or_default_on_getrandom` — prevents
    regression to the fail-open pattern.
  - `invite_code_generator_uses_rejection_sampling` — asserts the ceiling constant
    is still present and the old biased pattern is absent.

### Context

Both defects were identified by a Rust and security architect reviewing the
codebase for a potential `codlet` crate extraction (the one-time code auth
mechanism). §7.1 is a real security defect — a compromised or unavailable OS RNG
silently issues a predictable invite code. §7.2 is a quality defect — the bias is
minor in practice (0.4% over-representation behind expiry + throttling) but
unacceptable in a reusable security library. Both are worth fixing in the app
regardless of extraction status.

### Testing

- 223 passing (was 218). Zero warnings.

## [0.36.0] — 2026-06-13

RFC-052 closed; RFC-054 and RFC-045 updated for reviewer handoff.

### Added

- **`docs/src/audit-policy.md`** — operator-facing policy document for RFC-052.
  Covers access policy (operator-only, no UI), retention (indefinite for pilot),
  metadata allowlist (entity type, ID, action, timestamp; no note bodies, no
  tokens), full audit event inventory (16 `target_kind.action` pairs with their
  triggers), complete D1 query reference, and incident response procedures for
  invite abuse, member removal, and note moderation.

- **Audit policy added to mdbook SUMMARY.md** (`docs/src/SUMMARY.md`).

### Changed

- **RFC-052 moved to `rfcs/done/`** — marked Implemented (v0.36.0). The policy
  document satisfies the RFC's stated deliverable: "Document first."

- **RFC-054 updated** — corrects the stale string count (120 → 143), adds a
  complete grouped inventory of all 143 `JA_*` strings with per-string
  reviewer notes flagging technical jargon (`セッション`, `JSON`, `エクスポート`,
  `タイムゾーン`, `オフライン`) and the UX-architect's suggested alternatives for
  key strings (`カレンダーフィード`, `エクスポート`, `出席済み`). The inventory is
  organized by screen context to match the reviewer's workflow.

- **RFC-045 source-verification count updated** to 218 (v0.35.1).

- **`rfcs/README.md`** — RFC-052 moved from proposed to done; proposed count
  updated to 12.

### Testing

- 218 passing. Zero warnings.
- RFC counts: 43 done, 12 proposed.

## [0.35.1] — 2026-06-13

UX-architect review remediation: three English-text leaks fixed; memo counter wired.

### Fixed

- **Three English strings leaked into Japanese-only UI (RFC-049 violations).** These
  were inline string literals, not i18n constants, so the i18n parity gate did not
  catch them. All now use `JA_*` constants:
  - Event detail back-link rendered `← Home` → now `← ホーム` (`JA_NAV_HOME`).
  - Communities admin links rendered `Invite members` / `Manage members` → now
    `メンバーを招待` / `メンバー` (`JA_ADMIN_INVITES_TITLE` / `JA_ADMIN_MEMBERS_TITLE`).
  - Community switcher no-JS fallback button rendered `Go` → now `切り替え`
    (new `JA_NAV_SWITCH_GO`).

- **Memo character counter was dead.** `app.js` updates an element with class
  `.note-counter`, but `note_form` rendered the hint `<span>` without that class,
  so the live `N/200` counter never displayed (button-disable on overflow still
  worked). Added the `note-counter` class to the rendered span. The counter now
  updates as the user types, per the handoff specification.

### Added

- **Two regression gates in `release_gates.rs`:**
  - `no_known_english_ui_leaks_in_rendered_text` — scans handler/render sources
    for the specific English-text regressions and a small English UI vocabulary
    in `>Word</a>` / `>Word</button>` shapes. The i18n parity gate only covers
    constants; this catches inline literals.
  - `note_form_has_counter_element_for_js` — asserts the `.note-counter` class is
    rendered so the JS counter has a target.

- **New i18n pair:** `EN/JA_NAV_SWITCH_GO`.

### Context

These issues surfaced during the UX-architect review of the handoff document. The
reviewer's plain-language emphasis (no technical or English words in member-facing
copy) was directly validated: the audit found three concrete English leaks the
reviewer's principles would flag. The reviewer's other recommendations
(`カレンダーフィード` → `予定をカレンダーに入れる`, `エクスポート`/`JSON` softening)
are copy-tone refinements deferred to RFC-054 (native-speaker copy review), since
they are judgment calls best made by a Japanese reviewer, not unilateral string edits.

### Testing

- 218 passing (was 216). Zero warnings. i18n parity 143/143 pairs.

## [0.35.0] — 2026-06-13

Comprehensive audit: RFC-to-code verification, ad hoc code review, doc accuracy.

### Fixed

- **Day-not-found and event-not-found in `post_my_status` return 404 not 500.**
  Both errors previously raised `worker::Error::RustError("…")` which maps to a
  500. An invalid `day_id` in the URL (not belonging to the event) or a deleted
  event now returns `render::not_found()` instead, consistent with the community
  isolation RFC-004 principle.

- **Dead POST `/c/:cid/select` route removed.** No form in the application
  generates a POST to this URL (the community switcher uses a GET to `/switch`).
  The route was a leftover from an earlier iteration. Removing it avoids confusion
  for future readers.

- **`authz.rs` dead_code allow documented.** `MembershipContext.community_id` and
  `.user_id` are populated but not read by current handlers (they use the URL
  parameter directly). The file-level `#![allow(dead_code)]` was unexplained;
  it now has a comment describing which fields are unused and why they are kept.

### Documentation

- **README and `docs/src/quick-start.md` test command corrected.** Both said
  `cargo test -p zinnias-ciao-domain -p zinnias-ciao-contracts`, omitting
  `-p zinnias-ciao-ssr`. Developers following the README would silently skip
  13 SSR tests including the release gates.

### Verified (no changes needed)

RFC-to-code audit confirmed the following are correctly implemented:
- RFC-004 community isolation: every community-scoped handler calls
  `require_membership` or `require_admin`.
- RFC-006 status lifecycle: `validate_status_transition` enforces all state/role
  rules; NULL (no answer) is distinct from explicit values.
- RFC-007 note safety: 200-char limit enforced; all user text goes through
  `escape_html` before rendering.
- RFC-012 HMAC pepper: all secrets go through `crypto::pepper(env)`.
- RFC-013 CSRF: every state-changing handler consumes a form token.
- RFC-018/039 timezone write path: `local_to_utc` used in event creation/edit.
- RFC-037 token subject: token bound to `auth.user_id` + purpose + optional resource.
- RFC-038 session secrets: stored as HMAC hash only, never plaintext.
- RFC-041 atomic invite redemption: join-ticket cookie ensures two-phase commit.
- RFC-042 private cache: `Cache-Control: no-store` on all authenticated responses.
- RFC-046 event-bound token: `SET_STATUS` bound to `event_id`, day validated in handler.
- RFC-048 security headers: CSP, DENY framing, `base-uri 'none'`, Referrer-Policy all present.
- RFC-055 offline contract: service worker blocks non-GET; app.js disables submit buttons.
- Migration schema matches all DB query column names.

### Testing

- 216 passing. Zero warnings.

## [0.34.3] — 2026-06-12

Safety: remove silent panic paths in worker hot paths.

### Fixed

- **`rate_limit::record_failure` — removed `.unwrap()` on KV `put()` builder.**
  `kv.put(key, value)` returns `Result<KvBuilder, KvError>`. The previous
  `.unwrap()` would panic if the builder construction failed (e.g., a future
  refactor passes an oversized value). Replaced with `let Ok(put) = … else { return }`
  — rate-limit failures are already silently degraded (the `let Ok(kv)` guard
  at function entry does the same); consistency is the goal, not
  correctness-under-normal-conditions.

- **`join.rs` — `.unwrap()` on `invite` replaced with `.expect(…)`.**
  The unwrap was safe (guarded by an early return on `None`) but left no
  explanation. Now `.expect("invite is Some: None case returned early above")`
  so a future reader can verify the invariant without tracing the control flow.

### Testing

- 216 passing. Zero warnings.
- Post-fix audit: all remaining `.unwrap()` calls in non-test production paths
  confirmed absent. Remaining `.unwrap()` calls are exclusively inside
  `#[cfg(test)]` test functions in `event_admin.rs` and `tz.rs`.

## [0.34.2] — 2026-06-12

Code quality: wrong keep-note string fixed; set_result documented; parity updated.

### Fixed

- **Wrong string for "Keep note" button.** The admin hide-note confirm page and
  the member delete-note confirm page were using `JA_ADMIN_INVITES_REVOKE`
  ("無効化" — revoke/invalidate) as the "keep" button label. This is semantically
  wrong. A new paired constant `JA_NOTE_KEEP_ACTION = "メモを保持"` /
  `EN_NOTE_KEEP_ACTION = "Keep note"` was added and both pages updated.

### Changed

- **`form_token::set_result` `#[allow(dead_code)]` documented.** The function was
  written as deferred infrastructure for RFC-037 §4 (idempotency replay) but is
  not called by any current handler. The allow is correct; the comment now explains
  the reason, expected use in RFC-044 integration harness, and why the function is
  retained rather than deleted.

- **i18n parity gate updated.** `EN/JA_NOTE_KEEP_ACTION` added to the parity test
  (142 pairs, balanced).

### Testing

- 216 passing. Zero warnings. Parity: 142/142 EN/JA pairs, no duplicates.

## [0.34.1] — 2026-06-12

Documentation housekeeping: release checklist, launch runbook, ROADMAP.

### Changed

- **release-checklist.md:** three stale entries corrected:
  - i18n parity count updated from 120 to 141 pairs.
  - CSP `base-uri` entry corrected from `'self'` to `'none'` (tightened in v0.30.x).
  - RFC-055 offline submit-button gate added.
  - New v0.34.0 section: i18n parity gate and static query-count gates.
- **launch-runbook.md:** version references updated from v0.23.0 / v0.8.0
  to v0.34.0.
- **ROADMAP.md:** status section rewritten to accurately reflect the v0.34.0
  complete state; remaining pilot gates listed with explicit owners.

### Testing

- 216 passing. Zero warnings.

## [0.34.0] — 2026-06-12

Release gate hardening: i18n parity gap closed, static query-count guards added.

### Changed

- **i18n parity gate now covers all 141 EN/JA constant pairs.** The parity test
  in `release_gates.rs` was checking 120 pairs while 21 new constants added in
  v0.33.x went unregistered. All 21 pairs are now enumerated in the test, so a
  missing or empty JA string causes `cargo test` to fail immediately.

- **Duplicate EN_ADMIN_MEMBERS_TITLE removed.** A duplicate constant was
  introduced in v0.33.0 causing EN count (142) to exceed JA count (141). Removed
  the duplicate; counts are now 141/141.

- **Static query-count gate tests added (RFC-044 §6.1).** Three new tests in
  `release_gates.rs` read the handler source files via `include_str!` and assert
  that `.await` call counts stay within expected ceilings:
  - `home_handler_await_count_within_budget` — home.rs ≤ 2× `QUERY_BUDGET_HOME`
  - `event_detail_handler_await_count_within_budget` — event.rs ≤ 50 total
  - `export_handler_await_count_within_budget` — export.rs ≤ 30 total

  These fire on every `cargo test` run without a live database. They catch
  major N+1 regressions; the precise per-route live assertions remain in RFC-044
  pending staging infrastructure.

### Testing

- 216 passing (was 213). Zero warnings.
- i18n parity: 141 EN/JA pairs, all verified non-empty and non-identical.

## [0.33.1] — 2026-06-12

Complete EN→JA sweep: event form field labels and remaining error page titles.

### Changed

- **Event form field labels fully Japanese.** The `event_form_fields` helper
  passed EN strings (`"Title"`, `"Date"`, `"Start time"`, `"End time"`,
  `"Location (optional)"`, `"Description (optional)"`) directly to the `field()`
  closure. These are now `JA_FORM_FIELD_*` constants.
- **Remaining page titles fixed.** Two `render::page("Configuration error", …)` calls
  replaced with `i18n::JA_GENERAL_ERROR`.
- **New i18n constant pairs:** `FORM_FIELD_TITLE`, `FORM_FIELD_DATE`, `FORM_FIELD_START`,
  `FORM_FIELD_END`, `FORM_FIELD_LOCATION`, `FORM_FIELD_DESC`. All paired EN/JA.

### Testing

- 213 passing. Zero warnings. i18n parity gate passes.
- Final HTML-content scan confirms no bare EN words remain in user-facing HTML output.

## [0.33.0] — 2026-06-12

Complete EN→JA rendering sweep: all user-visible strings now Japanese.

### Changed

- **All user-visible strings converted to Japanese.** The RFC-049 sweep replaced
  `EN_*` i18n constants but missed inline string literals inside `format!` macros
  across every handler. v0.33.0 completes the sweep:
  - All `render::page(title, …)` and `render::header_with_switcher(title, …)` call
    sites now use `JA_*` constants.
  - All inline HTML text in `format!` blocks (h1 headings, button labels, paragraph
    copy, aria-labels, select option labels, status counts, confirmation dialogs,
    error messages) now use `JA_*` constants.
  - Offline fallback page (`static_files.rs`) converted to `lang="ja"` and Japanese copy.
  - `render::not_found()`, `render::internal_error()`, `render::session_expired()`,
    and `render::placeholder()` all use Japanese copy.
  - `admin/events.rs`: create, edit, cancel, attendance, hide-note pages.
  - `admin/members.rs`: invite, members list, remove-member pages.
  - `event.rs`: event detail counts, "Who's going?", "Notes", cancelled badge,
    delete-note confirm.
  - `calendar.rs`: calendar feed page title, offline unavailable messages.
  - `me.rs`: data export and calendar feed section labels.
  - `export.rs`: export page heading.
  - `communities.rs`: "Current" badge.
  - `templates.rs`: Use and Delete buttons.

- **New i18n constants (all paired EN/JA):** `NOTE_DELETE_BODY`, `NAV_BACK`,
  `GENERAL_BACK`, `ADMIN_EDIT_CANCELLED`, `ADMIN_EDIT_STARTED`,
  `ADMIN_ATTEND_CANCELLED`, `NOT_FOUND`, `INTERNAL_ERROR`, `EVENT_CANCELLED_BADGE`,
  `EVENT_WHOS_GOING`, `EVENT_NOTES_SECTION`, `TZ_ERROR`, `CURRENT_BADGE`,
  `ME_CALENDAR_LABEL`, `ME_DATA_EXPORT`. The i18n parity gate (`release_gates.rs`)
  enforces every JA constant has an EN pair.

### Testing

- 213 passing. Zero warnings. i18n parity gate passes.
- `parse_utc_display_uses_ja_format` and `status_display_going/not_going` regression
  guards remain in place.

## [0.32.0] — 2026-06-12

Complete Japanese rendering: home card dates and labels; render.rs tests.

### Changed

- **Home event card fully Japanese.** Several strings in `render::event_card` and
  its date-display helpers were hardcoded English and missed by the RFC-049
  `i18n::EN_*` sweep:
  - `apply_offset_display` / `parse_utc_display` — home card date labels now use
    `tz::date_label_ja`, producing e.g. `6月14日（土） 09:00` instead of `"Jun 14, 09:00"`.
  - Status counts row — `"Going N · No Go N · No answer N"` replaced with
    `JA_STATUS_GOING`, `JA_STATUS_NOT_GOING`, `JA_STATUS_NO_ANSWER`.
  - `"Cancelled"` event badge — replaced with `JA_ADMIN_CANCEL_EVENT_CONFIRM`.
  - `"N days"` multi-day badge — replaced with `N 日間`.
  - `admin_note_hide_form` link label — replaced with `JA_NOTE_DELETE`.
  - Empty participant list message — replaced with `JA_EVENT_MEMBER_FALLBACK`.

### Testing

- **5 new render tests** (213 total, was 208):
  - `parse_utc_display_uses_ja_format` — asserts the home card date contains 月/日,
    not "Jun". This is a regression guard against EN date format re-appearing.
  - `status_display_going` / `status_display_not_going` — assert labels are not
    English ("Going", "No Go").
  - `status_display_no_answer_is_default` — unknown status maps to the No Answer label.
  - `initials_japanese_name` — kanji name produces two-character initials.
- Zero warnings.

## [0.31.0] — 2026-06-12

Final in-repo pre-pilot work: offline submit-button contract.

### Changed

- **Offline submit-button disabling (RFC-055).** `app.js` now disables status,
  note, and attendance submit buttons while the browser is offline
  (`navigator.onLine === false`), restoring them on reconnect. A Japanese
  tooltip `オフラインです。保存はできません。` is shown. This makes the
  read-only offline contract visible to users instead of letting them hit a
  confusing network error. AD-1 preserved: without JS the form behaves
  normally (server returns a network error, which is acceptable for no-JS users).

### Verified in source

- **ICS feed scope (RFC-053 §3).** `get_ics_feed` and `build_vcalendar` emit
  SUMMARY (title), DTSTART/DTEND, LOCATION, and STATUS only. No participant
  status, notes, invite codes, or member names are included. The RFC-053
  content-scope concern from the architect review is satisfied in the existing
  code; remaining work is UX copy review.

### Documentation

- ROADMAP RFC counts corrected: 42 of 55 done, 13 proposed.
- RFC-055 moved from `proposed/` to `done/`.
- RFC-053 updated with source-verification note.

### Testing

- 208 passing. Zero warnings. SW version gate passes at v0.31.0.

## [0.30.0] — 2026-06-12

Pre-pilot hardening: security headers, Japanese rendering, timezone safety, query budget correction.

### Fixed

- **Query budget for max-recurring Event Detail (P2).** The release gate
  constant `QUERY_BUDGET_EVENT_DETAIL_MAX_RECURRING` was still 65 — the
  pre-RFC-046 value. After RFC-046 (event-bound token) the budget for any
  Event Detail render is 13 regardless of recurrence count. Updated to 13 and
  the regression guard changed to `assert_eq!(recurring, single_day)`.

### Security (RFC-048)

- **`Cache-Control: no-store` on all authenticated responses.** Applied
  globally via `attach_security_headers` when the handler has not already set
  a cache header. Static assets (CSS, JS, manifest) retain `public, max-age=N`
  since they set their own headers first. ICS and export retain `no-store, private`.
  This closes the browser-cache-after-logout risk on shared devices.
- **CSP extended.** Added `base-uri 'self'`, `form-action 'self'`,
  `object-src 'none'` to the Content Security Policy. The `style-src 'unsafe-inline'`
  exception is documented in code (272 inline `style=` attributes; extraction
  deferred to a future RFC).
- **`Permissions-Policy` header added.** Disables camera, microphone, and
  geolocation APIs.
- **`Referrer-Policy` tightened** from `strict-origin-when-cross-origin` to
  `same-origin`.

### Changed

- **Japanese-first rendering (RFC-049).** All UI strings now render from
  `JA_*` constants instead of `EN_*`. The HTML `lang` attribute is `"ja"`.
  86 call sites across 12 handler/render files updated. EN strings are
  retained in `i18n.rs` for future runtime locale switching. Zero new
  compile errors — guaranteed by the 120-pair parity gate and Rust's type system.
- **Unknown timezone is a hard error on write paths.** `tz::offset_minutes`
  now returns `Option<i32>`. Admin event create/edit returns a user-facing
  "Community timezone is not configured" error for unknown IANA zone names
  instead of silently using UTC. Display paths use the new
  `offset_minutes_or_utc` helper (UTC fallback safe for display). Tests updated.

### Testing

- 208 passing (was 207): +1 `offset_minutes_or_utc_falls_back_to_utc` test.
  Zero warnings. SW version gate passes.

## [0.29.0] — 2026-06-12

SSR crate tests fixed; admin handler split; ssr tests included in standard run.

### Fixed

- **Pre-existing broken `render::tests::title_escaped_in_shell`** — the test
  called `page(...)` which constructs a `worker::Response` via `web-sys::Headers`,
  a WASM stub that panics in native test runs. The test had been silently failing
  whenever someone ran `cargo test -p zinnias-ciao-ssr` (not the standard
  command). Fixed by removing the unreachable `page()` call; the actual assertion
  (that `escape_html` escapes `<` and `&`) is still exercised.

### Refactored

- **`admin.rs` split (1229 → 22-line facade + 855 + 388 subfiles).**
  `admin.rs` exceeded the 500 ELOC split threshold by 2.5×. The 16 public
  functions are now in two focused files under `handlers/admin/`:
  - `events.rs`: event create, cancel, edit, attendance, hide-note confirmation
    (~855 lines; large due to multi-field recurrence form)
  - `members.rs`: invite code generation/revocation and member management
    (~388 lines)
  - `admin.rs` becomes a 22-line re-export facade so all callsites in
    `community.rs` using `super::admin::*` continue to work without changes.

### Testing

- **SSR crate now included in the standard test run.** `package.json` test
  script updated to include `-p zinnias-ciao-ssr`. Total: 207 passing across
  all three crates (194 domain+contracts, 13 ssr). Zero warnings.

## [0.28.0] — 2026-06-12

Release-gate hardening: full i18n parity coverage, tested XSS escape boundary.

### Testing / release gates

- **i18n parity gate expanded from 9 to 120 pairs.** The `i18n_en_ja_parity_count`
  test in `release_gates.rs` now covers every `EN_*`/`JA_*` constant in
  `i18n.rs`. Previously only 9 of 120 pairs were spot-checked; a JA string
  going empty or missing would have silently shipped. The gate now also asserts
  `en != ja` (catches copy-paste where a JA string was accidentally set to the
  EN value), with a whitelist for the one intentionally-identical pair
  (`ciao.zinnias` product name). The gate was verified to catch a deliberate
  regression (temporarily emptying `JA_LOGOUT`).
- **`escape_html` moved to tested `contracts::html` module.** Previously the
  function lived in `workers/ssr/src/render.rs` (a `cdylib` crate, not natively
  runnable), making it impossible to unit-test. The authoritative implementation
  is now in `packages/contracts/src/html.rs` with 10 unit tests: plain text,
  each of the five escaped characters, an XSS vector, an attribute-injection
  vector, the combined `&<>"'` case, Japanese text preservation, and a
  multi-byte/special mix. `render::escape_html` now delegates to the contracts
  implementation so every production call exercises exactly the tested code path.
- **194 passing** (was 184): +10 from escape_html tests. Zero warnings.

## [0.27.0] — 2026-06-12

Handoff-review remediation: source verification, event-bound token, Japanese
dates, audit coverage.

### Verification (RFC-045)

- Discharged the entire handoff-review §8 **source-verification checklist** (11
  items) against actual source: token subjects all `auth.user_id`; atomic
  conditional consume; invite-claim-before-create ordering; HMAC-only invite
  storage; centralized `crypto::pepper`; host-only cookie default; SW caches no
  authenticated HTML; SW version gate; flat-batched export; tz conversion on
  write and display paths. All confirmed. The staging-runtime half (deploy,
  timezone round-trip, race tests) is documented in RFC-045 and remains pending
  a Cloudflare environment.

### Changed

- **Event-bound `SET_STATUS` token (RFC-046).** Event Detail now issues one
  status token bound to the event, reused for every day's form, instead of one
  token per day. Eliminates up to 52 D1 writes per render on max-recurring
  events — the last loop-based write in the hot path. Day-level authorization
  (day∈event∈community) is preserved and is now the explicit guard. The POST
  handler consumes the token bound to `event_id`.
- **Japanese date presentation (RFC-047).** Day labels now render in Japanese
  convention, e.g. `6月14日（土）09:00–10:30`, instead of `14 Jun 09:00–10:30`.
  Added pure, tested formatters to `contracts::tz`: `weekday_index` (Zeller's
  congruence), `weekday_ja`, `month_abbr_en`, `date_label_ja`, `date_label_en`.

### Added

- **Audit coverage for security-relevant non-admin events (review P1-5):**
  logout, calendar-token generation, and calendar-token revocation are now
  written to the audit log (no secrets logged). Invite redemption was already
  audited.

### Documentation

- **DST scope statement (review P1-2)** added to `docs/src/operations.md`:
  timezone conversion is validated for fixed-offset zones (e.g. `Asia/Tokyo`);
  DST-observing zones are not yet supported and must not be provisioned until
  DST support lands.

### Testing

- 184 passing (was 179): +5 date-formatter/weekday tests. Zero warnings.
- SW `CACHE_VERSION` updated to `v0.27.0`.

## [0.26.0] — 2026-06-12

RFC-044 partial: SW version gate and coverage completion.

### Testing / release gates

- **SW `CACHE_VERSION` gate** (RFC-044 §11 step 1). `release_gates.rs` now
  uses `include_str!` to read `sw.js` and the workspace `Cargo.toml` at test
  time and asserts `CACHE_VERSION` matches the package version. A mismatch
  fails `cargo test` immediately, catching forgotten SW version bumps before
  they ship. The gate was verified to catch real mismatches (tested by
  temporarily setting `v0.24.0` while on `v0.25.0`).
- **`Role::is_admin()` contract tests** in `membership.rs`: admin returns
  true, member returns false, active/removed membership states documented as
  tests.
- 179 passing (was 174). Zero warnings.

### Changed

- SW `CACHE_VERSION` updated to `v0.26.0`.
- RFC-044 status note updated to reflect partial completion.

## [0.25.0] — 2026-06-12

Query performance pass (RFC-029 / RFC-044 partial) and RFC-043 completion.

### Performance

- **Event Detail N+1s eliminated (RFC-029/RFC-044).** Two per-request N+1
  patterns are gone:
  - `attendance_db::list_for_day` was called once per event day inside the
    render loop. Replaced with a new `list_for_event_days` batch function that
    fetches all attendance rows for all days of an event in a single `IN`
    query. For a 7-day recurring event this reduces 7 attendance queries to 1.
  - `form_token::issue` for `ADMIN_HIDE_NOTE` was called once per other
    member's note during Event Detail render (effectively N writes to D1 where
    N = number of notes). Since v0.24.0's RFC-043 work changed admin note
    removal to a confirmation-page link, the token is no longer needed at
    render time. The dead token-issue loop was removed from `event.rs`.
- **Export N+1 eliminated (RFC-029/RFC-044).** `build_export` previously ran
  O(events × days) D1 queries (per-event days query, per-day attendance query,
  per-event notes query). Replaced with three batched `IN` queries — all days,
  all attendance, all notes — making export a flat 8 queries regardless of
  community size.

### Changed

- `render::note_form` — removed `delete_token` parameter (no longer needed;
  delete is now a link to the confirmation page, not an embedded form+token).
- `attendance_db::list_for_event_days` added (batch `IN` variant of
  `list_for_day`).

### Testing

- **D1 query-budget constants** added to `release_gates.rs` (RFC-044 §6.1
  compile-level gate): Home ≤ 8, Event Detail single-day ≤ 13, max-recurring
  ≤ 65, Export ≤ 8. A regression guard asserts these are positive, ordered,
  and within expected bounds.
- 174 passing (was 173). Zero warnings.

## [0.24.0] — 2026-06-12

Completes RFC-043 (pilot UX acceptance): all destructive actions now have
route-backed confirmation pages that work without JavaScript.

### Changed

- **No-JS confirmations for all destructive actions (RFC-043).** The three
  remaining `onclick="return confirm(…)"` guards were replaced with proper
  route-backed `GET` confirmation pages, matching the pattern already used by
  cancel-event and remove-member:
  - **Member delete own note:** `DELETE Note` button in Event Detail is now an
    `<a>` link to `GET /c/:cid/events/:eid/my-note/delete`, which renders a
    confirmation page with a server-issued `DELETE_NOTE` form token. The token
    is no longer pre-issued during Event Detail render (one fewer D1 write per
    page load for users who have a note).
  - **Admin remove note:** `Remove note` link in Event Detail navigates to
    `GET /c/:cid/admin/events/:eid/notes/:mid/hide`, which renders a
    confirmation page with an `ADMIN_HIDE_NOTE` token.
  - `render::note_form` signature simplified: `delete_token` parameter removed
    (the delete button is now a plain link; no token embedded in the form).
- **Docs and release-checklist corrections (RFC-038, RFC-042):**
  - Release checklist offline gates updated to reflect RFC-042 (no page cache;
    static offline fallback only).
  - Session cookie gate updated for RFC-038 host-only default.
  - Operational gate corrected: `SESSION_COOKIE_DOMAIN` is a `[vars]` binding,
    not `wrangler secret put`.
  - Launch runbook §2.2 / §2.3 corrected accordingly; version updated to v0.23.0.
  - ROADMAP status, RFC counts, and pre-pilot checklist updated.
- **SW `CACHE_VERSION`** updated to `v0.24.0`.

## [0.23.0] — 2026-06-12

Stabilization pass addressing an external architect's deep source review.
Each finding was verified against the source before fixing. No feature
expansion — correctness, security, and pilot-readiness only.

### Fixed (P0 — pilot blockers)

- **Attendance and note deletion were broken (token subject mismatch).**
  `SET_STATUS` and `DELETE_NOTE` form tokens were *issued* keyed on
  `membership_id` but *consumed* keyed on `user_id`, so the consume lookup
  always failed and members could not set Going/No Go/Attended or delete their
  own note. Both issue sites now use `user_id`, consistent with every other
  token. (`SAVE_NOTE` was already correct.)
- **Session cookie domain.** `SESSION_COOKIE_DOMAIN` was read via `env.var()`
  but documented as a secret, with an unconditional `Domain=localhost`
  fallback that breaks deployed login. Cookies are now host-only by default
  (`Option<&str>` domain, no `Domain` attribute unless explicitly configured);
  `wrangler.toml` clarifies it is a `[vars]` binding, not a secret.
- **Event times stored without timezone conversion.** Admin-entered local time
  was stored as `…Z` with no offset applied, so non-UTC communities saw wrong
  times. Added `tz::local_to_utc` (inverse of the display-side conversion) and
  wired community-timezone conversion into event create and edit. Includes the
  `09:00 Asia/Tokyo → 00:00Z` case plus day-wrap and round-trip tests.
- **Event edit silently discarded date/time.** The edit handler validated
  `day_date`/`starts_at`/`ends_at` but persisted only title/location/
  description. `edit_event` now persists single-day time changes; the edit form
  prefills current values and hides the recurrence selector. Multi-day and
  recurring events remain details-only edits.
- **Invite redemption was not safely one-time.** `mark_used` was an
  unconditional UPDATE with no affected-row check, so a race could redeem one
  invite twice. It is now a conditional state transition
  (`WHERE used_at IS NULL AND revoked_at IS NULL AND expires_at > now`) that
  returns whether it won; redemption claims the invite *first* and aborts if it
  loses the race.
- **Form-token consume was not atomic.** SELECT-then-UPDATE allowed two
  concurrent submits to both proceed. Rewritten as a single conditional UPDATE
  (`AND consumed_at IS NULL AND COALESCE(bound_resource,'')=?`) with an
  affected-row check; the zero-row case is classified (replay / invalid /
  expired) by a follow-up SELECT. The decision logic is extracted to
  `contracts::auth::classify_token_consume` and unit-tested, including an
  exhaustive guard that `changed == 0` can never proceed.
- **Service worker cached authenticated HTML.** Private community pages were
  stored in a page cache that was purged only via a best-effort JS message, a
  shared-device privacy risk; the cache version was also stale. Rewritten to
  cache static assets only; authenticated `/`, `/c/*`, and `/join` are
  network-only with a static offline fallback, and activate-time cleanup
  removes any legacy page cache.

### Fixed (P1)

- **HMAC pepper access centralized.** Six handlers read the pepper
  inconsistently (`env.secret` vs `env.var`, with two different dev fallbacks).
  All now use `crypto::pepper(env)`; `require_auth` too.
- **`generate_invite` token purpose** is now the `GENERATE_INVITE` contract
  constant rather than a raw string. While adding it we found `REMOVE_MEMBER`
  had never actually been added to the token-uniqueness tests; both are now in
  the `release_gates` and regression uniqueness sets.
- **Invisible action errors.** Create-event and event-detail error redirects
  (`?err=`) were never rendered. Both now show a visible `role="alert"` banner.
- **Session lifetime** raised from 24 h to 30 days. Invite-only members have no
  password and no self-service recovery, so a 24 h expiry generated needless
  re-invite burden. Sessions remain server-side revocable on logout.

### Fixed (P2)

- **No-JS community switcher.** The header switcher relied on a JS `onchange`
  redirect while claiming to work without JS. It is now a real
  `<form method="get" action="/switch">` with a `<noscript>` submit button;
  `onchange` auto-submit remains as progressive enhancement. The new `/switch`
  route validates the target is a community the user actually belongs to before
  redirecting.

### Tests

- 173 passing (was 160): added `local_to_utc` conversion tests and
  `classify_token_consume` race/idempotency tests. Zero warnings.

## [0.22.0] — 2026-06-12

### Documentation (docs verification pass)

All documentation was verified against the codebase. Issues found and corrected:

- **`docs/src/overview.md`:** Stack table said "Leptos SSR + minimal plain JS".
  The actual implementation uses no Leptos — plain Rust string-template SSR per
  AD-1. Corrected to "Plain Rust SSR + minimal plain JS (no browser WASM,
  no Leptos — AD-1)".

- **`wrangler.toml`:** The `[env.production]` section was missing
  `[[env.production.d1_databases]]`, `[[env.production.kv_namespaces]]`, and
  `BUILD_VERSION`. Without these, `bunx wrangler deploy --env production` would
  use the root-level bindings (which have `database_id = "local"`) or fail.
  Added the three missing blocks with `REPLACE_WITH_PRODUCTION_*` placeholders
  matching the staging pattern.

- **`docs/src/launch-runbook.md`:** Version header said "v0.8.0" (the version
  when the runbook was written). Updated to v0.21.0. Two places said "all three
  migrations applied (`0001`, `0002`, `0003`)" — there are now six migrations
  (0001–0006). Updated both to "all six migrations".

- **`docs/src/quick-start.md`:** Setup description said "applies the D1
  migration" (singular). Updated to describe what `bun run setup` actually does:
  runs all migrations, seeds a community + admin + invite code, and prints the
  code for first use.

### Confirmed accurate (no changes needed)

- `docs/src/deployment.md` — environment names, secrets, deploy commands, rollback all match.
- `docs/src/operations.md` — bootstrap SQL matches schema (incl. `grants_role` column from migration 0003); session/invite revocation SQL matches schema; audit_log query accurate.
- `docs/src/backup-recovery.md` — export command uses correct production database name.
- `docs/src/release-checklist.md` — all `[x]` claims verified against code in the audit pass.
- `docs/src/architecture.md` — rewritten in v0.19.0; accurate.

## [0.21.0] — 2026-06-12

### Security

- **Cross-community member removal (RFC-004/RFC-010 audit finding).**
  `soft_remove(db, membership_id)` accepted any `membership_id` without verifying
  it belonged to the current community. An admin of community A could remove a
  member of community B by crafting a POST to
  `/c/community_a_id/admin/members/community_b_membership_id/remove`.
  **Fixed:** `soft_remove` now requires `community_id` and adds
  `AND community_id = ?3 AND removed_at IS NULL` to the UPDATE.
  Same gap in `get_role(db, membership_id)` — now adds
  `AND community_id = ?2 AND removed_at IS NULL`. Both call sites in `admin.rs`
  updated.

### Correctness

- **Token purpose raw string literal (RFC-010 audit finding).**
  `form_token::issue`/`consume` for member removal used the raw string
  `"remove_member"` instead of a `token_purpose::` constant. This was outside
  the uniqueness test and not guaranteed to remain distinct from other purposes.
  **Fixed:** `REMOVE_MEMBER` constant added to `contracts/src/auth.rs`; wired
  in `admin.rs`; added to token uniqueness tests. Total token purposes: 18.

### Documentation accuracy

- **RFC-001 title corrected.**
  RFC-001 was titled "Cloudflare Workers, Leptos SSR, D1" and described a
  "Rust/Leptos SSR frontend". The actual implementation uses no Leptos at all —
  only `worker::*` with plain Rust string templating (per AD-1, adopted before
  coding began). RFC title and summary updated; implementation note added.

- **RFC-003 atomicity claim corrected.**
  RFC-003 and the code comment both claimed "atomic redemption". D1 via
  `worker-rs` does not support multi-statement transactions; the implementation
  uses four sequential individual queries. The code comment has been rewritten
  to accurately describe the sequential approach and its acceptable failure modes.
  RFC-003 header updated with an audit note.

### Internationalisation

- **`event.rs`:** 4 hardcoded strings wired to i18n constants — event page header
  title, "Available after the event", "Only admins can mark Attended", "Member"
  fallback name. `i18n` import added.

- **`join.rs`:** Page titles "Join" and "Your name" wired to i18n constants.

- **`templates.rs`:** All body strings wired — page h1, description, Save section
  h2, Title/Location/Duration field labels, Save button. Empty state wired.

- **`me.rs`:** About section heading, Version label, Ref label wired to i18n
  constants.

- **`i18n.rs`:** 6 new EN/JA pairs added (event page title, two attendee-status
  reasons, member fallback, two join page titles). Parity count: 114 → 120.

## [0.20.0] — 2026-06-12

### Added

- **`README.md`**: complete rewrite from placeholder title. Follows the
  project-mandated structure: hero section (CI badge, license badge, Rust badge,
  catchphrase), Overview, Why/When, Quick Start, Design Notes, More Detail table.
  Covers pure-SSR design, invite-only auth, offline behaviour, recurring events,
  templates, and community export. Links to all docs pages, ROADMAP, CHANGELOG,
  and RFC index.

- **`NOTICE`**: Apache-2.0 required notice file (was missing).

## [0.19.0] — 2026-06-12

### Documentation and project hygiene

- **`ROADMAP.md`**: new top-level roadmap listing all 32 implemented RFCs with
  version numbers, the 6 backlogged RFCs with their blockers, pre-pilot operator
  tasks, and the "after first pilot" revisit guide.

- **`docs/src/architecture.md`**: full rewrite:
  - Accurate file tree covering all new handlers (`export.rs`, `templates.rs`),
    DB modules (`event_template.rs`), and migrations (0001–0006).
  - Correct path to the Architecture Decisions document
    (`docs/src/ref/roadmap-and-rfcs-v1/ARCHITECTURE-DECISIONS.md`, not the
    previously referenced non-existent `rfcs/proposed/` path).
  - Data grain diagram showing `EventNote` alongside the main grain.
  - Test strategy section with exact counts and mandatory verification command.

- **`docs/src/release-checklist.md`**: 7 items upgraded from `[~]` to `[x]`
  based on code audit — previously marked "requires browser test" but verifiable
  by reading the implementation:
  - Offline banner: `sw.js` + `app.js` code paths confirmed.
  - Offline fallback: `OFFLINE_URL = '/offline'`, shell assets pre-cached on install.
  - Form submit offline: `sw.js` passes all non-GET requests through to network (AD-1).
  - 44px touch targets: `app.css` L88 universal selector covers all interactive elements.
  - Status icon+label+colour: `status_display()` always returns all three; AA ratios documented.
  - Reduced motion: `@media (prefers-reduced-motion: reduce)` block confirmed in `app.css`.
  - Error message plain language: `release_gates.rs` automated test confirmed.
  - Final count: 32 `[x]`, 2 `[~]` (phone-only), 4 `[ ]` (operator tasks).

- **`workers/ssr/static/sw.js`**: `CACHE_VERSION` updated from `'v0.5.0'` to
  `'v0.19.0'` to force cache invalidation on deploy.

## [0.18.0] — 2026-06-12

### Internationalisation

- **i18n pass: 74 → 114 EN/JA string pairs.**
  All user-facing strings added in v0.13.0–v0.17.0 are now wired through
  `packages/contracts/src/i18n.rs` with full EN and JA translations:
  - **Role labels** (`ROLE_ADMIN`, `ROLE_MEMBER`): wired in `me.rs`.
  - **Home first-run card** (RFC-030): 4 strings covering the welcome heading,
    no-events variant, create-first-event button, and invite-members hint.
    `home.rs` `intro` variable now uses `i18n::EN_HOME_FIRST_RUN_*`.
  - **Recurrence fields** (RFC-022): 7 strings covering the repeat label, all
    four frequency options, the count unit, and the count hint. `admin.rs`
    `event_form_fields` repeat `<select>` now uses named format args wired to
    `i18n::EN_REPEAT_*`. The previous raw string `r#"…"#` was converted to a
    `format!()` call.
  - **Event templates** (RFC-032): 11 strings covering page title, description,
    empty state, save section, field labels, and button labels. Page title and
    header wired in `templates.rs`.
  - **Community export** (RFC-027): 5 strings covering title, description,
    privacy note, download button, and single-use expiry notice. Page title and
    header wired in `export.rs`.
  - **Me / About** (RFC-035): 5 strings covering About section label, version
    label, ref label, Data section label, and export link. Role label wired.
  - **Calendar feed** (RFC-023): 6 strings covering page title, subscription
    description, and all button/action labels. Page title and header wired in
    `calendar.rs`.
  - Parity test (`i18n.rs` `en_ja_parity`) updated: count assertion changed
    from 74 to 114; all 40 new key suffixes added to the checked list.

## [0.17.0] — 2026-06-12

### Added

- **RFC-022 — Recurring events (bounded materialization).**

  Admins can now create repeating events directly from the Create Event form.
  The implementation uses **bounded materialization at creation time**: the
  handler generates all concrete `event_days` rows upfront rather than
  introducing a background scheduler or a separate series abstraction.
  Members always see concrete event instances with real dates.

  - **`packages/domain/src/event_admin.rs`:**
    - `RecurrenceFreq` enum: `None`, `Weekly`, `Biweekly`, `Monthly`.
      `from_str()`/`as_str()` for form round-trip. `is_recurring()` predicate.
    - `RECURRENCE_MAX_COUNT = 52`: hard cap on occurrences per creation.
    - `expand_recurrence(base, freq, count)`: pure function, returns a
      `Vec<DayInput>` of concrete dates. Weekly uses `time::Duration::weeks`.
      Biweekly uses 2× that. Monthly uses 0-indexed month arithmetic to avoid
      off-by-one at December→January boundaries; clamps day to end-of-month
      (e.g. Jan 31 + 1 month → Feb 28). Capped at `RECURRENCE_MAX_COUNT`.
    - 9 new unit tests covering all four frequencies, year-boundary crossing,
      end-of-February clamping, count capping, and time preservation.

  - **`migrations/0006_event_recurrence.sql`:** adds `repeat_rule` and
    `repeat_count` informational columns to `events`. The actual days are
    already in `event_days`; these columns let the export and future UI show
    what pattern was used.

  - **`db/event_write.rs`:** `create_event` gains `repeat_rule` and
    `repeat_count` parameters; stores them alongside the event row.
    Uses `worker::wasm_bindgen::JsValue::NULL` for optional `repeat_count`.

  - **`handlers/admin.rs`:**
    - `post_create_event` reads `repeat_rule` and `repeat_count` form fields,
      calls `expand_recurrence`, and passes all expanded day rows to
      `create_event`. Non-recurring events behave identically to before.
    - `event_form_fields` now includes a Repeat `<select>` (None / Weekly /
      Every 2 weeks / Monthly) and an occurrence count `<input type=number>`
      defaulting to 8, with a helper note that count is ignored for
      non-recurring events.

  - RFC-022 moved to `rfcs/done/` (v0.17.0). Total: 32 of 36 RFCs done.
  - Tests: 152 → 160 (+9 recurrence, -1 prior count correction).

## [0.16.0] — 2026-06-12

### Added

- **RFC-032 — Event templates and quick create.**
  - New migration: `migrations/0005_event_templates.sql` — `event_templates` table
    scoped to community; stores title (1–80 chars), optional location (≤120),
    optional description (≤500), optional default duration in minutes, active flag.
    Partial index on `(community_id) WHERE is_active = 1`.
  - New `db/event_template.rs`: `list_active`, `find_active`, `insert` (nullable
    fields bound via `worker::wasm_bindgen::JsValue::NULL`), `soft_delete`.
  - New `handlers/templates.rs`:
    - `GET /c/:cid/admin/templates` — lists active templates (each with a "Use"
      link and a CSRF-guarded delete button) plus a create form with title,
      location, and optional duration fields.
    - `POST /c/:cid/admin/templates` — validates `CREATE_TEMPLATE` form token,
      inserts template, audits, redirects with flash.
    - `POST /c/:cid/admin/templates/:tid/delete` — validates `DELETE_TEMPLATE`
      form token, soft-deletes, audits, redirects with flash.
  - `handlers/admin.rs` `get_create_event`: accepts `?template=:tid` query
    parameter; fetches the template and pre-fills title and location into the
    event form. Shows "Use a template" link below the submit button.
  - `contracts/src/auth.rs`: `CREATE_TEMPLATE` and `DELETE_TEMPLATE` token
    purposes (17 total).
  - Token uniqueness and regression tests updated.
  - RFC-032 moved to `rfcs/done/` (v0.16.0).

## [0.15.0] — 2026-06-12

### Added

- **RFC-027 — Admin community data export.**
  - New routes: `GET /c/:cid/admin/export` (landing page) and
    `GET /c/:cid/admin/export/json?token=…` (authenticated JSON download).
  - `handlers/export.rs`: export landing page shows community summary (event
    and member count) and a single-use download link. JSON download validates a
    `COMMUNITY_EXPORT` form token (single-use, 5-minute TTL), builds the payload,
    audits the export action, and returns `Content-Disposition: attachment` with
    `Cache-Control: no-store, private`.
  - Export payload (v1): `community` metadata, `members` list (id, display name,
    role, joined date, removed flag), `events` with days, per-day attendance
    (member name + status), and visible notes (member name + text). Admin-hidden
    and member-deleted notes are excluded. Session tokens, invite HMACs, and the
    HMAC pepper are never included.
  - `contracts/src/auth.rs`: `COMMUNITY_EXPORT` token purpose.
  - Token uniqueness tests updated (15 total purposes).
  - RFC-027 moved to `rfcs/done/` (v0.15.0).

- **RFC-035 — Support diagnostics in Me page.**
  - Me page now shows an "About" section with app version (`BUILD_VERSION` env var)
    and a short community reference code (first 8 chars of community ID) for use
    in support communication.
  - Admin Me page now shows an "Export community data" link under a "Data" section.
  - RFC-035 moved to `rfcs/done/` (v0.15.0).

- **RFC-036 — Public release readiness (formalised).**
  The launch runbook, backup/recovery doc, and release checklist collectively
  satisfy RFC-036's goals (release criteria, security review checkpoint, rollback
  procedure). No new code is needed beyond what has already shipped.
  RFC-036 moved to `rfcs/done/` (v0.15.0).

## [0.14.0] — 2026-06-12

### Added

- **RFC-028 — Backup and recovery documentation.**
  New `docs/src/backup-recovery.md` covering:
  - D1 built-in 30-day point-in-time snapshots (dashboard restore procedure).
  - Manual export with `wrangler d1 export` before every migration.
  - Restore procedure using a new D1 database from a SQL dump.
  - Recommended backup schedule (before migrations, weekly for active communities).
  - What is and is not in a backup (names/events/notes are; HMAC secrets are not).
  - Migration forward-only policy and the prohibition on deleting from `d1_migrations`.
  - Incident response checklist.
  Added to `docs/src/SUMMARY.md`. RFC-028 moved to `rfcs/done/` (v0.14.0).

- **RFC-030 — Admin first-run experience (empty states, onboarding).**
  `handlers/home.rs`: the admin empty-state is now a **first-run card** rather
  than a plain text paragraph:
  - When an admin is the only member (member_count ≤ 1) and no events exist,
    shows "Welcome. Your community is set up." with a secondary hint to invite
    members first.
  - When no events exist but members are present, shows a simpler "No events yet"
    card with the two action buttons embedded.
  - When events exist, shows the persistent admin shortcuts bar as before.
  - Non-admins continue to see the plain "Ask your community admin" message.
  RFC-030 moved to `rfcs/done/` (v0.14.0).

## [0.13.0] — 2026-06-12

### Added

- **RFC-025 — Community moderation UI (completes RFC-025).**
  The `post_admin_hide_note` handler and `admin_hide` DB function have existed
  since v0.6.0, but the hide button was never surfaced to admins in the event
  detail view. This release wires the UI:
  - `handlers/event.rs`: the other-members' notes loop converted from a sync
    `.map().collect()` to an async `for` loop that issues a per-note
    `ADMIN_HIDE_NOTE` form token for admin users. Each note card shows a "Hide"
    button (red, min-height 44px, aria-label) that POSTs to
    `/c/:cid/admin/events/:eid/notes/:mid/hide`. For non-admins the button is
    absent and no token is issued.
  - Route was already wired in `community.rs`; handler was already implemented
    and audits without preserving note body (RFC-014).
  - RFC-025 moved to `rfcs/done/` (v0.13.0). All three goals met: admin note
    hiding, member removal, and audit without harmful content exposure.

## [0.12.0] — 2026-06-12

### Performance (RFC-029 — Scalability and Query Performance Discipline)

- **N+1 query elimination on Home and Event Detail pages.**

  Home page previously issued one `counts_for_day` query per event card (N events
  = N queries, on top of the initial home_upcoming fetch). Event Detail issued
  `find_mine` + `counts_for_day` per event day (3 queries × N days).

  **New batch functions in `db/attendance.rs`:**
  - `counts_for_days(db, day_ids, member_count)` — single `GROUP BY event_day_id`
    query returning a `HashMap<day_id, DayCountRow>` for all requested days.
    Zero-fills days with no attendance rows (no_answer = member_count).
  - `list_mine_for_days` — rewritten from an N-query loop to a single
    `IN (?1, ?2, …)` query, using runtime-built positional placeholders.
    D1 supports positional `?N` placeholders; the previous comment claiming
    it did not was incorrect.

  **`contracts/src/lib.rs`:** `build_in_placeholders(count, offset)` — shared
  helper for building positional placeholder strings. 4 inline tests.

  **`handlers/home.rs`:** batch-fetches counts before the card loop using
  `counts_for_days`; removed the per-event `counts_for_day` call.

  **`handlers/event.rs`:** batch-fetches all per-day data using
  `list_mine_for_days` and `counts_for_days` before the day loop; removed
  the per-day `find_mine` and `counts_for_day` calls.

  Query count for a Home page with 10 events: **7 + N → 7** (constant).
  Query count for an Event Detail with 3 days: **8 + 3×3 → 8** (constant).

- RFC-029 moved to `rfcs/done/` (v0.12.0).

### Changed

- Total tests: 148 → 152 (+4 placeholder tests).

## [0.11.0] — 2026-06-12

### Fixed

- **SSR worker: zero warnings.** Resolved all 53 `cargo check` warnings that would
  become build failures under `worker-build --deny warnings`:
  - `calendar.rs`: removed dead local ICS builder functions (`build_ics`, `ics_text`,
    `fold_ics_line`, `utc_to_ics_dt`, `sanitize_filename`) and their tests that were
    left behind by the cleanup script (the marker used Unicode em-dashes which the
    script compared against plain hyphens). Also removed stale `token_hmac` variable
    and unused `req`/`_pp` parameters in `get_ics_feed`.
  - `me.rs`, `communities.rs`: unused `i18n` imports from failed Python wiring patches;
    re-wired the hardcoded strings correctly using named `format!()` arguments.
  - `db/event.rs`, `rate_limit.rs`, `handlers/auth.rs`, `handlers/join.rs`: removed
    stale unused imports.
  - All handler files: prefixed unused `rid` parameters with `_` across `admin.rs`,
    `event.rs`, `home.rs`, `join.rs`, `calendar.rs`.
  - All handler files: prefixed unused local variables (`_membership`, `_community_name`,
    `_community_tz`, `_all_members`, `_current_name`) with `_`.
  - `render.rs`, `errors.rs`, `form_token.rs`: added `#[allow(dead_code)]` to
    forward-declared design vocabulary items (constants, helper functions) that are
    part of the intended API but not yet fully wired to call sites.

- **CI** (`check-wasm` merged into `test` job): the `cargo check --target wasm32` step
  now runs in the same job as `cargo test`, sharing the build cache. A green test run
  is no longer possible without also passing the wasm type-check.

## [0.10.0] — 2026-06-12

### Added

- **RFC-026 — i18n wiring: all user-visible strings through constants (partial).**
  - `packages/contracts/src/i18n.rs`: expanded from 26 to 74 EN/JA constant pairs.
    Added: Nav, Home section labels, admin shortcuts, Status Clear, Note form labels,
    Me section headings, all Admin event/invite/member strings. Parity lint updated
    from 26 to 74 keys — any new string without a JA counterpart fails immediately.
  - `render.rs`, `home.rs`, `me.rs`, `communities.rs`, `admin.rs`: all user-visible
    strings wired to `i18n::EN_*` constants.
  - RFC-026 moved to `rfcs/done/` (v0.10.0). Partial: EN/JA string table complete and
    enforced; per-community language selection deferred post-MVP.

- **RFC-023 — ICS calendar export (fully implements RFC-023).**
  - `migrations/0004_calendar_tokens.sql`: `calendar_tokens` table — one active token
    per (membership_id, community_id), HMAC stored, revocable via `revoked_at`.
  - `packages/contracts/src/ics.rs`: pure-Rust ICS formatting — `build_vcalendar`,
    `ics_text` (RFC 5545 escaping), `fold_line` (75-octet folding, UTF-8 boundary
    safe), `utc_to_ics_dt`, `sanitize_filename`. 17 inline tests.
  - `db/calendar.rs`: `find_by_hmac`, `find_active_for_membership`, `insert`,
    `revoke_for_membership`, `events_for_feed`.
  - `handlers/calendar.rs`: four routes wired in `community.rs`:
    - `GET  /c/:cid/me/calendar` — member calendar page (show URL, revoke, regenerate).
    - `POST /c/:cid/me/calendar/regenerate` — rotate token (CSRF-guarded).
    - `POST /c/:cid/me/calendar/revoke` — disable feed (CSRF-guarded).
    - `GET  /c/:cid/cal/:token` — unauthenticated ICS bearer feed; validates HMAC,
      checks membership still active, returns `text/calendar; charset=utf-8` with
      `Cache-Control: no-store, private`.
  - Feed content: title, times, location, cancellation status only — no names, notes,
    invite codes, or participant counts.
  - Me page: "Calendar feed" link added. `db/membership.rs`: `find_active_by_id` added.
  - `contracts/src/auth.rs`: `CALENDAR_REGENERATE` and `CALENDAR_REVOKE` token purposes.
  - RFC-023 moved to `rfcs/done/` (v0.10.0).

- **SSR crate build verified.** `admin.rs` (i18n wiring syntax errors fixed),
  `home.rs` (community fetch moved before event loop), `community.rs` — all compile
  clean under `--target wasm32-unknown-unknown`, zero errors.

### Changed

- Parity lint count: 26 → 74. Token purpose count: 12 → 14.
- Total tests: 131 → 148.

## [0.9.0] — 2026-06-12

### Added

- **Launch runbook (`docs/src/launch-runbook.md`).** Step-by-step operator guide
  covering all seven phases: resource provisioning (D1 + KV for staging and
  production), secret generation and setting, migration application, build and
  deploy, staging QA, production seed, Logpush configuration, and final security
  review. Includes a rollback procedure and post-launch monitoring thresholds.

- **Staging environment (`wrangler.toml`, `package.json`).** `[env.staging]` block
  added to `wrangler.toml` with D1 and KV placeholders. `migrate:staging` script
  added to `package.json`. The deployment docs referenced staging but it was never
  actually configured.

### Changed

- **`docs/src/operations.md`** updated: bootstrap section now references `setup.mjs`
  and migration 0003's `grants_role` column (the old raw-SQL example was missing it);
  incident-response and log-access sections improved with exact `wrangler d1 execute`
  commands and a note on Logpush.

- **`docs/src/SUMMARY.md`**: launch runbook added to the docs navigation.

## [0.8.0] — 2026-06-12

### Added

- **Invite code revocation — closes the last functional release gate.**
  - `db/invite.rs`: `revoke(invite_id, community_id)` soft-sets `revoked_at`; scoped to
    `community_id` to prevent cross-community revocation. `list_active_for_community`
    returns metadata (id, expires_at, grants_role) for unused/unrevoked/unexpired codes
    — never the HMAC. `InviteMetaRow` struct.
  - `contracts/src/auth.rs`: new `token_purpose::REVOKE_INVITE`.
  - `handlers/admin.rs`: `post_revoke_invite` — CSRF-guarded, community-scoped, audited.
    `get_invites` rewritten: shows active codes list with per-row revoke buttons (token
    issued per code at render time); new-code confirmation banner uses AA-passing green.
  - Route wired: `POST /c/:cid/admin/invites/:iid/revoke`.

- **Release checklist ticked.** `docs/src/release-checklist.md` updated with
  code-verified `[x]` items (28 gates confirmed by inspection/tests) and `[~]` for
  browser/ops items that require a human pass. Four operator tasks remain `[ ]`
  (staging migration rehearsal, production secrets, Logpush, final security review).

### Changed

- `release_gates.rs` and `token_and_color_regression.rs`: `REVOKE_INVITE` added to
  token completeness and uniqueness tests (12 total purposes).

## [0.7.0] — 2026-06-12

### Added

- **RFC-018 — Timezone display (completes RFC-018).**
  - `packages/contracts/src/tz.rs`: IANA timezone name → UTC offset table (pure Rust,
    no OS timezone data). Covers UTC, major Asia/Pacific, Europe, Americas, Oceania zones.
    Unknown names fall back to UTC. `to_local_parts(utc, offset)` handles month-end,
    year-end, and leap-year Feb boundaries correctly.
  - `render.rs`: `format_day_time_tz(day, tz)` applies the community timezone for event
    time display. Internal helpers delegate to `contracts::tz`. Public re-exports for
    handler use (`tz_offset_minutes_pub`, `utc_to_local_parts_pub`, `apply_offset_time_pub`).
  - `handlers/home.rs`: fetches `community.timezone` and passes it to every
    `render::event_card` call. Previously all Home times displayed as UTC.
  - `handlers/event.rs`: fetches `community.timezone` before the day loop;
    `format_day_label` applies the offset for Event Detail time display.
    `classify_day` made `pub` for use by `admin.rs`.
  - `handlers/admin.rs`: `get_edit_event` / `post_edit_event` now reject edits once any
    day of the event has started or ended (RFC-018 §5 cutoff requirement). Previously
    only cancelled events were blocked.

- **16 new timezone tests (`contracts/src/tz.rs`).**
  Covers: UTC identity, Tokyo (same-day, next-day), New York (previous-day), Kolkata
  (half-hour offset), month boundary (both directions), year boundary, leap and non-leap
  Feb, unknown fallback, exact midnight, `days_in_month` table.

### Changed

- **RFC audit — 19 RFCs moved to `rfcs/done/`** (RFC-000 lifecycle policy).
  001–017 + 018 + 019. Status fields set to `Implemented (vX.Y.Z)`.
  `rfcs/README.md` rewritten with Done / Proposed / Backlog sections.

- **CI `migration-check` job** now validates all `migrations/*.sql` files: existence,
  non-empty, sequential numbering. Previously only checked `0001_initial.sql`.

## [0.6.0] — 2026-06-12

### Added

- **RFC-020 v1.2 — Status token triplets and WCAG AA fix.**
  - `app.css`: status triplet CSS variables (`--cz-status-{going,not-going,attended,no-answer}-{fg,bg,border}`).
    Raw `--cz-color-*` retained for decorative fills only.
  - `render.rs`: `CZ_STATUS_*` Rust consts mirror the CSS vars 1:1. `status_display` returns
    AA-passing foreground values. New `status_triplet` helper returns `(fg, bg, border)`.
  - `render::status_form`: buttons use triplet bg/border; selected state no longer inverts to
    `#FFFFFF` text on a raw iOS color (which failed AA for all three status colors).
  - `render::note_form`: flash uses AA-passing green; added "Community members can see this note."
    visibility disclosure (RFC-020 §19.3).
  - New `render::admin_note_hide_form`: scoped remove-note form for admin moderation.

- **RFC-020 v1.2 — Three scheduled admin handlers (decision 3).**
  - `get_edit_event` / `post_edit_event`: edit title/location/description on a scheduled event.
  - `get_attendance` / `post_attendance`: per-member attendance-correction screen distinct from
    admin's own status (RFC-020 §18.7). Batch `<select>` per member per day.
  - `post_admin_hide_note`: soft-hide any member's note without copying body to audit (RFC-014).
  - Two new token purposes: `ATTENDANCE_OVERRIDE`, `ADMIN_HIDE_NOTE`.
  - Routes wired: GET/POST `…/admin/events/:eid/edit`, `…/admin/events/:eid/attendance`,
    POST `…/admin/events/:eid/notes/:mid/hide`.

- **Tests — 9 new contracts tests (`token_and_color_regression.rs`).**
  - WCAG AA contrast verified via computed relative-luminance for all four status fg values.
  - Negative test confirms old iOS colors fail AA on text (proving the fix was necessary).
  - Token-purpose uniqueness guard across all 11 purpose strings.

- **`migrations/0003_invite_grants_role.sql`**: adds `grants_role TEXT NOT NULL DEFAULT 'member'`
  to `invite_codes` (CHECK: `'admin'` or `'member'`). Enables the setup bootstrap invite to
  grant admin role on first sign-in; admin-generated invites continue to grant member role.

### Fixed

- **Admin bootstrap: join handler hardcoded `role = 'member'`** for every invite redemption.
  First sign-in via the setup-printed code was silently created as a member; all admin routes
  returned generic 404 and admin UI was invisible.
  - `db/invite.rs`: `InviteRow` carries `grants_role`; `find_valid` and new `find_by_id` select it;
    `insert` accepts it as a parameter.
  - `handlers/join.rs` (`post_profile`): fetches invite by ID and uses `invite.grants_role`.
  - `handlers/admin.rs` (`post_generate_invite`): passes `"member"` explicitly.
  - `scripts/setup.mjs`: seeds bootstrap invite with `grants_role = 'admin'`.

- **Communities page showed raw IDs instead of community names.**
  `get_communities` used `list_active_for_user` (no community name); fixed to
  `list_communities_for_user` which JOINs `communities`.

- **Admin management links missing from Communities page.** "Invite members" and
  "Manage members" links added for communities where `role == "admin"`.

- **Admin shortcuts missing from Home page.** "+ Create event" and "Invite members"
  buttons added at top of Home `<main>` for admin users.

- `release_gates.rs`: extended token-purpose completeness test to include `EDIT_EVENT`,
  `ATTENDANCE_OVERRIDE`, `ADMIN_HIDE_NOTE`; removed two pre-existing unused-import warnings.

### Changed (RFC-020 v1.2 reconciliation)

- Token CSS vars and Rust consts split into AA-passing triplets; semantic names unchanged.
- `note_form` hint text and border reference `CZ_COLOR_TEXT_SECONDARY` / `CZ_BORDER` consts
  (were hardcoded hex).

## [0.5.0] — 2026-06-12

### Added

- **M5 — Security hardening and release gates.**
  - `rate_limit.rs`: KV-backed invite-redemption failure counter (10 failures per
    5-minute window per IP, cleared on success). Wired into `post_join`: check
    before any DB work, record on bad code, clear on successful redemption.
  - `handlers/me.rs`: Me page — display name, community + role, help text,
    logout form with form-token guard.
  - `handlers/communities.rs`: Communities list — all user communities with
    current badge, "Join another community" link.
  - Community dispatcher updated: `/me` and `/communities` routed.
  - `domain/tests/security_tests.rs`: XSS pass-through contract, control-char
    rejection, error-message internal-term guards, audit key documentation.
  - `domain/tests/event_admin_tests.rs`: 13 release-gate cases for event
    validation including multi-day, field lengths, plain-language error check.
  - `contracts/tests/release_gates.rs`: session TTL bounds, leeway-edge
    regression, error model, token purpose completeness, EN/JA i18n spot-check.

- **RFC-011 — Accessibility and design system.**
  - `static/app.css`: all CSS custom property tokens renamed to `--cz-*` prefix
    (`--cz-color-*`, `--cz-space-*`, `--cz-radius-*`, `--cz-touch-min`). Names
    map 1-to-1 with the future RFC-020 token JSON deliverable.
  - `render.rs`: named Rust `const` values (`CZ_COLOR_*`, `CZ_BORDER*`) mirror
    the CSS tokens, keeping inline styles in sync.
  - `render.rs`: four inline SVG icon constants (`ICON_GOING`, `ICON_NOT_GOING`,
    `ICON_ATTENDED`, `ICON_NO_ANSWER`) replace Unicode characters (✓ ✕ ○).
    Each is a 1em × 1em `aria-hidden` SVG with `fill='currentColor'`. Status is
    never conveyed by colour alone (RFC-011 §8).

- **M6 — Deployment and operations.**
  - `.github/workflows/ci.yml`: format, clippy (`-D warnings`), native tests,
    wasm32 type-check, migration existence check.
  - `docs/src/`: overview, quick-start, deployment, operations, architecture,
    release checklist covering all RFC-015 gates.

- **Community switcher in header.**
  - The community name label in every community-scoped page header is now a
    `<select>` that navigates to `/c/:cid/home` on change (`onchange` JS;
    falls back gracefully without JS). Populated via a single
    `JOIN community_memberships → communities` query per render.
  - `db/membership.rs`: `list_communities_for_user` helper added.

- **Dev setup script (`scripts/setup.mjs`).**
  - Generates the initial invite code automatically (same alphabet as Rust
    `INVITE_CODE_ALPHABET`, no ambiguous chars). No `--code` option.
  - `-y` / `--yes` skips all confirmation prompts including wrangler's own
    migration prompt (detaches stdin so wrangler sees non-TTY).
  - `--reset` wipes `.wrangler/state/v3/d1/` before running.
  - `--community` / `--admin` for custom seed names.
  - Prints the generated invite code in a summary box at the end.

- **`migrations/0002_form_tokens_nullable_user.sql`**: recreates `form_tokens`
  without the `REFERENCES users(id)` FK that caused a 500 on `GET /join`
  (pre-auth tokens have no user row yet).

### Fixed

- `form_tokens.user_id`: removed FK constraint that caused
  `FOREIGN KEY constraint failed` on `GET /join` (pre-auth tokens). Sentinel
  changed from `"anon"` to `""` throughout `join.rs`.
- `form_token::issue` / `consume`: standardised all call sites to `auth.user_id`
  (was inconsistently mixing `membership.membership_id`). Fixed logout 500 where
  issue used `membership_id` but consume used `user_id`.
- `scripts/setup.mjs`: `bun run setup -- -y` hung at wrangler's migration
  confirmation prompt. Fixed by passing `stdio: ['ignore', 'inherit', 'inherit']`
  when `-y` is active, making stdin non-TTY so wrangler skips its prompt.
- `package.json`: `test` and `lint` scripts corrected to use
  `zinnias-ciao-domain` / `zinnias-ciao-contracts` crate names.

## [0.4.0] — 2026-06-12

### Added

- **M3 PWA + offline (read-only).**
  - `static/manifest.webmanifest`: installable PWA — name, icons, `display:standalone`,
    start URL `/`, theme `#007AFF`.
  - `static/sw.js`: service worker — shell cache-first (versioned), page network-first
    with offline cache fallback, deploy cache-bust on activate, `PURGE_PRIVATE` message
    clears page cache on logout; never caches POSTs or cross-origin responses.
  - `static/app.js`: SW registration, offline banner toggle on network events, Unicode-
    aware note character counter (progressive enhancement), logout cache-purge trigger.
  - `static/app.css`: design tokens (RFC-011 colour/spacing/radius), base reset, offline
    banner, focus ring, reduced-motion support.
  - `handlers/static_files`: serve manifest, `sw.js` (no-cache), CSS, JS, and
    `/offline` fallback page from `include_str!` at compile time.
  - `/offline` route added to router.

- **M4 admin flows.**
  - `domain/event_admin`: `validate_event` — title/location/description length,
    ≥1 day required, per-day end-after-start check, normalisation.
  - `db/event_write`: `create_event` (event + N day rows), `edit_event`,
    `cancel_event` (soft).
  - `db/membership`: `count_admins`, `get_role`, `soft_remove`.
  - `handlers/admin`: create event (GET form + POST), cancel event (GET confirmation
    + POST), generate invite code (GET + POST — plaintext shown once via redirect,
    HMAC stored, audit written without plaintext), list members, remove member
    (GET confirmation + POST with last-admin guard).
  - Community dispatcher extended with all admin GET/POST routes.
  - `crate alias fix`: `admin.rs` `use contracts::` → `use zinnias_ciao_contracts::`.

## [0.3.0] — 2026-06-12

### Added

- **M2 member flow complete.**
- `domain`: `note` module — Unicode-aware ≤200-char validation, control-char guard,
  newline/tab allowed; XSS payload passthrough (escaping is the renderer's job).
- `db/event`: `find_for_community`, `days_for_event`, `home_upcoming` (bounded
  date-window query with per-day JOIN, no N+1).
- `db/attendance`: `find_mine`, `list_for_day`, `counts_for_day` (NULL = No answer
  preserved), `upsert` (INSERT OR REPLACE with explicit NULL for clear),
  `list_mine_for_days` (Home batch helper).
- `db/event_note`: `find_mine`, `list_for_event`, `upsert`, `soft_delete`,
  `admin_hide`.
- `db/membership`: `count_active`, `list_all_active`, `MemberSummary`.
- `handlers/home`: upcoming list grouped Today / This Week / Later; per-card
  status chip, counts, multi-day badge; empty state (member/admin variants).
- `handlers/event`: `get_event_detail` (full day loop — status form per day,
  counts, participant list ordered Going→Attended→No Go→No answer, notes list);
  `post_my_status` (form-token CSRF + idempotency, `validate_status_transition`,
  upsert, audit for admin attendance correction); `post_my_note` (form-token,
  `validate_note`, upsert); `delete_my_note` (form-token, soft-delete).
- `handlers/community`: full GET and POST dispatcher — parses `/c/:cid/...` to
  home, event detail, status, note, and note-delete routes.
- `render`: shell, `escape_html`, `bottom_nav`, `header`, `status_chip`,
  `status_form` (three-button group with Clear, disabled+reason for Attended),
  `note_form` (Save + Delete, character counter hint), `event_card`,
  `participant_list`, `session_expired` page.
- Crate alias fix: all test files and handler `use` paths updated to
  `zinnias_ciao_domain` / `zinnias_ciao_contracts`.
- `#![allow(dead_code)]` on forward-declared DB structs and helpers (used M3+).

### Fixed

- Inner `use domain::` in test functions updated to `zinnias_ciao_domain`.

## [0.2.1] — 2026-06-12

### Fixed

- `wrangler.toml`: `worker-build` was invoked from the workspace root, where
  `Cargo.toml` has only `[workspace]` and no `[package]`. `worker-build` requires
  a crate-level manifest. Fixed by passing the crate path as a **positional**
  argument — `worker-build --release workers/ssr`. The `--path` flag does not
  exist in `worker-build`; passing it caused it to be forwarded to `cargo` as an
  unknown flag, leaving the crate path unset and the root manifest found again.
- `wrangler.toml`: `main` was pointing at `workers/ssr/src/lib.rs` (the Rust source
  file). Wrangler must point at the build output. Changed to
  `workers/ssr/build/index.js`, which is where `worker-build` writes its output
  (default `out-dir = "build"` relative to the crate root).

## [0.2.0] — 2026-06-12

### Added

- **M1 trust boundary complete.**
- `domain`: `invite` module (code validation, normalization, alphabet); `display_name`
  module (Unicode-aware length, control-char guard).
- `contracts`: `i18n` module (EN/JA string table, parity lint test);
  `SESSION_COOKIE_NAME` exported; `FORM_TOKEN_TTL_SECONDS` constant.
- `ssr` worker: `db/` layer (session, invite, membership, community — all parameterized,
  no string-concatenation SQL); `session` middleware (cookie extraction, `build_session_cookie`
  with Max-Age from constant only); `form_token` service (issue, consume, set_result —
  single-use CSRF + idempotency, AD-4); `authz` guard (`require_membership`,
  `require_admin` — generic 404 on missing/removed member, RFC-004); `audit` writer
  (structured, key-redacted, request_id tagged); `errors` module.
- Real `handlers/join` (invite redemption → display-name → atomic user+membership+session
  creation, audit, cookie set) and `handlers/auth` (logout, revoke, cookie clear).
- Migration `0001_initial.sql` unchanged (already complete).
- **Regression test** for session-TTL decoupling (the Max-Age=0 cookie-discard bug, RFC-003 §8).
- Integration tests for invite validation and display-name validation.

## [0.1.0] — 2026-06-12

### Added

- Cargo workspace layout: `packages/domain`, `packages/contracts`, `workers/ssr`.
- `domain` crate: `AttendanceStatus`, `DayTimeState`, `validate_status_transition`,
  `Event`, `EventDay`, `Community`, `Membership`, `Role`, `SessionState`.
- `contracts` crate: `SESSION_TTL_SECONDS` / `FORM_TOKEN_TTL_SECONDS` constants,
  `token_purpose` strings, `AppError` / `ErrorCode` (plain-language error model),
  `EventCapabilities` and full view-model types.
- `ssr` worker: request router, `request_id` generation, security-header middleware,
  `crypto` module (HMAC-SHA256, `random_token`, `normalize_invite_code`),
  placeholder render and handler stubs for all M0 routes.
- Migration `0001_initial.sql`: all RFC-002 tables and indexes
  (communities, users, memberships, invite\_codes, sessions, events, event\_days,
  attendances, event\_notes, form\_tokens, audit\_log).
- `wrangler.toml`: dev / production environments, D1 + KV bindings.
- `package.json`: `setup` / `dev` / `test` / `lint` scripts.
- Tests: status transition matrix, error-message language guards,
  HMAC/crypto unit tests, HTML-escape tests, TTL regression guards.
