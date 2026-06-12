# Changelog

All notable changes to ciao.zinnias are documented here.

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
