# Changelog

All notable changes to ciao.zinnias are documented here.

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
