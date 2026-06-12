# Changelog

All notable changes to ciao.zinnias are documented here.

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
