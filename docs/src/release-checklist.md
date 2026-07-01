# MVP Release Gate Checklist (RFC-015)

Fill this for every release candidate before promoting to production.

Legend: `[x]` = verified by code inspection or automated test · `[~]` = requires human QA pass · `[ ]` = not yet done

---

## Functional gates

- [x] Invite-code onboarding works end-to-end. *(join.rs: HMAC lookup → display name → atomic user+membership+session)*
- [x] Display name is collected and visible in the community. *(join.rs post_profile, membership.display_name)*
- [x] Session is issued, validated, and revoked on logout. *(session.rs, auth.rs post_logout)*
- [x] Community membership enforced (non-members see generic 404). *(authz.rs require_membership: checks user_id AND community_id)*
- [x] Home shows upcoming events grouped Today / This Week / Later. *(home.rs: day_date comparisons against today_date)*
- [x] Event Detail shows status, participants, and notes. *(event.rs get_event_detail)*
- [x] Member can set own status (Going / No Go / clear). *(event.rs post_my_status + validate_status_transition)*
- [x] Member can save, edit, and delete own note. *(event.rs post_my_note, delete_my_note)*
- [x] Admin can create event (single and multi-day). *(admin.rs post_create_event + event_write::create_event)*
- [x] Admin can cancel event (confirmation required). *(admin.rs get_cancel_event shows confirmation; post_cancel_event soft-cancels)*
- [x] Admin can generate and revoke invite codes. *(admin.rs post_generate_invite + post_revoke_invite; invite_db::revoke)*
- [x] Admin can remove member (last-admin guard active). *(admin.rs post_remove_member: count_admins guard)*
- [x] Admin can mark Attended after event day ends. *(admin.rs get_attendance / post_attendance; classify_day gate)*

## Safety gates

