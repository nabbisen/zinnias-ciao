# ciao.zinnias — Architecture Decisions (ADR)

**Status.** Accepted
**Date:** 2026-06-11
**Applies to:** all RFCs in this pack; supersedes conflicting assumptions in `requirements-v1` and `external-design-v1` (amendments listed in §5).

These four decisions were settled before the RFC reconciliation. RFCs reference this file rather than re-litigating them.

---

## AD-1 — Rendering: SSR + progressive enhancement (no Leptos hydration)

The frontend is **server-side-rendered HTML with progressive enhancement**. State changes are `<form method="post">` → 303 redirect → re-render. Forms work with no client JavaScript. Small, optional plain-JS enhancements (character counter, disable-on-submit, in-place status toggle) may be layered where they help, but the page is always fully functional without them.

- No browser WASM bundle for the app. No Leptos `hydrate`. No client-side reactive state as a source of truth. (`spawn_local`, `LocalResource`, reactive signals, `ActionForm`/`ServerAction` are no-ops/panics in this model — do not use them in Worker render paths.)
- A hydrated **island** (selective hydration) is permitted only where one interaction measurably earns it; added case-by-case, never as the default. This is the escape valve, not the baseline.
- **The server is authoritative for every write.** Integrity and authorization never depend on client state.
- Offline **writes** are out of MVP. Offline is read-only at most (static-shell caching + an honest "you are offline" fallback). There is no client mutation queue.

## AD-2 — Identity: invite-code + cookie session now; OIDC deferred (model kept extensible)

MVP identity is **invite-code redemption + a server-side cookie session**. No mandatory external account, lowest friction for non-technical members.

- **OIDC is the intended future**, deferred out of MVP. The data model reserves space for it (`users.idp_subject` nullable; room for a `user_identities` table) so it can be added later without a painful migration. It must never become a hard wall for members who do not have an IdP account.
- Invite code = community-join authorization, redeemed once, short-lived, rate-limited.
- Lost-session recovery in the invite-only era is admin-mediated (re-invite, or the optional relink path in RFC-024). A stable IdP subject will make this self-healing once OIDC lands.

## AD-3 — CPU budget: design to Workers Free (10 ms CPU/invocation)

Design to the Workers **Free** tier: 10 ms of **CPU** time per invocation. CPU time counts active execution only — D1 queries, `fetch`, and other I/O do **not** count (they are latency/subrequest concerns; Free allows 50 external + 1000 Cloudflare-service subrequests). Paid Standard relaxes this to a 30 s default, so the design is portable upward.

Consequences:
- **No slow KDFs in the request path.** argon2/bcrypt/scrypt are designed to burn CPU and would blow 10 ms. (This also reinforces "no passwords.")
- Secrets are hashed with **HMAC-SHA256 under a server-held pepper** (Workers secret binding): microsecond CPU, and a database leak alone cannot brute-force low-entropy invite codes without the pepper.
- Keep SSR pages lean; paginate large lists; avoid heavy serialization. D1 round-trip count is bounded for latency, not CPU.

## AD-4 — One mechanism for CSRF + idempotency: the server-issued form token

Because there is no client to generate a `mutation_id` (AD-1), every state-changing form embeds a **server-issued single-use token** rendered as a hidden field.

- Issued per render, bound to `(user_id, purpose, optional resource)`, short expiry, stored as an HMAC (cheap).
- On POST: validate the token belongs to the session → **CSRF protection**; mark consumed on first success → **idempotency** (a replayed/double-submitted token returns the prior result or a benign "already done", never a duplicate write).
- Replaces the client-generated `mutation_id` and the `client_mutations` table from the original pack with a `form_tokens` table (RFC-002).
- For any future hydrated island that calls a small JSON endpoint, the same token travels in a header; the contract is in RFC-013.

---

## 5. Baseline amendments these decisions imply

The RFCs are reconciled to the above; the product baseline must catch up to match (tracked, not yet applied here):

- `requirements` §12.1 (frontend = WASM) → SSR + progressive enhancement.
- `requirements` NG4 — keep "no OAuth **in MVP**", but record OIDC as planned/deferred rather than permanently out.
- `external-design` §7.2 (per-action sync states / optimistic UI), §12 (offline mutation queue), §13 (JSON API as the FE/BE boundary) → the boundary is HTML + form posts; JSON survives only for `/healthz`, `/version`, ICS feed, and any future island endpoint.

## 6. Confirmed grain — Event → EventDay → Attendance

Confirmed. An event spans one or more dated days; attendance status is per `event_day`; the single ≤200-char note is per event (`event_notes`). One-day events are the common case (one `event_days` row); multi-day is native. Finalized in RFC-002; propagated through RFC-005/006/009/018/019. This requires the matching `requirements`/`external-design` amendment (Event → EventDay → Attendance), tracked in §5.
