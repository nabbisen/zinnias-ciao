# Changelog

All notable changes to ciao.zinnias are documented here.

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