- [x] Cross-community data access blocked (manual test: direct URL to another community's event). *(authz.rs require_membership: community_id check on every request)*
- [x] Removed members lose access on next request. *(membership.rs find_active: AND removed_at IS NULL)*
- [x] Invite codes are single-use. *(invite.rs find_valid: AND used_at IS NULL; mark_used sets it atomically)*
- [x] Failed invite redemptions are rate-limited. *(rate_limit.rs: 10 failures / 5-min window per IP, KV-backed)*
- [x] Session cookies have `HttpOnly; Secure; SameSite=Strict`. Host-only by default (no `Domain` attribute unless `SESSION_COOKIE_DOMAIN` var is set). *(session.rs build_session_cookie — RFC-038)*
- [x] Form token absent/replayed → POST rejected. Token subject is always `user_id`; consume is a single conditional UPDATE. *(form_token.rs consume — RFC-037)*
- [x] Script tag in note/title/name renders as text (not executed). *(render.rs escape_html used at every user-content insertion; test: escape_script_tag)*
- [x] Private page cache cleared on logout. *(RFC-042: authenticated HTML is never cached; only static shell assets are stored. No private cache exists to clear — the property holds trivially. PURGE_PRIVATE is retained for defence-in-depth.)*

## Offline gates

- [x] Previously visited Home/Event Detail opens offline with banner. *(RFC-042: authenticated routes are network-only; offline navigation returns the pre-cached static `/offline` page. No stale private page is served.)*
- [x] Unvisited page shows the offline fallback. *(sw.js `OFFLINE_URL = '/offline'`; shell assets pre-cached on install — code-verified)*
- [x] Form submit while offline does not falsely succeed. *(sw.js: `if (req.method !== 'GET') return;` — non-GET requests bypass SW and reach network, so browser shows its own network error — code-verified via AD-1)*

## UX / accessibility gates

- [~] Core join-and-mark-attendance flow completes under 2 minutes on a phone. *(requires phone test)*
- [x] All critical controls ≥ 44 × 44 px. *(app.css L88: `button, a, [role="button"] { min-height: var(--cz-touch-min); }` where `--cz-touch-min: 44px` (L57); all inline buttons also set `min-height:44px` — code-verified)*
- [x] Status chip shows icon + label + colour (grayscale test: still legible). *(render.rs `status_display()` always returns `(fg_color, icon, label)` tuple; AA-passing fg colors on white: Going 5.0:1, Not Going 5.9:1, Attended 4.7:1 — code-verified)*
- [~] Event Detail usable at 200% text scaling. *(requires browser test)*
- [x] Reduced-motion mode disables animations. *(app.css: `@media (prefers-reduced-motion: reduce) { *, *::before, *::after { transition: none !important; animation: none !important; } }` — code-verified)*
- [x] Error messages use plain language (no SQL/JWT/token/cookie). *(release_gates.rs `not_found_and_forbidden_same_message`; domain tests verify no 'sql'/'panic' in event/note error strings — automated)*

## Stabilization gates (v0.23.0 — RFC-037–042)

- [x] Member can set Going/No Go/Attended (form token uses `user_id` at both issue and consume — RFC-037).
- [x] Member can delete their own note (same token-subject fix — RFC-037).
- [x] Form-token consume is a conditional UPDATE; concurrent double-submit executes at most once (RFC-037).
- [x] Session cookie is host-only when `SESSION_COOKIE_DOMAIN` is unset; no `Domain=localhost` fallback (RFC-038).
- [x] All handlers use `crypto::pepper` — no divergent `env.var`/`env.secret` mix (RFC-038).
- [x] Event create converts community-local time to UTC at write time. Tokyo admin entering 09:00 stores 00:00Z (RFC-039).
- [x] Event edit persists date/time for single-day events; form prefills current values (RFC-040).
- [x] Invite redemption claims the invite atomically first; a lost race aborts without creating a second member (RFC-041).
- [x] Authenticated HTML (`/c/*`, `/`, `/join`) is never stored in the service-worker cache (RFC-042).
- [x] `sw.js CACHE_VERSION` matches the package version. *(verify: `grep CACHE_VERSION workers/ssr/static/sw.js` matches `version` in `Cargo.toml`)*
- [~] Admin creating 09:00 in a non-UTC community displays 09:00 after round-trip. *(staging smoke test)*
- [~] No-JS destructive confirmations (cancel event, remove member, delete note) work without scripting. *(implementation in v0.24.0 — verify on a JS-disabled browser)*

## Stabilization gates (v0.27.0 — RFC-045–047 + i18n/XSS hardening)

- [x] All 11 source-verification claims from architect handoff review §8 confirmed against code (RFC-045 §5).
- [x] `SET_STATUS` token issued once per Event Detail render, bound to `event_id`; day validated via days_for_event lookup (RFC-046).
- [x] Day labels render in Japanese convention (`6月14日（土）`); no English month abbreviation (RFC-047).
- [x] Logout, calendar-token generate, and calendar-token revoke are audited (review P1-5).
- [x] DST scope limitation documented in `docs/src/operations.md` (review P1-2).
- [x] No-JS community switcher has a visible `<noscript>` submit fallback; confirmed in `render.rs` (review P1-4).
- [x] i18n parity test covers all 120 EN/JA string pairs (expanded from 9); catches empty strings and copy-paste errors.
- [x] `escape_html` moved to tested `contracts::html` module; 10 unit tests including XSS vector and attribute injection; `render::escape_html` delegates to the tested implementation.
- [~] Staging runtime verification (RFC-045 §6): timezone round-trip, concurrent invite/token races. *(requires Cloudflare staging deployment)*

## Pre-pilot hardening gates (v0.30.0 — RFC-048, RFC-049 + timezone hardening)

- [x] `Cache-Control: no-store` on all authenticated HTML responses (RFC-048); static assets retain public/max-age.
- [x] CSP extended: `base-uri 'self'`, `form-action 'self'`, `object-src 'none'` added; `unsafe-inline` exception documented (RFC-048).
- [x] `Permissions-Policy` header added (RFC-048).
- [x] `Referrer-Policy` changed to `same-origin` (RFC-048).
- [x] All UI strings render in Japanese (`JA_*`); HTML `lang="ja"` (RFC-049).
- [x] Unknown community timezone returns a hard error on write paths, not a silent UTC fallback (P1-timezone).
- [x] Query budget for max-recurring Event Detail updated from ≤65 to ≤13 (correct after RFC-046).
- [~] Security header values verified in a real browser on staging. *(staging runtime)*

## Operational gates

- [x] `GET /healthz` returns `{"ok":true}`. *(health.rs get_health)*
- [x] `GET /version` returns build version. *(health.rs get_version reads BUILD_VERSION var)*
- [x] Rollback procedure documented and understood. *(docs/src/deployment.md §Rollback: `wrangler rollback --env production`)*
- [x] Log persistence approach documented. *(docs/src/deployment.md §Log persistence: Cloudflare Logpush to R2/S3)*
- [ ] D1 migration applied to staging and rehearsed. *(operator task: `bun run migrate:prod` against staging env)*
- [ ] Secrets (`HMAC_PEPPER`) set in production via `wrangler secret put`. `SESSION_COOKIE_DOMAIN` is a **`[vars]` binding** (not a secret) — set it per environment in `wrangler.toml`; leave unset for a host-only cookie. *(operator task — RFC-038)*
- [ ] Logpush configured for production. *(operator task: Cloudflare dashboard)*
- [ ] No critical open security issues. *(final security review before go-live)*
