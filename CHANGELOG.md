# Changelog

All notable changes to ciao.zinnias are documented here.

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
- Migration `0001_initial.sql`: all RFC-002 tables and indexes.
- `wrangler.toml`: dev / production environments, D1 + KV bindings.
- `package.json`: `setup` / `dev` / `test` / `lint` scripts.
- Tests: status transition matrix (10), error-message language guards (2),
  HMAC/crypto unit tests, HTML-escape tests, TTL regression guards (2).
