# RFC 013 — Internal Contracts, Error Model, and Idempotency

**Status.** Implemented (v0.3.0)
**Phase:** M2/M5 / Boundary
**Project:** ciao.zinnias
**Date:** 2026-06-11
**Reconciled:** AD-1 — the member boundary is HTML + form posts, not a JSON API. Re-scoped from "frontend/backend API" to internal handler contracts + the small real JSON surface.

---

## 1. Summary

Under SSR (AD-1) there is no JSON API consumed by a client for the member UI; the boundary is server-rendered HTML and `<form method="post">`. This RFC therefore defines: the internal handler/view-model contracts (in `packages/contracts`), the shared error model, the form-token idempotency contract (AD-4), and the only genuine JSON endpoints — `/healthz`, `/version`, the ICS feed (RFC-023), and any future hydrated-island endpoint.

## 2. Goals

- Stable view-model and DTO types in `packages/contracts`.
- One error model (internal reason vs plain external message).
- Idempotency via the server-issued form token (no client mutation_id).
- Capability flags in view models so render code never guesses permissions.

## 3. Non-Goals

- No public third-party API, GraphQL, WebSocket/SSE, or generated SDK in MVP. No JSON envelope for SSR page rendering.

## 4. External Behavior

Members interact with HTML pages and forms; errors appear as plain in-page messages (RFC-012), not JSON. The few JSON endpoints return stable shapes for machine consumers (health checks, calendar clients).

## 5. Internal Design

- **View models / DTOs** live in `packages/contracts`, shared by `packages/domain` and the SSR worker; they carry server-computed `capabilities` with a `reason_if_disabled`, so the renderer shows disabled+explained controls without inferring from role strings.
- **Idempotency = form token** (AD-4): issued per render, single-use, bound to `(user_id, purpose, resource)`. A replay returns the prior result or a benign no-op. There is no client-generated mutation ID and no `client_mutations` table.
- **Small JSON surface**: `GET /healthz`, `GET /version`, the ICS feed (RFC-023, token-authenticated), and — only if a hydrated island is later added (AD-1 escape valve) — a small endpoint that accepts the same form token in a header and returns a fragment/JSON.

Error model (shared):

```rust
struct AppError { internal_code, external_code, user_message: &'static str, retryable: bool }
```

## 6. Data and API Design

Route groups: server-rendered `/`, `/join/*`, `/c/:cid/*`, `/c/:cid/admin/*`; JSON only at `/healthz`, `/version`, `/ics/*`. Mutating routes require a valid form token. Stable error codes are reused across handlers for logging/tests.

## 7. Security, Privacy, and Safety

- External messages never disclose private resource existence; internal/SQL/platform detail is redacted.
- Form tokens are scoped per user/session so one user cannot replay another's action.
- DTOs exclude session secrets, invite/token HMACs, and audit-private fields.

## 8. Acceptance Criteria

- Handlers use the shared error model; no raw backend error reaches a page.
- A replayed form token is idempotent.
- Render code shows disabled controls purely from `capabilities`.
- JSON endpoints return documented stable shapes.

## 9. Test Plan

- Contract tests for key view models/DTOs.
- Error-model snapshot tests.
- Form-token idempotency/replay tests.
- Capability-flag tests for member/admin/removed member.
- JSON-endpoint shape tests (health/version/ICS).

## 10. Open Questions / Decisions

Decision: no public API promise in MVP. The SSR boundary is HTML+forms; JSON is reserved for health/version/ICS and future islands, all using the form-token contract.
